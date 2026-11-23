use super::*;
use glam::Vec3;

impl EditorState {
    // --- Multi-Object Selection & Batching ---
    pub fn select_brush_toggle(&mut self, index: usize, multi: bool) {
        if multi {
            if self.multi_selected_brushes.contains(&index) {
                self.multi_selected_brushes.remove(&index);
            } else {
                self.multi_selected_brushes.insert(index);
            }
            self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        } else {
            self.clear_multi_selection();
            self.multi_selected_brushes.insert(index);
            self.selected_brush_index = Some(index);
            self.selected_actor_index = None;
            self.selected_mover_index = None;
            self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
        }
    }

    pub fn select_actor_toggle(&mut self, index: usize, multi: bool) {
        if multi {
            if self.multi_selected_actors.contains(&index) {
                self.multi_selected_actors.remove(&index);
            } else {
                self.multi_selected_actors.insert(index);
            }
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        } else {
            self.clear_multi_selection();
            self.multi_selected_actors.insert(index);
            self.selected_actor_index = Some(index);
            self.selected_brush_index = None;
            self.selected_mover_index = None;
            self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
        }
    }

    pub fn clear_multi_selection(&mut self) {
        self.multi_selected_brushes.clear();
        self.multi_selected_actors.clear();
    }

    pub fn has_multi_selection(&self) -> bool {
        self.multi_selected_brushes.len() + self.multi_selected_actors.len() > 1
    }

    pub fn total_selected_count(&self) -> usize {
        self.multi_selected_brushes.len() + self.multi_selected_actors.len()
    }

    pub fn is_brush_selected(&self, index: usize) -> bool {
        self.multi_selected_brushes.contains(&index) || self.selected_brush_index == Some(index)
    }

    pub fn is_actor_selected(&self, index: usize) -> bool {
        self.multi_selected_actors.contains(&index) || self.selected_actor_index == Some(index)
    }

    pub fn select_actors_matching_tag(&mut self) -> usize {
        let Some(sel_idx) = self.selected_actor_index else {
            self.log("No actor currently selected.");
            return 0;
        };

        let tag = if let Some(ref unr) = self.loaded_unr {
            unr.actors.get(sel_idx).map(|a| a.tag.clone()).unwrap_or_default()
        } else {
            return 0;
        };

        if tag.trim().is_empty() {
            self.log("Selected actor has no Tag specified.");
            return 0;
        }

        let mut matched_indices = HashSet::new();
        if let Some(ref unr) = self.loaded_unr {
            for (i, actor) in unr.actors.iter().enumerate() {
                if actor.tag.eq_ignore_ascii_case(tag.trim()) {
                    matched_indices.insert(i);
                }
            }
        }

        let count = matched_indices.len();
        self.clear_multi_selection();
        self.multi_selected_actors = matched_indices;
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        self.selected_brush_index = None;
        self.log(format!("Selected {} actor(s) with matching Tag '{}'.", count, tag.trim()));
        count
    }

    pub fn select_actors_matching_event(&mut self) -> usize {
        let Some(sel_idx) = self.selected_actor_index else {
            self.log("No actor currently selected.");
            return 0;
        };

        let event = if let Some(ref unr) = self.loaded_unr {
            unr.actors.get(sel_idx).map(|a| a.event.clone()).unwrap_or_default()
        } else {
            return 0;
        };

        if event.trim().is_empty() {
            self.log("Selected actor has no Event specified.");
            return 0;
        }

        let mut matched_indices = HashSet::new();
        if let Some(ref unr) = self.loaded_unr {
            for (i, actor) in unr.actors.iter().enumerate() {
                if actor.event.eq_ignore_ascii_case(event.trim()) {
                    matched_indices.insert(i);
                }
            }
        }

        let count = matched_indices.len();
        self.clear_multi_selection();
        self.multi_selected_actors = matched_indices;
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        self.selected_brush_index = None;
        self.log(format!("Selected {} actor(s) with matching Event '{}'.", count, event.trim()));
        count
    }

    pub fn select_actors_targeted_by_event(&mut self) -> usize {
        let Some(sel_idx) = self.selected_actor_index else {
            self.log("No actor currently selected.");
            return 0;
        };

        let event = if let Some(ref unr) = self.loaded_unr {
            unr.actors.get(sel_idx).map(|a| a.event.clone()).unwrap_or_default()
        } else {
            return 0;
        };

        if event.trim().is_empty() {
            self.log("Selected actor has no Event specified to target.");
            return 0;
        }

        let mut matched_indices = HashSet::new();
        if let Some(ref unr) = self.loaded_unr {
            for (i, actor) in unr.actors.iter().enumerate() {
                if actor.tag.eq_ignore_ascii_case(event.trim()) {
                    matched_indices.insert(i);
                }
            }
        }

        let count = matched_indices.len();
        if count > 0 {
            self.clear_multi_selection();
            self.multi_selected_actors = matched_indices;
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
            self.selected_brush_index = None;
            self.log(format!("Selected {} target actor(s) activated by Event '{}' (Event -> Tag).", count, event.trim()));
        } else {
            self.log(format!("No target actors found with Tag matching Event '{}'.", event.trim()));
        }
        count
    }

    pub fn select_actors_triggering_tag(&mut self) -> usize {
        let Some(sel_idx) = self.selected_actor_index else {
            self.log("No actor currently selected.");
            return 0;
        };

        let tag = if let Some(ref unr) = self.loaded_unr {
            unr.actors.get(sel_idx).map(|a| a.tag.clone()).unwrap_or_default()
        } else {
            return 0;
        };

        if tag.trim().is_empty() {
            self.log("Selected actor has no Tag specified to trigger.");
            return 0;
        }

        let mut matched_indices = HashSet::new();
        if let Some(ref unr) = self.loaded_unr {
            for (i, actor) in unr.actors.iter().enumerate() {
                if actor.event.eq_ignore_ascii_case(tag.trim()) {
                    matched_indices.insert(i);
                }
            }
        }

        let count = matched_indices.len();
        if count > 0 {
            self.clear_multi_selection();
            self.multi_selected_actors = matched_indices;
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
            self.selected_brush_index = None;
            self.log(format!("Selected {} source trigger(s) that activate Tag '{}' (Tag -> Event).", count, tag.trim()));
        } else {
            self.log(format!("No triggers or source actors found with Event matching Tag '{}'.", tag.trim()));
        }
        count
    }

    pub fn delete_selected_batch(&mut self) -> (usize, usize) {
        let mut actions = Vec::new();
        let mut brushes_to_delete: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brushes_to_delete.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brushes_to_delete.push(b);
            }
        }
        brushes_to_delete.sort_unstable_by(|a, b| b.cmp(a)); // descending

        let mut deleted_brushes = 0;
        for idx in brushes_to_delete {
            if idx < self.world_brushes.len() {
                let brush = self.world_brushes.remove(idx);
                actions.push(crate::history::Action::DeleteBrush { index: idx, brush });
                deleted_brushes += 1;
            }
        }

        let mut actors_to_delete: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actors_to_delete.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actors_to_delete.push(a);
            }
        }
        actors_to_delete.sort_unstable_by(|a, b| b.cmp(a)); // descending

        let mut deleted_actors = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for idx in actors_to_delete {
                if idx < unr.actors.len() {
                    let actor = unr.actors.remove(idx);
                    actions.push(crate::history::Action::DeleteActor { index: idx, actor });
                    deleted_actors += 1;
                }
            }
            if deleted_actors > 0 {
                unr.sync_actor_views();
            }
        }

        if deleted_brushes > 0 || deleted_actors > 0 {
            self.map_is_dirty = true;
        }

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Delete batch ({} brushes, {} actors)", deleted_brushes, deleted_actors),
            });
        }

        self.clear_multi_selection();
        self.selected_brush_index = None;
        self.selected_actor_index = None;
        self.log(format!("Deleted batch: {} brush(es), {} actor(s).", deleted_brushes, deleted_actors));
        (deleted_brushes, deleted_actors)
    }

    /// Centralized single-actor deletion (`UX-06`):
    /// Removes `unr.actors[act_idx]`, records [`Action::DeleteActor`], adjusts `selected_actor_index`
    /// and `multi_selected_actors` indices, sets `map_is_dirty = true`, and invokes `unr.sync_actor_views()`.
    pub fn delete_actor_at(&mut self, act_idx: usize) -> bool {
        let mut deleted_name = None;
        if let Some(ref mut unr) = self.loaded_unr {
            if act_idx < unr.actors.len() {
                let actor = unr.actors.remove(act_idx);
                deleted_name = Some(actor.name.clone());
                self.history.push(crate::history::Action::DeleteActor {
                    index: act_idx,
                    actor,
                });
                unr.sync_actor_views();
            }
        }

        if let Some(name) = deleted_name {
            match self.selected_actor_index {
                Some(sel) if sel == act_idx => self.selected_actor_index = None,
                Some(sel) if sel > act_idx => self.selected_actor_index = Some(sel - 1),
                _ => {}
            }
            let updated_multi: HashSet<usize> = self
                .multi_selected_actors
                .iter()
                .filter_map(|&idx| {
                    if idx == act_idx {
                        None
                    } else if idx > act_idx {
                        Some(idx - 1)
                    } else {
                        Some(idx)
                    }
                })
                .collect();
            self.multi_selected_actors = updated_multi;
            self.map_is_dirty = true;
            self.log(format!("Deleted Actor #{} ('{}').", act_idx, name));
            true
        } else {
            false
        }
    }

    // --- Clipboard Operations (UnrealEd 2.0 Parity) ---
    pub fn copy_selected_to_clipboard(&mut self) -> (usize, usize) {
        self.clipboard.brushes.clear();
        self.clipboard.actors.clear();

        if self.has_multi_selection() {
            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    self.clipboard.brushes.push(b.clone());
                }
            }
            if let Some(ref unr) = self.loaded_unr {
                for &a_idx in &self.multi_selected_actors {
                    if let Some(a) = unr.actors.get(a_idx) {
                        self.clipboard.actors.push(a.clone());
                    }
                }
            }
        } else {
            if let Some(b_idx) = self.selected_brush_index {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    self.clipboard.brushes.push(b.clone());
                }
            }
            if let Some(a_idx) = self.selected_actor_index {
                if let Some(ref unr) = self.loaded_unr {
                    if let Some(a) = unr.actors.get(a_idx) {
                        self.clipboard.actors.push(a.clone());
                    }
                }
            }
        }

        let b_count = self.clipboard.brushes.len();
        let a_count = self.clipboard.actors.len();
        if b_count > 0 || a_count > 0 {
            let mut t3d_map = ut99_format::T3dMap::new();
            for (idx, b) in self.clipboard.brushes.iter().enumerate() {
                let mut actor = ut99_format::T3dActor {
                    class: "Brush".to_string(),
                    name: b.name.clone(),
                    properties: std::collections::BTreeMap::new(),
                    brush: Some(crate::compiler::CompilerBridge::convert_brush_to_t3d(
                        b,
                        format!("ClipModel{}", idx),
                    )),
                };
                let csg = match b.csg_oper {
                    CsgOperation::Subtract => "CSG_Subtract",
                    CsgOperation::Intersect => "CSG_Intersect",
                    CsgOperation::Deintersect => "CSG_Deintersect",
                    _ => "CSG_Add",
                };
                actor.properties.insert("CsgOper".to_string(), csg.to_string());
                actor.properties.insert(
                    "Location".to_string(),
                    format!("(X={:.6},Y={:.6},Z={:.6})", b.location.x, b.location.y, b.location.z),
                );
                let (pitch, yaw, roll) = crate::compiler::deg_to_rotator(b.rotation);
                if pitch != 0 || yaw != 0 || roll != 0 {
                    actor.properties.insert(
                        "Rotation".to_string(),
                        format!("(Pitch={},Yaw={},Roll={})", pitch, yaw, roll),
                    );
                }
                t3d_map.actors.push(actor);
            }
            for a in &self.clipboard.actors {
                let mut actor = ut99_format::T3dActor {
                    class: a.class_name.clone(),
                    name: a.name.clone(),
                    properties: std::collections::BTreeMap::new(),
                    brush: None,
                };
                actor.properties.insert(
                    "Location".to_string(),
                    format!("(X={:.6},Y={:.6},Z={:.6})", a.location.x, a.location.y, a.location.z),
                );
                if !a.tag.is_empty() {
                    actor.properties.insert("Tag".to_string(), a.tag.clone());
                }
                if !a.event.is_empty() {
                    actor.properties.insert("Event".to_string(), a.event.clone());
                }
                t3d_map.actors.push(actor);
            }
            let mut buf = Vec::new();
            let _ = write_t3d(&mut buf, &t3d_map);
            self.clipboard.t3d_text = String::from_utf8_lossy(&buf).to_string();
            self.log(format!("Copied to clipboard: {} brush(es), {} actor(s) (Ctrl+C, .t3d ready).", b_count, a_count));
        } else {
            self.clipboard.t3d_text.clear();
        }
        (b_count, a_count)
    }

    /// Parses UnrealEd 2.0 `.t3d` text (`Begin Map ... End Map` or `Begin PolyList ... End PolyList`)
    /// into the clipboard and pastes it into the current level.
    pub fn paste_from_t3d_text(&mut self, t3d: &str) -> (usize, usize) {
        if let Ok(parsed_map) = ut99_format::parse_t3d(std::io::Cursor::new(t3d.as_bytes())) {
            let mut imported_brushes = Vec::new();
            let mut imported_actors = Vec::new();

            for t3d_actor in &parsed_map.actors {
                if let Some(ref poly_list) = t3d_actor.brush {
                    if t3d_actor.name == "Brush0" || t3d_actor.name == "Brush" {
                        continue;
                    }
                    let csg_oper = match t3d_actor.get_property("CsgOper") {
                        Some(s) if s.contains("CSG_Subtract") => CsgOperation::Subtract,
                        Some(s) if s.contains("CSG_Intersect") => CsgOperation::Intersect,
                        Some(s) if s.contains("CSG_Deintersect") => CsgOperation::Deintersect,
                        _ => CsgOperation::Add,
                    };
                    let mut brush = Brush::new(t3d_actor.name.clone(), csg_oper);
                    brush.location = t3d_actor.get_location();
                    for tp in &poly_list.polys {
                        if let Some(mut poly) = ut99_bsp::Polygon::new(tp.vertices.clone()) {
                            poly.flags = tp.flags;
                            if !tp.texture.is_empty() {
                                poly.texture = tp.texture.clone();
                            }
                            if tp.base != Vec3::ZERO {
                                poly.base = tp.base;
                            }
                            if tp.texture_u != Vec3::ZERO {
                                poly.texture_u = tp.texture_u;
                            }
                            if tp.texture_v != Vec3::ZERO {
                                poly.texture_v = tp.texture_v;
                            }
                            brush.polys.push(poly);
                        }
                    }
                    if !brush.polys.is_empty() {
                        imported_brushes.push(brush);
                    }
                } else if !t3d_actor.class.eq_ignore_ascii_case("LevelInfo") {
                    let mut rec = ut99_format::ActorRecord::default();
                    rec.class_name = t3d_actor.class.clone();
                    rec.name = t3d_actor.name.clone();
                    rec.location = t3d_actor.get_location();
                    rec.tag = t3d_actor.get_property("Tag").unwrap_or_default().to_string();
                    rec.event = t3d_actor.get_property("Event").unwrap_or_default().to_string();
                    rec.group = t3d_actor.get_property("Group").unwrap_or_default().to_string();
                    imported_actors.push(rec);
                }
            }

            if !imported_brushes.is_empty() || !imported_actors.is_empty() {
                self.clipboard.brushes = imported_brushes;
                self.clipboard.actors = imported_actors;
                self.clipboard.t3d_text = t3d.to_string();
                return self.paste_from_clipboard();
            }
        }
        (0, 0)
    }

    pub fn cut_selected_to_clipboard(&mut self) -> (usize, usize) {
        let counts = self.copy_selected_to_clipboard();
        if counts.0 > 0 || counts.1 > 0 {
            self.delete_selected_batch();
            self.log(format!("Cut to clipboard: {} brush(es), {} actor(s) (Ctrl+X).", counts.0, counts.1));
        }
        counts
    }

    pub fn paste_from_clipboard(&mut self) -> (usize, usize) {
        if self.clipboard.brushes.is_empty() && self.clipboard.actors.is_empty() {
            return (0, 0);
        }

        let offset = if self.snap_to_grid && self.grid_step > 0.0 {
            Vec3::new(self.grid_step, self.grid_step, 0.0)
        } else {
            Vec3::new(32.0, 32.0, 0.0)
        };

        self.clear_multi_selection();
        let mut actions = Vec::new();
        let mut new_brush_indices = Vec::new();
        for brush in self.clipboard.brushes.clone() {
            let mut b = brush;
            b.location += offset;
            let count = self.world_brushes.len();
            b.name = format!("{}_{}", b.name.split('_').next().unwrap_or("Brush"), count);
            self.world_brushes.push(b.clone());
            let idx = self.world_brushes.len() - 1;
            actions.push(Action::AddBrush { brush: b });
            new_brush_indices.push(idx);
        }

        let mut new_actor_indices = Vec::new();
        if !self.clipboard.actors.is_empty() {
            if let Some(ref mut unr) = self.loaded_unr {
                for actor in self.clipboard.actors.clone() {
                    let mut a = actor;
                    a.location += offset;
                    let count = unr.actors.len();
                    a.name = format!("{}_{}", a.class_name, count);
                    unr.actors.push(a.clone());
                    let idx = unr.actors.len() - 1;
                    actions.push(Action::AddActor { actor: a });
                    new_actor_indices.push(idx);
                }
            }
        }

        // Advance clipboard coordinates for consecutive cascading pastes
        for brush in &mut self.clipboard.brushes {
            brush.location += offset;
        }
        for actor in &mut self.clipboard.actors {
            actor.location += offset;
        }

        let total = new_brush_indices.len() + new_actor_indices.len();
        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(Action::Compound {
                actions,
                description: format!("Paste {} items from clipboard", total),
            });
        }

        if total == 1 {
            if let Some(&b) = new_brush_indices.first() {
                self.selected_brush_index = Some(b);
                self.selected_actor_index = None;
            } else if let Some(&a) = new_actor_indices.first() {
                self.selected_actor_index = Some(a);
                self.selected_brush_index = None;
            }
        } else if total > 1 {
            for idx in new_brush_indices {
                self.multi_selected_brushes.insert(idx);
            }
            for idx in new_actor_indices {
                self.multi_selected_actors.insert(idx);
            }
            self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        }

        let b_count = self.clipboard.brushes.len();
        let a_count = self.clipboard.actors.len();
        self.log(format!("Pasted from clipboard: {} brush(es), {} actor(s) (Ctrl+V).", b_count, a_count));
        (b_count, a_count)
    }

    // --- Mirroring & Transformation Operations (UnrealEd 2.0 Parity) ---
    pub fn mirror_selected(&mut self, axis: MirrorAxis) {
        let axis_str = match axis {
            MirrorAxis::X => "X",
            MirrorAxis::Y => "Y",
            MirrorAxis::Z => "Z",
        };

        if self.has_multi_selection() {
            let mut mirrored_brushes = 0;
            for &b_idx in &self.multi_selected_brushes {
                if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = brush.polys.clone();
                    mirror_brush_geometry(brush, axis);
                    let new_polys = brush.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Mirror across {axis_str}"),
                    });
                    mirrored_brushes += 1;
                }
            }
            self.log(format!("Mirrored {} brush(es) across {} axis.", mirrored_brushes, axis_str));
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                mirror_brush_geometry(brush, axis);
                let new_polys = brush.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Mirror across {axis_str}"),
                });
                self.log(format!("Mirrored Brush #{} across {} axis.", b_idx, axis_str));
            }
        } else {
            // Mirror Active Red Builder Brush
            mirror_brush_geometry(&mut self.builder_brush, axis);
            self.log(format!("Mirrored Red Builder Brush across {} axis.", axis_str));
        }
    }

    pub fn invert_selected_brush(&mut self) {
        if self.has_multi_selection() {
            let mut inverted_count = 0;
            for &b_idx in &self.multi_selected_brushes {
                if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = brush.polys.clone();
                    invert_brush_geometry(brush);
                    let new_polys = brush.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: "Invert Normals / Winding".to_string(),
                    });
                    inverted_count += 1;
                }
            }
            self.log(format!("Inverted normals and winding on {} brush(es).", inverted_count));
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                invert_brush_geometry(brush);
                let new_polys = brush.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: "Invert Normals / Winding".to_string(),
                });
                self.log(format!("Inverted normals and winding on Brush #{}.", b_idx));
            }
        } else {
            // Invert Builder Brush
            let old_brush = self.builder_brush.clone();
            invert_brush_geometry(&mut self.builder_brush);
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Invert Builder Brush Normals / Winding".to_string(),
            });
            self.log("Inverted normals and winding on Red Builder Brush.");
        }
    }

    pub fn invert_selected_surfaces(&mut self) -> usize {
        let mut inverted_count = 0;
        let selected: Vec<(usize, usize)> = self.selected_surfaces.iter().copied().collect();
        let mut by_brush: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for (b_idx, p_idx) in selected {
            by_brush.entry(b_idx).or_default().push(p_idx);
        }
        for (b_idx, poly_indices) in by_brush {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                for &p_idx in &poly_indices {
                    if let Some(poly) = brush.polys.get_mut(p_idx) {
                        poly.vertices.reverse();
                        if poly.vertices.len() >= 3 {
                            if let Some(plane) = ut99_bsp::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                                poly.plane = plane;
                            } else {
                                poly.plane.normal = -poly.plane.normal;
                                poly.plane.dist = -poly.plane.dist;
                            }
                        } else {
                            poly.plane.normal = -poly.plane.normal;
                            poly.plane.dist = -poly.plane.dist;
                        }
                        inverted_count += 1;
                    }
                }
                let new_polys = brush.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: "Invert Surface Normals".to_string(),
                });
            }
        }
        self.log(format!("Inverted normals on {} surface(s).", inverted_count));
        inverted_count
    }

    // --- Transform Reset & Baking Operations (UnrealEd 2.0 Parity) ---
    pub fn transform_permanently_selected(&mut self) -> bool {
        let mut target_indices = Vec::new();
        if self.has_multi_selection() {
            target_indices.extend(self.multi_selected_brushes.iter().copied());
        }
        if target_indices.is_empty() {
            if let Some(idx) = self.selected_brush_index {
                target_indices.push(idx);
            }
        }

        if target_indices.is_empty() {
            // Transform Builder Brush
            if self.builder_brush.rotation == Vec3::ZERO && self.builder_brush.scale == Vec3::ONE {
                self.log("Builder Brush rotation and scale are already identity; no baking needed.");
                return false;
            }
            let old_brush = self.builder_brush.clone();
            self.builder_brush.transform_permanently();
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Transform Builder Brush Permanently".to_string(),
            });
            self.log("Baked rotation and scale permanently into Red Builder Brush geometry.");
            return true;
        }

        let mut baked_count = 0;
        for &idx in &target_indices {
            if let Some(b) = self.world_brushes.get_mut(idx) {
                if b.rotation == Vec3::ZERO && b.scale == Vec3::ONE {
                    continue;
                }
                let old_rotation = b.rotation;
                let old_scale = b.scale;
                let old_polys = b.polys.clone();

                b.transform_permanently();

                let new_rotation = b.rotation;
                let new_scale = b.scale;
                let new_polys = b.polys.clone();

                self.history.push(Action::TransformBrush {
                    brush_index: idx,
                    old_rotation,
                    new_rotation,
                    old_scale,
                    new_scale,
                    old_polys,
                    new_polys,
                    description: "Transform Permanently".to_string(),
                });
                baked_count += 1;
            }
        }

        self.log(format!("Baked rotation and scale permanently into {} brush(es).", baked_count));
        baked_count > 0
    }

    pub fn reset_rotation_selected(&mut self) -> bool {
        let mut target_indices = Vec::new();
        if self.has_multi_selection() {
            target_indices.extend(self.multi_selected_brushes.iter().copied());
        }
        if target_indices.is_empty() {
            if let Some(idx) = self.selected_brush_index {
                target_indices.push(idx);
            }
        }

        if target_indices.is_empty() {
            // Check selected actor
            if let Some(act_idx) = self.selected_actor_index {
                let mut rot_info = None;
                if let Some(ref mut unr) = self.loaded_unr {
                    if let Some(a) = unr.actors.get_mut(act_idx) {
                        let old_rot = a.rotation;
                        a.rotation = (0, 0, 0);
                        rot_info = Some((old_rot, a.name.clone()));
                    }
                }
                if let Some((old_rot, name)) = rot_info {
                    self.history.push(Action::RotateActor {
                        actor_index: act_idx,
                        old_rotation: old_rot,
                        new_rotation: (0, 0, 0),
                    });
                    self.log(format!("Reset rotation on actor '{}' (Pitch=0, Yaw=0, Roll=0).", name));
                    return true;
                }
            }

            // Reset Builder Brush Rotation
            let old_brush = self.builder_brush.clone();
            self.builder_brush.rotation = Vec3::ZERO;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Reset Builder Brush Rotation".to_string(),
            });
            self.log("Reset Builder Brush rotation to (0, 0, 0).");
            return true;
        }

        let mut changed_count = 0;
        for &idx in &target_indices {
            if let Some(b) = self.world_brushes.get_mut(idx) {
                if b.rotation == Vec3::ZERO {
                    continue;
                }
                let old_rotation = b.rotation;
                b.rotation = Vec3::ZERO;
                self.history.push(Action::RotateBrush {
                    brush_index: idx,
                    old_rotation,
                    new_rotation: Vec3::ZERO,
                });
                changed_count += 1;
            }
        }
        self.log(format!("Reset rotation on {} brush(es).", changed_count));
        changed_count > 0
    }

    pub fn reset_scaling_selected(&mut self) -> bool {
        let mut target_indices = Vec::new();
        if self.has_multi_selection() {
            target_indices.extend(self.multi_selected_brushes.iter().copied());
        }
        if target_indices.is_empty() {
            if let Some(idx) = self.selected_brush_index {
                target_indices.push(idx);
            }
        }

        if target_indices.is_empty() {
            let old_brush = self.builder_brush.clone();
            self.builder_brush.scale = Vec3::ONE;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Reset Builder Brush Scaling".to_string(),
            });
            self.log("Reset Builder Brush scale to (1, 1, 1).");
            return true;
        }

        let mut changed_count = 0;
        for &idx in &target_indices {
            if let Some(b) = self.world_brushes.get_mut(idx) {
                if b.scale == Vec3::ONE {
                    continue;
                }
                let old_scale = b.scale;
                let old_rotation = b.rotation;
                let old_polys = b.polys.clone();
                b.scale = Vec3::ONE;
                let new_scale = b.scale;
                let new_rotation = b.rotation;
                let new_polys = b.polys.clone();

                self.history.push(Action::TransformBrush {
                    brush_index: idx,
                    old_rotation,
                    new_rotation,
                    old_scale,
                    new_scale,
                    old_polys,
                    new_polys,
                    description: "Reset Scaling".to_string(),
                });
                changed_count += 1;
            }
        }
        self.log(format!("Reset scaling on {} brush(es).", changed_count));
        changed_count > 0
    }

    pub fn reset_all_selected(&mut self) -> bool {
        let r = self.reset_rotation_selected();
        let s = self.reset_scaling_selected();
        r || s
    }

    // --- Selection Operations (UnrealEd 2.0 Parity) ---
    pub fn select_all(&mut self) {
        self.clear_multi_selection();
        for i in 0..self.world_brushes.len() {
            self.multi_selected_brushes.insert(i);
        }
        let count = self.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);
        for i in 0..count {
            self.multi_selected_actors.insert(i);
        }
        if !self.world_brushes.is_empty() {
            self.selected_brush_index = Some(0);
        }
        if count > 0 {
            self.selected_actor_index = Some(0);
        }
        self.log(format!("Selected all: {} brush(es), {} actor(s) (Ctrl+A).", self.world_brushes.len(), count));
    }

    pub fn select_all_brushes(&mut self) {
        self.clear_multi_selection();
        for i in 0..self.world_brushes.len() {
            self.multi_selected_brushes.insert(i);
        }
        if !self.world_brushes.is_empty() {
            self.selected_brush_index = Some(0);
        }
        self.log(format!("Selected all {} world brush(es).", self.world_brushes.len()));
    }

    pub fn select_additive_brushes(&mut self) -> usize {
        self.clear_multi_selection();
        for (i, b) in self.world_brushes.iter().enumerate() {
            if b.csg_oper == ut99_bsp::CsgOperation::Add {
                self.multi_selected_brushes.insert(i);
            }
        }
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        let count = self.multi_selected_brushes.len();
        self.log(format!("Selected {} Additive brush(es).", count));
        count
    }

    pub fn select_subtractive_brushes(&mut self) -> usize {
        self.clear_multi_selection();
        for (i, b) in self.world_brushes.iter().enumerate() {
            if b.csg_oper == ut99_bsp::CsgOperation::Subtract {
                self.multi_selected_brushes.insert(i);
            }
        }
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        let count = self.multi_selected_brushes.len();
        self.log(format!("Selected {} Subtractive brush(es).", count));
        count
    }

    pub fn select_semisolid_brushes(&mut self) -> usize {
        self.clear_multi_selection();
        for i in 0..self.world_brushes.len() {
            if self.get_brush_solidity(i) == BrushSolidity::SemiSolid {
                self.multi_selected_brushes.insert(i);
            }
        }
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        let count = self.multi_selected_brushes.len();
        self.log(format!("Selected {} Semi-Solid brush(es).", count));
        count
    }

    pub fn select_nonsolid_brushes(&mut self) -> usize {
        self.clear_multi_selection();
        for i in 0..self.world_brushes.len() {
            if self.get_brush_solidity(i) == BrushSolidity::NonSolid {
                self.multi_selected_brushes.insert(i);
            }
        }
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        let count = self.multi_selected_brushes.len();
        self.log(format!("Selected {} Non-Solid brush(es).", count));
        count
    }

    pub fn select_all_actors(&mut self) {
        self.clear_multi_selection();
        let count = self.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);
        for i in 0..count {
            self.multi_selected_actors.insert(i);
        }
        if count > 0 {
            self.selected_actor_index = Some(0);
        }
        self.log(format!("Selected all {} actor(s).", count));
    }

    pub fn select_all_lights(&mut self) -> usize {
        self.clear_multi_selection();
        if let Some(ref unr) = self.loaded_unr {
            for (i, a) in unr.actors.iter().enumerate() {
                if a.class_name.to_lowercase().contains("light") || a.name.to_lowercase().contains("light") {
                    self.multi_selected_actors.insert(i);
                }
            }
        }
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        let count = self.multi_selected_actors.len();
        self.log(format!("Selected {} Light actor(s).", count));
        count
    }

    pub fn select_all_pathnodes(&mut self) -> usize {
        self.clear_multi_selection();
        if let Some(ref unr) = self.loaded_unr {
            for (i, a) in unr.actors.iter().enumerate() {
                let name = a.class_name.to_lowercase();
                if name.contains("pathnode") || name.contains("playerstart") || name.contains("inventoryspot") || name.contains("navigationpoint") {
                    self.multi_selected_actors.insert(i);
                }
            }
        }
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        let count = self.multi_selected_actors.len();
        self.log(format!("Selected {} Navigation / PathNode actor(s).", count));
        count
    }

    /// Scales all world brushes, movers, actors, lights and geometry coordinates in the entire map (UnrealEd 2.0 Parity).
    pub fn scale_entire_map(&mut self, factor: Vec3, scale_actors: bool, scale_lights: bool) -> bool {
        if factor.x.abs() < 1e-4 || factor.y.abs() < 1e-4 || factor.z.abs() < 1e-4 {
            self.log("Cannot scale map: factor components must be non-zero.");
            return false;
        }

        let old_brushes = self.world_brushes.to_vec();
        let old_actors = self.loaded_unr.as_ref().map(|u| u.actors.to_vec());
        let old_movers = self.movers.to_vec();

        // 1. Scale World Brushes & their polygon geometry
        for b in &mut self.world_brushes {
            b.location = Vec3::new(b.location.x * factor.x, b.location.y * factor.y, b.location.z * factor.z);
            for p in &mut b.polys {
                for v in &mut p.vertices {
                    *v = Vec3::new(v.x * factor.x, v.y * factor.y, v.z * factor.z);
                }
                p.base = Vec3::new(p.base.x * factor.x, p.base.y * factor.y, p.base.z * factor.z);
                if p.vertices.len() >= 3 {
                    if let Some(new_plane) = ut99_bsp::Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                        p.plane = new_plane;
                    }
                }
            }
        }

        // 2. Scale Movers & their keyframes
        for m in &mut self.movers {
            m.brush.location = Vec3::new(m.brush.location.x * factor.x, m.brush.location.y * factor.y, m.brush.location.z * factor.z);
            for p in &mut m.brush.polys {
                for v in &mut p.vertices {
                    *v = Vec3::new(v.x * factor.x, v.y * factor.y, v.z * factor.z);
                }
                p.base = Vec3::new(p.base.x * factor.x, p.base.y * factor.y, p.base.z * factor.z);
                if p.vertices.len() >= 3 {
                    if let Some(new_plane) = ut99_bsp::Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                        p.plane = new_plane;
                    }
                }
            }
            for kf_opt in &mut m.keyframes {
                if let Some(ref mut kf) = kf_opt {
                    kf.location = Vec3::new(kf.location.x * factor.x, kf.location.y * factor.y, kf.location.z * factor.z);
                }
            }
        }

        // 3. Scale Actors in loaded_unr if requested
        let avg_factor = (factor.x.abs() + factor.y.abs() + factor.z.abs()) / 3.0;
        if let Some(ref mut level) = self.loaded_unr {
            if scale_actors {
                for actor in &mut level.actors {
                    actor.location = Vec3::new(actor.location.x * factor.x, actor.location.y * factor.y, actor.location.z * factor.z);
                    if scale_lights && actor.class_name.to_lowercase().contains("light") {
                        actor.light_radius = ((actor.light_radius as f32) * avg_factor).clamp(1.0, 255.0).round() as u8;
                    }
                }
                for light in &mut level.lights {
                    light.location = Vec3::new(light.location.x * factor.x, light.location.y * factor.y, light.location.z * factor.z);
                    if scale_lights {
                        light.light_radius = ((light.light_radius as f32) * avg_factor).clamp(1.0, 255.0).round() as u8;
                    }
                }
                for pstart in &mut level.player_starts {
                    pstart.location = Vec3::new(pstart.location.x * factor.x, pstart.location.y * factor.y, pstart.location.z * factor.z);
                }
                for pnode in &mut level.path_nodes {
                    pnode.location = Vec3::new(pnode.location.x * factor.x, pnode.location.y * factor.y, pnode.location.z * factor.z);
                }
                for pickup in &mut level.weapons_and_pickups {
                    pickup.location = Vec3::new(pickup.location.x * factor.x, pickup.location.y * factor.y, pickup.location.z * factor.z);
                }
                for other in &mut level.other_actors {
                    other.location = Vec3::new(other.location.x * factor.x, other.location.y * factor.y, other.location.z * factor.z);
                }
            }
            // Also scale level.brushes world polys
            for b in &mut level.brushes {
                b.actor.location = Vec3::new(b.actor.location.x * factor.x, b.actor.location.y * factor.y, b.actor.location.z * factor.z);
                for p in &mut b.world_polys {
                    for v in &mut p.vertices {
                        *v = Vec3::new(v.x * factor.x, v.y * factor.y, v.z * factor.z);
                    }
                    p.base = Vec3::new(p.base.x * factor.x, p.base.y * factor.y, p.base.z * factor.z);
                }
            }
        }

        self.build_ai_paths();

        let new_brushes = self.world_brushes.to_vec();
        let new_actors = self.loaded_unr.as_ref().map(|u| u.actors.to_vec());
        let new_movers = self.movers.to_vec();

        self.history.push(Action::ScaleEntireLevel {
            old_brushes,
            new_brushes,
            old_actors,
            new_actors,
            old_movers,
            new_movers,
            factor,
        });

        self.log(format!(
            "Scaled entire map by ({:.2}, {:.2}, {:.2}) (Actors: {}, Lights: {}).",
            factor.x, factor.y, factor.z, scale_actors, scale_lights
        ));
        true
    }

    /// Updates authoritative Level Properties and synchronizes with LevelInfo actor if present (UnrealEd 2.0 F6 Parity).
    pub fn update_level_properties(&mut self, title: String, author: String, ideal_players: String, song: String, game_type: String, enter_text: String) {
        self.level_title = title.clone();
        self.level_author = author.clone();
        self.level_ideal_player_count = ideal_players.clone();
        self.level_song = song.clone();
        self.level_game_type = game_type.clone();
        self.level_enter_text = enter_text.clone();

        // If loaded_unr exists, update its LevelInfo actor
        if let Some(ref mut level) = self.loaded_unr {
            if let Some(info) = level.actors.iter_mut().find(|a| {
                a.class_name.eq_ignore_ascii_case("LevelInfo") || a.class_name.eq_ignore_ascii_case("Engine.LevelInfo")
            }) {
                info.properties.insert("Title".into(), ut99_format::TaggedProperty {
                    name: "Title".into(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Str(title.clone()),
                });
                info.properties.insert("Author".into(), ut99_format::TaggedProperty {
                    name: "Author".into(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Str(author.clone()),
                });
                if !ideal_players.is_empty() {
                    info.properties.insert("IdealPlayerCount".into(), ut99_format::TaggedProperty {
                        name: "IdealPlayerCount".into(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Str(ideal_players),
                    });
                }
                if !song.is_empty() {
                    info.properties.insert("Song".into(), ut99_format::TaggedProperty {
                        name: "Song".into(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Str(song),
                    });
                }
                if !game_type.is_empty() {
                    info.properties.insert("DefaultGameType".into(), ut99_format::TaggedProperty {
                        name: "DefaultGameType".into(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Str(game_type),
                    });
                }
                if !enter_text.is_empty() {
                    info.properties.insert("LevelEnterText".into(), ut99_format::TaggedProperty {
                        name: "LevelEnterText".into(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Str(enter_text),
                    });
                }
            }
        }
        self.log(format!("Updated Level Properties: '{}' by '{}'", self.level_title, self.level_author));
    }

    /// Applies metadata from the Map Manifest and Packaging dialog, synchronizing
    /// with EditorState, LevelInfo0 actor properties, map filename, and refreshing the manifest cache.
    pub fn apply_manifest_metadata(
        &mut self,
        map_name: &str,
        title: &str,
        author: &str,
        ideal_players: &str,
        song: &str,
        game_type: Option<&str>,
        enter_text: Option<&str>,
    ) {
        let title_str = title.trim().to_string();
        let author_str = author.trim().to_string();
        let players_str = ideal_players.trim().to_string();
        let song_str = song.trim().to_string();
        let game_type_str = game_type.unwrap_or(&self.level_game_type).trim().to_string();
        let enter_text_str = enter_text.unwrap_or(&self.level_enter_text).trim().to_string();

        self.update_level_properties(
            title_str,
            author_str,
            players_str,
            song_str,
            game_type_str,
            enter_text_str,
        );

        let clean_map_name = map_name.trim();
        if !clean_map_name.is_empty() {
            let stem = if clean_map_name.to_lowercase().ends_with(".unr") {
                &clean_map_name[..clean_map_name.len() - 4]
            } else {
                clean_map_name
            };
            if let Some(ref mut path) = self.map_path {
                path.set_file_name(format!("{stem}.unr"));
            } else {
                self.map_path = Some(std::path::PathBuf::from(format!("UnrealTournament/Maps/{stem}.unr")));
            }
        }

        self.map_is_dirty = true;
        self.refresh_map_manifest();
        self.log(format!(
            "Applied Map Manifest Metadata: Title='{}', Author='{}', Players='{}', Song='{}'",
            self.level_title,
            self.level_author,
            self.level_ideal_player_count,
            self.level_song
        ));
    }

    /// Closes the single topmost or most specific open floating dialog/studio.
    /// Returns true if an open dialog was found and dismissed, false otherwise.
    pub fn close_one_floating_dialog(&mut self) -> bool {
        // 1. Specialized Studios & Advanced Crafting Studios (highest priority to dismiss)
        let studio_flags = [
            &mut self.show_lighting_studio_dialog,
            &mut self.show_skybox_studio_dialog,
            &mut self.show_warp_zone_studio_dialog,
            &mut self.show_zone_fog_studio_dialog,
            &mut self.show_skeletal_studio_dialog,
            &mut self.show_skin_studio_dialog,
            &mut self.show_package_extractor_dialog,
            &mut self.show_uc_class_wizard_dialog,
            &mut self.show_particle_studio_dialog,
            &mut self.show_bot_nav_studio_dialog,
            &mut self.show_architectural_studio_dialog,
            &mut self.show_terrain_studio_dialog,
            &mut self.show_lod_collision_dialog,
            &mut self.show_cinematic_studio_dialog,
            &mut self.show_light_effects_studio_dialog,
            &mut self.show_zone_occlusion_studio_dialog,
            &mut self.show_music_studio_dialog,
            &mut self.show_corona_studio_dialog,
            &mut self.show_fluid_studio_dialog,
            &mut self.show_ballistics_studio_dialog,
            &mut self.show_teleporter_studio_dialog,
            &mut self.show_translocator_studio_dialog,
            &mut self.show_lift_pathing_studio_dialog,
            &mut self.show_water_surface_studio_dialog,
            &mut self.show_mover_studio_dialog,
            &mut self.show_team_objective_dialog,
            &mut self.show_destructible_studio_dialog,
            &mut self.show_kicker_studio_dialog,
            &mut self.show_bsp_quality_studio_dialog,
            &mut self.show_loft_studio_dialog,
            &mut self.show_stairway_studio_dialog,
            &mut self.show_scatter_studio_dialog,
            &mut self.show_light_shaft_dialog,
            &mut self.show_track_studio_dialog,
            &mut self.show_surveillance_dialog,
            &mut self.show_laser_grid_dialog,
            &mut self.show_ventilator_dialog,
            &mut self.show_conveyor_dialog,
            &mut self.show_turret_dialog,
            &mut self.show_soundscape_dialog,
            &mut self.show_hazard_chamber_dialog,
            &mut self.show_pipe_conduit_dialog,
            &mut self.show_hologram_dialog,
            &mut self.show_weather_dialog,
            &mut self.show_arena_rules_dialog,
            &mut self.show_armor_stacking_dialog,
            &mut self.show_bot_jump_routes_dialog,
            &mut self.show_weapon_circuit_dialog,
            &mut self.show_powerup_control_dialog,
            &mut self.show_spawn_distribution_dialog,
            &mut self.show_domination_dialog,
            &mut self.show_assault_dialog,
            &mut self.show_radar_dialog,
            &mut self.show_ctf_base_dialog,
            &mut self.show_team_cannon_dialog,
            &mut self.show_bio_trap_dialog,
            &mut self.show_trigger_network_dialog,
            &mut self.show_elevator_dialog,
            &mut self.show_zone_velocity_dialog,
            &mut self.show_hazard_barrel_dialog,
            &mut self.show_special_lit_dialog,
            &mut self.show_sniper_vantage_dialog,
            &mut self.show_secret_door_dialog,
            &mut self.show_teleporter_arch_dialog,
            &mut self.show_fluid_basin_dialog,
            &mut self.show_alarm_beacon_dialog,
            &mut self.show_supply_locker_dialog,
            &mut self.show_skylight_dialog,
            &mut self.show_vent_duct_dialog,
            &mut self.show_piston_dialog,
            &mut self.show_jump_boots_dialog,
            &mut self.show_force_field_dialog,
            &mut self.show_reactor_core_dialog,
            &mut self.show_decal_studio_dialog,
            &mut self.show_uc_viewer_dialog,
            &mut self.show_procedural_texture_dialog,
            &mut self.show_package_manifest_dialog,
            &mut self.show_ctf_symmetry_dialog,
            &mut self.show_dispatcher_dialog,
            &mut self.show_zone_seal_dialog,
            &mut self.show_gameplay_balance_dialog,
            &mut self.show_map_packaging_dialog,
            &mut self.show_acoustics_dialog,
            &mut self.show_play_settings_dialog,
            &mut self.show_autosave_dialog,
            &mut self.show_camera_bookmarks_dialog,
            &mut self.show_map_budget_dialog,
            &mut self.show_action_wizards_dialog,
            &mut self.show_map_wizard_dialog,
            &mut self.show_visual_presets_dialog,
        ];
        for flag in studio_flags {
            if *flag {
                *flag = false;
                return true;
            }
        }

        // 2. Modals, Operations & Tool Dialogs
        let tool_flags = [
            &mut self.show_find_actors_dialog,
            &mut self.show_replace_textures_dialog,
            &mut self.show_build_options_dialog,
            &mut self.show_scale_brush_dialog,
            &mut self.show_shear_brush_dialog,
            &mut self.show_actor_align_dialog,
            &mut self.show_map_statistics_dialog,
            &mut self.show_level_doctor_window,
            &mut self.show_bot_simulator_window,
            &mut self.show_level_properties_dialog,
            &mut self.show_scale_map_dialog,
            &mut self.show_settings_dialog,
        ];
        for flag in tool_flags {
            if *flag {
                *flag = false;
                return true;
            }
        }

        // 3. Properties and System Browsers
        let browser_flags = [
            &mut self.show_actor_properties,
            &mut self.show_surface_properties,
            &mut self.show_texture_browser,
            &mut self.show_actor_browser,
            &mut self.show_mesh_browser,
            &mut self.show_sound_browser,
            &mut self.show_music_browser,
            &mut self.show_group_browser,
            &mut self.show_prefab_browser,
            &mut self.show_shape_editor,
            &mut self.show_add_special_dialog,
        ];
        for flag in browser_flags {
            if *flag {
                *flag = false;
                return true;
            }
        }

        false
    }

    /// Closes all open floating dialogs, studios, and browsers at once.
    /// Returns the number of dialogs that were closed.
    pub fn close_all_floating_dialogs(&mut self) -> usize {
        let mut count = 0;
        while self.close_one_floating_dialog() {
            count += 1;
        }
        if count > 0 {
            self.log(format!("Closed {count} floating window(s)."));
            self.toast_info(format!("Closed {count} floating window(s) (Ctrl+Shift+W)"));
        }
        count
    }

    /// Applies a predefined workspace layout preset for specific authoring tasks.
    pub fn apply_workspace_preset(&mut self, preset: WorkspacePreset) {
        self.close_all_floating_dialogs();
        match preset {
            WorkspacePreset::Default => {
                self.viewport_layout = crate::camera::ViewportLayout::Single;
                self.active_viewport_type = crate::camera::ViewportType::Perspective;
                self.viewport_shading = crate::camera::ViewportShading::Lit;
                self.show_texture_browser = true;
                self.show_bottom_console = false;
                self.toast_success("Workspace: Default Layout applied");
            }
            WorkspacePreset::LevelDesign => {
                self.viewport_layout = crate::camera::ViewportLayout::FourViewports;
                self.viewport_shading = crate::camera::ViewportShading::Lit;
                self.show_texture_browser = true;
                self.show_surface_properties = true;
                self.show_bottom_console = false;
                self.toast_success("Workspace: Level Design (4-View + Textures + Surfaces) applied");
            }
            WorkspacePreset::LightingAndAtmosphere => {
                self.viewport_layout = crate::camera::ViewportLayout::Single;
                self.active_viewport_type = crate::camera::ViewportType::Perspective;
                self.viewport_shading = crate::camera::ViewportShading::Lit;
                self.show_lighting_studio_dialog = true;
                self.show_skybox_studio_dialog = true;
                self.render_lux_heatmap = true;
                self.toast_success("Workspace: Lighting & Atmosphere applied");
            }
            WorkspacePreset::BotAndGameplay => {
                self.viewport_layout = crate::camera::ViewportLayout::FourViewports;
                self.viewport_shading = crate::camera::ViewportShading::ZonePortal;
                self.show_bot_nav_studio_dialog = true;
                self.show_nav_coverage = true;
                self.show_level_properties_dialog = true;
                self.toast_success("Workspace: Bot & Gameplay Navigation applied");
            }
        }
    }

    pub fn handle_escape(&mut self) -> bool {
        // Priority 0: Active 3D Drag & Drop Cancellation
        if self.active_drag_drop.is_some() {
            self.active_drag_drop = None;
            self.drag_drop_preview_pos = None;
            self.drag_drop_preview_normal = None;
            self.log("Cancelled 3D Drag & Drop (Esc)");
            return true;
        }

        // Priority 1: Viewport Context Menu, Command Palette & Help Cheatsheet
        if self.show_viewport_context_menu {
            self.show_viewport_context_menu = false;
            return true;
        }
        if self.show_command_palette {
            self.show_command_palette = false;
            return true;
        }
        if self.show_help_dialog {
            self.show_help_dialog = false;
            return true;
        }
        if self.show_shortcut_help_dialog {
            self.show_shortcut_help_dialog = false;
            return true;
        }

        // Priority 2: Floating tools, browsers & studio dialogs (closes the active floating window)
        if self.close_one_floating_dialog() {
            return true;
        }

        // Priority 3: Bottom Console Drawer
        if self.show_bottom_console {
            self.show_bottom_console = false;
            return true;
        }

        // Priority 4: Clear multi-selection if more than 1 item is selected
        if self.has_multi_selection() {
            self.clear_multi_selection();
            self.log("Cleared multi-selection (Esc).");
            return true;
        }

        // Priority 5: Clear single selection
        if self.selected_brush_index.is_some()
            || self.selected_actor_index.is_some()
            || self.selected_mover_index.is_some()
            || self.selected_static_mesh_index.is_some()
            || !self.selected_vertex_indices.is_empty()
            || !self.selected_surfaces.is_empty()
        {
            self.deselect_all();
            return true;
        }

        false
    }

    pub fn deselect_all(&mut self) {
        self.clear_multi_selection();
        self.selected_brush_index = None;
        self.selected_actor_index = None;
        self.selected_mover_index = None;
        self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
        self.selected_vertex_indices.clear();
        self.selected_surfaces.clear();
        self.log("Deselected all objects (Esc).");
    }

    pub fn invert_selection(&mut self) {
        let mut new_brushes = HashSet::new();
        for i in 0..self.world_brushes.len() {
            if !self.is_brush_selected(i) {
                new_brushes.insert(i);
            }
        }

        let mut new_actors = HashSet::new();
        let count = self.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);
        for i in 0..count {
            if !self.is_actor_selected(i) {
                new_actors.insert(i);
            }
        }

        self.clear_multi_selection();
        self.multi_selected_brushes = new_brushes;
        self.multi_selected_actors = new_actors;
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        self.log(format!(
            "Inverted selection: {} brush(es), {} actor(s) selected (Ctrl+I).",
            self.multi_selected_brushes.len(),
            self.multi_selected_actors.len()
        ));
    }

    pub fn snap_multi_selected_to_floor(&mut self) -> usize {
        let mut snapped = 0;
        let mut actions = Vec::new();
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        for b_idx in brush_indices {
            let (pos, current_bottom) = if let Some(b) = self.world_brushes.get(b_idx) {
                let (min_b, _) = b.world_bounds();
                (b.location, min_b.z)
            } else {
                continue;
            };

            if let Some(floor_z) = self.find_floor_z_below(pos, current_bottom, Some(b_idx)) {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_loc = b.location;
                    let (min_b, _) = b.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    b.location.z += delta_z;
                    let new_loc = b.location;
                    actions.push(crate::history::Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    snapped += 1;
                }
            }
        }

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        let mut actor_snaps = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for &a_idx in &actor_indices {
                if let Some(actor) = unr.actors.get(a_idx) {
                    let pos = actor.location;
                    if let Some(floor_z) = self.find_floor_z_below(pos, pos.z, None) {
                        actor_snaps.push((a_idx, floor_z));
                    }
                }
            }
        }

        if let Some(ref mut unr) = self.loaded_unr {
            for (a_idx, floor_z) in actor_snaps {
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let old_loc = actor.location;
                    let target_z = floor_z + 16.0;
                    actor.location.z = target_z;
                    let new_loc = actor.location;
                    actions.push(crate::history::Action::MoveActor {
                        actor_index: a_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    snapped += 1;
                }
            }
        }

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Snap {} objects to floor", snapped),
            });
        }

        self.log(format!("Snap to Floor: Snapped {} object(s) onto floor.", snapped));
        snapped
    }

    pub fn snap_multi_selected_to_floor_aligned(&mut self) -> usize {
        let mut snapped = 0;
        let mut actions = Vec::new();
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        for b_idx in brush_indices {
            let (pos, current_bottom) = if let Some(b) = self.world_brushes.get(b_idx) {
                let (min_b, _) = b.world_bounds();
                (b.location, min_b.z)
            } else {
                continue;
            };

            if let Some((floor_z, normal)) = self.find_floor_hit_below(pos, current_bottom, Some(b_idx)) {
                let normal = if !normal.is_finite() || normal.length_squared() < 1e-4 {
                    Vec3::Z
                } else {
                    normal.normalize()
                };

                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_loc = b.location;
                    let (min_b, _) = b.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    b.location.z += delta_z;
                    let new_loc = b.location;

                    let old_rot = b.rotation;
                    let rad_pitch = old_rot.x.to_radians();
                    let rad_yaw = old_rot.y.to_radians();
                    let rad_roll = old_rot.z.to_radians();
                    let cur_rot = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                    let mut old_fwd = cur_rot.transform_vector3(Vec3::X).normalize();
                    if old_fwd.cross(normal).length_squared() < 1e-4 {
                        old_fwd = Vec3::Y;
                    }
                    let right = normal.cross(old_fwd).normalize();
                    let forward = right.cross(normal).normalize();
                    let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                    let (rad_y, rad_x, rad_z) = rot_mat.to_euler(glam::EulerRot::YXZ);
                    b.rotation = Vec3::new(rad_x.to_degrees(), rad_y.to_degrees(), rad_z.to_degrees());
                    let new_rot = b.rotation;

                    actions.push(crate::history::Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    actions.push(crate::history::Action::RotateBrush {
                        brush_index: b_idx,
                        old_rotation: old_rot,
                        new_rotation: new_rot,
                    });
                    snapped += 1;
                }
            }
        }

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        let mut actor_snaps = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for &a_idx in &actor_indices {
                if let Some(actor) = unr.actors.get(a_idx) {
                    let pos = actor.location;
                    if let Some((floor_z, normal)) = self.find_floor_hit_below(pos, pos.z, None) {
                        actor_snaps.push((a_idx, floor_z, normal));
                    }
                }
            }
        }

        if let Some(ref mut unr) = self.loaded_unr {
            const ROT_TO_RAD: f32 = std::f32::consts::PI / 32768.0;
            const RAD_TO_ROT: f32 = 32768.0 / std::f32::consts::PI;

            for (a_idx, floor_z, normal) in actor_snaps {
                let normal = if !normal.is_finite() || normal.length_squared() < 1e-4 {
                    Vec3::Z
                } else {
                    normal.normalize()
                };

                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let old_loc = actor.location;
                    let target_z = floor_z + 16.0;
                    actor.location.z = target_z;
                    let new_loc = actor.location;

                    let old_rot = actor.rotation;
                    let pitch_rad = old_rot.0 as f32 * ROT_TO_RAD;
                    let yaw_rad = old_rot.1 as f32 * ROT_TO_RAD;
                    let roll_rad = old_rot.2 as f32 * ROT_TO_RAD;

                    let rotation_quat = glam::Quat::from_rotation_z(yaw_rad)
                        * glam::Quat::from_rotation_y(pitch_rad)
                        * glam::Quat::from_rotation_x(roll_rad);
                    let mut old_fwd = rotation_quat.mul_vec3(Vec3::X).normalize();
                    if old_fwd.cross(normal).length_squared() < 1e-4 {
                        old_fwd = Vec3::Y;
                    }
                    let right = normal.cross(old_fwd).normalize();
                    let forward = right.cross(normal).normalize();
                    let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                    let (yaw_out, pitch_out, roll_out) = rot_mat.to_euler(glam::EulerRot::ZYX);
                    let new_rot = (
                        (pitch_out * RAD_TO_ROT).round() as i32,
                        (yaw_out * RAD_TO_ROT).round() as i32,
                        (roll_out * RAD_TO_ROT).round() as i32,
                    );
                    actor.rotation = new_rot;

                    actions.push(crate::history::Action::MoveActor {
                        actor_index: a_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    actions.push(crate::history::Action::RotateActor {
                        actor_index: a_idx,
                        old_rotation: old_rot,
                        new_rotation: new_rot,
                    });
                    snapped += 1;
                }
            }
        }

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Snap and align {} objects to floor", snapped),
            });
        }

        self.log(format!("Snap to Floor Aligned: Snapped and aligned {} object(s) onto floor.", snapped));
        snapped
    }

    // --- Grid Snapping (UnrealEd 2.0 Parity) ---
    pub fn snap_selected_to_grid(&mut self) -> bool {
        if self.has_multi_selection() {
            return self.snap_multi_selected_to_grid() > 0;
        }

        let step = self.grid_step.max(1.0);

        if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let name = b.name.clone();
                let old_loc = b.location;
                let snapped = Vec3::new(
                    (old_loc.x / step).round() * step,
                    (old_loc.y / step).round() * step,
                    (old_loc.z / step).round() * step,
                );
                if (snapped - old_loc).length_squared() > 1e-4 {
                    b.location = snapped;
                    self.history.push(crate::history::Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: snapped,
                    });
                    self.log(format!("Snapped Brush '{}' to grid at ({:.0}, {:.0}, {:.0}) (step {:.0} uu).", name, snapped.x, snapped.y, snapped.z, step));
                    return true;
                } else {
                    self.log(format!("Brush '{}' is already on grid (step {:.0} uu).", name, step));
                    return false;
                }
            }
        } else if let Some(a_idx) = self.selected_actor_index {
            if let Some(ref mut unr) = self.loaded_unr {
                if let Some(a) = unr.actors.get_mut(a_idx) {
                    let name = a.name.clone();
                    let old_loc = a.location;
                    let snapped = Vec3::new(
                        (old_loc.x / step).round() * step,
                        (old_loc.y / step).round() * step,
                        (old_loc.z / step).round() * step,
                    );
                    if (snapped - old_loc).length_squared() > 1e-4 {
                        a.location = snapped;
                        self.history.push(crate::history::Action::MoveActor {
                            actor_index: a_idx,
                            old_location: old_loc,
                            new_location: snapped,
                        });
                        self.log(format!("Snapped Actor '{}' to grid at ({:.0}, {:.0}, {:.0}) (step {:.0} uu).", name, snapped.x, snapped.y, snapped.z, step));
                        return true;
                    } else {
                        self.log(format!("Actor '{}' is already on grid (step {:.0} uu).", name, step));
                        return false;
                    }
                }
            }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get_mut(m_idx) {
                let old_loc = m.brush.location;
                let snapped = Vec3::new(
                    (old_loc.x / step).round() * step,
                    (old_loc.y / step).round() * step,
                    (old_loc.z / step).round() * step,
                );
                if (snapped - old_loc).length_squared() > 1e-4 {
                    let delta = snapped - old_loc;
                    m.brush.location = snapped;
                    let cur_key = m.current_key;
                    if let Some(ref mut kf) = m.keyframes[cur_key] {
                        kf.location += delta;
                    }
                    let name = m.name.clone();
                    self.history.push(crate::history::Action::MoveMover {
                        mover_index: m_idx,
                        old_location: old_loc,
                        new_location: snapped,
                    });
                    self.log(format!("Snapped Mover '{}' to grid at ({:.0}, {:.0}, {:.0}) (step {:.0} uu).", name, snapped.x, snapped.y, snapped.z, step));
                    return true;
                }
            }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            if let Some(sm) = self.static_meshes.get_mut(sm_idx) {
                let old_loc = sm.location;
                let snapped = Vec3::new(
                    (old_loc.x / step).round() * step,
                    (old_loc.y / step).round() * step,
                    (old_loc.z / step).round() * step,
                );
                if (snapped - old_loc).length_squared() > 1e-4 {
                    sm.location = snapped;
                    let name = sm.name.clone();
                    self.history.push(crate::history::Action::MoveStaticMesh {
                        mesh_index: sm_idx,
                        old_location: old_loc,
                        new_location: snapped,
                    });
                    self.log(format!("Snapped Static Mesh '{}' to grid at ({:.0}, {:.0}, {:.0}) (step {:.0} uu).", name, snapped.x, snapped.y, snapped.z, step));
                    return true;
                }
            }
        } else {
            // Snap Active Red Builder Brush
            let old_loc = self.builder_brush.location;
            let snapped = Vec3::new(
                (old_loc.x / step).round() * step,
                (old_loc.y / step).round() * step,
                (old_loc.z / step).round() * step,
            );
            if (snapped - old_loc).length_squared() > 1e-4 {
                self.builder_brush.location = snapped;
                self.log(format!("Snapped Red Builder Brush to grid at ({:.0}, {:.0}, {:.0}) (step {:.0} uu).", snapped.x, snapped.y, snapped.z, step));
                return true;
            } else {
                self.log(format!("Red Builder Brush is already on grid (step {:.0} uu).", step));
                return false;
            }
        }

        false
    }

    pub fn snap_multi_selected_to_grid(&mut self) -> usize {
        let step = self.grid_step.max(1.0);
        let mut snapped_count = 0;
        let mut actions = Vec::new();

        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        for b_idx in brush_indices {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_loc = b.location;
                let snapped = Vec3::new(
                    (old_loc.x / step).round() * step,
                    (old_loc.y / step).round() * step,
                    (old_loc.z / step).round() * step,
                );
                if (snapped - old_loc).length_squared() > 1e-4 {
                    b.location = snapped;
                    actions.push(crate::history::Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: snapped,
                    });
                    snapped_count += 1;
                }
            }
        }

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        if let Some(ref mut unr) = self.loaded_unr {
            for a_idx in actor_indices {
                if let Some(a) = unr.actors.get_mut(a_idx) {
                    let old_loc = a.location;
                    let snapped = Vec3::new(
                        (old_loc.x / step).round() * step,
                        (old_loc.y / step).round() * step,
                        (old_loc.z / step).round() * step,
                    );
                    if (snapped - old_loc).length_squared() > 1e-4 {
                        a.location = snapped;
                        actions.push(crate::history::Action::MoveActor {
                            actor_index: a_idx,
                            old_location: old_loc,
                            new_location: snapped,
                        });
                        snapped_count += 1;
                    }
                }
            }
        }

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Snap {} objects to grid", snapped_count),
            });
        }

        if snapped_count > 0 {
            self.log(format!("Snapped {} object(s) to grid (step {:.0} uu) (Ctrl+Alt+G).", snapped_count, step));
        } else {
            self.log(format!("All selected objects already aligned to grid (step {:.0} uu).", step));
        }

        snapped_count
    }

    // --- Grid Step Management (UnrealEd 2.0 Parity) ---
    pub const CANONICAL_GRID_STEPS: [f32; 13] = [
        1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0, 1024.0, 2048.0, 4096.0,
    ];

    pub fn set_grid_step(&mut self, step: f32) {
        let step = step.clamp(1.0, 4096.0);
        self.grid_step = step;
        self.grid.step = step;
        self.log(format!("Grid Step: {:.0} uu", step));
    }

    pub fn step_grid_up(&mut self) -> f32 {
        let cur = self.grid_step;
        let mut target = Self::CANONICAL_GRID_STEPS[Self::CANONICAL_GRID_STEPS.len() - 1];
        for &s in &Self::CANONICAL_GRID_STEPS {
            if s > cur + 0.01 {
                target = s;
                break;
            }
        }
        self.set_grid_step(target);
        target
    }

    pub fn step_grid_down(&mut self) -> f32 {
        let cur = self.grid_step;
        let mut target = Self::CANONICAL_GRID_STEPS[0];
        for &s in Self::CANONICAL_GRID_STEPS.iter().rev() {
            if s < cur - 0.01 {
                target = s;
                break;
            }
        }
        self.set_grid_step(target);
        target
    }

    // --- Camera FOV Controls ---
    pub fn set_camera_fov_degrees(&mut self, deg: f32) {
        let deg = deg.clamp(30.0, 120.0);
        self.camera.fov_y = deg.to_radians();
        self.log(format!("Camera Field of View: {:.0}°", deg));
    }

    pub fn get_camera_fov_degrees(&self) -> f32 {
        self.camera.fov_y.to_degrees()
    }

    // --- Actor & Brush Group Management (UnrealEd 2.0 Group Browser Parity) ---
    pub fn get_map_groups(&self) -> Vec<(String, usize, usize)> {
        // Collect group names and counts: (group_name, actor_count, brush_count)
        let mut group_map: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();

        if let Some(ref level) = self.loaded_unr {
            for actor in &level.actors {
                let grp = actor.group.trim();
                if !grp.is_empty() {
                    let entry = group_map.entry(grp.to_string()).or_insert((0, 0));
                    entry.0 += 1;
                }
            }
        }

        for (_b_idx, grp) in &self.brush_groups {
            let trimmed = grp.trim();
            if !trimmed.is_empty() {
                let entry = group_map.entry(trimmed.to_string()).or_insert((0, 0));
                entry.1 += 1;
            }
        }

        let mut groups: Vec<(String, usize, usize)> = group_map
            .into_iter()
            .map(|(name, (a_cnt, b_cnt))| (name, a_cnt, b_cnt))
            .collect();

        groups.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
        groups
    }

    pub fn is_actor_group_visible(&self, group: &str) -> bool {
        let trimmed = group.trim();
        trimmed.is_empty() || !self.hidden_actor_groups.contains(trimmed)
    }

    pub fn is_brush_group_visible(&self, brush_idx: usize) -> bool {
        if let Some(grp) = self.brush_groups.get(&brush_idx) {
            let trimmed = grp.trim();
            trimmed.is_empty() || !self.hidden_actor_groups.contains(trimmed)
        } else {
            true
        }
    }

    pub fn toggle_group_visibility(&mut self, group: &str) -> bool {
        let trimmed = group.trim().to_string();
        if trimmed.is_empty() {
            return true;
        }
        let now_visible = if self.hidden_actor_groups.contains(&trimmed) {
            self.hidden_actor_groups.remove(&trimmed);
            true
        } else {
            self.hidden_actor_groups.insert(trimmed.clone());
            false
        };
        self.log(format!("Group '{}' visibility: {}", trimmed, if now_visible { "VISIBLE" } else { "HIDDEN" }));
        now_visible
    }

    pub fn set_all_groups_visibility(&mut self, visible: bool) {
        if visible {
            self.hidden_actor_groups.clear();
            self.log("All groups set to VISIBLE.");
        } else {
            for (name, _, _) in self.get_map_groups() {
                self.hidden_actor_groups.insert(name);
            }
            self.log("All groups set to HIDDEN.");
        }
    }

    // --- Per-Element Visibility & Lock Engine (Photoshop-Style Layers 01) ---

    pub fn is_brush_visible(&self, idx: usize) -> bool {
        !self.hidden_brushes.contains(&idx) && self.is_brush_group_visible(idx)
    }

    pub fn toggle_brush_visibility(&mut self, idx: usize) -> bool {
        let vis = if self.hidden_brushes.contains(&idx) {
            self.hidden_brushes.remove(&idx);
            true
        } else {
            self.hidden_brushes.insert(idx);
            false
        };
        self.map_is_dirty = true;
        vis
    }

    pub fn set_brush_visibility(&mut self, idx: usize, visible: bool) {
        if visible {
            self.hidden_brushes.remove(&idx);
        } else {
            self.hidden_brushes.insert(idx);
        }
        self.map_is_dirty = true;
    }

    pub fn is_actor_visible(&self, idx: usize) -> bool {
        if self.hidden_actors.contains(&idx) {
            return false;
        }
        if let Some(ref unr) = self.loaded_unr {
            if let Some(a) = unr.actors.get(idx) {
                return self.is_actor_group_visible(&a.group);
            }
        }
        true
    }

    pub fn toggle_actor_visibility(&mut self, idx: usize) -> bool {
        let vis = if self.hidden_actors.contains(&idx) {
            self.hidden_actors.remove(&idx);
            true
        } else {
            self.hidden_actors.insert(idx);
            false
        };
        self.map_is_dirty = true;
        vis
    }

    pub fn set_actor_visibility(&mut self, idx: usize, visible: bool) {
        if visible {
            self.hidden_actors.remove(&idx);
        } else {
            self.hidden_actors.insert(idx);
        }
        self.map_is_dirty = true;
    }

    pub fn is_mover_visible(&self, idx: usize) -> bool {
        !self.hidden_movers.contains(&idx)
    }

    pub fn toggle_mover_visibility(&mut self, idx: usize) -> bool {
        let vis = if self.hidden_movers.contains(&idx) {
            self.hidden_movers.remove(&idx);
            true
        } else {
            self.hidden_movers.insert(idx);
            false
        };
        self.map_is_dirty = true;
        vis
    }

    pub fn set_mover_visibility(&mut self, idx: usize, visible: bool) {
        if visible {
            self.hidden_movers.remove(&idx);
        } else {
            self.hidden_movers.insert(idx);
        }
        self.map_is_dirty = true;
    }

    pub fn is_static_mesh_visible(&self, idx: usize) -> bool {
        !self.hidden_static_meshes.contains(&idx)
    }

    pub fn toggle_static_mesh_visibility(&mut self, idx: usize) -> bool {
        let vis = if self.hidden_static_meshes.contains(&idx) {
            self.hidden_static_meshes.remove(&idx);
            true
        } else {
            self.hidden_static_meshes.insert(idx);
            false
        };
        self.map_is_dirty = true;
        vis
    }

    pub fn set_static_mesh_visibility(&mut self, idx: usize, visible: bool) {
        if visible {
            self.hidden_static_meshes.remove(&idx);
        } else {
            self.hidden_static_meshes.insert(idx);
        }
        self.map_is_dirty = true;
    }

    pub fn is_builder_visible(&self) -> bool {
        self.show_builder_brush
    }

    pub fn toggle_builder_visibility(&mut self) -> bool {
        self.show_builder_brush = !self.show_builder_brush;
        self.show_builder_brush
    }

    // --- Per-Element Lock Engine (02) ---

    pub fn is_brush_locked(&self, idx: usize) -> bool {
        self.locked_brushes.contains(&idx)
    }

    pub fn toggle_brush_lock(&mut self, idx: usize) -> bool {
        if self.locked_brushes.contains(&idx) {
            self.locked_brushes.remove(&idx);
            false
        } else {
            self.locked_brushes.insert(idx);
            true
        }
    }

    pub fn is_actor_locked(&self, idx: usize) -> bool {
        self.locked_actors.contains(&idx)
    }

    pub fn toggle_actor_lock(&mut self, idx: usize) -> bool {
        if self.locked_actors.contains(&idx) {
            self.locked_actors.remove(&idx);
            false
        } else {
            self.locked_actors.insert(idx);
            true
        }
    }

    pub fn is_mover_locked(&self, idx: usize) -> bool {
        self.locked_movers.contains(&idx)
    }

    pub fn toggle_mover_lock(&mut self, idx: usize) -> bool {
        if self.locked_movers.contains(&idx) {
            self.locked_movers.remove(&idx);
            false
        } else {
            self.locked_movers.insert(idx);
            true
        }
    }

    pub fn is_static_mesh_locked(&self, idx: usize) -> bool {
        self.locked_static_meshes.contains(&idx)
    }

    pub fn toggle_static_mesh_lock(&mut self, idx: usize) -> bool {
        if self.locked_static_meshes.contains(&idx) {
            self.locked_static_meshes.remove(&idx);
            false
        } else {
            self.locked_static_meshes.insert(idx);
            true
        }
    }

    pub fn toggle_builder_lock(&mut self) -> bool {
        self.is_builder_locked = !self.is_builder_locked;
        self.is_builder_locked
    }

    pub fn show_all_elements(&mut self) {
        self.hidden_brushes.clear();
        self.hidden_actors.clear();
        self.hidden_movers.clear();
        self.hidden_static_meshes.clear();
        self.hidden_actor_groups.clear();
        self.show_builder_brush = true;
        self.log("All elements set to VISIBLE.");
    }

    pub fn hide_all_elements(&mut self) {
        for i in 0..self.world_brushes.len() {
            self.hidden_brushes.insert(i);
        }
        if let Some(ref unr) = self.loaded_unr {
            for i in 0..unr.actors.len() {
                self.hidden_actors.insert(i);
            }
        }
        for i in 0..self.movers.len() {
            self.hidden_movers.insert(i);
        }
        for i in 0..self.static_meshes.len() {
            self.hidden_static_meshes.insert(i);
        }
        self.show_builder_brush = false;
        self.log("All elements set to HIDDEN.");
    }

    pub fn unlock_all_elements(&mut self) {
        self.locked_brushes.clear();
        self.locked_actors.clear();
        self.locked_movers.clear();
        self.locked_static_meshes.clear();
        self.is_builder_locked = false;
        self.selection_locked = false;
        self.log("All elements UNLOCKED.");
    }

    pub fn select_objects_by_group(&mut self, group: &str) -> (usize, usize) {
        let target = group.trim();
        if target.is_empty() {
            return (0, 0);
        }

        let mut selected_actors = 0;
        if let Some(ref level) = self.loaded_unr {
            for (i, actor) in level.actors.iter().enumerate() {
                if actor.group.trim().eq_ignore_ascii_case(target) {
                    self.multi_selected_actors.insert(i);
                    selected_actors += 1;
                }
            }
        }

        let mut selected_brushes = 0;
        for (&b_idx, grp) in &self.brush_groups {
            if grp.trim().eq_ignore_ascii_case(target) && b_idx < self.world_brushes.len() {
                self.multi_selected_brushes.insert(b_idx);
                selected_brushes += 1;
            }
        }

        if selected_actors > 0 && self.selected_actor_index.is_none() {
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        }
        if selected_brushes > 0 && self.selected_brush_index.is_none() {
            self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        }

        self.log(format!("Selected group '{}': {} actor(s), {} brush(es).", target, selected_actors, selected_brushes));
        (selected_actors, selected_brushes)
    }

    pub fn deselect_objects_by_group(&mut self, group: &str) -> (usize, usize) {
        let target = group.trim();
        if target.is_empty() {
            return (0, 0);
        }

        let mut deselected_actors = 0;
        if let Some(ref level) = self.loaded_unr {
            for (i, actor) in level.actors.iter().enumerate() {
                if actor.group.trim().eq_ignore_ascii_case(target) {
                    if self.multi_selected_actors.remove(&i) {
                        deselected_actors += 1;
                    }
                }
            }
        }

        let mut deselected_brushes = 0;
        for (&b_idx, grp) in &self.brush_groups {
            if grp.trim().eq_ignore_ascii_case(target) {
                if self.multi_selected_brushes.remove(&b_idx) {
                    deselected_brushes += 1;
                }
            }
        }

        if let Some(sel_a) = self.selected_actor_index {
            if !self.multi_selected_actors.contains(&sel_a) {
                self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
            }
        }
        if let Some(sel_b) = self.selected_brush_index {
            if !self.multi_selected_brushes.contains(&sel_b) {
                self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
            }
        }

        self.log(format!("Deselected group '{}': {} actor(s), {} brush(es).", target, deselected_actors, deselected_brushes));
        (deselected_actors, deselected_brushes)
    }

    pub fn set_selected_objects_group(&mut self, group: &str) -> (usize, usize) {
        let target = group.trim().to_string();

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        let mut actors_updated = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for a_idx in actor_indices {
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    actor.group = target.clone();
                    actors_updated += 1;
                }
            }
        }

        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        let mut brushes_updated = 0;
        for b_idx in brush_indices {
            if b_idx < self.world_brushes.len() {
                if target.is_empty() {
                    self.world_brushes[b_idx].group = None;
                    self.brush_groups.remove(&b_idx);
                } else {
                    self.world_brushes[b_idx].group = Some(target.clone());
                    self.brush_groups.insert(b_idx, target.clone());
                }
                brushes_updated += 1;
            }
        }

        self.map_is_dirty = true;
        self.log(format!("Assigned group '{}' to {} actor(s) and {} brush(es).", target, actors_updated, brushes_updated));
        (actors_updated, brushes_updated)
    }

    pub fn remove_selected_from_group(&mut self) -> (usize, usize) {
        self.set_selected_objects_group("")
    }

    pub fn rename_group(&mut self, old_name: &str, new_name: &str) -> (usize, usize) {
        let old = old_name.trim();
        let new = new_name.trim();
        if old.is_empty() || new.is_empty() || old == new {
            return (0, 0);
        }

        let mut actors_updated = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for actor in &mut unr.actors {
                if actor.group.trim().eq_ignore_ascii_case(old) {
                    actor.group = new.to_string();
                    actors_updated += 1;
                }
            }
        }

        let mut brushes_updated = 0;
        for (_idx, grp) in &mut self.brush_groups {
            if grp.trim().eq_ignore_ascii_case(old) {
                *grp = new.to_string();
                brushes_updated += 1;
            }
        }
        for b in &mut self.world_brushes {
            if let Some(ref mut grp) = b.group {
                if grp.trim().eq_ignore_ascii_case(old) {
                    *grp = new.to_string();
                }
            }
        }

        if self.hidden_actor_groups.remove(old) {
            self.hidden_actor_groups.insert(new.to_string());
        }

        self.map_is_dirty = true;
        self.log(format!("Renamed group '{}' to '{}' across {} actor(s) and {} brush(es).", old, new, actors_updated, brushes_updated));
        (actors_updated, brushes_updated)
    }

    pub fn delete_group(&mut self, group: &str) -> (usize, usize) {
        let target = group.trim();
        if target.is_empty() {
            return (0, 0);
        }

        let mut actors_cleared = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for actor in &mut unr.actors {
                if actor.group.trim().eq_ignore_ascii_case(target) {
                    actor.group.clear();
                    actors_cleared += 1;
                }
            }
        }

        let mut brushes_cleared = 0;
        self.brush_groups.retain(|_, grp| {
            if grp.trim().eq_ignore_ascii_case(target) {
                brushes_cleared += 1;
                false
            } else {
                true
            }
        });
        for b in &mut self.world_brushes {
            if b.group
                .as_ref()
                .map(|g| g.trim().eq_ignore_ascii_case(target))
                .unwrap_or(false)
            {
                b.group = None;
            }
        }

        self.hidden_actor_groups.remove(target);
        self.map_is_dirty = true;
        self.log(format!("Deleted group '{}' (cleared {} actor(s) and {} brush(es)).", target, actors_cleared, brushes_cleared));
        (actors_cleared, brushes_cleared)
    }

    // --- Camera Focus & Targeting Operations ---
    pub fn focus_camera_on(&mut self, target: Vec3) {
        self.camera.focus_on(target, 400.0);
        self.log(format!("Camera focused on target at ({:.0}, {:.0}, {:.0}).", target.x, target.y, target.z));
    }

    // --- Brush Scaling Dialog Operations (UnrealEd 2.0 Parity) ---
    pub fn scale_selected_brush_geometry(&mut self, scale: Vec3, adjust_textures: bool) -> bool {
        if scale.x.abs() < 1e-4 || scale.y.abs() < 1e-4 || scale.z.abs() < 1e-4 {
            self.log("Cannot scale brush by zero.");
            return false;
        }

        if self.has_multi_selection() {
            let mut count = 0;
            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    scale_brush_geometry(b, scale, adjust_textures);
                    let new_polys = b.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Scale geometry ({:.2}, {:.2}, {:.2})", scale.x, scale.y, scale.z),
                    });
                    count += 1;
                }
            }
            self.map_is_dirty = true;
            self.log(format!("Scaled geometry of {} brush(es) by ({:.2}, {:.2}, {:.2}).", count, scale.x, scale.y, scale.z));
            count > 0
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_polys = b.polys.clone();
                scale_brush_geometry(b, scale, adjust_textures);
                let new_polys = b.polys.clone();
                let name = b.name.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Scale geometry ({:.2}, {:.2}, {:.2})", scale.x, scale.y, scale.z),
                });
                self.map_is_dirty = true;
                self.log(format!("Scaled geometry of Brush '{}' by ({:.2}, {:.2}, {:.2}).", name, scale.x, scale.y, scale.z));
                true
            } else {
                false
            }
        } else {
            // Scale Active Red Builder Brush
            scale_brush_geometry(&mut self.builder_brush, scale, adjust_textures);
            self.log(format!("Scaled Red Builder Brush geometry by ({:.2}, {:.2}, {:.2}).", scale.x, scale.y, scale.z));
            true
        }
    }

    pub fn scale_selected_transform(&mut self, scale: Vec3) -> bool {
        if scale.x.abs() < 1e-4 || scale.y.abs() < 1e-4 || scale.z.abs() < 1e-4 {
            self.log("Cannot scale transform by zero.");
            return false;
        }

        if self.has_multi_selection() {
            let mut count = 0;
            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    b.scale.x *= scale.x;
                    b.scale.y *= scale.y;
                    b.scale.z *= scale.z;
                    count += 1;
                }
            }
            self.map_is_dirty = true;
            self.log(format!("Scaled transform of {} brush(es) by ({:.2}, {:.2}, {:.2}).", count, scale.x, scale.y, scale.z));
            count > 0
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                b.scale.x *= scale.x;
                b.scale.y *= scale.y;
                b.scale.z *= scale.z;
                let name = b.name.clone();
                let new_scale = b.scale;
                self.map_is_dirty = true;
                self.log(format!("Scaled transform of Brush '{}' to ({:.2}, {:.2}, {:.2}).", name, new_scale.x, new_scale.y, new_scale.z));
                true
            } else {
                false
            }
        } else {
            self.builder_brush.scale.x *= scale.x;
            self.builder_brush.scale.y *= scale.y;
            self.builder_brush.scale.z *= scale.z;
            self.log(format!("Scaled Red Builder Brush transform to ({:.2}, {:.2}, {:.2}).", self.builder_brush.scale.x, self.builder_brush.scale.y, self.builder_brush.scale.z));
            true
        }
    }

    pub fn scale_selected(&mut self, scale: Vec3, mode: BrushScaleMode, adjust_textures: bool) -> bool {
        match mode {
            BrushScaleMode::GeometryVertices => self.scale_selected_brush_geometry(scale, adjust_textures),
            BrushScaleMode::ActorTransform => self.scale_selected_transform(scale),
        }
    }

    // --- Actor Search / Find Actors Dialog Operations (UnrealEd 2.0 Parity) ---
    pub fn find_actors_matching(&self, query: &str, category: FindActorCategory) -> Vec<ActorSearchResult> {
        let mut results = Vec::new();
        let q = query.trim().to_lowercase();
        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                let cat_match = match category {
                    FindActorCategory::All => true,
                    FindActorCategory::Lights => {
                        let c = actor.class_name.to_lowercase();
                        c.contains("light") || c.contains("sunlight")
                    }
                    FindActorCategory::Navigation => {
                        let c = actor.class_name.to_lowercase();
                        (c.contains("path") || c.contains("playerstart") || c.contains("inventoryspot") || c.contains("teleport") || c.contains("lift") || c.contains("navigation")) && !c.contains("spotlight")
                    }
                    FindActorCategory::Triggers => {
                        let c = actor.class_name.to_lowercase();
                        c.contains("trigger") || c.contains("dispatcher") || c.contains("counter") || c.contains("specialevent")
                    }
                    FindActorCategory::Inventory => {
                        let c = actor.class_name.to_lowercase();
                        c.contains("weapon") || c.contains("pickup") || c.contains("ammo") || c.contains("armor") || c.contains("relic") || c.contains("health") || c.contains("flak") || c.contains("shock") || c.contains("sniper") || c.contains("rifle") || c.contains("cannon") || c.contains("gun") || c.contains("launcher") || c.contains("inventory")
                    }
                    FindActorCategory::Movers => {
                        let c = actor.class_name.to_lowercase();
                        c.contains("mover")
                    }
                    FindActorCategory::Sounds => {
                        let c = actor.class_name.to_lowercase();
                        c.contains("sound")
                    }
                    FindActorCategory::Other => {
                        let c = actor.class_name.to_lowercase();
                        !c.contains("light") && !c.contains("path") && !c.contains("playerstart") && !c.contains("trigger") && !c.contains("weapon") && !c.contains("pickup") && !c.contains("mover") && !c.contains("sound")
                    }
                };

                if !cat_match {
                    continue;
                }

                if q.is_empty()
                    || actor.name.to_lowercase().contains(&q)
                    || actor.class_name.to_lowercase().contains(&q)
                    || actor.tag.to_lowercase().contains(&q)
                    || actor.event.to_lowercase().contains(&q)
                    || actor.group.to_lowercase().contains(&q)
                {
                    results.push(ActorSearchResult {
                        actor_index: idx,
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        tag: actor.tag.clone(),
                        event: actor.event.clone(),
                        group: actor.group.clone(),
                        location: actor.location,
                    });
                }
            }
        }
        results
    }

    pub fn select_and_focus_actor(&mut self, actor_index: usize) -> bool {
        if let Some(ref unr) = self.loaded_unr {
            if let Some(actor) = unr.actors.get(actor_index) {
                let loc = actor.location;
                let name = actor.name.clone();
                self.clear_multi_selection();
                self.selected_brush_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
                self.selected_actor_index = Some(actor_index);
                self.focus_camera_on(loc);
                self.log(format!("Selected & focused on Actor #{}: '{}' at ({:.0}, {:.0}, {:.0}).", actor_index, name, loc.x, loc.y, loc.z));
                return true;
            }
        }
        false
    }

    pub fn select_all_matching_actors(&mut self, query: &str, category: FindActorCategory) -> usize {
        let matches = self.find_actors_matching(query, category);
        let count = matches.len();
        self.clear_multi_selection();
        self.selected_brush_index = None;
        for m in &matches {
            self.multi_selected_actors.insert(m.actor_index);
        }
        if let Some(first) = matches.first() {
            self.selected_actor_index = Some(first.actor_index);
        }
        self.log(format!("Selected all {} matching actor(s) for query '{}'.", count, query));
        count
    }

    // --- Brush Shearing Operations (UnrealEd 2.0 Parity) ---
    pub fn shear_selected(&mut self, axis: ShearAxis, factor: f32, anchor_to_base: bool) -> bool {
        if factor.abs() < 1e-4 {
            self.log("Shear factor is zero, no geometry change applied.");
            return false;
        }

        let axis_str = axis.label();

        if self.has_multi_selection() {
            let mut count = 0;
            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_polys = b.polys.clone();
                    shear_brush_geometry(b, axis, factor, anchor_to_base);
                    let new_polys = b.polys.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Shear ({axis_str}, factor {factor:.2})"),
                    });
                    count += 1;
                }
            }
            self.map_is_dirty = true;
            self.log(format!("Sheared {} brush(es) with {} by {:.2}.", count, axis_str, factor));
            count > 0
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_polys = b.polys.clone();
                shear_brush_geometry(b, axis, factor, anchor_to_base);
                let new_polys = b.polys.clone();
                let name = b.name.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: format!("Shear ({axis_str}, factor {factor:.2})"),
                });
                self.map_is_dirty = true;
                self.log(format!("Sheared Brush '{}' with {} by {:.2}.", name, axis_str, factor));
                true
            } else {
                false
            }
        } else {
            // Shear Active Red Builder Brush
            shear_brush_geometry(&mut self.builder_brush, axis, factor, anchor_to_base);
            self.log(format!("Sheared Red Builder Brush with {} by {:.2}.", axis_str, factor));
            true
        }
    }

    // --- Actor Alignment & Distribution Operations (UnrealEd 2.0 Parity) ---
    pub fn align_selected_actors(&mut self, axis: AlignAxis, mode: AlignMode) -> usize {
        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        if actor_indices.len() < 2 {
            self.log("Actor Alignment requires at least 2 selected actors.");
            return 0;
        }

        let unr = match self.loaded_unr.as_ref() {
            Some(u) => u,
            None => return 0,
        };

        let coords: Vec<f32> = actor_indices.iter().filter_map(|&idx| {
            unr.actors.get(idx).map(|a| match axis {
                AlignAxis::X => a.location.x,
                AlignAxis::Y => a.location.y,
                AlignAxis::Z => a.location.z,
            })
        }).collect();

        if coords.is_empty() {
            return 0;
        }

        let target_val = match mode {
            AlignMode::Min => coords.iter().cloned().fold(f32::INFINITY, f32::min),
            AlignMode::Max => coords.iter().cloned().fold(f32::NEG_INFINITY, f32::max),
            AlignMode::Center => coords.iter().sum::<f32>() / (coords.len() as f32),
        };

        let mut actions = Vec::new();
        let mut aligned = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for &a_idx in &actor_indices {
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let old_loc = actor.location;
                    match axis {
                        AlignAxis::X => actor.location.x = target_val,
                        AlignAxis::Y => actor.location.y = target_val,
                        AlignAxis::Z => actor.location.z = target_val,
                    }
                    let new_loc = actor.location;
                    if (old_loc - new_loc).length_squared() > 1e-4 {
                        actions.push(Action::MoveActor {
                            actor_index: a_idx,
                            old_location: old_loc,
                            new_location: new_loc,
                        });
                        aligned += 1;
                    }
                }
            }
        }

        let axis_str = match axis { AlignAxis::X => "X", AlignAxis::Y => "Y", AlignAxis::Z => "Z" };
        let mode_str = match mode { AlignMode::Min => "Min", AlignMode::Center => "Center", AlignMode::Max => "Max" };

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(Action::Compound {
                actions,
                description: format!("Align {} actors to {} ({}) = {:.1}", aligned, axis_str, mode_str, target_val),
            });
        }

        self.map_is_dirty = true;
        self.log(format!("Aligned {} actor(s) to {} ({}) = {:.1}.", aligned, axis_str, mode_str, target_val));
        aligned
    }

    pub fn distribute_selected_actors_evenly(&mut self, axis: AlignAxis) -> usize {
        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.len() < 3 {
            self.log("Even distribution requires at least 3 selected actors.");
            return 0;
        }

        let unr = match self.loaded_unr.as_ref() {
            Some(u) => u,
            None => return 0,
        };

        actor_indices.sort_by(|&a, &b| {
            let val_a = unr.actors.get(a).map(|act| match axis {
                AlignAxis::X => act.location.x,
                AlignAxis::Y => act.location.y,
                AlignAxis::Z => act.location.z,
            }).unwrap_or(0.0);
            let val_b = unr.actors.get(b).map(|act| match axis {
                AlignAxis::X => act.location.x,
                AlignAxis::Y => act.location.y,
                AlignAxis::Z => act.location.z,
            }).unwrap_or(0.0);
            val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let n = actor_indices.len();
        let min_val = match axis {
            AlignAxis::X => unr.actors[actor_indices[0]].location.x,
            AlignAxis::Y => unr.actors[actor_indices[0]].location.y,
            AlignAxis::Z => unr.actors[actor_indices[0]].location.z,
        };
        let max_val = match axis {
            AlignAxis::X => unr.actors[actor_indices[n - 1]].location.x,
            AlignAxis::Y => unr.actors[actor_indices[n - 1]].location.y,
            AlignAxis::Z => unr.actors[actor_indices[n - 1]].location.z,
        };

        let step = (max_val - min_val) / ((n - 1) as f32);
        let mut actions = Vec::new();
        let mut distributed = 0;

        if let Some(ref mut unr) = self.loaded_unr {
            for (i, &a_idx) in actor_indices.iter().enumerate() {
                let target = min_val + step * (i as f32);
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let old_loc = actor.location;
                    match axis {
                        AlignAxis::X => actor.location.x = target,
                        AlignAxis::Y => actor.location.y = target,
                        AlignAxis::Z => actor.location.z = target,
                    }
                    let new_loc = actor.location;
                    if (old_loc - new_loc).length_squared() > 1e-4 {
                        actions.push(Action::MoveActor {
                            actor_index: a_idx,
                            old_location: old_loc,
                            new_location: new_loc,
                        });
                        distributed += 1;
                    }
                }
            }
        }

        let axis_str = match axis { AlignAxis::X => "X", AlignAxis::Y => "Y", AlignAxis::Z => "Z" };

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(Action::Compound {
                actions,
                description: format!("Distribute {} actors evenly along {}", distributed, axis_str),
            });
        }

        self.map_is_dirty = true;
        self.log(format!("Distributed {} actor(s) evenly along {} (spacing: {:.1} uu).", distributed, axis_str, step));
        distributed
    }

    pub fn distribute_selected_actors_spacing(&mut self, axis: AlignAxis, spacing: f32) -> usize {
        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.len() < 2 {
            self.log("Fixed spacing distribution requires at least 2 selected actors.");
            return 0;
        }

        let unr = match self.loaded_unr.as_ref() {
            Some(u) => u,
            None => return 0,
        };

        actor_indices.sort_by(|&a, &b| {
            let val_a = unr.actors.get(a).map(|act| match axis {
                AlignAxis::X => act.location.x,
                AlignAxis::Y => act.location.y,
                AlignAxis::Z => act.location.z,
            }).unwrap_or(0.0);
            let val_b = unr.actors.get(b).map(|act| match axis {
                AlignAxis::X => act.location.x,
                AlignAxis::Y => act.location.y,
                AlignAxis::Z => act.location.z,
            }).unwrap_or(0.0);
            val_a.partial_cmp(&val_b).unwrap_or(std::cmp::Ordering::Equal)
        });

        let start_val = match axis {
            AlignAxis::X => unr.actors[actor_indices[0]].location.x,
            AlignAxis::Y => unr.actors[actor_indices[0]].location.y,
            AlignAxis::Z => unr.actors[actor_indices[0]].location.z,
        };

        let mut actions = Vec::new();
        let mut updated = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for (i, &a_idx) in actor_indices.iter().enumerate() {
                let target = start_val + spacing * (i as f32);
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let old_loc = actor.location;
                    match axis {
                        AlignAxis::X => actor.location.x = target,
                        AlignAxis::Y => actor.location.y = target,
                        AlignAxis::Z => actor.location.z = target,
                    }
                    let new_loc = actor.location;
                    if (old_loc - new_loc).length_squared() > 1e-4 {
                        actions.push(Action::MoveActor {
                            actor_index: a_idx,
                            old_location: old_loc,
                            new_location: new_loc,
                        });
                        updated += 1;
                    }
                }
            }
        }

        let axis_str = match axis { AlignAxis::X => "X", AlignAxis::Y => "Y", AlignAxis::Z => "Z" };

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(Action::Compound {
                actions,
                description: format!("Distribute {} actors along {} with spacing {:.1} uu", updated, axis_str, spacing),
            });
        }

        self.map_is_dirty = true;
        self.log(format!("Distributed {} actor(s) along {} with spacing {:.1} uu.", updated, axis_str, spacing));
        updated
    }

    // --- Map Statistics & Metrics (UnrealEd 2.0 Parity) ---
    pub fn compute_map_statistics(&self) -> MapStatistics {
        let mut stats = MapStatistics::default();
        stats.total_brushes = self.world_brushes.len();
        for b in &self.world_brushes {
            match b.csg_oper {
                ut99_bsp::CsgOperation::Add => stats.solid_brushes += 1,
                ut99_bsp::CsgOperation::Subtract => stats.subtract_brushes += 1,
                ut99_bsp::CsgOperation::Intersect | ut99_bsp::CsgOperation::Deintersect | ut99_bsp::CsgOperation::Active => stats.solid_brushes += 1,
            }
            stats.total_polys += b.polys.len();
            for p in &b.polys {
                stats.total_vertices += p.vertices.len();
            }
        }

        let mut textures = std::collections::HashSet::new();
        for b in &self.world_brushes {
            for p in &b.polys {
                if !p.texture.is_empty() {
                    textures.insert(&p.texture);
                }
            }
        }
        stats.unique_textures_count = textures.len();

        if let Some(ref unr) = self.loaded_unr {
            stats.total_actors = unr.actors.len();
            for a in &unr.actors {
                let c = a.class_name.to_lowercase();
                if c.contains("light") {
                    stats.lights_count += 1;
                } else if c.contains("path") || c.contains("playerstart") || c.contains("spot") || c.contains("teleport") {
                    stats.nav_points_count += 1;
                } else if c.contains("weapon") || c.contains("pickup") || c.contains("ammo") || c.contains("armor") || c.contains("flak") || c.contains("shock") {
                    stats.weapons_pickups_count += 1;
                } else if c.contains("trigger") || c.contains("dispatcher") {
                    stats.triggers_count += 1;
                } else if c.contains("mover") {
                    stats.movers_count += 1;
                }
            }
        }

        stats.movers_count += self.movers.len();
        stats.static_meshes_count = self.static_meshes.len();

        stats
    }

    pub fn apply_active_texture_to_multi_selected(&mut self) -> usize {
        let tex = match &self.active_texture {
            Some(t) => t.clone(),
            None => {
                self.log("Apply Texture: No active texture selected in browser.");
                return 0;
            }
        };

        let mut target_brushes: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if target_brushes.is_empty() {
            if let Some(b) = self.selected_brush_index {
                target_brushes.push(b);
            }
        }

        let mut actions = Vec::new();
        let mut updated = 0;
        for b_idx in target_brushes {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                for poly in &mut brush.polys {
                    poly.texture = tex.clone();
                }
                actions.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys: brush.polys.clone(),
                    description: format!("Batch applied texture '{}'", tex),
                });
                updated += 1;
            }
        }

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Batch applied texture '{}' to {} brushes", tex, updated),
            });
        }

        self.log(format!("Applied texture '{}' to {} brush(es).", tex, updated));
        updated
    }

    pub fn duplicate_selected_batch(&mut self, offset: Vec3) -> (usize, usize) {
        let mut actions = Vec::new();
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        let mut new_brush_indices = HashSet::new();
        for b_idx in brush_indices {
            if let Some(brush) = self.world_brushes.get(b_idx).cloned() {
                let mut new_b = brush;
                new_b.location += offset;
                let count = self.world_brushes.len();
                new_b.name = format!("{}_{}", new_b.name.split('_').next().unwrap_or("Brush"), count);
                self.world_brushes.push(new_b.clone());
                let new_idx = self.world_brushes.len() - 1;
                actions.push(crate::history::Action::AddBrush { brush: new_b });
                new_brush_indices.insert(new_idx);
            }
        }

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }

        let mut new_actor_indices = HashSet::new();
        if let Some(ref mut unr) = self.loaded_unr {
            for a_idx in actor_indices {
                if let Some(actor) = unr.actors.get(a_idx).cloned() {
                    let mut new_act = actor;
                    new_act.location += offset;
                    let count = unr.actors.len();
                    new_act.name = format!("{}_{}", new_act.class_name, count);
                    if !new_act.tag.is_empty() {
                        let inc_tag = Self::increment_numbered_tag(&new_act.tag);
                        new_act.tag = inc_tag.clone();
                        if let Some(prop) = new_act.properties.get_mut("Tag") {
                            prop.value = ut99_format::TaggedValue::Name(inc_tag);
                        }
                    }
                    unr.actors.push(new_act.clone());
                    let new_idx = unr.actors.len() - 1;
                    actions.push(crate::history::Action::AddActor { actor: new_act });
                    new_actor_indices.insert(new_idx);
                }
            }
        }

        let b_count = new_brush_indices.len();
        let a_count = new_actor_indices.len();

        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Duplicate batch ({} brushes, {} actors)", b_count, a_count),
            });
        }

        self.multi_selected_brushes = new_brush_indices;
        self.multi_selected_actors = new_actor_indices;
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();

        self.log(format!("Duplicated batch: {} brush(es), {} actor(s).", b_count, a_count));
        self.toast_success(format!("Duplicated batch: {} brush(es), {} actor(s)", b_count, a_count));
        (b_count, a_count)
    }

    pub fn create_prefab_from_selection(&mut self, name: &str, category: &str) -> bool {
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }

        if brush_indices.is_empty() {
            self.log("Create Prefab: No brushes selected.");
            return false;
        }

        let mut prefab_brushes = Vec::new();
        let mut sum_loc = Vec3::ZERO;
        for &idx in &brush_indices {
            if let Some(b) = self.world_brushes.get(idx) {
                sum_loc += b.location;
            }
        }
        let center = sum_loc / (brush_indices.len() as f32);

        for &idx in &brush_indices {
            if let Some(b) = self.world_brushes.get(idx).cloned() {
                let mut rel_brush = b;
                rel_brush.location -= center;
                prefab_brushes.push(rel_brush);
            }
        }

        let prefab = crate::prefab::Prefab {
            name: if name.is_empty() { format!("Prefab_{}", self.prefabs.len()) } else { name.to_string() },
            category: if category.is_empty() { "Custom".to_string() } else { category.to_string() },
            description: format!("Custom composite prefab with {} brushes", prefab_brushes.len()),
            brushes: prefab_brushes,
            movers: Vec::new(),
            actors: Vec::new(),
        };

        self.prefabs.push(prefab);
        self.log(format!("Created Prefab '{}' in category '{}'.", name, category));
        true
    }


    pub fn snap_selected_brush_pivot_to_nearest_vertex(&mut self, cursor_world_pos: Option<Vec3>) {
        if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let world_polys = b.to_world_polys();
                let mut unique_world_verts: Vec<Vec3> = Vec::new();
                for p in &world_polys {
                    for &v in &p.vertices {
                        if !unique_world_verts.iter().any(|&u| (u - v).length_squared() < 1e-4) {
                            unique_world_verts.push(v);
                        }
                    }
                }

                if unique_world_verts.is_empty() {
                    return;
                }

                let target_vert = if let Some(ref_pos) = cursor_world_pos {
                    unique_world_verts
                        .iter()
                        .min_by(|&&a, &&b| a.distance_squared(ref_pos).total_cmp(&b.distance_squared(ref_pos)))
                        .copied()
                        .unwrap_or(unique_world_verts[0])
                } else {
                    unique_world_verts
                        .iter()
                        .min_by(|&&a, &&b| (a.x + a.y + a.z).total_cmp(&(b.x + b.y + b.z)))
                        .copied()
                        .unwrap_or(unique_world_verts[0])
                };

                let l_old = b.location;
                let l_new = target_vert;
                let old_polys = b.polys.clone();

                let rad_pitch = b.rotation.x.to_radians();
                let rad_yaw = b.rotation.y.to_radians();
                let rad_roll = b.rotation.z.to_radians();
                let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                let scale_matrix = glam::Mat4::from_scale(b.scale);
                let rot_scale = rot_matrix * scale_matrix;
                let inv_rot_scale = rot_scale.inverse();
                let local_offset = inv_rot_scale.transform_vector3(l_old - l_new);

                for poly in &mut b.polys {
                    for v in &mut poly.vertices {
                        *v += local_offset;
                    }
                    poly.base += local_offset;
                }
                b.location = l_new;

                self.history.push(crate::history::Action::ModifyBrushPivot {
                    brush_index: b_idx,
                    old_location: l_old,
                    new_location: l_new,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Snap Pivot to Vertex ({:.1}, {:.1}, {:.1})", l_new.x, l_new.y, l_new.z),
                });
                self.map_is_dirty = true;
                self.log(format!(
                    "Snapped Brush #{} pivot to vertex at ({:.1}, {:.1}, {:.1}) (Shift+End).",
                    b_idx, l_new.x, l_new.y, l_new.z
                ));
            }
        } else {
            let b = &mut self.builder_brush;
            let world_polys = b.to_world_polys();
            let mut unique_world_verts: Vec<Vec3> = Vec::new();
            for p in &world_polys {
                for &v in &p.vertices {
                    if !unique_world_verts.iter().any(|&u| (u - v).length_squared() < 1e-4) {
                        unique_world_verts.push(v);
                    }
                }
            }

            if let Some(target_vert) = unique_world_verts.first().copied() {
                let l_old = b.location;
                let l_new = target_vert;
                let local_offset = l_old - l_new;

                for poly in &mut b.polys {
                    for v in &mut poly.vertices {
                        *v += local_offset;
                    }
                    poly.base += local_offset;
                }
                b.location = l_new;
                self.log(format!("Snapped Builder Brush pivot to vertex at ({:.1}, {:.1}, {:.1}).", l_new.x, l_new.y, l_new.z));
            }
        }
    }

    pub fn snap_brush_pivot_to_center(&mut self) {
        if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let (min_b, max_b) = b.world_bounds();
                let center = (min_b + max_b) * 0.5;

                let l_old = b.location;
                let l_new = center;
                let old_polys = b.polys.clone();

                let rad_pitch = b.rotation.x.to_radians();
                let rad_yaw = b.rotation.y.to_radians();
                let rad_roll = b.rotation.z.to_radians();
                let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                let scale_matrix = glam::Mat4::from_scale(b.scale);
                let rot_scale = rot_matrix * scale_matrix;
                let inv_rot_scale = rot_scale.inverse();
                let local_offset = inv_rot_scale.transform_vector3(l_old - l_new);

                for poly in &mut b.polys {
                    for v in &mut poly.vertices {
                        *v += local_offset;
                    }
                    poly.base += local_offset;
                }
                b.location = l_new;

                self.history.push(crate::history::Action::ModifyBrushPivot {
                    brush_index: b_idx,
                    old_location: l_old,
                    new_location: l_new,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: format!("Center Pivot to Bounding Box ({:.1}, {:.1}, {:.1})", l_new.x, l_new.y, l_new.z),
                });
                self.map_is_dirty = true;
                self.log(format!("Centered Brush #{} pivot to bounding box ({:.1}, {:.1}, {:.1}).", b_idx, l_new.x, l_new.y, l_new.z));
            }
        }
    }

    /// Extracts the currently selected surfaces (or whole selected brush) into the Red Builder Brush
    /// as a planar sheet or custom polyhedral brush, aligned at its world centroid.
    pub fn get_gizmo_scale(&self, center: Vec3, vp_type: crate::camera::ViewportType, vp_height: f32) -> f32 {
        let vh = vp_height.max(100.0);
        match vp_type {
            crate::camera::ViewportType::Perspective => {
                let dist = (center - self.camera.position).length().max(10.0);
                let fov_factor = 2.0 * (self.camera.fov_y * 0.5).tan();
                (80.0 * dist * fov_factor / vh).clamp(12.0, 4000.0)
            }
            _ => {
                let ortho_zoom = self.camera.get_ortho_zoom(vp_type);
                let zoom = if ortho_zoom != 1024.0 {
                    ortho_zoom
                } else {
                    self.camera.ortho_zoom
                };
                (80.0 * 2.0 * zoom / vh).clamp(12.0, 4000.0)
            }
        }
    }

    /// Computes both adaptive axis length and generous proportional hit-testing radius for transform gizmos.
    pub fn get_gizmo_scale_and_hit_radius(&self, center: Vec3, vp_type: crate::camera::ViewportType, vp_height: f32) -> (f32, f32) {
        let scale = self.get_gizmo_scale(center, vp_type, vp_height);
        let hit_radius = (scale * 0.22).clamp(8.0, 400.0);
        (scale, hit_radius)
    }

    /// Returns the orientation matrix for the currently selected object when in Local coordinate space,
    /// or Mat3::IDENTITY when in World coordinate space.
    pub fn get_selected_rotation_matrix(&self) -> glam::Mat3 {
        if self.gizmo_space == GizmoCoordinateSpace::World {
            return glam::Mat3::IDENTITY;
        }

        if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get(m_idx) {
                return euler_degrees_to_mat3(m.brush.rotation.x, m.brush.rotation.y, m.brush.rotation.z);
            }
        } else if let Some(a_idx) = self.selected_actor_index {
            if let Some(u) = &self.loaded_unr {
                if let Some(a) = u.actors.get(a_idx) {
                    return unreal_rotator_to_mat3(a.rotation.0, a.rotation.1, a.rotation.2);
                }
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                return euler_degrees_to_mat3(b.rotation.x, b.rotation.y, b.rotation.z);
            }
        } else if let Some(s_idx) = self.selected_static_mesh_index {
            if let Some(m) = self.static_meshes.get(s_idx) {
                return euler_degrees_to_mat3(m.rotation.x, m.rotation.y, m.rotation.z);
            }
        } else {
            return euler_degrees_to_mat3(
                self.builder_brush.rotation.x,
                self.builder_brush.rotation.y,
                self.builder_brush.rotation.z,
            );
        }

        glam::Mat3::IDENTITY
    }

    /// Toggles between World and Local coordinate systems for 3D Gizmos.
    pub fn toggle_gizmo_space(&mut self) {
        self.gizmo_space = match self.gizmo_space {
            GizmoCoordinateSpace::World => GizmoCoordinateSpace::Local,
            GizmoCoordinateSpace::Local => GizmoCoordinateSpace::World,
        };
        let label = match self.gizmo_space {
            GizmoCoordinateSpace::World => "World (🌐)",
            GizmoCoordinateSpace::Local => "Local (🎯)",
        };
        self.log(format!("Gizmo Coordinate Space: {}", label));
        self.toast_info(format!("Gizmo Space: {}", label));
    }

    /// Toggles the selection lock state. When locked, viewport clicks cannot change selection
    /// or initiate direct drag on objects, protecting existing selection from accidental shifts.
    pub fn toggle_selection_lock(&mut self) {
        self.selection_locked = !self.selection_locked;
        let label = if self.selection_locked { "LOCKED (🔒)" } else { "UNLOCKED (🔓)" };
        self.log(format!("Selection Lock: {}", label));
        self.toast_info(format!("Selection {}", label));
    }


}

impl EditorState {
    /// Duplicates the currently selected Brush, Mover, Static Mesh, Actor, or Multi-Selection with an offset
    pub fn duplicate_selected(&mut self) {
        let offset = if self.snap_to_grid && self.grid_step > 0.0 {
            Vec3::new(self.grid_step, 0.0, 0.0)
        } else {
            Vec3::new(32.0, 0.0, 0.0)
        };

        if self.has_multi_selection() {
            self.duplicate_selected_batch(offset);
            return;
        }

        if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get(b_idx).cloned() {
                let mut new_brush = brush;
                new_brush.location += offset;
                let count = self.world_brushes.len();
                new_brush.name = format!("{}_{}", new_brush.name.split('_').next().unwrap_or("Brush"), count);
                self.world_brushes.push(new_brush.clone());
                self.history.push(Action::AddBrush { brush: new_brush });
                self.selected_brush_index = Some(self.world_brushes.len() - 1);
                self.selected_actor_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
                self.log(format!("Duplicated Brush to new index #{}", self.world_brushes.len() - 1));
            }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get(m_idx).cloned() {
                let mut new_mover = mover;
                new_mover.brush.location += offset;
                for kf in &mut new_mover.keyframes {
                    if let Some(ref mut k) = kf {
                        k.location += offset;
                    }
                }
                let count = self.movers.len();
                new_mover.name = format!("Mover{}", count);
                if new_mover.tag.is_empty() {
                    new_mover.tag = format!("MoverTag{}", count);
                } else {
                    new_mover.tag = Self::increment_numbered_tag(&new_mover.tag);
                }
                self.movers.push(new_mover.clone());
                self.history.push(Action::AddMover { mover: new_mover });
                self.selected_mover_index = Some(self.movers.len() - 1);
                self.selected_brush_index = None;
                self.selected_actor_index = None;
                self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
                self.log(format!("Duplicated Mover to new index #{}", self.movers.len() - 1));
            }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            if let Some(mesh) = self.static_meshes.get(sm_idx).cloned() {
                let mut new_mesh = mesh;
                new_mesh.location += offset;
                let count = self.static_meshes.len();
                new_mesh.name = format!("{}_{}", new_mesh.name.split('_').next().unwrap_or("Mesh"), count);
                self.static_meshes.push(new_mesh.clone());
                self.history.push(Action::AddStaticMesh { mesh: new_mesh });
                self.selected_static_mesh_index = Some(self.static_meshes.len() - 1);
                self.selected_brush_index = None;
                self.selected_actor_index = None;
                self.selected_mover_index = None;
                self.log(format!("Duplicated Static Mesh to new index #{}", self.static_meshes.len() - 1));
            }
        } else if let Some(act_idx) = self.selected_actor_index {
            let mut new_actor_info = None;
            if let Some(ref unr) = self.loaded_unr {
                if let Some(actor) = unr.actors.get(act_idx) {
                    let mut new_actor = actor.clone();
                    new_actor.location += offset;
                    let count = unr.actors.len();
                    new_actor.name = format!("{}_{}", new_actor.class_name, count);
                    if !new_actor.tag.is_empty() {
                        let inc_tag = Self::increment_numbered_tag(&new_actor.tag);
                        new_actor.tag = inc_tag.clone();
                        if let Some(prop) = new_actor.properties.get_mut("Tag") {
                            prop.value = ut99_format::TaggedValue::Name(inc_tag);
                        }
                    }
                    new_actor_info = Some(new_actor);
                }
            }
            if let Some(new_actor) = new_actor_info {
                let new_idx = if let Some(ref mut unr) = self.loaded_unr {
                    unr.actors.push(new_actor.clone());
                    unr.actors.len() - 1
                } else {
                    0
                };
                self.history.push(Action::AddActor { actor: new_actor });
                self.selected_actor_index = Some(new_idx);
                self.selected_brush_index = None;
                self.selected_mover_index = None;
                self.selected_static_mesh_index = None;
        self.is_builder_selected = false;
                self.log(format!("Duplicated Actor to new index #{}", new_idx));
            }
        }

        if self.selected_brush_index.is_some() || self.selected_mover_index.is_some() || self.selected_static_mesh_index.is_some() || self.selected_actor_index.is_some() {
            self.toast_success("Duplicated selection");
        }
    }

}


impl EditorState {
    /// Selects actors and world brushes enclosed by the 2D marquee box in the specified orthographic viewport.
    /// If `additive` is false, existing multi-selection is cleared first.
    /// Returns (selected_brushes_count, selected_actors_count).
    pub fn select_within_marquee(&mut self, marquee: MarqueeBox, additive: bool) -> (usize, usize) {
        if marquee.viewport_type == ViewportType::Perspective {
            let [vx, vy, vw, vh] = self.quadrant_rect_for_type(ViewportType::Perspective);
            let aspect = vw / vh.max(1.0);
            let inv_vp = self.camera.view_projection_matrix_for_type(ViewportType::Perspective, aspect).inverse();
            let cam_pos = self.camera.position;
            let (sx0, sy0) = marquee.start_screen;
            let (sx1, sy1) = marquee.current_screen;
            let lx0 = sx0 - vx;
            let ly0 = sy0 - vy;
            let lx1 = sx1 - vx;
            let ly1 = sy1 - vy;

            let Some(frustum) = crate::raycast::Frustum3D::from_screen_rect(
                lx0, ly0, lx1, ly1,
                vw, vh,
                inv_vp, cam_pos,
            ) else {
                return (0, 0);
            };

            if !additive {
                self.clear_multi_selection();
            }

            self.ensure_world_brush_bounds();
            let matched_brushes: Vec<usize> = {
                let cache = self.scene_bounds_cache.borrow();
                cache
                    .world_brushes
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &(min, max))| frustum.intersects_aabb(min, max).then_some(i))
                    .collect()
            };

            let mut added_brushes = 0;
            let mut added_actors = 0;

            for i in matched_brushes {
                self.multi_selected_brushes.insert(i);
                added_brushes += 1;
            }

            if let Some(ref unr) = self.loaded_unr {
                for (i, actor) in unr.actors.iter().enumerate() {
                    if frustum.intersects_sphere(actor.location, 36.0) {
                        self.multi_selected_actors.insert(i);
                        added_actors += 1;
                    }
                }
            }

            if added_brushes > 0 && self.selected_brush_index.is_none() {
                self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
            }
            if added_actors > 0 && self.selected_actor_index.is_none() {
                self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
            }

            return (added_brushes, added_actors);
        }

        let vp_rect = self.quadrant_rect_for_type(marquee.viewport_type);
        let p1 = self.camera.screen_to_ortho_world(marquee.viewport_type, marquee.start_screen.0, marquee.start_screen.1, vp_rect);
        let p2 = self.camera.screen_to_ortho_world(marquee.viewport_type, marquee.current_screen.0, marquee.current_screen.1, vp_rect);

        let (Some((u1, v1)), Some((u2, v2))) = (p1, p2) else {
            return (0, 0);
        };

        let u_min = u1.min(u2);
        let u_max = u1.max(u2);
        let v_min = v1.min(v2);
        let v_max = v1.max(v2);

        if !additive {
            self.clear_multi_selection();
        }

        let mut added_brushes = 0;
        let mut added_actors = 0;

        // 1. Check World Brushes
        self.ensure_world_brush_bounds();
        let matched_brushes: Vec<usize> = {
            let cache = self.scene_bounds_cache.borrow();
            self.world_brushes
                .iter()
                .zip(cache.world_brushes.iter())
                .enumerate()
                .filter_map(|(i, (brush, &(min_b, max_b)))| {
                    let (bu, bv) = match marquee.viewport_type {
                        ViewportType::Top => (brush.location.x, brush.location.y),
                        ViewportType::Front => (brush.location.x, brush.location.z),
                        ViewportType::Side => (brush.location.y, brush.location.z),
                        ViewportType::Perspective => return None,
                    };
                    let (b_u_min, b_u_max, b_v_min, b_v_max) = match marquee.viewport_type {
                        ViewportType::Top => (min_b.x, max_b.x, min_b.y, max_b.y),
                        ViewportType::Front => (min_b.x, max_b.x, min_b.z, max_b.z),
                        ViewportType::Side => (min_b.y, max_b.y, min_b.z, max_b.z),
                        ViewportType::Perspective => return None,
                    };
                    let center_inside = bu >= u_min && bu <= u_max && bv >= v_min && bv <= v_max;
                    let bounds_overlap =
                        b_u_min <= u_max && b_u_max >= u_min && b_v_min <= v_max && b_v_max >= v_min;
                    (center_inside || bounds_overlap).then_some(i)
                })
                .collect()
        };
        for i in matched_brushes {
            self.multi_selected_brushes.insert(i);
            added_brushes += 1;
        }

        // 2. Check Actors
        if let Some(ref unr) = self.loaded_unr {
            for (i, actor) in unr.actors.iter().enumerate() {
                let (au, av) = match marquee.viewport_type {
                    ViewportType::Top => (actor.location.x, actor.location.y),
                    ViewportType::Front => (actor.location.x, actor.location.z),
                    ViewportType::Side => (actor.location.y, actor.location.z),
                    ViewportType::Perspective => continue,
                };

                if au >= u_min && au <= u_max && av >= v_min && av <= v_max {
                    self.multi_selected_actors.insert(i);
                    added_actors += 1;
                }
            }
        }

        if self.selected_brush_index.is_none() {
            self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        }
        if self.selected_actor_index.is_none() {
            self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        }

        (added_brushes, added_actors)
    }


    /// UnrealEd 2.0 `Select Inside` (`Deliverable B Categoria A.3 Gruppo 5` & `Categoria C.1`):
    /// Selects all Actors and World Brushes whose world-space positions lie inside the active Red Builder Brush volume.
    pub fn select_inside_builder_brush(&mut self) -> (usize, usize) {
        self.clear_multi_selection();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let (min_b, max_b) = brush.world_bounds();
            let center = (min_b + max_b) * 0.5;
            if self.builder_brush.contains_world_point(center) {
                self.multi_selected_brushes.insert(b_idx);
            }
        }
        if let Some(ref unr) = self.loaded_unr {
            for (a_idx, actor) in unr.actors.iter().enumerate() {
                if self.builder_brush.contains_world_point(actor.location) {
                    self.multi_selected_actors.insert(a_idx);
                }
            }
        }
        self.selected_brush_index = self.multi_selected_brushes.iter().next().copied();
        self.selected_actor_index = self.multi_selected_actors.iter().next().copied();
        let b_cnt = self.multi_selected_brushes.len();
        let a_cnt = self.multi_selected_actors.len();
        self.log(format!(
            "Select Inside Builder Brush: selected {} brush(es) and {} actor(s).",
            b_cnt, a_cnt
        ));
        self.toast_info(format!("Select Inside: {} brush(es), {} actor(s)", b_cnt, a_cnt));
        (b_cnt, a_cnt)
    }


    /// Translates all currently selected objects (single or multi-selection) by `delta` and pushes an atomic undo action.
    pub fn translate_multi_selection_with_undo(&mut self, delta: Vec3) -> usize {
        if delta.length_squared() < 1e-6 {
            return 0;
        }
        let mut actions = Vec::new();
        let mut brush_indices: Vec<usize> = self.multi_selected_brushes.iter().copied().collect();
        if brush_indices.is_empty() {
            if let Some(b) = self.selected_brush_index {
                brush_indices.push(b);
            }
        }
        brush_indices.sort_unstable();

        for b_idx in brush_indices {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_location = b.location;
                let new_location = old_location + delta;
                b.location = new_location;
                actions.push(crate::history::Action::MoveBrush {
                    brush_index: b_idx,
                    old_location,
                    new_location,
                });
            }
        }

        let mut actor_indices: Vec<usize> = self.multi_selected_actors.iter().copied().collect();
        if actor_indices.is_empty() {
            if let Some(a) = self.selected_actor_index {
                actor_indices.push(a);
            }
        }
        actor_indices.sort_unstable();

        if let Some(ref mut unr) = self.loaded_unr {
            for a_idx in actor_indices {
                if let Some(a) = unr.actors.get_mut(a_idx) {
                    let old_location = a.location;
                    let new_location = old_location + delta;
                    a.location = new_location;
                    actions.push(crate::history::Action::MoveActor {
                        actor_index: a_idx,
                        old_location,
                        new_location,
                    });
                }
            }
        }

        let moved_count = actions.len();
        if actions.len() == 1 {
            self.history.push(actions.pop().unwrap());
        } else if actions.len() > 1 {
            self.history.push(crate::history::Action::Compound {
                actions,
                description: format!("Translate {} selected objects", moved_count),
            });
        }
        moved_count
    }


    /// Generates an invisible blocking collision hull (PF_INVISIBLE) enclosing the selected static mesh
    pub fn generate_collision_hull_for_selected_static_mesh(&mut self) {
        if let Some(idx) = self.selected_static_mesh_index {
            if let Some(mesh) = self.static_meshes.get(idx) {
                let hull = mesh.build_collision_hull(2.0);
                self.world_brushes.push(hull.clone());
                self.history.push(crate::history::Action::AddBrush { brush: hull });
                self.log(format!("Generated blocking collision hull for static mesh '{}'", mesh.name));
            }
        }
    }


    /// Deliverable I.3: Increments the trailing integer suffix of a numbered Tag (e.g. `"Lift1"` -> `"Lift2"`, `"Door_01"` -> `"Door_02"`).
    pub fn increment_numbered_tag(tag: &str) -> String {
        let trimmed = tag.trim();
        if trimmed.is_empty() {
            return String::new();
        }
        let digit_count = trimmed
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_digit())
            .count();
        if digit_count == 0 {
            return trimmed.to_string();
        }
        let split_pos = trimmed.len() - digit_count;
        let prefix = &trimmed[..split_pos];
        let digits = &trimmed[split_pos..];
        if let Ok(num) = digits.parse::<u64>() {
            format!("{}{:0width$}", prefix, num + 1, width = digit_count)
        } else {
            trimmed.to_string()
        }
    }


    /// Searches for the highest solid floor polygon directly beneath the given position,
    /// returning both the floor Z coordinate and the surface normal.
    pub fn find_floor_hit_below(&self, pos: Vec3, current_bottom: f32, ignore_brush_idx: Option<usize>) -> Option<(f32, Vec3)> {
        let ray_start = Vec3::new(pos.x, pos.y, current_bottom + 32.0);
        let ray_dir = Vec3::new(0.0, 0.0, -1.0);
        let mut best: Option<(f32, Vec3)> = None;

        for (b_i, brush) in self.world_brushes.iter().enumerate() {
            if Some(b_i) == ignore_brush_idx {
                continue;
            }
            for poly in brush.to_world_polys() {
                if poly.plane.normal.z > 0.05 && poly.vertices.len() >= 3 {
                    let v0 = poly.vertices[0];
                    for i in 1..(poly.vertices.len() - 1) {
                        let v1 = poly.vertices[i];
                        let v2 = poly.vertices[i + 1];
                        if let Some(t) = ut99_bsp::lighting::ray_intersects_triangle(ray_start, ray_dir, v0, v1, v2) {
                            let hit_z = ray_start.z - t;
                            if hit_z <= current_bottom + 1.0 {
                                if best.is_none() || hit_z > best.as_ref().unwrap().0 {
                                    best = Some((hit_z, poly.plane.normal.normalize()));
                                }
                            }
                        }
                    }
                }
            }
        }
        best
    }


    /// Searches for the highest solid floor polygon directly beneath the given position
    pub fn find_floor_z_below(&self, pos: Vec3, current_bottom: f32, ignore_brush_idx: Option<usize>) -> Option<f32> {
        self.find_floor_hit_below(pos, current_bottom, ignore_brush_idx).map(|(z, _)| z)
    }


    /// Snaps the currently selected object (Brush, Mover, Static Mesh, or Actor) onto the solid floor directly beneath it
    pub fn snap_selected_to_floor(&mut self) -> bool {
        let (pos, current_bottom, ignore_b) = if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                let (min_b, _) = b.world_bounds();
                (b.location, min_b.z, Some(b_idx))
            } else { return false; }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get(m_idx) {
                let (min_b, _) = m.brush.world_bounds();
                (m.brush.location, min_b.z, None)
            } else { return false; }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            if let Some(m) = self.static_meshes.get(sm_idx) {
                let (min_b, _) = m.world_bounds();
                (m.location, min_b.z, None)
            } else { return false; }
        } else if let Some(act_idx) = self.selected_actor_index {
            if let Some(ref unr) = self.loaded_unr {
                if let Some(a) = unr.actors.get(act_idx) {
                    (a.location, a.location.z, None)
                } else { return false; }
            } else { return false; }
        } else {
            return false;
        };

        let best_z = self.find_floor_z_below(pos, current_bottom, ignore_b);

        if let Some(floor_z) = best_z {
            if let Some(b_idx) = self.selected_brush_index {
                let log_info = if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_loc = b.location;
                    let (min_b, _) = b.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    b.location.z += delta_z;
                    let new_loc = b.location;
                    let name = b.name.clone();
                    Some((old_loc, new_loc, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, name)) = log_info {
                    self.history.push(Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Brush '{}' to floor at Z={:.1}", name, floor_z));
                    return true;
                }
            } else if let Some(m_idx) = self.selected_mover_index {
                let log_info = if let Some(m) = self.movers.get_mut(m_idx) {
                    let old_loc = m.brush.location;
                    let (min_b, _) = m.brush.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    m.brush.location.z += delta_z;
                    let cur_key = m.current_key;
                    if let Some(ref mut kf) = m.keyframes[cur_key] {
                        kf.location.z += delta_z;
                    }
                    let new_loc = m.brush.location;
                    let name = m.name.clone();
                    Some((old_loc, new_loc, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, name)) = log_info {
                    self.history.push(Action::MoveMover {
                        mover_index: m_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Mover '{}' to floor at Z={:.1}", name, floor_z));
                    return true;
                }
            } else if let Some(sm_idx) = self.selected_static_mesh_index {
                let log_info = if let Some(m) = self.static_meshes.get_mut(sm_idx) {
                    let old_loc = m.location;
                    let (min_b, _) = m.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    m.location.z += delta_z;
                    let new_loc = m.location;
                    let name = m.name.clone();
                    Some((old_loc, new_loc, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, name)) = log_info {
                    self.history.push(Action::MoveStaticMesh {
                        mesh_index: sm_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Static Mesh '{}' to floor at Z={:.1}", name, floor_z));
                    return true;
                }
            } else if let Some(act_idx) = self.selected_actor_index {
                let log_info = if let Some(ref mut unr) = self.loaded_unr {
                    if let Some(a) = unr.actors.get_mut(act_idx) {
                        let old_loc = a.location;
                        let target_z = floor_z + 16.0;
                        a.location.z = target_z;
                        let new_loc = a.location;
                        let name = a.name.clone();
                        Some((old_loc, new_loc, name))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((old_loc, new_loc, name)) = log_info {
                    self.history.push(Action::MoveActor {
                        actor_index: act_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Actor '{}' to floor at Z={:.1}", name, floor_z));
                    return true;
                }
            }
        } else {
            self.log("Snap to Floor: No solid floor surface found directly below selection.");
        }
        false
    }


    /// Snaps the currently selected object (Brush, Mover, Static Mesh, or Actor) onto the solid floor
    /// directly beneath it AND aligns its orientation to the floor's surface normal (Shift+End).
    pub fn snap_selected_to_floor_aligned(&mut self) -> bool {
        let (pos, current_bottom, ignore_b) = if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                let (min_b, _) = b.world_bounds();
                (b.location, min_b.z, Some(b_idx))
            } else { return false; }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(m) = self.movers.get(m_idx) {
                let (min_b, _) = m.brush.world_bounds();
                (m.brush.location, min_b.z, None)
            } else { return false; }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            if let Some(m) = self.static_meshes.get(sm_idx) {
                let (min_b, _) = m.world_bounds();
                (m.location, min_b.z, None)
            } else { return false; }
        } else if let Some(act_idx) = self.selected_actor_index {
            if let Some(ref unr) = self.loaded_unr {
                if let Some(a) = unr.actors.get(act_idx) {
                    (a.location, a.location.z, None)
                } else { return false; }
            } else { return false; }
        } else {
            return false;
        };

        let best_hit = self.find_floor_hit_below(pos, current_bottom, ignore_b);

        if let Some((floor_z, normal)) = best_hit {
            let normal = if !normal.is_finite() || normal.length_squared() < 1e-4 {
                Vec3::Z
            } else {
                normal.normalize()
            };

            if let Some(b_idx) = self.selected_brush_index {
                let log_info = if let Some(b) = self.world_brushes.get_mut(b_idx) {
                    let old_loc = b.location;
                    let (min_b, _) = b.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    b.location.z += delta_z;
                    let new_loc = b.location;

                    let old_rot = b.rotation;
                    let rad_pitch = old_rot.x.to_radians();
                    let rad_yaw = old_rot.y.to_radians();
                    let rad_roll = old_rot.z.to_radians();
                    let cur_rot = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                    let mut old_fwd = cur_rot.transform_vector3(Vec3::X).normalize();
                    if old_fwd.cross(normal).length_squared() < 1e-4 {
                        old_fwd = Vec3::Y;
                    }
                    let right = normal.cross(old_fwd).normalize();
                    let forward = right.cross(normal).normalize();
                    let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                    let (rad_y, rad_x, rad_z) = rot_mat.to_euler(glam::EulerRot::YXZ);
                    b.rotation = Vec3::new(rad_x.to_degrees(), rad_y.to_degrees(), rad_z.to_degrees());
                    let new_rot = b.rotation;
                    let name = b.name.clone();
                    Some((old_loc, new_loc, old_rot, new_rot, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, old_rot, new_rot, name)) = log_info {
                    self.history.push(Action::MoveBrush {
                        brush_index: b_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.history.push(Action::RotateBrush {
                        brush_index: b_idx,
                        old_rotation: old_rot,
                        new_rotation: new_rot,
                    });
                    self.log(format!("Snapped Brush '{}' to floor at Z={:.1} with normal-aligned rotation", name, floor_z));
                    return true;
                }
            } else if let Some(m_idx) = self.selected_mover_index {
                let log_info = if let Some(m) = self.movers.get_mut(m_idx) {
                    let old_loc = m.brush.location;
                    let (min_b, _) = m.brush.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    m.brush.location.z += delta_z;
                    let new_loc = m.brush.location;

                    let old_rot = m.brush.rotation;
                    let rad_pitch = old_rot.x.to_radians();
                    let rad_yaw = old_rot.y.to_radians();
                    let rad_roll = old_rot.z.to_radians();
                    let cur_rot = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                    let mut old_fwd = cur_rot.transform_vector3(Vec3::X).normalize();
                    if old_fwd.cross(normal).length_squared() < 1e-4 {
                        old_fwd = Vec3::Y;
                    }
                    let right = normal.cross(old_fwd).normalize();
                    let forward = right.cross(normal).normalize();
                    let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                    let (rad_y, rad_x, rad_z) = rot_mat.to_euler(glam::EulerRot::YXZ);
                    m.brush.rotation = Vec3::new(rad_x.to_degrees(), rad_y.to_degrees(), rad_z.to_degrees());
                    let new_rot = m.brush.rotation;

                    let cur_key = m.current_key;
                    if let Some(ref mut kf) = m.keyframes[cur_key] {
                        kf.location.z += delta_z;
                        kf.rotation = new_rot;
                    }
                    let name = m.name.clone();
                    Some((old_loc, new_loc, old_rot, new_rot, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, _old_rot, _new_rot, name)) = log_info {
                    self.history.push(Action::MoveMover {
                        mover_index: m_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Mover '{}' to floor at Z={:.1} with normal-aligned rotation", name, floor_z));
                    return true;
                }
            } else if let Some(sm_idx) = self.selected_static_mesh_index {
                let log_info = if let Some(m) = self.static_meshes.get_mut(sm_idx) {
                    let old_loc = m.location;
                    let (min_b, _) = m.world_bounds();
                    let delta_z = floor_z - min_b.z;
                    m.location.z += delta_z;
                    let new_loc = m.location;

                    let old_rot = m.rotation;
                    let rad_pitch = old_rot.x.to_radians();
                    let rad_yaw = old_rot.y.to_radians();
                    let rad_roll = old_rot.z.to_radians();
                    let cur_rot = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
                    let mut old_fwd = cur_rot.transform_vector3(Vec3::X).normalize();
                    if old_fwd.cross(normal).length_squared() < 1e-4 {
                        old_fwd = Vec3::Y;
                    }
                    let right = normal.cross(old_fwd).normalize();
                    let forward = right.cross(normal).normalize();
                    let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                    let (rad_y, rad_x, rad_z) = rot_mat.to_euler(glam::EulerRot::YXZ);
                    m.rotation = Vec3::new(rad_x.to_degrees(), rad_y.to_degrees(), rad_z.to_degrees());
                    let name = m.name.clone();
                    Some((old_loc, new_loc, name))
                } else {
                    None
                };
                if let Some((old_loc, new_loc, name)) = log_info {
                    self.history.push(Action::MoveStaticMesh {
                        mesh_index: sm_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.log(format!("Snapped Static Mesh '{}' to floor at Z={:.1} with normal-aligned rotation", name, floor_z));
                    return true;
                }
            } else if let Some(act_idx) = self.selected_actor_index {
                let log_info = if let Some(ref mut unr) = self.loaded_unr {
                    if let Some(a) = unr.actors.get_mut(act_idx) {
                        let old_loc = a.location;
                        let target_z = floor_z + 16.0;
                        a.location.z = target_z;
                        let new_loc = a.location;

                        let old_rot = a.rotation;
                        const ROT_TO_RAD: f32 = std::f32::consts::PI / 32768.0;
                        const RAD_TO_ROT: f32 = 32768.0 / std::f32::consts::PI;
                        let pitch_rad = old_rot.0 as f32 * ROT_TO_RAD;
                        let yaw_rad = old_rot.1 as f32 * ROT_TO_RAD;
                        let roll_rad = old_rot.2 as f32 * ROT_TO_RAD;

                        let rotation_quat = glam::Quat::from_rotation_z(yaw_rad)
                            * glam::Quat::from_rotation_y(pitch_rad)
                            * glam::Quat::from_rotation_x(roll_rad);
                        let mut old_fwd = rotation_quat.mul_vec3(Vec3::X).normalize();
                        if old_fwd.cross(normal).length_squared() < 1e-4 {
                            old_fwd = Vec3::Y;
                        }
                        let right = normal.cross(old_fwd).normalize();
                        let forward = right.cross(normal).normalize();
                        let rot_mat = glam::Mat3::from_cols(forward, right, normal);
                        let (yaw_out, pitch_out, roll_out) = rot_mat.to_euler(glam::EulerRot::ZYX);
                        let new_rot = (
                            (pitch_out * RAD_TO_ROT).round() as i32,
                            (yaw_out * RAD_TO_ROT).round() as i32,
                            (roll_out * RAD_TO_ROT).round() as i32,
                        );
                        a.rotation = new_rot;
                        let name = a.name.clone();
                        Some((old_loc, new_loc, old_rot, new_rot, name))
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some((old_loc, new_loc, old_rot, new_rot, name)) = log_info {
                    self.history.push(Action::MoveActor {
                        actor_index: act_idx,
                        old_location: old_loc,
                        new_location: new_loc,
                    });
                    self.history.push(Action::RotateActor {
                        actor_index: act_idx,
                        old_rotation: old_rot,
                        new_rotation: new_rot,
                    });
                    self.log(format!("Snapped Actor '{}' to floor at Z={:.1} with normal-aligned rotation", name, floor_z));
                    return true;
                }
            }
        } else {
            self.log("Snap to Floor Aligned: No solid floor surface found directly below selection.");
        }
        false
    }


    /// Selects brushes matching the UnrealEd 2.0 `MAP SELECT` filter (`ADDS`, `SUBTRACTS`, `SEMISOLIDS`, `NONSOLIDS`, `FIRST`, `LAST`) (`REQ-K03`).
    pub fn select_brushes_by_filter(&mut self, filter: &str) -> usize {
        self.multi_selected_brushes.clear();
        self.selected_brush_index = None;
        let mode = filter.trim().to_ascii_uppercase();

        match mode.as_str() {
            "FIRST" => {
                if !self.world_brushes.is_empty() {
                    self.selected_brush_index = Some(0);
                    self.multi_selected_brushes.insert(0);
                }
            }
            "LAST" => {
                if !self.world_brushes.is_empty() {
                    let last = self.world_brushes.len() - 1;
                    self.selected_brush_index = Some(last);
                    self.multi_selected_brushes.insert(last);
                }
            }
            _ => {
                for (idx, brush) in self.world_brushes.iter().enumerate() {
                    let is_semi = brush.polys.iter().any(|p| (p.flags & ut99_bsp::PF_SEMISOLID) != 0);
                    let is_non = brush.polys.iter().any(|p| (p.flags & ut99_bsp::PF_NOT_SOLID) != 0);
                    let matched = match mode.as_str() {
                        "ADDS" => brush.csg_oper == CsgOperation::Add && !is_semi && !is_non,
                        "SUBTRACTS" => brush.csg_oper == CsgOperation::Subtract,
                        "SEMISOLIDS" => is_semi,
                        "NONSOLIDS" => is_non,
                        _ => false,
                    };
                    if matched {
                        self.multi_selected_brushes.insert(idx);
                        if self.selected_brush_index.is_none() {
                            self.selected_brush_index = Some(idx);
                        }
                    }
                }
            }
        }

        let count = self.multi_selected_brushes.len();
        self.log(format!("MAP SELECT {}: Selected {} brush(es).", mode, count));
        count
    }


    /// UnrealEd 2.0 `ACTOR RESET [ALL|LOCATION|PIVOT|ROTATION|SCALE]`:
    /// Resets transform components on selected Actors, World Brushes, or Builder Brush with Undo/Redo support.
    pub fn reset_selected_actors_transform(
        &mut self,
        reset_loc: bool,
        reset_pivot: bool,
        reset_rot: bool,
        reset_scale: bool,
    ) -> usize {
        let mut count = 0usize;

        // 1. Reset selected Actors
        let actor_indices: Vec<usize> = if !self.multi_selected_actors.is_empty() {
            self.multi_selected_actors.iter().copied().collect()
        } else if let Some(a_idx) = self.selected_actor_index {
            vec![a_idx]
        } else {
            Vec::new()
        };
        if let Some(ref mut unr) = self.loaded_unr {
            for idx in actor_indices {
                if let Some(act) = unr.actors.get_mut(idx) {
                    let old_actor = act.clone();
                    if reset_loc {
                        act.location = Vec3::ZERO;
                    }
                    if reset_pivot {
                        act.properties.remove("PrePivot");
                    }
                    if reset_rot {
                        act.rotation = (0, 0, 0);
                        act.properties.remove("Rotation");
                    }
                    if reset_scale {
                        act.properties.remove("DrawScale");
                        act.properties.remove("MainScale");
                        act.properties.remove("PostScale");
                    }
                    let new_actor = act.clone();
                    self.history.push(Action::UpdateZonePhysics {
                        actor_index: idx,
                        old_actor,
                        new_actor,
                        description: "ACTOR RESET Transform".to_string(),
                    });
                    count += 1;
                }
            }
        }

        // 2. Reset selected World Brushes
        let brush_indices: Vec<usize> = if !self.multi_selected_brushes.is_empty() {
            self.multi_selected_brushes.iter().copied().collect()
        } else if let Some(b_idx) = self.selected_brush_index {
            vec![b_idx]
        } else {
            Vec::new()
        };
        for b_idx in brush_indices {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_location = b.location;
                let old_rotation = b.rotation;
                let old_scale = b.scale;
                let old_polys = b.polys.clone();
                if reset_loc || reset_pivot {
                    b.location = Vec3::ZERO;
                }
                if reset_rot {
                    b.rotation = Vec3::ZERO;
                }
                if reset_scale {
                    b.scale = Vec3::ONE;
                }
                if b.location != old_location {
                    self.history.push(Action::MoveBrush {
                        brush_index: b_idx,
                        old_location,
                        new_location: b.location,
                    });
                }
                if b.rotation != old_rotation || b.scale != old_scale {
                    self.history.push(Action::TransformBrush {
                        brush_index: b_idx,
                        old_rotation,
                        new_rotation: b.rotation,
                        old_scale,
                        new_scale: b.scale,
                        old_polys,
                        new_polys: b.polys.clone(),
                        description: "ACTOR RESET Brush Transform".to_string(),
                    });
                }
                count += 1;
            }
        }

        // 3. Fallback to Builder Brush if nothing else was selected
        if count == 0 {
            let old_brush = self.builder_brush.clone();
            if reset_loc || reset_pivot {
                self.builder_brush.location = Vec3::ZERO;
            }
            if reset_rot {
                self.builder_brush.rotation = Vec3::ZERO;
            }
            if reset_scale {
                self.builder_brush.scale = Vec3::ONE;
            }
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Reset Builder Brush Transform".to_string(),
            });
            count = 1;
        }

        self.map_is_dirty = true;
        count
    }


    /// UnrealEd 2.0 `ACTOR MIRROR [X=-1] [Y=-1] [Z=-1]`:
    /// Mirrors selected brushes along each axis where `mirror_vec[axis] < 0.0`, and mirrors selected actors' orientations/coordinates.
    pub fn mirror_selected_along_vector(&mut self, mirror_vec: Vec3) -> usize {
        let mut mirrored = 0usize;
        if mirror_vec.x < 0.0 {
            self.mirror_selected(MirrorAxis::X);
            mirrored += 1;
        }
        if mirror_vec.y < 0.0 {
            self.mirror_selected(MirrorAxis::Y);
            mirrored += 1;
        }
        if mirror_vec.z < 0.0 {
            self.mirror_selected(MirrorAxis::Z);
            mirrored += 1;
        }

        let actor_indices: Vec<usize> = if !self.multi_selected_actors.is_empty() {
            self.multi_selected_actors.iter().copied().collect()
        } else if let Some(a_idx) = self.selected_actor_index {
            vec![a_idx]
        } else {
            Vec::new()
        };
        if let Some(ref mut unr) = self.loaded_unr {
            for idx in actor_indices {
                if let Some(act) = unr.actors.get_mut(idx) {
                    let old_actor = act.clone();
                    if mirror_vec.x < 0.0 {
                        act.rotation.1 = (32768 - act.rotation.1).rem_euclid(65536);
                    }
                    if mirror_vec.y < 0.0 {
                        act.rotation.1 = (-act.rotation.1).rem_euclid(65536);
                    }
                    if mirror_vec.z < 0.0 {
                        act.rotation.0 = (-act.rotation.0).rem_euclid(65536);
                    }
                    let new_actor = act.clone();
                    self.history.push(Action::UpdateZonePhysics {
                        actor_index: idx,
                        old_actor,
                        new_actor,
                        description: format!("ACTOR MIRROR ({}, {}, {})", mirror_vec.x, mirror_vec.y, mirror_vec.z),
                    });
                    mirrored += 1;
                }
            }
        }
        mirrored
    }

}
