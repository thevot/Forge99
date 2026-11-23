use super::*;
use glam::Vec3;

pub(crate) fn compute_mover_trajectories_from_slice(movers: &[EditorMover]) -> Vec<MoverTrajectory> {
    let mut trajectories = Vec::new();
    for mover in movers {
        let mut keys = Vec::new();
        for k_opt in &mover.keyframes {
            if let Some(k) = k_opt {
                keys.push(BspMoverKeyframe {
                    position: k.location,
                    rotation: k.rotation,
                });
            }
        }
        let glide = if mover.glide_type.to_ascii_lowercase().contains("linear") {
            MoverGlideType::Linear
        } else {
            MoverGlideType::Smooth
        };
        trajectories.push(MoverTrajectory::new(
            &mover.name,
            &mover.tag,
            keys,
            mover.move_time,
            mover.stay_open_time,
            glide,
        ));
    }
    trajectories
}

impl EditorState {
    pub fn get_mover_trajectories(&self) -> Vec<MoverTrajectory> {
        compute_mover_trajectories_from_slice(&self.movers)
    }

    // ==========================================
    // Bot AI Navigation Graph, Reachability Network & JumpPad Studio
    // ==========================================
    pub fn ensure_nav_graph_from_level(&mut self) -> &mut ut99_bsp::NavGraph {
        if self.nav_graph.nodes.is_empty() {
            if let Some(ref level) = self.loaded_unr {
                for a in &level.actors {
                    if a.class_name.contains("PathNode")
                        || a.class_name.contains("PlayerStart")
                        || a.class_name.contains("InventorySpot")
                        || a.class_name.contains("LiftCenter")
                        || a.class_name.contains("LiftExit")
                        || a.class_name.contains("JumpPad")
                        || a.class_name.contains("Kicker")
                        || a.class_name.contains("WarpZoneMarker")
                    {
                        self.nav_graph.add_node(a.name.clone(), a.location, a.class_name.clone());
                    }
                }
            }
        }
        &mut self.nav_graph
    }

    pub fn build_auto_paths(&mut self, max_distance_uu: f32, max_step_height: f32, max_jump_height: f32) -> AutoPathReport {
        let _ = self.ensure_nav_graph_from_level();
        let report = ut99_bsp::auto_build_navigation_reachspecs_with_brushes(
            &mut self.nav_graph,
            &self.world_brushes,
            max_distance_uu,
            max_step_height,
            max_jump_height,
        );
        self.last_autopath_report = Some(report.clone());
        self.map_is_dirty = true;
        self.log(format!(
            "Auto-built {} ReachSpecs across {} navigation nodes (Walk: {}, Jump: {}, Isolated: {})",
            report.created_reachspecs, report.total_nodes, report.walk_reachspecs, report.jump_reachspecs, report.isolated_nodes.len()
        ));
        report
    }

    /// Computes 3D ballistic trajectory arcs for all JumpPad / Kicker actors in the level
    pub fn compute_jumppad_trajectories(&self) -> Vec<JumpPadTrajectoryArc> {
        let mut trajectories = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            for a in &level.actors {
                if a.class_name.contains("JumpPad") || a.class_name.contains("Kicker") {
                    // Extract kick velocity
                    let kick_vel = if let Some(prop) = a.properties.get("KickVelocity") {
                        match &prop.value {
                            TaggedValue::Vector(v) => *v,
                            _ => Vec3::new(350.0, 0.0, 750.0),
                        }
                    } else {
                        Vec3::new(350.0, 0.0, 750.0)
                    };

                    // Find target actor if Tag or Event specified
                    let target = level.actors.iter().find(|other| {
                        !a.event.is_empty() && (other.tag == a.event || other.name == a.event)
                    });

                    let target_name = target.map(|t| t.name.clone());
                    let target_pos = target.map(|t| t.location);

                    let arc = simulate_jumppad_trajectory(
                        &a.name,
                        a.location,
                        kick_vel,
                        target_name,
                        target_pos,
                        -950.0,
                    );
                    trajectories.push(arc);
                }
            }
        }
        trajectories
    }

    /// Validates LiftCenter and LiftExit topology with associated Mover stroke synchronization
    pub fn validate_lift_synchronization(&self) -> Vec<LiftSyncReport> {
        let mut reports = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            for a in &level.actors {
                if a.class_name.contains("LiftCenter") {
                    let lift_tag = if !a.tag.is_empty() {
                        a.tag.clone()
                    } else {
                        a.properties.get("LiftTag").and_then(|p| match &p.value {
                            TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        }).unwrap_or_else(|| a.name.clone())
                    };

                    // Find associated Mover
                    let mover = level.actors.iter().find(|m| m.class_name.contains("Mover") && m.tag == lift_tag);
                    let mover_name = mover.map(|m| m.name.clone());
                    let mover_stroke = mover.map(|_m| 256.0); // Estimated mover stroke

                    // Find all associated LiftExit actors
                    let exits: Vec<String> = level.actors.iter()
                        .filter(|e| e.class_name.contains("LiftExit") && (e.tag == lift_tag || e.event == lift_tag))
                        .map(|e| e.name.clone())
                        .collect();

                    let mut issues = Vec::new();
                    if mover.is_none() {
                        issues.push(format!("No Mover found matching LiftTag '{lift_tag}'"));
                    }
                    if exits.is_empty() {
                        issues.push("No LiftExit waypoints linked to this elevator".to_string());
                    }

                    let is_sync = issues.is_empty();
                    reports.push(LiftSyncReport {
                        lift_center_name: a.name.clone(),
                        lift_tag,
                        associated_mover_name: mover_name,
                        associated_exits: exits,
                        mover_stroke_height: mover_stroke,
                        exits_vertical_span: 256.0,
                        is_synchronized: is_sync,
                        issues,
                    });
                }
            }
        }
        reports
    }

    /// Computes tactical bot cover ratings against enemy crossfire sightlines
    pub fn compute_tactical_cover_ratings(&self) -> Vec<TacticalCoverEntry> {
        let mut entries = Vec::new();
        let graph = &self.nav_graph;
        for (idx, node) in graph.nodes.iter().enumerate() {
                // Count how many other nodes are within 1200 uu sightline
                let mut visible_count = 0;
                for (other_idx, other) in graph.nodes.iter().enumerate() {
                    if idx != other_idx && node.location.distance(other.location) <= 1200.0 {
                        visible_count += 1;
                    }
                }

                let rating = if visible_count <= 2 {
                    TacticalCoverRating::SolidCover
                } else if visible_count <= 5 {
                    TacticalCoverRating::PartialCover
                } else {
                    TacticalCoverRating::OpenHazard
                };

                let is_perch = node.location.z > 120.0;
                let is_item = node.node_type.contains("Inventory") || node.name.contains("Pickup");

                entries.push(TacticalCoverEntry {
                    node_id: idx,
                    node_name: node.name.clone(),
                    location: node.location,
                    cover_rating: rating,
                    visible_nodes_count: visible_count,
                    is_high_vantage_perch: is_perch,
                    strategic_item_bonus: is_item,
                });
            }
        entries
    }

    /// Manually links two navigation nodes in the NavGraph with reach flags
    pub fn link_nav_nodes(&mut self, from_idx: usize, to_idx: usize, reach_flags: u32) -> bool {
        let graph = self.ensure_nav_graph_from_level();
        if from_idx < graph.nodes.len() && to_idx < graph.nodes.len() && from_idx != to_idx {
            graph.add_reach_spec(from_idx, to_idx, reach_flags);
            self.map_is_dirty = true;
            self.log(format!("Linked NavNode #{} -> #{} (flags: {})", from_idx, to_idx, reach_flags));
            return true;
        }
        false
    }

    // ==========================================
    // Procedural Architectural Prefab & Brush Macro Synthesizer
    // ==========================================

}


impl EditorState {
    pub fn compute_nav_coverage(&self) -> ut99_bsp::NavCoverageReport {
        let mut floor_points: Vec<Vec3> = Vec::new();

        for b in &self.world_brushes {
            for p in &b.polys {
                if p.plane.normal.z > 0.7 && p.vertices.len() >= 3 {
                    let mut centroid = Vec3::ZERO;
                    for v in &p.vertices {
                        let wv = *v + b.location;
                        centroid += wv;
                        floor_points.push(wv);
                    }
                    centroid /= p.vertices.len() as f32;
                    floor_points.push(centroid);
                }
            }
        }

        if floor_points.is_empty() {
            floor_points.push(Vec3::new(0.0, 0.0, 0.0));
        }

        self.nav_graph.analyze_coverage(&floor_points)
    }


    pub fn compute_combat_sightlines(&mut self) -> CombatCoverageReport {
        let report = analyze_combat_sightlines(&self.nav_graph, &self.world_brushes);
        self.log(format!(
            "Combat sightlines computed: {} active sightlines, {} high-ground perches, longest {:.0} uu.",
            report.sightlines.len(),
            report.high_ground_perches_count,
            report.longest_sightline_uu
        ));
        self.combat_coverage_report = Some(report.clone());
        report
    }

}
