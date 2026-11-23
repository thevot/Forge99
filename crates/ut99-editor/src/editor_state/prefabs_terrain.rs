use super::*;

impl EditorState {
    pub fn apply_architectural_preset(&mut self, preset: ut99_bsp::ArchitecturalPrefabPreset) {
        self.architectural_settings.config = preset.create_config();
        self.log(format!("Applied Architectural Preset: {}", preset.title()));
    }

    /// Synthesizes the active architectural prefab directly into the Builder Brush
    pub fn synthesize_prefab_to_builder(&mut self) {
        let polys = ut99_bsp::build_prefab_polygons(&self.architectural_settings.config);
        let poly_count = polys.len();
        self.builder_brush.polys = polys;
        self.map_is_dirty = true;
        self.log(format!("Synthesized architectural prefab into Builder Brush ({} polygons)", poly_count));
    }

    /// Bakes the active architectural prefab into the world as a permanent CSG Brush with Undo/Redo
    pub fn bake_prefab_to_world(&mut self, csg_oper: ut99_bsp::CsgOperation) -> usize {
        let name = format!("ArchPrefab_{}", self.world_brushes.len());
        let mut brush = ut99_bsp::build_prefab_brush(&name, csg_oper, &self.architectural_settings.config);
        brush.location = self.builder_brush.location;

        let index = self.world_brushes.len();
        self.world_brushes.push(brush.clone());
        self.history.push(crate::history::Action::AddBrush {
            brush: brush.clone(),
        });
        self.selected_brush_index = Some(index);
        self.map_is_dirty = true;
        self.log(format!("Baked architectural prefab to World Brush #{index} ({:?})", csg_oper));
        index
    }

    /// Computes preview polygons transformed to the builder brush location
    pub fn get_prefab_preview_polygons(&self) -> Vec<ut99_bsp::Polygon> {
        let polys = ut99_bsp::build_prefab_polygons(&self.architectural_settings.config);
        let offset = self.builder_brush.location;
        polys.into_iter().filter_map(|p| {
            let moved_verts: Vec<glam::Vec3> = p.vertices.iter().map(|&v| v + offset).collect();
            ut99_bsp::Polygon::new(moved_verts)
        }).collect()
    }

    // ==========================================
    // Terrain & Heightmap Mesh Sculptor Studio
    // ==========================================

    /// Applies a canonical topographic terrain preset
    pub fn apply_terrain_preset(&mut self, preset: ut99_bsp::TerrainPreset) {
        self.terrain_settings.grid.apply_preset(preset);
        self.log(format!("Applied Terrain Preset: {}", preset.title()));
    }

    /// Performs interactive radial sculpting on the heightmap grid
    pub fn sculpt_terrain_at(&mut self, center: glam::Vec2, mode: ut99_bsp::TerrainSculptMode, radius: f32, strength: f32, target_h: f32) {
        self.terrain_settings.grid.sculpt(center, radius, strength, mode, target_h);
        self.map_is_dirty = true;
    }

    /// Generates multi-octave harmonic procedural noise across the entire terrain
    pub fn generate_terrain_noise(&mut self, amplitude: f32, frequency: f32) {
        self.terrain_settings.grid.generate_procedural_noise(amplitude, frequency);
        self.map_is_dirty = true;
        self.log(format!("Generated procedural terrain noise (amp: {:.1}, freq: {:.3})", amplitude, frequency));
    }

    /// Injects the sculpted terrain mesh into the Builder Brush
    pub fn synthesize_terrain_to_builder(&mut self) {
        let polys = self.terrain_settings.grid.to_polygons();
        let poly_count = polys.len();
        self.builder_brush.polys = polys;
        self.map_is_dirty = true;
        self.log(format!("Synthesized terrain mesh into Builder Brush ({} polygons)", poly_count));
    }

    /// Bakes the sculpted terrain into the world as a permanent CSG Brush with Undo/Redo
    pub fn bake_terrain_to_world(&mut self, csg_oper: ut99_bsp::CsgOperation) -> usize {
        let name = format!("Terrain_{}", self.world_brushes.len());
        let mut brush = self.terrain_settings.grid.to_brush(&name, csg_oper);
        brush.location = self.builder_brush.location;

        let index = self.world_brushes.len();
        self.world_brushes.push(brush.clone());
        self.history.push(crate::history::Action::AddBrush {
            brush,
        });
        self.selected_brush_index = Some(index);
        self.map_is_dirty = true;
        self.log(format!("Baked terrain mesh to World Brush #{index} ({:?})", csg_oper));
        index
    }

    /// Computes preview polygons transformed to the builder brush location
    pub fn get_terrain_preview_polygons(&self) -> Vec<ut99_bsp::Polygon> {
        let polys = self.terrain_settings.grid.to_polygons();
        let offset = self.builder_brush.location;
        polys.into_iter().filter_map(|p| {
            let moved_verts: Vec<glam::Vec3> = p.vertices.iter().map(|&v| v + offset).collect();
            ut99_bsp::Polygon::new(moved_verts)
        }).collect()
    }

    // ==========================================
    // Static Mesh LOD & Convex Collision Hull Studio
    // ==========================================

}


impl EditorState {
    /// Instantiates an architectural or gameplay prefab into the map at the given location
    pub fn instantiate_prefab(&mut self, prefab_idx: usize, location: Vec3) -> bool {
        if let Some(prefab) = self.prefabs.get(prefab_idx).cloned() {
            let mut added_brushes = 0;
            let mut added_movers = 0;
            let mut added_actors = 0;

            for mut b in prefab.brushes {
                b.location += location;
                self.world_brushes.push(b.clone());
                self.history.push(Action::AddBrush { brush: b });
                added_brushes += 1;
            }

            for mut m in prefab.movers {
                m.brush.location += location;
                for kf in &mut m.keyframes {
                    if let Some(ref mut k) = kf {
                        k.location += location;
                    }
                }
                self.movers.push(m.clone());
                self.history.push(Action::AddMover { mover: m });
                added_movers += 1;
            }

            if self.loaded_unr.is_none() {
                self.loaded_unr = Some(ut99_format::UnrLevel::default());
            }

            if let Some(ref mut unr) = self.loaded_unr {
                for mut a in prefab.actors {
                    a.location += location;
                    unr.actors.push(a.clone());
                    self.history.push(Action::AddActor { actor: a });
                    added_actors += 1;
                }
            }

            self.log(format!(
                "Instantiated Prefab '{}' at ({:.0}, {:.0}, {:.0}): {} brush(es), {} mover(s), {} actor(s)",
                prefab.name, location.x, location.y, location.z, added_brushes, added_movers, added_actors
            ));
            true
        } else {
            false
        }
    }


    /// Saves the currently selected Brush or Static Mesh as a custom reusable prefab
    pub fn save_selection_as_prefab(&mut self, name: &str, category: &str, description: &str) -> Result<(), String> {
        let mut p = crate::prefab::Prefab::new(name, category, description);

        if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                let mut brush_copy = b.clone();
                brush_copy.location = Vec3::ZERO;
                p.brushes.push(brush_copy);
            }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get(m_idx) {
                let mut mover_copy = m.clone();
                let base_loc = mover_copy.brush.location;
                mover_copy.brush.location = Vec3::ZERO;
                for kf in &mut mover_copy.keyframes {
                    if let Some(ref mut k) = kf {
                        k.location -= base_loc;
                    }
                }
                p.movers.push(mover_copy);
            }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            if let Some(m) = self.static_meshes.get(sm_idx) {
                let mut mesh_copy = m.clone();
                mesh_copy.location = Vec3::ZERO;
                p.brushes.push(mesh_copy);
            }
        } else {
            return Err("No brush, mover, or static mesh selected to save as prefab.".to_string());
        }

        self.prefabs.push(p);
        self.log(format!("Saved new custom prefab '{}' in category '{}'", name, category));
        Ok(())
    }


    /// Instantiates a data-driven [`crate::prefab_templates::SmartPrefabTemplate`] at `origin`
    /// and commits all generated brushes, movers, and actors as a single atomic `Action::Compound` (`Deliverable E.4` & `F.1`).
    pub fn instantiate_smart_prefab(
        &mut self,
        id: &str,
        origin: Vec3,
    ) -> Result<(usize, usize, usize), String> {
        let output = crate::prefab_templates::SmartPrefabLibrary::instantiate(
            id,
            origin,
            &self.smart_prefab_params,
        )?;

        let b_cnt = output.brushes.len();
        let m_cnt = output.movers.len();
        let a_cnt = output.actors.len();

        let mut actions = Vec::with_capacity(b_cnt + m_cnt + a_cnt);

        for b in output.brushes {
            self.world_brushes.push(b.clone());
            actions.push(Action::AddBrush { brush: b });
        }

        for m in output.movers {
            self.movers.push(m.clone());
            actions.push(Action::AddMover { mover: m });
        }

        let unr = self.loaded_unr.get_or_insert_with(ut99_format::UnrLevel::default);
        for a in output.actors {
            unr.actors.push(a.clone());
            actions.push(Action::AddActor { actor: a });
        }

        if !actions.is_empty() {
            self.history.push(Action::Compound {
                actions,
                description: format!("Instantiate Smart Prefab '{}'", id),
            });
            self.map_is_dirty = true;
        }

        self.log(format!(
            "Smart Prefab '{}' instantiated at ({:.0}, {:.0}, {:.0}): +{} brushes, +{} movers, +{} actors.",
            id, origin.x, origin.y, origin.z, b_cnt, m_cnt, a_cnt
        ));
        Ok((b_cnt, m_cnt, a_cnt))
    }

}
