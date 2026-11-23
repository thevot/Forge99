use super::*;
use glam::Vec3;

impl EditorState {
    // ==========================================
    // Brush Solidity Methods
    // ==========================================
    pub fn get_brush_solidity(&self, brush_index: usize) -> BrushSolidity {
        if let Some(b) = self.world_brushes.get(brush_index) {
            let has_semi = b.polys.iter().any(|p| (p.flags & ut99_bsp::PF_SEMISOLID) != 0);
            let has_non = b.polys.iter().any(|p| (p.flags & ut99_bsp::PF_NOT_SOLID) != 0);
            if has_semi {
                BrushSolidity::SemiSolid
            } else if has_non {
                BrushSolidity::NonSolid
            } else {
                BrushSolidity::Solid
            }
        } else {
            BrushSolidity::Solid
        }
    }

    pub fn set_selected_brush_solidity(&mut self, solidity: BrushSolidity) -> bool {
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
            // Modify builder brush solidity
            let old_brush = self.builder_brush.clone();
            for p in &mut self.builder_brush.polys {
                p.flags &= !(ut99_bsp::PF_SEMISOLID | ut99_bsp::PF_NOT_SOLID);
                match solidity {
                    BrushSolidity::Solid => {}
                    BrushSolidity::SemiSolid => p.flags |= ut99_bsp::PF_SEMISOLID,
                    BrushSolidity::NonSolid => p.flags |= ut99_bsp::PF_NOT_SOLID,
                }
            }
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: format!("Set Builder Brush Solidity to {}", solidity.label()),
            });
            self.log(format!("Set Builder Brush solidity to {}.", solidity.label()));
            return true;
        }

        let mut changed_count = 0;
        for &idx in &target_indices {
            if let Some(b) = self.world_brushes.get_mut(idx) {
                let old_polys = b.polys.clone();
                for p in &mut b.polys {
                    p.flags &= !(ut99_bsp::PF_SEMISOLID | ut99_bsp::PF_NOT_SOLID);
                    match solidity {
                        BrushSolidity::Solid => {}
                        BrushSolidity::SemiSolid => p.flags |= ut99_bsp::PF_SEMISOLID,
                        BrushSolidity::NonSolid => p.flags |= ut99_bsp::PF_NOT_SOLID,
                    }
                }
                let new_polys = b.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: idx,
                    old_polys,
                    new_polys,
                    description: format!("Set Brush #{} Solidity to {}", idx, solidity.label()),
                });
                changed_count += 1;
            }
        }

        self.log(format!("Set solidity of {} brush(es) to {}.", changed_count, solidity.label()));
        changed_count > 0
    }

    // ==========================================
    // Vertex Editing Mode Methods
    // ==========================================
    pub fn get_active_brush_unique_vertices(&self) -> Vec<Vec3> {
        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get(idx).unwrap_or(&self.builder_brush)
        } else {
            &self.builder_brush
        };

        let mut unique: Vec<Vec3> = Vec::new();
        for poly in &brush.polys {
            for v in &poly.vertices {
                let world_v = *v + brush.location;
                if !unique.iter().any(|u| u.distance_squared(world_v) < 0.01) {
                    unique.push(world_v);
                }
            }
        }
        unique
    }

    pub fn move_selected_vertices(&mut self, delta: Vec3) {
        if delta.length_squared() < 0.0001 {
            return;
        }

        let unique = self.get_active_brush_unique_vertices();
        let target_world_points: Vec<Vec3> = self
            .selected_vertex_indices
            .iter()
            .filter_map(|&idx| unique.get(idx).copied())
            .collect();

        if target_world_points.is_empty() {
            return;
        }

        let is_world_brush = self.selected_brush_index.is_some();
        let sel_idx = self.selected_brush_index;
        let old_polys = if let Some(idx) = sel_idx {
            self.world_brushes.get(idx).map(|b| b.polys.clone())
        } else {
            None
        };
        let old_builder = if !is_world_brush {
            Some(self.builder_brush.clone())
        } else {
            None
        };

        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get_mut(idx).unwrap_or(&mut self.builder_brush)
        } else {
            &mut self.builder_brush
        };

        let loc = brush.location;
        let mut modified_count = 0;

        for poly in &mut brush.polys {
            let mut poly_modified = false;
            for v in &mut poly.vertices {
                let world_v = *v + loc;
                for target_p in &target_world_points {
                    if world_v.distance_squared(*target_p) < 0.01 {
                        *v += delta;
                        poly_modified = true;
                        modified_count += 1;
                        break;
                    }
                }
            }
            if poly_modified {
                // Deliverable I.2: Safe Vertex Auto-Weld when two vertices overlap on the same grid point
                let mut welded_verts: Vec<Vec3> = Vec::with_capacity(poly.vertices.len());
                for &v in &poly.vertices {
                    if welded_verts.last().map_or(true, |prev| prev.distance_squared(v) >= 0.0001) {
                        welded_verts.push(v);
                    }
                }
                if welded_verts.len() > 1
                    && welded_verts.first().unwrap().distance_squared(*welded_verts.last().unwrap()) < 0.0001
                {
                    welded_verts.pop();
                }
                poly.vertices = welded_verts;

                if poly.vertices.len() >= 3 {
                    if let Some(new_plane) = Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                        poly.plane = new_plane;
                    }
                }
            }
        }

        // Deliverable I.2: Auto-Planar Triangulation on Vertex Edit + degenerate polygon removal
        let mut auto_triangulated_faces = 0usize;
        if modified_count > 0 {
            let mut planar_polys = Vec::with_capacity(brush.polys.len());
            for poly in brush.polys.drain(..) {
                if poly.vertices.len() < 3 {
                    continue;
                }
                if poly.vertices.len() >= 4 && !poly.is_planar(0.01) {
                    auto_triangulated_faces += 1;
                    planar_polys.extend(poly.triangulate_if_non_planar(0.01));
                } else {
                    planar_polys.push(poly);
                }
            }
            brush.polys = planar_polys;
        }

        if modified_count > 0 {
            let is_gizmo_drag = self.active_gizmo_axis != crate::raycast::GizmoAxis::None;
            if is_world_brush {
                if let Some(idx) = sel_idx {
                    if let Some(old) = old_polys {
                        if let Some(b) = self.world_brushes.get(idx) {
                            let action = crate::history::Action::ModifyBrushGeometry {
                                brush_index: idx,
                                old_polys: old,
                                new_polys: b.polys.clone(),
                                description: format!("Move Vertices ({:.1}, {:.1}, {:.1})", delta.x, delta.y, delta.z),
                            };
                            if is_gizmo_drag {
                                self.history.push_interactive(action, true);
                            } else {
                                self.history.push(action);
                            }
                        }
                    }
                }
            } else if let Some(old) = old_builder {
                let action = crate::history::Action::ModifyBuilderBrush {
                    old_brush: old,
                    new_brush: self.builder_brush.clone(),
                    description: format!("Move Builder Vertices ({:.1}, {:.1}, {:.1})", delta.x, delta.y, delta.z),
                };
                if is_gizmo_drag {
                    self.history.push_interactive(action, true);
                } else {
                    self.history.push(action);
                }
            }
        }

        if auto_triangulated_faces > 0 {
            self.log(format!(
                "Vertex Edit: Moved selected vertices by ({:.1}, {:.1}, {:.1}). Updated {} vertices and auto-triangulated {} non-planar face(s).",
                delta.x, delta.y, delta.z, modified_count, auto_triangulated_faces
            ));
        } else {
            self.log(format!("Vertex Edit: Moved selected vertices by ({:.1}, {:.1}, {:.1}). Updated {} vertices.", delta.x, delta.y, delta.z, modified_count));
        }
    }

    pub fn snap_selected_vertices_to_nearest(&mut self, snap_radius: f32) -> bool {
        let unique = self.get_active_brush_unique_vertices();
        let selected_points: Vec<Vec3> = self
            .selected_vertex_indices
            .iter()
            .filter_map(|&idx| unique.get(idx).copied())
            .collect();

        if selected_points.is_empty() {
            self.log("Vertex Snap: No vertices selected.");
            return false;
        }

        // Collect all candidate vertices from other world brushes
        let active_idx = self.selected_brush_index;
        let mut candidate_verts: Vec<Vec3> = Vec::new();
        for (i, b) in self.world_brushes.iter().enumerate() {
            if Some(i) == active_idx {
                continue;
            }
            for p in &b.polys {
                for v in &p.vertices {
                    candidate_verts.push(*v + b.location);
                }
            }
        }

        if candidate_verts.is_empty() {
            self.log("Vertex Snap: No adjacent world brush vertices found to snap to.");
            return false;
        }

        // Find nearest pair
        let mut best_delta = None;
        let mut min_dist_sq = snap_radius * snap_radius;

        for sel_v in &selected_points {
            for cand_v in &candidate_verts {
                let dist_sq = sel_v.distance_squared(*cand_v);
                if dist_sq > 0.0001 && dist_sq < min_dist_sq {
                    min_dist_sq = dist_sq;
                    best_delta = Some(*cand_v - *sel_v);
                }
            }
        }

        if let Some(delta) = best_delta {
            self.move_selected_vertices(delta);
            self.log(format!("Vertex Snap: Snapped seamlessly to nearest vertex (delta: {:.1}, {:.1}, {:.1}).", delta.x, delta.y, delta.z));
            true
        } else {
            self.log(format!("Vertex Snap: No other vertex within snap radius of {:.0} units.", snap_radius));
            false
        }
    }

    pub fn snap_selected_vertices_to_grid(&mut self) -> usize {
        let step = self.grid_step.max(1.0);
        let unique = self.get_active_brush_unique_vertices();
        let target_world_points: Vec<Vec3> = self
            .selected_vertex_indices
            .iter()
            .filter_map(|&idx| unique.get(idx).copied())
            .collect();

        if target_world_points.is_empty() {
            self.log("Vertex Snap to Grid: No vertices selected.");
            return 0;
        }

        let is_world_brush = self.selected_brush_index.is_some();
        let sel_idx = self.selected_brush_index;
        let old_polys = if let Some(idx) = sel_idx {
            self.world_brushes.get(idx).map(|b| b.polys.clone())
        } else {
            None
        };
        let old_builder = if !is_world_brush {
            Some(self.builder_brush.clone())
        } else {
            None
        };

        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get_mut(idx).unwrap_or(&mut self.builder_brush)
        } else {
            &mut self.builder_brush
        };

        let loc = brush.location;
        let mut modified_count = 0;

        for poly in &mut brush.polys {
            let mut poly_modified = false;
            for v in &mut poly.vertices {
                let world_v = *v + loc;
                for target_p in &target_world_points {
                    if world_v.distance_squared(*target_p) < 0.01 {
                        let snapped_world = Vec3::new(
                            (world_v.x / step).round() * step,
                            (world_v.y / step).round() * step,
                            (world_v.z / step).round() * step,
                        );
                        let delta = snapped_world - world_v;
                        if delta.length_squared() > 0.0001 {
                            *v += delta;
                            poly_modified = true;
                            modified_count += 1;
                        }
                        break;
                    }
                }
            }
            if poly_modified && poly.vertices.len() >= 3 {
                if let Some(new_plane) = Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                    poly.plane = new_plane;
                }
            }
        }

        if modified_count > 0 {
            if is_world_brush {
                if let Some(idx) = sel_idx {
                    if let Some(old) = old_polys {
                        if let Some(b) = self.world_brushes.get(idx) {
                            self.history.push(crate::history::Action::ModifyBrushGeometry {
                                brush_index: idx,
                                old_polys: old,
                                new_polys: b.polys.clone(),
                                description: format!("Snap Vertices to Grid ({:.0} uu)", step),
                            });
                        }
                    }
                }
            } else if let Some(old) = old_builder {
                self.history.push(crate::history::Action::ModifyBuilderBrush {
                    old_brush: old,
                    new_brush: self.builder_brush.clone(),
                    description: format!("Snap Builder Vertices to Grid ({:.0} uu)", step),
                });
            }
        }

        self.log(format!("Vertex Grid Snap: Snapped {} vertex occurrences to grid step {:.0} uu.", modified_count, step));
        modified_count
    }

    pub fn weld_selected_vertices_to_nearest(&mut self, tolerance: f32) -> usize {
        let unique = self.get_active_brush_unique_vertices();
        let selected_points: Vec<Vec3> = self
            .selected_vertex_indices
            .iter()
            .filter_map(|&idx| unique.get(idx).copied())
            .collect();

        if selected_points.is_empty() {
            self.log("Magnetic Weld: No vertices selected.");
            return 0;
        }

        // Candidates from all other world brushes
        let active_idx = self.selected_brush_index;
        let mut candidate_verts: Vec<Vec3> = Vec::new();
        for (i, b) in self.world_brushes.iter().enumerate() {
            if Some(i) == active_idx {
                continue;
            }
            for p in &b.polys {
                for v in &p.vertices {
                    candidate_verts.push(*v + b.location);
                }
            }
        }

        // Also add non-selected vertices from active brush as weld targets
        for (idx, &v) in unique.iter().enumerate() {
            if !self.selected_vertex_indices.contains(&idx) {
                candidate_verts.push(v);
            }
        }

        if candidate_verts.is_empty() {
            self.log("Magnetic Weld: No target vertices found in range.");
            return 0;
        }

        let is_world_brush = self.selected_brush_index.is_some();
        let sel_idx = self.selected_brush_index;
        let old_polys = if let Some(idx) = sel_idx {
            self.world_brushes.get(idx).map(|b| b.polys.clone())
        } else {
            None
        };
        let old_builder = if !is_world_brush {
            Some(self.builder_brush.clone())
        } else {
            None
        };

        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get_mut(idx).unwrap_or(&mut self.builder_brush)
        } else {
            &mut self.builder_brush
        };

        let loc = brush.location;
        let mut welded_count = 0;
        let tol_sq = tolerance * tolerance;

        // Map each selected point to its nearest candidate
        let mut weld_targets: Vec<(Vec3, Vec3)> = Vec::new();
        for sel_v in &selected_points {
            let mut nearest = None;
            let mut min_d = tol_sq;
            for cand_v in &candidate_verts {
                let d = sel_v.distance_squared(*cand_v);
                if d > 0.0001 && d <= min_d {
                    min_d = d;
                    nearest = Some(*cand_v);
                }
            }
            if let Some(target) = nearest {
                weld_targets.push((*sel_v, target));
            }
        }

        if weld_targets.is_empty() {
            self.log(format!("Magnetic Weld: No vertices within magnetic radius of {:.0} uu.", tolerance));
            return 0;
        }

        for poly in &mut brush.polys {
            let mut poly_modified = false;
            for v in &mut poly.vertices {
                let world_v = *v + loc;
                for (from_w, to_w) in &weld_targets {
                    if world_v.distance_squared(*from_w) < 0.01 {
                        *v = *to_w - loc;
                        poly_modified = true;
                        welded_count += 1;
                        break;
                    }
                }
            }

            if poly_modified {
                let mut deduplicated: Vec<Vec3> = Vec::new();
                for vert in &poly.vertices {
                    if let Some(last) = deduplicated.last() {
                        if vert.distance_squared(*last) < 0.01 {
                            continue;
                        }
                    }
                    deduplicated.push(*vert);
                }
                if deduplicated.len() > 1 && deduplicated.first().unwrap().distance_squared(*deduplicated.last().unwrap()) < 0.01 {
                    deduplicated.pop();
                }

                if deduplicated.len() >= 3 {
                    poly.vertices = deduplicated;
                    if let Some(new_plane) = Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                        poly.plane = new_plane;
                    }
                }
            }
        }

        if welded_count > 0 {
            if is_world_brush {
                if let Some(idx) = sel_idx {
                    if let Some(old) = old_polys {
                        if let Some(b) = self.world_brushes.get(idx) {
                            self.history.push(crate::history::Action::ModifyBrushGeometry {
                                brush_index: idx,
                                old_polys: old,
                                new_polys: b.polys.clone(),
                                description: format!("Magnetic Weld (tol {:.0} uu)", tolerance),
                            });
                        }
                    }
                }
            } else if let Some(old) = old_builder {
                self.history.push(crate::history::Action::ModifyBuilderBrush {
                    old_brush: old,
                    new_brush: self.builder_brush.clone(),
                    description: format!("Magnetic Weld Builder (tol {:.0} uu)", tolerance),
                });
            }
        }

        self.log(format!("Magnetic Weld: Welded {} vertex occurrences within radius {:.0} uu.", welded_count, tolerance));
        welded_count
    }

    pub fn split_selected_edge(&mut self) -> bool {
        if self.selected_vertex_indices.len() != 2 {
            self.log("Edge Split: Exactly 2 vertices must be selected to split an edge.");
            return false;
        }

        let unique = self.get_active_brush_unique_vertices();
        let idx0 = self.selected_vertex_indices[0];
        let idx1 = self.selected_vertex_indices[1];

        let p0 = match unique.get(idx0) {
            Some(&p) => p,
            None => return false,
        };
        let p1 = match unique.get(idx1) {
            Some(&p) => p,
            None => return false,
        };

        let midpoint_world = (p0 + p1) * 0.5;

        let is_world_brush = self.selected_brush_index.is_some();
        let sel_idx = self.selected_brush_index;
        let old_polys = if let Some(idx) = sel_idx {
            self.world_brushes.get(idx).map(|b| b.polys.clone())
        } else {
            None
        };
        let old_builder = if !is_world_brush {
            Some(self.builder_brush.clone())
        } else {
            None
        };

        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get_mut(idx).unwrap_or(&mut self.builder_brush)
        } else {
            &mut self.builder_brush
        };

        let loc = brush.location;
        let midpoint_local = midpoint_world - loc;
        let mut modified_polys = 0;

        for poly in &mut brush.polys {
            let n = poly.vertices.len();
            if n < 3 {
                continue;
            }

            let mut insert_pos = None;
            for i in 0..n {
                let v_a = poly.vertices[i] + loc;
                let v_b = poly.vertices[(i + 1) % n] + loc;

                let matches_forward = v_a.distance_squared(p0) < 0.01 && v_b.distance_squared(p1) < 0.01;
                let matches_reverse = v_a.distance_squared(p1) < 0.01 && v_b.distance_squared(p0) < 0.01;

                if matches_forward || matches_reverse {
                    insert_pos = Some(i + 1);
                    break;
                }
            }

            if let Some(pos) = insert_pos {
                poly.vertices.insert(pos, midpoint_local);
                if let Some(new_plane) = Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                    poly.plane = new_plane;
                }
                modified_polys += 1;
            }
        }

        if modified_polys > 0 {
            if is_world_brush {
                if let Some(idx) = sel_idx {
                    if let Some(old) = old_polys {
                        if let Some(b) = self.world_brushes.get(idx) {
                            self.history.push(crate::history::Action::ModifyBrushGeometry {
                                brush_index: idx,
                                old_polys: old,
                                new_polys: b.polys.clone(),
                                description: "Split Polygon Edge".to_string(),
                            });
                        }
                    }
                }
            } else if let Some(old) = old_builder {
                self.history.push(crate::history::Action::ModifyBuilderBrush {
                    old_brush: old,
                    new_brush: self.builder_brush.clone(),
                    description: "Split Builder Brush Edge".to_string(),
                });
            }

            // Auto-select the newly created vertex
            let new_unique = self.get_active_brush_unique_vertices();
            if let Some(new_idx) = new_unique.iter().position(|&v| v.distance_squared(midpoint_world) < 0.01) {
                self.selected_vertex_indices = vec![new_idx];
            }

            self.log(format!("Edge Split: Inserted new vertex at ({:.1}, {:.1}, {:.1}) across {} shared polygon(s).", midpoint_world.x, midpoint_world.y, midpoint_world.z, modified_polys));
            true
        } else {
            self.log("Edge Split: Selected vertices do not form an existing polygon edge.");
            false
        }
    }

    pub fn collapse_selected_edge(&mut self) -> bool {
        if self.selected_vertex_indices.len() != 2 {
            self.log("Edge Collapse: Exactly 2 vertices must be selected to collapse an edge.");
            return false;
        }

        let unique = self.get_active_brush_unique_vertices();
        let idx0 = self.selected_vertex_indices[0];
        let idx1 = self.selected_vertex_indices[1];

        let p0 = match unique.get(idx0) {
            Some(&p) => p,
            None => return false,
        };
        let p1 = match unique.get(idx1) {
            Some(&p) => p,
            None => return false,
        };

        let midpoint_world = (p0 + p1) * 0.5;

        let is_world_brush = self.selected_brush_index.is_some();
        let sel_idx = self.selected_brush_index;
        let old_polys = if let Some(idx) = sel_idx {
            self.world_brushes.get(idx).map(|b| b.polys.clone())
        } else {
            None
        };
        let old_builder = if !is_world_brush {
            Some(self.builder_brush.clone())
        } else {
            None
        };

        let brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get_mut(idx).unwrap_or(&mut self.builder_brush)
        } else {
            &mut self.builder_brush
        };

        let loc = brush.location;
        let midpoint_local = midpoint_world - loc;
        let mut modified_polys = 0;

        for poly in &mut brush.polys {
            let mut touched = false;
            for v in &mut poly.vertices {
                let v_world = *v + loc;
                if v_world.distance_squared(p0) < 0.01 || v_world.distance_squared(p1) < 0.01 {
                    *v = midpoint_local;
                    touched = true;
                }
            }

            if touched {
                // Deduplicate consecutive cyclic vertices
                let mut deduped: Vec<Vec3> = Vec::new();
                for &v in &poly.vertices {
                    if deduped.is_empty() || deduped.last().unwrap().distance_squared(v) >= 0.01 {
                        deduped.push(v);
                    }
                }
                if deduped.len() > 1 && deduped.first().unwrap().distance_squared(*deduped.last().unwrap()) < 0.01 {
                    deduped.pop();
                }
                poly.vertices = deduped;

                if poly.vertices.len() >= 3 {
                    if let Some(new_plane) = Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                        poly.plane = new_plane;
                    }
                }
                modified_polys += 1;
            }
        }

        // Drop any collapsed/degenerate polygons (< 3 vertices)
        let before_count = brush.polys.len();
        brush.polys.retain(|p| p.vertices.len() >= 3);
        let dropped_polys = before_count.saturating_sub(brush.polys.len());

        if modified_polys > 0 {
            if is_world_brush {
                if let Some(idx) = sel_idx {
                    if let Some(old) = old_polys {
                        if let Some(b) = self.world_brushes.get(idx) {
                            self.history.push(crate::history::Action::ModifyBrushGeometry {
                                brush_index: idx,
                                old_polys: old,
                                new_polys: b.polys.clone(),
                                description: "Collapse Edge / Merge Vertices".to_string(),
                            });
                        }
                    }
                }
            } else if let Some(old) = old_builder {
                self.history.push(crate::history::Action::ModifyBuilderBrush {
                    old_brush: old,
                    new_brush: self.builder_brush.clone(),
                    description: "Collapse Edge / Merge Vertices".to_string(),
                });
            }

            // Auto-select the merged vertex
            let new_unique = self.get_active_brush_unique_vertices();
            if let Some(new_idx) = new_unique.iter().position(|&v| v.distance_squared(midpoint_world) < 0.01) {
                self.selected_vertex_indices = vec![new_idx];
            } else {
                self.selected_vertex_indices.clear();
            }

            self.log(format!(
                "Edge Collapse: Merged 2 vertices into ({:.1}, {:.1}, {:.1}) across {} polygon(s) ({} degenerate polygon(s) dissolved).",
                midpoint_world.x, midpoint_world.y, midpoint_world.z, modified_polys, dropped_polys
            ));
            true
        } else {
            self.log("Edge Collapse: Selected vertices did not affect any polygons.");
            false
        }
    }


    // ==========================================
    // Brush Clipping Tool Methods
    // ==========================================
    pub fn add_clip_point(&mut self, point: Vec3) {
        if self.clip_points.len() >= 3 {
            self.clip_points.clear();
        }
        self.clip_points.push(point);
        self.log(format!("Brush Clip: Placed clip marker #{} at ({:.0}, {:.0}, {:.0})", self.clip_points.len(), point.x, point.y, point.z));
    }

    pub fn clear_clip_points(&mut self) {
        self.clip_points.clear();
        self.log("Brush Clip: Cleared all clip markers.");
    }

    pub fn flip_clip_normal(&mut self) {
        self.clip_normal_flipped = !self.clip_normal_flipped;
        self.log(format!("Brush Clip: Normal flipped = {}", self.clip_normal_flipped));
    }

    pub fn get_clipping_plane(&self) -> Option<Plane> {
        if self.clip_points.len() < 2 {
            return None;
        }

        let mut plane = if self.clip_points.len() == 3 {
            Plane::from_points(self.clip_points[0], self.clip_points[1], self.clip_points[2])?
        } else {
            // 2 points: vertical cutting plane
            let dir = (self.clip_points[1] - self.clip_points[0]).normalize();
            let norm = dir.cross(Vec3::Z).normalize();
            if norm.length_squared() < 0.5 {
                let alt_norm = dir.cross(Vec3::Y).normalize();
                Plane::from_point_and_normal(self.clip_points[0], alt_norm)
            } else {
                Plane::from_point_and_normal(self.clip_points[0], norm)
            }
        };

        if self.clip_normal_flipped {
            plane.normal = -plane.normal;
            plane.dist = -plane.dist;
        }
        Some(plane)
    }

    pub fn execute_clip_brush(&mut self) {
        let plane = match self.get_clipping_plane() {
            Some(p) => p,
            None => {
                self.log("Brush Clip: Need at least 2 or 3 clip markers to form a cutting plane.");
                return;
            }
        };

        let target_brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get(idx).cloned().unwrap_or_else(|| self.builder_brush.clone())
        } else {
            self.builder_brush.clone()
        };

        // Convert target brush to world coordinates for clipping
        let mut world_b = target_brush.clone();
        let loc = target_brush.location;
        for p in &mut world_b.polys {
            for v in &mut p.vertices {
                *v += loc;
            }
            if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                p.plane = np;
            }
        }

        if let Some(mut clipped_world) = clip_brush_by_plane(&world_b, &plane, true) {
            // Convert back to local coordinates
            for p in &mut clipped_world.polys {
                for v in &mut p.vertices {
                    *v -= loc;
                }
                if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                    p.plane = np;
                }
            }

            if let Some(idx) = self.selected_brush_index {
                let old_polys = self.world_brushes[idx].polys.clone();
                self.world_brushes[idx].polys = clipped_world.polys;
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: idx,
                    old_polys,
                    new_polys: self.world_brushes[idx].polys.clone(),
                    description: "Clip Brush".to_string(),
                });
                self.log(format!("Brush Clip: Cut world brush {} along plane ({} polys remaining).", idx, self.world_brushes[idx].polys.len()));
            } else {
                let old_brush = self.builder_brush.clone();
                self.builder_brush.polys = clipped_world.polys;
                self.history.push(Action::ModifyBuilderBrush {
                    old_brush,
                    new_brush: self.builder_brush.clone(),
                    description: "Clip Builder Brush".to_string(),
                });
                self.log(format!("Brush Clip: Cut Active Red Builder Brush ({} polys remaining).", self.builder_brush.polys.len()));
            }
        } else {
            self.log("Brush Clip: Entire brush was outside the kept half-space.");
        }
    }

    pub fn execute_split_brush(&mut self) {
        let plane = match self.get_clipping_plane() {
            Some(p) => p,
            None => {
                self.log("Brush Split: Need at least 2 or 3 clip markers to form a cutting plane.");
                return;
            }
        };

        let target_brush = if let Some(idx) = self.selected_brush_index {
            self.world_brushes.get(idx).cloned().unwrap_or_else(|| self.builder_brush.clone())
        } else {
            self.builder_brush.clone()
        };

        let mut world_b = target_brush.clone();
        let loc = target_brush.location;
        for p in &mut world_b.polys {
            for v in &mut p.vertices {
                *v += loc;
            }
            if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                p.plane = np;
            }
        }

        let (front_opt, back_opt) = split_brush_by_plane(&world_b, &plane);
        if let (Some(mut front_b), Some(mut back_b)) = (front_opt, back_opt) {
            for p in &mut front_b.polys {
                for v in &mut p.vertices { *v -= loc; }
                if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) { p.plane = np; }
            }
            for p in &mut back_b.polys {
                for v in &mut p.vertices { *v -= loc; }
                if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) { p.plane = np; }
            }

            if let Some(idx) = self.selected_brush_index {
                let old_polys = self.world_brushes[idx].polys.clone();
                self.world_brushes[idx].polys = front_b.polys;
                back_b.name = format!("{}_Split", self.world_brushes[idx].name);
                let added_brush = back_b.clone();
                self.world_brushes.push(back_b);
                self.history.push(Action::Compound {
                    actions: vec![
                        Action::ModifyBrushGeometry {
                            brush_index: idx,
                            old_polys,
                            new_polys: self.world_brushes[idx].polys.clone(),
                            description: "Split Brush (Front)".to_string(),
                        },
                        Action::AddBrush {
                            brush: added_brush,
                        },
                    ],
                    description: "Split Brush".to_string(),
                });
                self.log("Brush Split: Split brush into two valid solid brushes (front kept, back added as new brush).".to_string());
            } else {
                let old_builder = self.builder_brush.clone();
                self.builder_brush.polys = front_b.polys;
                back_b.name = format!("Brush_SplitPiece_{}", self.world_brushes.len());
                back_b.csg_oper = CsgOperation::Add;
                let added_brush = back_b.clone();
                self.world_brushes.push(back_b);
                self.history.push(Action::Compound {
                    actions: vec![
                        Action::ModifyBuilderBrush {
                            old_brush: old_builder,
                            new_brush: self.builder_brush.clone(),
                            description: "Split Builder Brush (Front)".to_string(),
                        },
                        Action::AddBrush {
                            brush: added_brush,
                        },
                    ],
                    description: "Split Builder Brush".to_string(),
                });
                self.log("Brush Split: Active Builder Brush split into two pieces (one kept in builder, one placed in world).".to_string());
            }
        } else {
            self.log("Brush Split: Plane did not cross the brush geometry.");
        }
    }

    // ==========================================
    // 2D Shape Editor Engine Methods
    // ==========================================
    pub fn apply_shape2d_extrude(&mut self) {
        let b = self.shape2d.extrude(self.shape_editor_depth);
        if !b.polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let rot = self.builder_brush.rotation;
            self.builder_brush = b;
            self.builder_brush.location = loc;
            self.builder_brush.rotation = rot;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: format!("2D Shape Extrude ({:.0})", self.shape_editor_depth),
            });
            self.log(format!("2D Shape Editor: Extruded 2D polygon (depth: {:.0}) into Active Builder Brush.", self.shape_editor_depth));
        } else {
            self.log("2D Shape Editor: Shape needs at least 3 points to extrude.");
        }
    }

    pub fn apply_shape2d_revolve(&mut self) {
        let b = self.shape2d.revolve(self.shape_editor_revolve_angle, self.shape_editor_revolve_segs, 0.0);
        if !b.polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let rot = self.builder_brush.rotation;
            self.builder_brush = b;
            self.builder_brush.location = loc;
            self.builder_brush.rotation = rot;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: format!("2D Shape Revolve ({:.0}°)", self.shape_editor_revolve_angle),
            });
            self.log(format!("2D Shape Editor: Revolved profile (angle: {:.0}°, segs: {}) into Active Builder Brush.", self.shape_editor_revolve_angle, self.shape_editor_revolve_segs));
        } else {
            self.log("2D Shape Editor: Profile needs at least 2 points to revolve.");
        }
    }

    pub fn apply_shape2d_pyramid(&mut self, height: f32) {
        let b = self.shape2d.extrude_to_point(height);
        if !b.polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let rot = self.builder_brush.rotation;
            self.builder_brush = b;
            self.builder_brush.location = loc;
            self.builder_brush.rotation = rot;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: format!("2D Shape Pyramid ({:.0})", height),
            });
            self.log(format!("2D Shape Editor: Extruded to apex point (height: {:.0}) into Active Builder Brush.", height));
        }
    }

    pub fn apply_shape2d_bevel(&mut self) {
        let b = self.shape2d.extrude_to_bevel(self.shape_editor_depth, self.shape_editor_bevel_depth, self.shape_editor_bevel_width);
        if !b.polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let rot = self.builder_brush.rotation;
            self.builder_brush = b;
            self.builder_brush.location = loc;
            self.builder_brush.rotation = rot;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: format!("2D Shape Bevel ({:.0})", self.shape_editor_depth),
            });
            self.log(format!("2D Shape Editor: Beveled extrusion (depth: {:.0}, bevel: {:.0}x{:.0}) into Builder Brush.", self.shape_editor_depth, self.shape_editor_bevel_depth, self.shape_editor_bevel_width));
        }
    }

    pub fn apply_shape2d_sheet(&mut self) {
        let b = self.shape2d.to_sheet();
        if !b.polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let rot = self.builder_brush.rotation;
            self.builder_brush = b;
            self.builder_brush.location = loc;
            self.builder_brush.rotation = rot;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "2D Shape Sheet".to_string(),
            });
            self.log("2D Shape Editor: Converted 2D shape to sheet into Active Builder Brush.");
        }
    }


    pub fn extract_selected_surfaces_to_builder_brush(&mut self) -> bool {
        let mut extracted_world_polys = Vec::new();

        if !self.selected_surfaces.is_empty() {
            for &(b_idx, p_idx) in &self.selected_surfaces {
                if let Some(brush) = self.world_brushes.get(b_idx) {
                    let w_polys = brush.to_world_polys();
                    if let Some(poly) = w_polys.get(p_idx) {
                        extracted_world_polys.push(poly.clone());
                    }
                }
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get(b_idx) {
                extracted_world_polys = brush.to_world_polys();
            }
        }

        if extracted_world_polys.is_empty() {
            self.log("No surface selected to extract to Builder Brush.");
            return false;
        }

        // Calculate centroid of all extracted vertices in world space
        let mut sum = Vec3::ZERO;
        let mut total_verts = 0;
        for poly in &extracted_world_polys {
            for &v in &poly.vertices {
                sum += v;
                total_verts += 1;
            }
        }
        if total_verts == 0 {
            return false;
        }
        let center = sum / (total_verts as f32);

        // Convert world polys to local polys relative to center
        let mut local_polys = Vec::new();
        for w_poly in &extracted_world_polys {
            let local_verts: Vec<Vec3> = w_poly.vertices.iter().map(|&v| v - center).collect();
            if let Some(mut local_poly) = ut99_bsp::Polygon::new(local_verts) {
                local_poly.texture = w_poly.texture.clone();
                local_poly.flags = w_poly.flags;
                local_poly.base = w_poly.base - center;
                local_poly.texture_u = w_poly.texture_u;
                local_poly.texture_v = w_poly.texture_v;
                local_polys.push(local_poly);
            }
        }

        if local_polys.is_empty() {
            return false;
        }

        let old_brush = self.builder_brush.clone();
        let mut new_brush = ut99_bsp::Brush::new("ExtractedSheet", ut99_bsp::CsgOperation::Add);
        new_brush.location = center;
        new_brush.rotation = Vec3::ZERO;
        new_brush.scale = Vec3::ONE;
        new_brush.polys = local_polys;

        self.history.push(crate::history::Action::ModifyBuilderBrush {
            old_brush,
            new_brush: new_brush.clone(),
            description: format!("Extracted {} surface(s) to Builder Brush", extracted_world_polys.len()),
        });

        self.builder_brush = new_brush;
        self.selected_brush_index = None;
        self.deselect_all_surfaces();
        self.log(format!("Extracted {} surface(s) into Red Builder Brush at ({:.0}, {:.0}, {:.0}).",
            extracted_world_polys.len(), center.x, center.y, center.z));
        true
    }

    /// Turns the current Red Builder Brush into a hollow room with the specified wall thickness.
    /// Generates inward-facing shell polygons with inverted normals.
    pub fn make_builder_brush_hollow(&mut self, wall_thickness: f32) -> bool {
        if wall_thickness <= 0.0 || self.builder_brush.polys.is_empty() {
            self.log("Invalid wall thickness or empty builder brush.");
            return false;
        }

        let old_brush = self.builder_brush.clone();
        let mut inner_polys = Vec::new();

        for poly in &old_brush.polys {
            let n = poly.plane.normal;
            if n.length_squared() < 1e-4 {
                continue;
            }
            let mut inner_verts: Vec<Vec3> = poly.vertices.iter().map(|&v| v - n * wall_thickness).collect();
            // Invert winding so the inner normal points inward into the hollow room
            inner_verts.reverse();

            if let Some(mut inner_poly) = ut99_bsp::Polygon::new(inner_verts) {
                inner_poly.texture = poly.texture.clone();
                inner_poly.flags = poly.flags;
                inner_poly.texture_u = poly.texture_u;
                inner_poly.texture_v = poly.texture_v;
                inner_poly.base = poly.base - n * wall_thickness;
                inner_polys.push(inner_poly);
            }
        }

        if inner_polys.is_empty() {
            self.log("Failed to construct hollow inner shell.");
            return false;
        }

        let inner_count = inner_polys.len();
        self.builder_brush.polys.extend(inner_polys);

        self.history.push(crate::history::Action::ModifyBuilderBrush {
            old_brush,
            new_brush: self.builder_brush.clone(),
            description: format!("Make Hollow Brush ({:.0} uu)", wall_thickness),
        });

        self.log(format!("Made Builder Brush hollow with wall thickness {:.0} uu (added {} inner faces, {} total).",
            wall_thickness, inner_count, self.builder_brush.polys.len()));
        true
    }

}
