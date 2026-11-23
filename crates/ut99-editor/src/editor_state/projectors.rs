use super::*;

impl EditorState {
    pub fn add_projector_at_builder(&mut self) {
        let mut proj = Projector::default();
        proj.name = format!("Projector_{}", self.projectors.len());
        proj.location = self.builder_brush.location;
        proj.rotation = self.builder_brush.rotation;
        proj.width = self.decal_width;
        proj.height = self.decal_height;
        proj.blend_mode = self.decal_blend_mode;
        proj.max_trace_distance = self.decal_max_distance;
        if let Some(ref tex) = self.active_texture {
            proj.texture = tex.clone();
        }

        self.projectors.push(proj.clone());
        let idx = self.projectors.len() - 1;
        self.selected_projector_index = Some(idx);
        self.bake_projector_decals();
        self.history.push(Action::AddProjector { index: idx, projector: proj });
        self.log(format!("Added Projector decal at ({:.0}, {:.0}, {:.0}).", self.builder_brush.location.x, self.builder_brush.location.y, self.builder_brush.location.z));
    }

    /// Raycasts against all solid world geometry (brushes, movers, and static meshes) and returns the closest (hit_point, normal) if any.

    pub fn stamp_decal_at_ray(&mut self, ray: Ray) -> Option<usize> {
        let hit_info = self.raycast_world(ray);

        if let Some((hit_pos, normal)) = hit_info {
            let tex = self.active_texture.clone().unwrap_or_else(|| "Botpack.BloodSplat".to_string());
            let mut proj = Projector::from_surface_hit(
                hit_pos,
                normal,
                self.decal_width,
                self.decal_height,
                tex.clone(),
                self.decal_blend_mode,
            );
            proj.name = format!("DecalProjector_{}", self.projectors.len());
            if self.decal_max_distance > 0.0 {
                proj.max_trace_distance = self.decal_max_distance;
            }

            self.projectors.push(proj.clone());
            let idx = self.projectors.len() - 1;
            self.selected_projector_index = Some(idx);
            self.bake_projector_decals();
            self.history.push(Action::AddProjector { index: idx, projector: proj });
            self.log(format!("🎯 Stamped Decal '{}' ({:.0}x{:.0}) at ({:.1}, {:.1}, {:.1})", tex, self.decal_width, self.decal_height, hit_pos.x, hit_pos.y, hit_pos.z));
            Some(idx)
        } else {
            self.log("⚠ No surface intersected by cursor ray to stamp decal.");
            None
        }
    }

    pub fn remove_selected_projector(&mut self) -> bool {
        if let Some(idx) = self.selected_projector_index {
            if idx < self.projectors.len() {
                let p = self.projectors.remove(idx);
                self.selected_projector_index = None;
                self.bake_projector_decals();
                self.history.push(Action::DeleteProjector { index: idx, projector: p.clone() });
                self.log(format!("🗑 Removed Decal Projector '{}'", p.name));
                return true;
            }
        }
        false
    }

    pub fn clear_all_decals(&mut self) {
        let count = self.projectors.len();
        self.projectors.clear();
        self.selected_projector_index = None;
        self.active_projector_decals.clear();
        self.log(format!("🧹 Cleared all {} decal projector(s).", count));
    }

    pub fn bake_projector_decals(&mut self) {
        let mut candidate_polys = Vec::new();
        for b in &self.world_brushes {
            candidate_polys.extend(b.to_world_polys());
        }
        for m in &self.movers {
            candidate_polys.extend(m.brush.to_world_polys());
        }
        for sm in &self.static_meshes {
            candidate_polys.extend(sm.to_world_polys());
        }

        self.active_projector_decals.clear();
        for proj in &self.projectors {
            let decals = proj.project_onto_polygons(&candidate_polys);
            self.active_projector_decals.extend(decals);
        }
        self.log(format!("Bake complete: {} projector decal fragments generated.", self.active_projector_decals.len()));
    }

    // --- Fase 6: AI Navigation & Bot Simulator ---

}
