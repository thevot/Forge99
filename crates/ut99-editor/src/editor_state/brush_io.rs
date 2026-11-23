use super::*;
use glam::Vec3;

impl EditorState {
    pub fn export_builder_brush_to_t3d(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Begin Brush Name={}\n", self.builder_brush.name));
        out.push_str("   Begin PolyList\n");
        for poly in &self.builder_brush.polys {
            let tex = if poly.texture.is_empty() { "DefaultTexture" } else { &poly.texture };
            if poly.flags != 0 {
                out.push_str(&format!("      Begin Polygon Texture={} Flags={}\n", tex, poly.flags));
            } else {
                out.push_str(&format!("      Begin Polygon Texture={}\n", tex));
            }
            out.push_str(&format!(
                "         Origin   {:+.6},{:+.6},{:+.6}\n",
                poly.base.x, poly.base.y, poly.base.z
            ));
            out.push_str(&format!(
                "         Normal   {:+.6},{:+.6},{:+.6}\n",
                poly.plane.normal.x, poly.plane.normal.y, poly.plane.normal.z
            ));
            out.push_str(&format!(
                "         TextureU {:+.6},{:+.6},{:+.6}\n",
                poly.texture_u.x, poly.texture_u.y, poly.texture_u.z
            ));
            out.push_str(&format!(
                "         TextureV {:+.6},{:+.6},{:+.6}\n",
                poly.texture_v.x, poly.texture_v.y, poly.texture_v.z
            ));
            for v in &poly.vertices {
                out.push_str(&format!(
                    "         Vertex   {:+.6},{:+.6},{:+.6}\n",
                    v.x, v.y, v.z
                ));
            }
            out.push_str("      End Polygon\n");
        }
        out.push_str("   End PolyList\n");
        out.push_str("End Brush\n");
        out
    }

    /// Imports an UnrealEd 2.0 `.t3d` `Begin PolyList` / `Begin Brush` text representation into the Red Builder Brush with Undo/Redo support.
    pub fn import_t3d_to_builder_brush(&mut self, t3d_text: &str) -> Result<usize, String> {
        fn parse_t3d_xyz(token: &str) -> Option<Vec3> {
            let cleaned = token.trim();
            let parts: Vec<&str> = cleaned.split(',').collect();
            if parts.len() >= 3 {
                let x = parts[0].trim().parse::<f32>().ok()?;
                let y = parts[1].trim().parse::<f32>().ok()?;
                let z = parts[2].trim().parse::<f32>().ok()?;
                return Some(Vec3::new(x, y, z));
            }
            None
        }

        let mut polys = Vec::new();
        let mut in_poly = false;
        let mut cur_tex = String::from("DefaultTexture");
        let mut cur_flags = 0u32;
        let mut cur_origin = None;
        let mut cur_u = None;
        let mut cur_v = None;
        let mut cur_verts: Vec<Vec3> = Vec::new();

        for raw_line in t3d_text.lines() {
            let line = raw_line.trim();
            if line.to_ascii_lowercase().starts_with("begin polygon") {
                in_poly = true;
                cur_tex = self.active_texture.clone().unwrap_or_else(|| "DefaultTexture".to_string());
                cur_flags = 0;
                cur_origin = None;
                cur_u = None;
                cur_v = None;
                cur_verts.clear();
                for part in line.split_whitespace() {
                    if let Some(rest) = part.strip_prefix("Texture=").or_else(|| part.strip_prefix("texture=")) {
                        cur_tex = rest.trim_matches('"').to_string();
                    } else if let Some(rest) = part.strip_prefix("Flags=").or_else(|| part.strip_prefix("flags=")) {
                        if let Ok(f) = rest.parse::<u32>() {
                            cur_flags = f;
                        }
                    }
                }
            } else if line.to_ascii_lowercase().starts_with("end polygon") {
                if in_poly && cur_verts.len() >= 3 {
                    if let Some(mut poly) = ut99_bsp::Polygon::new(cur_verts.clone()) {
                        poly.texture = cur_tex.clone();
                        poly.flags = cur_flags;
                        if let Some(o) = cur_origin {
                            poly.base = o;
                        }
                        if let Some(u) = cur_u {
                            poly.texture_u = u;
                        }
                        if let Some(v) = cur_v {
                            poly.texture_v = v;
                        }
                        polys.push(poly);
                    }
                }
                in_poly = false;
            } else if in_poly {
                let mut words = line.split_whitespace();
                if let Some(key) = words.next() {
                    let rest: String = words.collect::<Vec<_>>().join("");
                    match key.to_ascii_lowercase().as_str() {
                        "vertex" => {
                            if let Some(v) = parse_t3d_xyz(&rest) {
                                cur_verts.push(v);
                            }
                        }
                        "origin" => cur_origin = parse_t3d_xyz(&rest),
                        "textureu" => cur_u = parse_t3d_xyz(&rest),
                        "texturev" => cur_v = parse_t3d_xyz(&rest),
                        _ => {}
                    }
                }
            }
        }

        if polys.is_empty() {
            return Err("No valid polygons found in T3D brush text.".to_string());
        }

        let old_brush = self.builder_brush.clone();
        self.builder_brush.polys = polys;
        let poly_count = self.builder_brush.polys.len();
        let new_brush = self.builder_brush.clone();
        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush,
            description: format!("Import T3D Brush ({} polys)", poly_count),
        });
        self.log(format!("Imported T3D Brush into Red Builder Brush ({} polygons).", poly_count));
        self.toast_success(format!("Imported T3D Brush ({} polys)", poly_count));
        Ok(poly_count)
    }

    /// Exports the current Red Builder Brush to AutoCAD `.dxf` (`3DFACE` entities inside `ENTITIES` section),
    /// compatible with UnrealEd 2.0 `Brush -> Export... (.dxf)`.
    pub fn export_builder_brush_to_dxf(&self) -> String {
        let mut dxf = String::new();
        dxf.push_str("0\nSECTION\n2\nENTITIES\n");
        for poly in &self.builder_brush.polys {
            if poly.vertices.len() < 3 {
                continue;
            }
            // Triangulate or emit 4-vertex 3DFACE records
            let v0 = poly.vertices[0];
            let mut idx = 1;
            while idx + 1 < poly.vertices.len() {
                let v1 = poly.vertices[idx];
                let v2 = poly.vertices[idx + 1];
                let v3 = if idx + 2 < poly.vertices.len() {
                    idx += 2;
                    poly.vertices[idx]
                } else {
                    idx += 1;
                    v2
                };
                dxf.push_str("0\n3DFACE\n8\n0\n");
                // Vertex 1 (10, 20, 30)
                dxf.push_str(&format!("10\n{:.6}\n20\n{:.6}\n30\n{:.6}\n", v0.x, v0.y, v0.z));
                // Vertex 2 (11, 21, 31)
                dxf.push_str(&format!("11\n{:.6}\n21\n{:.6}\n31\n{:.6}\n", v1.x, v1.y, v1.z));
                // Vertex 3 (12, 22, 32)
                dxf.push_str(&format!("12\n{:.6}\n22\n{:.6}\n32\n{:.6}\n", v2.x, v2.y, v2.z));
                // Vertex 4 (13, 23, 33)
                dxf.push_str(&format!("13\n{:.6}\n23\n{:.6}\n33\n{:.6}\n", v3.x, v3.y, v3.z));
            }
        }
        dxf.push_str("0\nENDSEC\n0\nEOF\n");
        dxf
    }

    /// Imports an AutoCAD `.dxf` (`3DFACE` entities) into the Red Builder Brush with Undo/Redo support,
    /// compatible with UnrealEd 2.0 `Brush -> Import... (.dxf)`.
    pub fn import_dxf_to_builder_brush(&mut self, dxf_text: &str) -> Result<usize, String> {
        let lines: Vec<&str> = dxf_text.lines().map(|l| l.trim()).collect();
        let mut polys = Vec::new();
        let default_tex = self
            .active_texture
            .clone()
            .unwrap_or_else(|| "DefaultTexture".to_string());

        let mut i = 0;
        let mut in_3dface = false;
        let mut coords = [[0.0f32; 3]; 4];
        let mut has_v3 = false;

        let flush_face = |coords: &[[f32; 3]; 4],
                          has_v3: bool,
                          polys: &mut Vec<ut99_bsp::Polygon>,
                          tex: &str| {
            let v0 = Vec3::from(coords[0]);
            let v1 = Vec3::from(coords[1]);
            let v2 = Vec3::from(coords[2]);
            let v3 = Vec3::from(coords[3]);
            let mut verts = vec![v0, v1, v2];
            if has_v3 && v3.distance(v2) > 1e-3 && v3.distance(v0) > 1e-3 {
                verts.push(v3);
            }
            if let Some(mut poly) = ut99_bsp::Polygon::new(verts) {
                poly.texture = tex.to_string();
                polys.push(poly);
            }
        };

        while i + 1 < lines.len() {
            let code = lines[i];
            let val = lines[i + 1];
            i += 2;

            if code == "0" {
                if in_3dface {
                    flush_face(&coords, has_v3, &mut polys, &default_tex);
                }
                if val.eq_ignore_ascii_case("3DFACE") {
                    in_3dface = true;
                    coords = [[0.0; 3]; 4];
                    has_v3 = false;
                } else {
                    in_3dface = false;
                }
            } else if in_3dface {
                if let Ok(num) = val.parse::<f32>() {
                    match code {
                        "10" => coords[0][0] = num,
                        "20" => coords[0][1] = num,
                        "30" => coords[0][2] = num,
                        "11" => coords[1][0] = num,
                        "21" => coords[1][1] = num,
                        "31" => coords[1][2] = num,
                        "12" => coords[2][0] = num,
                        "22" => coords[2][1] = num,
                        "32" => coords[2][2] = num,
                        "13" => {
                            coords[3][0] = num;
                            has_v3 = true;
                        }
                        "23" => {
                            coords[3][1] = num;
                            has_v3 = true;
                        }
                        "33" => {
                            coords[3][2] = num;
                            has_v3 = true;
                        }
                        _ => {}
                    }
                }
            }
        }
        if in_3dface {
            flush_face(&coords, has_v3, &mut polys, &default_tex);
        }

        if polys.is_empty() {
            return Err("No valid 3DFACE entities found in DXF file.".to_string());
        }

        let old_brush = self.builder_brush.clone();
        self.builder_brush.polys = polys;
        let poly_count = self.builder_brush.polys.len();
        let new_brush = self.builder_brush.clone();
        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush,
            description: format!("Import DXF Brush ({} faces)", poly_count),
        });
        self.log(format!("Imported DXF 3DFACE Brush into Red Builder Brush ({} faces).", poly_count));
        self.toast_success(format!("Imported DXF Brush ({} faces)", poly_count));
        Ok(poly_count)
    }

    /// Exports the current Red Builder Brush to 3D Studio `.asc` (`Tri-mesh, Vertices: N Faces: M` format),
    /// compatible with UnrealEd 2.0 `Brush -> Import/Export... (.asc)`.
    pub fn export_builder_brush_to_asc(&self) -> String {
        let mut verts: Vec<Vec3> = Vec::new();
        let mut faces: Vec<[usize; 3]> = Vec::new();

        for poly in &self.builder_brush.polys {
            if poly.vertices.len() < 3 {
                continue;
            }
            let base_idx = verts.len();
            for &v in &poly.vertices {
                verts.push(v);
            }
            for k in 1..(poly.vertices.len() - 1) {
                faces.push([base_idx, base_idx + k, base_idx + k + 1]);
            }
        }

        let mut asc = String::new();
        asc.push_str(&format!("Named object: \"{}\"\n", self.builder_brush.name));
        asc.push_str(&format!(
            "Tri-mesh, Vertices: {}     Faces: {}\n",
            verts.len(),
            faces.len()
        ));
        asc.push_str("Vertex list:\n");
        for (idx, v) in verts.iter().enumerate() {
            asc.push_str(&format!(
                "Vertex {}:  X:{:.6}     Y:{:.6}     Z:{:.6}\n",
                idx, v.x, v.y, v.z
            ));
        }
        asc.push_str("Face list:\n");
        for (idx, f) in faces.iter().enumerate() {
            asc.push_str(&format!(
                "Face {}:    A:{} B:{} C:{} AB:1 BC:1 CA:1\n",
                idx, f[0], f[1], f[2]
            ));
        }
        asc
    }

    /// Imports a 3D Studio `.asc` (`Tri-mesh` vertex & face list) into the Red Builder Brush with Undo/Redo support,
    /// compatible with UnrealEd 2.0 `Brush -> Import... (.asc)`.
    pub fn import_asc_to_builder_brush(&mut self, asc_text: &str) -> Result<usize, String> {
        fn extract_prefixed_num<T: std::str::FromStr>(line: &str, prefix: &str) -> Option<T> {
            let lower = line.to_ascii_lowercase();
            let p_lower = prefix.to_ascii_lowercase();
            let pos = lower.find(&p_lower)?;
            let after = &line[(pos + prefix.len())..];
            let token: String = after
                .trim_start()
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != ',' && *c != ';')
                .collect();
            token.parse::<T>().ok()
        }

        let mut verts: Vec<Vec3> = Vec::new();
        let mut polys: Vec<ut99_bsp::Polygon> = Vec::new();
        let default_tex = self
            .active_texture
            .clone()
            .unwrap_or_else(|| "DefaultTexture".to_string());

        for raw_line in asc_text.lines() {
            let line = raw_line.trim();
            let lower = line.to_ascii_lowercase();
            if lower.starts_with("vertex ") && lower.contains("x:") && lower.contains("y:") && lower.contains("z:") {
                if let (Some(x), Some(y), Some(z)) = (
                    extract_prefixed_num::<f32>(line, "x:"),
                    extract_prefixed_num::<f32>(line, "y:"),
                    extract_prefixed_num::<f32>(line, "z:"),
                ) {
                    verts.push(Vec3::new(x, y, z));
                }
            } else if lower.starts_with("face ") && lower.contains("a:") && lower.contains("b:") && lower.contains("c:") {
                if let (Some(a), Some(b), Some(c)) = (
                    extract_prefixed_num::<usize>(line, "a:"),
                    extract_prefixed_num::<usize>(line, "b:"),
                    extract_prefixed_num::<usize>(line, "c:"),
                ) {
                    if a < verts.len() && b < verts.len() && c < verts.len() {
                        if let Some(mut poly) = ut99_bsp::Polygon::new(vec![verts[a], verts[b], verts[c]]) {
                            poly.texture = default_tex.clone();
                            polys.push(poly);
                        }
                    }
                }
            }
        }

        if polys.is_empty() {
            return Err("No valid triangular faces found in 3D Studio .asc file.".to_string());
        }

        let old_brush = self.builder_brush.clone();
        self.builder_brush.polys = polys;
        let poly_count = self.builder_brush.polys.len();
        let new_brush = self.builder_brush.clone();
        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush,
            description: format!("Import ASC Brush ({} faces)", poly_count),
        });
        self.log(format!("Imported 3D Studio .asc Brush into Red Builder Brush ({} faces).", poly_count));
        self.toast_success(format!("Imported .asc Brush ({} faces)", poly_count));
        Ok(poly_count)
    }

}
