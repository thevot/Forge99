use super::*;
use glam::Vec3;

impl EditorState {
    /// Executes a classic UnrealEd 2.0 Console Command (`POLY SELECT ...`, `BRUSH ...`, `ACTOR ...`, `MAP REBUILD`, `LIGHT APPLY`, `PATHS DEFINE`, `RMODE 1..6`).
    pub fn execute_unrealed_command(&mut self, cmd: &str) -> String {
        let raw = cmd.trim();
        if raw.is_empty() {
            return String::new();
        }
        let upper = raw.to_ascii_uppercase();
        self.log(format!("> {}", raw));

        let response = match upper.as_str() {
            "POLY SELECT ALL" => {
                self.select_all_surfaces();
                format!("POLY SELECT ALL: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT NONE" => {
                self.deselect_all_surfaces();
                "POLY SELECT NONE: 0 surfaces selected.".to_string()
            }
            "POLY SELECT REVERSE" => {
                self.invert_surface_selection();
                format!("POLY SELECT REVERSE: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MATCHING BRUSH" => {
                self.select_surfaces_on_selected_brushes();
                format!("POLY SELECT MATCHING BRUSH: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MATCHING TEXTURE" => {
                self.select_surfaces_matching_texture();
                format!("POLY SELECT MATCHING TEXTURE: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MATCHING GROUPS" => {
                self.select_surfaces_matching_groups();
                format!("POLY SELECT MATCHING GROUPS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MATCHING ITEMS" => {
                self.select_surfaces_matching_items();
                format!("POLY SELECT MATCHING ITEMS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT ZONE" | "POLY SELECT MATCHING ZONE" => {
                let cnt = self.select_surfaces_matching_zone();
                format!("POLY SELECT ZONE: {} surface(s) selected.", cnt)
            }
            "POLY SELECT ADJACENT ALL" => {
                self.select_surfaces_adjacent_all();
                format!("POLY SELECT ADJACENT ALL: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT ADJACENT COPLANARS" => {
                self.select_surfaces_coplanar_adjacent();
                format!("POLY SELECT ADJACENT COPLANARS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT ADJACENT WALLS" => {
                self.select_surfaces_adjacent_walls();
                format!("POLY SELECT ADJACENT WALLS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT ADJACENT FLOORS" | "POLY SELECT ADJACENT CEILINGS" => {
                self.select_surfaces_adjacent_floors();
                format!("POLY SELECT ADJACENT FLOORS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT ADJACENT SLANTS" => {
                self.select_surfaces_adjacent_slants();
                format!("POLY SELECT ADJACENT SLANTS: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MEMORY SET" | "POLY MEMORIZE" => {
                self.memorize_surface_selection();
                format!("POLY MEMORIZE: {} surface(s) memorized.", self.memorized_surfaces.len())
            }
            "POLY SELECT MEMORY RECALL" | "POLY RECALL" => {
                self.recall_surface_selection();
                format!("POLY RECALL: {} surface(s) recalled.", self.selected_surfaces.len())
            }
            "POLY SELECT MEMORY UNION" | "POLY OR" => {
                self.or_surface_selection();
                format!("POLY OR: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MEMORY INTERSECT" | "POLY AND" => {
                self.and_surface_selection();
                format!("POLY AND: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "POLY SELECT MEMORY XOR" | "POLY XOR" => {
                self.xor_surface_selection();
                format!("POLY XOR: {} surface(s) selected.", self.selected_surfaces.len())
            }
            "BRUSH ADD" => {
                self.csg_add();
                "BRUSH ADD executed.".to_string()
            }
            "BRUSH SUBTRACT" => {
                self.csg_subtract();
                "BRUSH SUBTRACT executed.".to_string()
            }
            "BRUSH FROM INTERSECTION" => {
                self.csg_intersect();
                "BRUSH FROM INTERSECTION executed.".to_string()
            }
            "BRUSH FROM DEINTERSECTION" => {
                self.csg_deintersect();
                "BRUSH FROM DEINTERSECTION executed.".to_string()
            }
            "ACTOR SELECT ALL" | "SELECT ALL" => {
                self.select_all();
                "ACTOR SELECT ALL executed.".to_string()
            }
            "ACTOR SELECT NONE" | "SELECT NONE" => {
                self.clear_multi_selection();
                self.selected_brush_index = None;
                self.selected_actor_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
                "ACTOR SELECT NONE executed.".to_string()
            }
            "ACTOR SELECT INSIDE" | "SELECT INSIDE" => {
                let (b_cnt, a_cnt) = self.select_inside_builder_brush();
                format!("ACTOR SELECT INSIDE: {} brush(es), {} actor(s) selected.", b_cnt, a_cnt)
            }
            "POLY MERGE" | "BRUSH MERGEPOLYS" => {
                let merged = self.merge_selected_brush_polygons();
                format!("POLY MERGE: merged {} coplanar polygon pair(s).", merged)
            }
            "POLY SEPARATE" | "BRUSH SEPARATEPOLYS" => {
                let sep = self.separate_selected_brush_polygons();
                format!("POLY SEPARATE: separated {} polygon(s).", sep)
            }
            "BRUSH TO" => {
                self.polygons_to_brush();
                "BRUSH TO (Polygons -> To Brush) executed.".to_string()
            }
            "BRUSH FROM" => {
                self.polygons_from_brush();
                "BRUSH FROM (Polygons -> From Brush) executed.".to_string()
            }
            "DELETE" | "ACTOR DELETE" => {
                self.delete_selected_batch();
                "DELETE executed.".to_string()
            }
            "DUPLICATE" | "ACTOR DUPLICATE" => {
                self.duplicate_selected();
                "ACTOR DUPLICATE executed.".to_string()
            }
            "LIGHT APPLY" => {
                self.build_lighting();
                "LIGHT APPLY executed.".to_string()
            }
            "LIGHT APPLY SELECTED" => {
                let cnt = self.build_lighting_for_selected_surfaces();
                format!("LIGHT APPLY SELECTED: baked lighting on {} selected surface(s).", cnt)
            }
            "PATHS DEFINE" | "PATHS BUILD" | "PATHS BUILD LOWOPT" | "PATHS BUILD HIGHOPT" => {
                self.build_ai_paths();
                format!("{} executed.", upper)
            }
            "PATHS UNDEFINE" | "PATHS CLEAR" | "PATHS REMOVE" => {
                let removed = self.clear_ai_paths();
                format!("{} executed ({} path entries cleared; navigation actors retained).", upper, removed)
            }
            "MAP REBUILD" | "BSP REBUILD" => {
                self.build_geometry();
                self.build_lighting();
                self.build_ai_paths();
                "MAP REBUILD executed (BSP + Lighting + AI Paths).".to_string()
            }
            "MAP BRUSH GET" => {
                self.polygons_to_brush();
                "MAP BRUSH GET (Polygons -> To Brush) executed.".to_string()
            }
            "MAP BRUSH PUT" => {
                self.polygons_from_brush();
                "MAP BRUSH PUT (Polygons -> From Brush) executed.".to_string()
            }
            "BRUSH APPLYTRANSFORM" => {
                self.transform_permanently_selected();
                "BRUSH APPLYTRANSFORM executed.".to_string()
            }
            "BRUSH RESET" => {
                self.reset_builder_brush();
                "BRUSH RESET executed.".to_string()
            }
            "BRUSH MIRROR X" => {
                self.mirror_selected(MirrorAxis::X);
                "BRUSH MIRROR X executed.".to_string()
            }
            "BRUSH MIRROR Y" => {
                self.mirror_selected(MirrorAxis::Y);
                "BRUSH MIRROR Y executed.".to_string()
            }
            "BRUSH MIRROR Z" => {
                self.mirror_selected(MirrorAxis::Z);
                "BRUSH MIRROR Z executed.".to_string()
            }
            "BRUSH MOVEFIRST" | "MAP SENDTO FIRST" => {
                self.send_selected_brush_to_first();
                "BRUSH MOVEFIRST executed.".to_string()
            }
            "BRUSH MOVELAST" | "MAP SENDTO LAST" => {
                self.send_selected_brush_to_last();
                "BRUSH MOVELAST executed.".to_string()
            }
            "MODE CAMERAMOVE" => {
                self.editor_mode = EditorMode::CameraSelection;
                "MODE CAMERAMOVE active.".to_string()
            }
            "MODE VERTEXEDIT" => {
                self.editor_mode = EditorMode::VertexEdit;
                "MODE VERTEXEDIT active.".to_string()
            }
            "MODE BRUSHCLIP" => {
                self.editor_mode = EditorMode::BrushClip;
                "MODE BRUSHCLIP active.".to_string()
            }
            "MODE FREEHAND" => {
                self.editor_mode = EditorMode::FreehandPolygon;
                "MODE FREEHAND active.".to_string()
            }
            "CAMERA ALIGN" => {
                let target = self.align_cameras_to_selected(None);
                format!("CAMERA ALIGN: Centered viewports on ({:.0}, {:.0}, {:.0}).", target.x, target.y, target.z)
            }
            "SELECT INVERT" => {
                self.invert_selection();
                "SELECT INVERT executed.".to_string()
            }
            "TEXTURE CULL" => {
                let (tex_c, snd_c) = self.prune_mylevel_assets();
                format!("TEXTURE CULL: Pruned {} texture(s) and {} sound(s) from MyLevel.", tex_c, snd_c)
            }
            "TEXTURE CLEAR" => {
                let cleared = self.clear_selected_surfaces_texture();
                format!("TEXTURE CLEAR: reset {} surface(s) to DefaultTexture.", cleared)
            }
            "TEXTURE APPLYDETAIL" => {
                if let Some(active) = self.active_texture.clone() {
                    let mut meta = self.texture_manager.get_texture_metadata(&active);
                    if meta.detail_texture.is_empty() {
                        meta.detail_texture = "Detail.DefaultDetail".to_string();
                    }
                    let detail = meta.detail_texture.clone();
                    self.texture_manager.set_texture_metadata(&active, meta);
                    format!("TEXTURE APPLYDETAIL: bound detail texture '{}' to '{}'.", detail, active)
                } else {
                    "TEXTURE APPLYDETAIL: no active texture selected.".to_string()
                }
            }
            "TEXTURE BATCHAPPLY" => {
                if !self.selected_surfaces.is_empty() || !self.has_multi_selection() {
                    self.apply_active_texture_to_selected_brush();
                } else {
                    self.apply_active_texture_to_multi_selected();
                }
                format!("TEXTURE BATCHAPPLY: applied '{:?}' to selected surfaces.", self.active_texture)
            }
            "STAT FPS" | "STAT GLOBAL" | "STAT LIGHT" => {
                format!("{} toggled.", upper)
            }
            "RMODE 1" => {
                self.viewport_shading = crate::camera::ViewportShading::Wireframe;
                "RMODE 1 (Wireframe) active.".to_string()
            }
            "RMODE 2" => {
                self.viewport_shading = crate::camera::ViewportShading::ZonePortal;
                "RMODE 2 (Zone/Portal) active.".to_string()
            }
            "RMODE 3" | "RMODE 5" => {
                self.viewport_shading = crate::camera::ViewportShading::Unlit;
                "RMODE 5 (Unlit/Textured) active.".to_string()
            }
            "RMODE 4" | "RMODE 6" => {
                self.viewport_shading = crate::camera::ViewportShading::Lit;
                "RMODE 6 (Dynamic Lighting) active.".to_string()
            }
            _ if upper.starts_with("CAMERA ALIGN ") => {
                let mut name_opt: Option<&str> = None;
                for tok in raw["CAMERA ALIGN ".len()..].split_whitespace() {
                    if tok.to_ascii_uppercase().starts_with("NAME=") {
                        name_opt = Some(tok["NAME=".len()..].trim());
                    }
                }
                let target = self.align_cameras_to_selected(name_opt);
                format!(
                    "CAMERA ALIGN{}: Centered viewports on ({:.0}, {:.0}, {:.0}).",
                    name_opt.map(|n| format!(" NAME={}", n)).unwrap_or_default(),
                    target.x,
                    target.y,
                    target.z
                )
            }
            _ if upper.starts_with("BRUSH ADD ") => {
                let mut flags = 0u32;
                for tok in upper["BRUSH ADD ".len()..].split_whitespace() {
                    if let Some(val_s) = tok.strip_prefix("FLAGS=") {
                        if let Ok(v) = val_s.parse::<u32>() {
                            flags |= v;
                        }
                    }
                }
                self.add_special_brush(flags, false);
                format!("BRUSH ADD FLAGS={:#010X} executed.", flags)
            }
            _ if upper.starts_with("EXEC ") => {
                let file_arg = raw[5..].trim().trim_matches('"');
                match std::fs::read_to_string(file_arg) {
                    Ok(contents) => {
                        let mut executed = 0usize;
                        for line in contents.lines() {
                            let l = line.trim();
                            if l.is_empty() || l.starts_with('#') || l.starts_with("//") || l.starts_with(';') {
                                continue;
                            }
                            if !l.to_ascii_uppercase().starts_with("EXEC ") {
                                let _ = self.execute_unrealed_command(l);
                                executed += 1;
                            }
                        }
                        format!("EXEC '{}': executed {} command(s).", file_arg, executed)
                    }
                    Err(e) => format!("EXEC error reading '{}': {}", file_arg, e),
                }
            }
            _ if upper.starts_with("MAP GRID ") => {
                for tok in upper["MAP GRID ".len()..].split_whitespace() {
                    if let Some(val_str) = tok.strip_prefix("X=").or_else(|| tok.strip_prefix("Y=")).or_else(|| tok.strip_prefix("Z=")) {
                        if let Ok(g) = val_str.parse::<f32>() {
                            self.set_grid_step(g.max(1.0));
                        }
                    }
                }
                format!("MAP GRID: size set to {}.", self.grid_step)
            }
            _ if upper.starts_with("ACTOR ADD CLASS=") => {
                let cls_name = raw["ACTOR ADD CLASS=".len()..].split_whitespace().next().unwrap_or("Actor").trim();
                let loc = self.builder_brush.location;
                if self.loaded_unr.is_none() {
                    self.loaded_unr = Some(ut99_format::UnrLevel::default());
                }
                let mut added_name = String::new();
                if let Some(ref mut unr) = self.loaded_unr {
                    let idx = unr.actors.len();
                    added_name = format!("{}_{}", cls_name, idx);
                    let act = ut99_format::ActorRecord {
                        class_name: cls_name.to_string(),
                        name: added_name.clone(),
                        location: loc,
                        ..Default::default()
                    };
                    unr.actors.push(act.clone());
                    self.selected_actor_index = Some(idx);
                    self.history.push(Action::AddActor { actor: act });
                }
                format!("ACTOR ADD CLASS={}: spawned '{}'.", cls_name, added_name)
            }
            _ if upper.starts_with("ACTOR REPLACE CLASS=") => {
                let cls_name = raw["ACTOR REPLACE CLASS=".len()..].split_whitespace().next().unwrap_or("Actor").trim();
                let replaced = self.replace_selected_actors_with_class(cls_name);
                format!("ACTOR REPLACE CLASS={}: replaced {} actor(s).", cls_name, replaced)
            }
            _ if upper.starts_with("ACTOR MIRROR") => {
                let mut mx = 1.0f32;
                let mut my = 1.0f32;
                let mut mz = 1.0f32;
                for tok in upper["ACTOR MIRROR".len()..].split_whitespace() {
                    if let Some(v) = tok.strip_prefix("X=") {
                        if let Ok(f) = v.parse::<f32>() {
                            mx = f;
                        }
                    } else if let Some(v) = tok.strip_prefix("Y=") {
                        if let Ok(f) = v.parse::<f32>() {
                            my = f;
                        }
                    } else if let Some(v) = tok.strip_prefix("Z=") {
                        if let Ok(f) = v.parse::<f32>() {
                            mz = f;
                        }
                    }
                }
                let mirrored = self.mirror_selected_along_vector(Vec3::new(mx, my, mz));
                format!("ACTOR MIRROR X={} Y={} Z={}: mirrored {} object(s).", mx, my, mz, mirrored)
            }
            _ if upper.starts_with("ACTOR RESET") => {
                let sub = upper["ACTOR RESET".len()..].trim();
                let (r_loc, r_piv, r_rot, r_scale) = match sub {
                    "LOCATION" => (true, false, false, false),
                    "PIVOT" => (false, true, false, false),
                    "ROTATION" => (false, false, true, false),
                    "SCALE" => (false, false, false, true),
                    _ => (true, true, true, true), // ALL or empty
                };
                let cnt = self.reset_selected_actors_transform(r_loc, r_piv, r_rot, r_scale);
                format!("ACTOR RESET {}: reset {} object(s).", if sub.is_empty() { "ALL" } else { sub }, cnt)
            }
            _ if upper.starts_with("BRUSH SCALE") => {
                let sub = upper["BRUSH SCALE".len()..].trim();
                if sub == "RESET" {
                    if let Some(sel_idx) = self.selected_brush_index {
                        if let Some(b) = self.world_brushes.get_mut(sel_idx) {
                            b.scale = Vec3::ONE;
                        }
                    } else {
                        self.builder_brush.scale = Vec3::ONE;
                    }
                    "BRUSH SCALE RESET executed.".to_string()
                } else {
                    let mut sx = 1.0f32;
                    let mut sy = 1.0f32;
                    let mut sz = 1.0f32;
                    for tok in sub.split_whitespace() {
                        if let Some(v) = tok.strip_prefix("X=") {
                            if let Ok(f) = v.parse::<f32>() {
                                sx = f;
                            }
                        } else if let Some(v) = tok.strip_prefix("Y=") {
                            if let Ok(f) = v.parse::<f32>() {
                                sy = f;
                            }
                        } else if let Some(v) = tok.strip_prefix("Z=") {
                            if let Ok(f) = v.parse::<f32>() {
                                sz = f;
                            }
                        }
                    }
                    self.scale_selected(Vec3::new(sx, sy, sz), BrushScaleMode::ActorTransform, false);
                    format!("BRUSH SCALE X={} Y={} Z={} executed.", sx, sy, sz)
                }
            }
            _ if upper.starts_with("SELECT OFCLASS CLASS=") || upper.starts_with("ACTOR SELECT OFCLASS CLASS=") => {
                let prefix = if upper.starts_with("SELECT OFCLASS CLASS=") {
                    "SELECT OFCLASS CLASS="
                } else {
                    "ACTOR SELECT OFCLASS CLASS="
                };
                let cls_name = raw[prefix.len()..].split_whitespace().next().unwrap_or("").trim();
                let cnt = self.select_actors_of_class(cls_name, false);
                format!("SELECT OFCLASS CLASS={}: selected {} actor(s).", cls_name, cnt)
            }
            _ if upper.starts_with("SELECT OFSUBCLASS CLASS=") || upper.starts_with("ACTOR SELECT OFSUBCLASS CLASS=") => {
                let prefix = if upper.starts_with("SELECT OFSUBCLASS CLASS=") {
                    "SELECT OFSUBCLASS CLASS="
                } else {
                    "ACTOR SELECT OFSUBCLASS CLASS="
                };
                let cls_name = raw[prefix.len()..].split_whitespace().next().unwrap_or("").trim();
                let cnt = self.select_actors_of_class(cls_name, true);
                format!("SELECT OFSUBCLASS CLASS={}: selected {} actor(s).", cls_name, cnt)
            }
            _ if upper.starts_with("MAP SELECT ") => {
                let filter = upper["MAP SELECT ".len()..].trim();
                let cnt = self.select_brushes_by_filter(filter);
                format!("MAP SELECT {}: selected {} brush(es).", filter, cnt)
            }
            _ if upper.starts_with("MAP IMPORTADD FILE=") => {
                let file_arg = raw["MAP IMPORTADD FILE=".len()..].trim().trim_matches('"');
                match self.import_t3d_into_existing_map(file_arg) {
                    Ok((b, m, a)) => format!("MAP IMPORTADD FILE={}: merged {} brush(es), {} mover(s), {} actor(s).", file_arg, b, m, a),
                    Err(e) => format!("MAP IMPORTADD error: {}", e),
                }
            }
            _ if upper.starts_with("ACTOR KEYFRAME NUM=") => {
                let num_str = upper["ACTOR KEYFRAME NUM=".len()..].trim();
                if let Ok(k) = num_str.parse::<usize>() {
                    if let Some(m_idx) = self.selected_mover_index {
                        if let Some(m) = self.movers.get_mut(m_idx) {
                            m.current_key = k.min(7);
                        }
                    }
                    format!("ACTOR KEYFRAME NUM={}: set active Mover keyframe.", k.min(7))
                } else {
                    "ACTOR KEYFRAME error: invalid keyframe number.".to_string()
                }
            }
            _ if upper.starts_with("POLY DEFAULT TEXTURE=") || upper.starts_with("POLY SET ") => {
                if upper.starts_with("POLY DEFAULT TEXTURE=") {
                    let tex_name = raw["POLY DEFAULT TEXTURE=".len()..].split_whitespace().next().unwrap_or("").trim();
                    if !tex_name.is_empty() {
                        self.active_texture = Some(tex_name.to_string());
                        if !self.selected_surfaces.is_empty() || !self.has_multi_selection() {
                            self.apply_active_texture_to_selected_brush();
                        } else {
                            self.apply_active_texture_to_multi_selected();
                        }
                    }
                    format!("Applied default texture '{}'.", tex_name)
                } else {
                    let mut tex_applied = None;
                    let mut set_flags = 0u32;
                    let mut clear_flags = 0u32;
                    for tok in raw["POLY SET ".len()..].split_whitespace() {
                        let tok_upper = tok.to_ascii_uppercase();
                        if tok_upper.starts_with("TEXTURE=") {
                            let t = tok["TEXTURE=".len()..].trim();
                            if !t.is_empty() {
                                self.active_texture = Some(t.to_string());
                                if !self.selected_surfaces.is_empty() || !self.has_multi_selection() {
                                    self.apply_active_texture_to_selected_brush();
                                } else {
                                    self.apply_active_texture_to_multi_selected();
                                }
                                tex_applied = Some(t.to_string());
                            }
                        } else if let Some(val_s) = tok_upper.strip_prefix("SETFLAGS=") {
                            if let Ok(v) = val_s.parse::<u32>() {
                                set_flags |= v;
                            }
                        } else if let Some(val_s) = tok_upper.strip_prefix("CLEARFLAGS=") {
                            if let Ok(v) = val_s.parse::<u32>() {
                                clear_flags |= v;
                            }
                        }
                    }
                    let flag_cnt = if set_flags != 0 || clear_flags != 0 {
                        self.set_selected_surfaces_flags(set_flags, clear_flags)
                    } else {
                        0
                    };
                    format!(
                        "POLY SET: texture={:?}, set_flags=0x{:X}, clear_flags=0x{:X} ({} surface(s) updated).",
                        tex_applied, set_flags, clear_flags, flag_cnt
                    )
                }
            }
            _ if upper.starts_with("POLY TEXPAN") => {
                let args = upper["POLY TEXPAN".len()..].trim();
                if args.contains("RESET") {
                    self.reset_selected_surfaces_pan();
                }
                let mut u_pan = 0.0f32;
                let mut v_pan = 0.0f32;
                for tok in args.split_whitespace() {
                    if let Some(val_s) = tok.strip_prefix("U=") {
                        if let Ok(f) = val_s.parse::<f32>() {
                            u_pan = f;
                        }
                    } else if let Some(val_s) = tok.strip_prefix("V=") {
                        if let Ok(f) = val_s.parse::<f32>() {
                            v_pan = f;
                        }
                    }
                }
                if u_pan.abs() > 1e-5 || v_pan.abs() > 1e-5 {
                    self.pan_selected_brush_texture(u_pan, v_pan);
                }
                format!("POLY TEXPAN U={} V={} executed.", u_pan, v_pan)
            }
            _ if upper.starts_with("POLY TEXSCALE") || upper.starts_with("POLY TEXMULT") => {
                let prefix_len = if upper.starts_with("POLY TEXSCALE") {
                    "POLY TEXSCALE".len()
                } else {
                    "POLY TEXMULT".len()
                };
                let args = upper[prefix_len..].trim();
                let mut uu = 1.0f32;
                let mut vv = 1.0f32;
                let mut uv = 0.0f32;
                let mut vu = 0.0f32;
                for tok in args.split_whitespace() {
                    if let Some(val_s) = tok.strip_prefix("UU=").or_else(|| tok.strip_prefix("U=")) {
                        if let Ok(f) = val_s.parse::<f32>() {
                            uu = f;
                        }
                    } else if let Some(val_s) = tok.strip_prefix("VV=").or_else(|| tok.strip_prefix("V=")) {
                        if let Ok(f) = val_s.parse::<f32>() {
                            vv = f;
                        }
                    } else if let Some(val_s) = tok.strip_prefix("UV=") {
                        if let Ok(f) = val_s.parse::<f32>() {
                            uv = f;
                        }
                    } else if let Some(val_s) = tok.strip_prefix("VU=") {
                        if let Ok(f) = val_s.parse::<f32>() {
                            vu = f;
                        }
                    }
                }
                if uv.abs() > 1e-5 || vu.abs() > 1e-5 {
                    self.transform_selected_brush_texture_matrix(uu, uv, vu, vv);
                } else {
                    self.scale_selected_brush_texture(uu, vv);
                }
                format!("POLY TEXSCALE UU={} VV={} UV={} VU={} executed.", uu, vv, uv, vu)
            }
            _ if upper.starts_with("TEXTURE SCALE") => {
                let args = upper["TEXTURE SCALE".len()..].trim();
                let mut delta = 1.0f32;
                for tok in args.split_whitespace() {
                    if let Some(val_s) = tok.strip_prefix("DELTA=") {
                        if let Ok(f) = val_s.parse::<f32>() {
                            delta = f;
                        }
                    } else if let Ok(f) = tok.parse::<f32>() {
                        delta = f;
                    }
                }
                if delta.abs() > 1e-5 {
                    self.scale_selected_brush_texture(delta, delta);
                }
                format!("TEXTURE SCALE DELTA={} executed.", delta)
            }
            _ if upper.starts_with("LIGHT SCALE ") => {
                let arg = upper.trim_start_matches("LIGHT SCALE ").trim().trim_end_matches('%');
                if let Ok(pct) = arg.parse::<f32>() {
                    let count = self.scale_lights_brightness(pct, true, false);
                    format!("LIGHT SCALE {}%: scaled {} light(s).", pct, count)
                } else {
                    "LIGHT SCALE error: expected numeric percentage.".to_string()
                }
            }
            "MAP NEW" => {
                if self.new_map() {
                    "MAP NEW executed: created empty map.".to_string()
                } else {
                    "MAP NEW not executed: finish the active save, export, or viewport transform.".to_string()
                }
            }
            "ACTOR ALIGN" => {
                let floor_ok = self.snap_selected_to_floor();
                let grid_ok = self.snap_selected_to_grid();
                format!("ACTOR ALIGN executed (floor_snap={}, grid_snap={}).", floor_ok, grid_ok)
            }
            _ if upper.starts_with("BRUSH LOAD") => {
                let file_arg = if upper.starts_with("BRUSH LOAD FILE=") {
                    raw["BRUSH LOAD FILE=".len()..].trim().trim_matches('"')
                } else {
                    raw["BRUSH LOAD".len()..].trim().trim_matches('"')
                };
                if file_arg.is_empty() {
                    "BRUSH LOAD error: missing file path.".to_string()
                } else {
                    match std::fs::read_to_string(file_arg) {
                        Ok(t3d) => match self.import_t3d_to_builder_brush(&t3d) {
                            Ok(cnt) => format!("BRUSH LOAD '{}': loaded builder brush with {} polygon(s).", file_arg, cnt),
                            Err(e) => format!("BRUSH LOAD error parsing '{}': {}", file_arg, e),
                        },
                        Err(e) => format!("BRUSH LOAD error reading '{}': {}", file_arg, e),
                    }
                }
            }
            _ if upper.starts_with("BRUSH SAVE") => {
                let file_arg = if upper.starts_with("BRUSH SAVE FILE=") {
                    raw["BRUSH SAVE FILE=".len()..].trim().trim_matches('"')
                } else {
                    raw["BRUSH SAVE".len()..].trim().trim_matches('"')
                };
                if file_arg.is_empty() {
                    "BRUSH SAVE error: missing file path.".to_string()
                } else {
                    let t3d = self.export_builder_brush_to_t3d();
                    match std::fs::write(file_arg, t3d) {
                        Ok(_) => format!("BRUSH SAVE '{}': exported builder brush to file.", file_arg),
                        Err(e) => format!("BRUSH SAVE error writing '{}': {}", file_arg, e),
                    }
                }
            }
            _ if upper.starts_with("MAP LOAD") => {
                let file_arg = if upper.starts_with("MAP LOAD FILE=") {
                    raw["MAP LOAD FILE=".len()..].trim().trim_matches('"')
                } else {
                    raw["MAP LOAD".len()..].trim().trim_matches('"')
                };
                if file_arg.is_empty() {
                    "MAP LOAD error: missing file path.".to_string()
                } else if file_arg.to_ascii_lowercase().ends_with(".t3d") {
                    match self.load_t3d(file_arg) {
                        Ok(_) => format!("MAP LOAD '{}': loaded T3D map.", file_arg),
                        Err(e) => format!("MAP LOAD error loading T3D '{}': {}", file_arg, e),
                    }
                } else {
                    match self.load_unr(file_arg) {
                        Ok(_) => format!("MAP LOAD '{}': loaded UNR map.", file_arg),
                        Err(e) => format!("MAP LOAD error loading UNR '{}': {}", file_arg, e),
                    }
                }
            }
            _ if upper.starts_with("MAP SAVE") => {
                let file_arg = if upper.starts_with("MAP SAVE FILE=") {
                    raw["MAP SAVE FILE=".len()..].trim().trim_matches('"')
                } else {
                    raw["MAP SAVE".len()..].trim().trim_matches('"')
                };
                if file_arg.is_empty() {
                    "MAP SAVE error: missing file path.".to_string()
                } else if file_arg.to_ascii_lowercase().ends_with(".t3d") {
                    match self.save_t3d(file_arg) {
                        Ok(_) => format!("MAP SAVE '{}': saved T3D map.", file_arg),
                        Err(e) => format!("MAP SAVE error saving T3D '{}': {}", file_arg, e),
                    }
                } else {
                    match self.save_unr(file_arg) {
                        Ok(p) => format!("MAP SAVE '{:?}': saved UNR map.", p),
                        Err(e) => format!("MAP SAVE error saving UNR '{}': {}", file_arg, e),
                    }
                }
            }
            _ if upper.starts_with("POLY ROTATE") => {
                let args = upper["POLY ROTATE".len()..].trim();
                let mut deg = 45.0f32;
                for tok in args.split_whitespace() {
                    if let Some(val_s) = tok.strip_prefix("ANGLE=").or_else(|| tok.strip_prefix("U=")) {
                        if let Ok(f) = val_s.parse::<f32>() {
                            deg = f;
                        }
                    } else if let Ok(f) = tok.parse::<f32>() {
                        deg = f;
                    }
                }
                self.rotate_selected_brush_texture(deg);
                format!("POLY ROTATE ANGLE={} executed.", deg)
            }
            _ => {
                let msg = format!("Unrecognized or custom command: '{}'", raw);
                self.log(&msg);
                msg
            }
        };

        response
    }

}
