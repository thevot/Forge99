use super::*;

impl EditorState {
    pub fn generate_lod_chain_for_selected_brush(&mut self) -> Option<ut99_bsp::MeshLodChain> {
        let (name, polys) = if let Some(idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(idx) {
                (b.name.clone(), b.polys.clone())
            } else {
                return None;
            }
        } else {
            ("BuilderBrush".to_string(), self.builder_brush.polys.clone())
        };

        if polys.is_empty() {
            return None;
        }

        let chain = ut99_bsp::build_mesh_lod_chain(&name, &polys);
        self.lod_collision_settings.active_lod_chain = Some(chain.clone());
        self.log(format!("Generated LOD Chain for '{}' (LOD0: {}, LOD1: {}, LOD2: {}, LOD3: {})",
            name, chain.lod0_polys.len(), chain.lod1_polys.len(), chain.lod2_polys.len(), chain.lod3_polys.len()));
        Some(chain)
    }

    /// Replaces the selected brush's geometry with the decimated polygons of the requested LOD level
    pub fn apply_lod_to_selected_brush(&mut self, lod_level: usize) -> bool {
        let chain = if let Some(ref c) = self.lod_collision_settings.active_lod_chain {
            c.clone()
        } else if let Some(c) = self.generate_lod_chain_for_selected_brush() {
            c
        } else {
            return false;
        };

        let target_polys = match lod_level {
            1 => chain.lod1_polys,
            2 => chain.lod2_polys,
            3 => chain.lod3_polys,
            _ => chain.lod0_polys,
        };

        if let Some(idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(idx) {
                let old_polys = b.polys.clone();
                let poly_len = target_polys.len();
                b.polys = target_polys.clone();
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: idx,
                    old_polys,
                    new_polys: target_polys,
                    description: format!("Apply LOD{lod_level} to Brush #{idx}"),
                });
                self.map_is_dirty = true;
                self.log(format!("Applied LOD{} to Brush #{idx} ({} polygons)", lod_level, poly_len));
                return true;
            }
        } else {
            let old_brush = self.builder_brush.clone();
            self.builder_brush.polys = target_polys.clone();
            let new_brush = self.builder_brush.clone();
            self.history.push(crate::history::Action::ModifyBuilderBrush {
                old_brush,
                new_brush,
                description: format!("Apply LOD{lod_level} to Builder Brush"),
            });
            self.map_is_dirty = true;
            self.log(format!("Applied LOD{} to Builder Brush ({} polygons)", lod_level, self.builder_brush.polys.len()));
            return true;
        }
        false
    }

    /// Synthesizes a convex collision hull brush for the selected brush or builder brush
    pub fn generate_collision_hull_for_selected_brush(&self, hull_type: ut99_bsp::CollisionHullType, margin: f32) -> Option<ut99_bsp::Brush> {
        let (name, polys, location) = if let Some(idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(idx) {
                (format!("{}_Collision", b.name), &b.polys, b.location)
            } else {
                return None;
            }
        } else {
            ("Builder_Collision".to_string(), &self.builder_brush.polys, self.builder_brush.location)
        };

        let mut hull_brush = ut99_bsp::generate_collision_hull(&name, polys, hull_type, margin)?;
        hull_brush.location = location;
        Some(hull_brush)
    }

    /// Bakes the synthesized collision hull into the world as a dedicated collision brush with Undo/Redo
    pub fn bake_collision_hull_as_brush(&mut self, hull_type: ut99_bsp::CollisionHullType, margin: f32) -> Option<usize> {
        let hull_brush = self.generate_collision_hull_for_selected_brush(hull_type, margin)?;
        let index = self.world_brushes.len();
        self.world_brushes.push(hull_brush.clone());
        self.history.push(crate::history::Action::AddBrush {
            brush: hull_brush,
        });
        self.selected_brush_index = Some(index);
        self.map_is_dirty = true;
        self.log(format!("Baked Collision Hull {:?} to World Brush #{index}", hull_type));
        Some(index)
    }

    /// Evaluates the polygon budget of the selected brush or builder brush
    pub fn evaluate_selected_mesh_budget(&self) -> (usize, ut99_bsp::PolygonBudgetRating) {
        let count = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get(idx).map(|b| b.polys.len()).unwrap_or(0)
        } else {
            self.builder_brush.polys.len()
        };
        (count, ut99_bsp::PolygonBudgetRating::from_poly_count(count))
    }

    // ==========================================
    // Cinematic Matinee Camera Path & Cutscene Sequencer Studio
    // ==========================================

}
