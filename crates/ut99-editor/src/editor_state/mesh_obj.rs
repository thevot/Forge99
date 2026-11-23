use super::*;
use glam::Vec3;
use ut99_bsp::{CsgOperation, ObjModel};

impl EditorState {
    pub fn import_obj_to_builder(&mut self, obj_text: &str) -> Result<(), String> {
        let model = ObjModel::parse(obj_text)?;
        let mut brush = model.to_brush("ActiveBuilder", CsgOperation::Active);
        brush.location = self.builder_brush.location;

        if let Some(ref active_tex) = self.active_texture {
            for poly in &mut brush.polys {
                if poly.texture.is_empty() || poly.texture == "DefaultTexture" {
                    poly.texture = active_tex.clone();
                }
            }
        }

        self.builder_brush = brush;
        self.log(format!("Imported OBJ model into Active Builder Brush ({} vertices, {} polys).", model.vertices.len(), self.builder_brush.polys.len()));
        Ok(())
    }

    pub fn import_obj_as_static_mesh(&mut self, name: &str, obj_text: &str, loc: Vec3) -> Result<(), String> {
        let model = ObjModel::parse(obj_text)?;
        let mut brush = model.to_brush(name, CsgOperation::Add);
        brush.location = loc;

        if let Some(ref active_tex) = self.active_texture {
            for poly in &mut brush.polys {
                if poly.texture.is_empty() || poly.texture == "DefaultTexture" {
                    poly.texture = active_tex.clone();
                }
            }
        }

        self.history.push(Action::AddStaticMesh {
            mesh: brush.clone(),
        });
        self.static_meshes.push(brush);
        self.selected_static_mesh_index = Some(self.static_meshes.len() - 1);
        self.log(format!("Imported StaticMesh '{}' at ({:.0}, {:.0}, {:.0}).", name, loc.x, loc.y, loc.z));
        Ok(())
    }

    pub fn place_mesh_from_library(&mut self, mesh_name: &str, location: Vec3) -> Result<(), String> {
        let mesh_res = self.mesh_library.get_mesh(mesh_name)
            .ok_or_else(|| format!("Mesh '{}' not found in library", mesh_name))?
            .clone();

        let mut brush = mesh_res.brush;
        brush.name = format!("{}_{}", mesh_res.name, self.static_meshes.len());
        brush.location = location;

        if let Some(ref active_tex) = self.active_texture {
            for poly in &mut brush.polys {
                if poly.texture.is_empty() || poly.texture == "DefaultTexture" {
                    poly.texture = active_tex.clone();
                }
            }
        }

        self.history.push(Action::AddStaticMesh {
            mesh: brush.clone(),
        });
        self.static_meshes.push(brush);
        self.selected_static_mesh_index = Some(self.static_meshes.len() - 1);
        self.log(format!("Placed Mesh '{}' at ({:.0}, {:.0}, {:.0}).", mesh_name, location.x, location.y, location.z));
        Ok(())
    }

    pub fn extract_mesh_to_builder(&mut self, mesh_name: &str) -> Result<(), String> {
        let mesh_res = self.mesh_library.get_mesh(mesh_name)
            .ok_or_else(|| format!("Mesh '{}' not found in library", mesh_name))?
            .clone();

        let mut brush = mesh_res.brush;
        brush.name = "ActiveBuilder".to_string();
        brush.csg_oper = CsgOperation::Active;
        brush.location = self.builder_brush.location;

        if let Some(ref active_tex) = self.active_texture {
            for poly in &mut brush.polys {
                if poly.texture.is_empty() || poly.texture == "DefaultTexture" {
                    poly.texture = active_tex.clone();
                }
            }
        }

        self.builder_brush = brush;
        self.log(format!("Extracted Mesh '{}' to Active Builder Brush.", mesh_name));
        Ok(())
    }

    pub fn import_obj_to_mesh_library(&mut self, name: &str, category: &str, desc: &str, obj_text: &str) -> Result<(), String> {
        self.mesh_library.import_obj(name, category, desc, obj_text)?;
        self.selected_mesh_name = Some(name.to_string());
        self.log(format!("Imported OBJ mesh '{}' into category '{}'.", name, category));
        Ok(())
    }

    // --- Procedural Textures (FireEngine UE1 Parity ) ---
    pub fn step_procedural_texture(&mut self) {
        if self.procedural_anim_playing {
            self.procedural_simulator.step();
        }
    }

    pub fn apply_procedural_texture_as_active(&mut self) {
        let rgba = self.procedural_simulator.to_rgba8();
        let w = self.procedural_simulator.width as u32;
        let h = self.procedural_simulator.height as u32;
        let name = self.procedural_texture_name.trim();
        let tex_name = if name.is_empty() { "ProceduralFx" } else { name };

        let utx_tex = ut99_format::UtxTexture {
            name: tex_name.to_string(),
            group: "Procedural".to_string(),
            width: w,
            height: h,
            format: 1, // RGBA / Texture
            masked: false,
            rgba_pixels: rgba,
        };

        self.texture_manager.upsert_texture("Procedural", utx_tex);
        let full_name = format!("Procedural.{}", tex_name);
        self.active_texture = Some(full_name.clone());
        self.log(format!("Applied '{}' as active material for surfaces and brushes.", full_name));
    }

    pub fn export_procedural_animation_strip(&mut self, dir: &std::path::Path, frame_count: usize) -> anyhow::Result<usize> {
        std::fs::create_dir_all(dir)?;
        let name = if self.procedural_texture_name.trim().is_empty() { "Procedural" } else { self.procedural_texture_name.trim() };
        let mut count = 0;
        let w = self.procedural_simulator.width as u32;
        let h = self.procedural_simulator.height as u32;

        for i in 0..frame_count {
            self.procedural_simulator.step();
            let rgba = self.procedural_simulator.to_rgba8();
            let file_path = dir.join(format!("{}_{:03}.png", name, i));
            if image::save_buffer(
                &file_path,
                &rgba[..(w * h * 4) as usize],
                w,
                h,
                image::ExtendedColorType::Rgba8,
            ).is_ok() {
                count += 1;
            }
        }
        self.log(format!("Exported {} animation frames for '{}' to '{}'.", count, name, dir.display()));
        Ok(count)
    }

    pub fn export_map_to_obj(&self, map_name: &str) -> String {
        let mut all_brushes: Vec<&Brush> = self.world_brushes.iter().collect();
        all_brushes.extend(self.static_meshes.iter());
        export_brushes_to_obj(map_name, &all_brushes)
    }

    pub fn export_selected_brush_to_obj(&self) -> Option<String> {
        if let Some(idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get(idx) {
                return Some(export_brushes_to_obj(&brush.name, &[brush]));
            }
        }
        if let Some(idx) = self.selected_static_mesh_index {
            if let Some(mesh) = self.static_meshes.get(idx) {
                return Some(export_brushes_to_obj(&mesh.name, &[mesh]));
            }
        }
        Some(export_brushes_to_obj("ActiveBuilder", &[&self.builder_brush]))
    }

    pub fn export_builder_brush_to_obj(&self) -> String {
        export_brushes_to_obj("BuilderBrush", &[&self.builder_brush])
    }

    pub fn import_obj_to_builder_brush(&mut self, obj_text: &str) -> Result<(), String> {
        let model = ObjModel::parse(obj_text)?;
        let mut imported_brush = model.to_brush("BuilderBrush", CsgOperation::Active);
        imported_brush.location = self.builder_brush.location;

        if let Some(ref active_tex) = self.active_texture {
            for poly in &mut imported_brush.polys {
                if poly.texture.is_empty() || poly.texture == "DefaultTexture" {
                    poly.texture = active_tex.clone();
                }
            }
        }

        let old_brush = self.builder_brush.clone();
        self.builder_brush = imported_brush.clone();
        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush: imported_brush,
            description: "Import OBJ to Builder Brush".to_string(),
        });
        self.log(format!("Imported OBJ into Builder Brush ({} polygons).", self.builder_brush.polys.len()));
        Ok(())
    }


}
