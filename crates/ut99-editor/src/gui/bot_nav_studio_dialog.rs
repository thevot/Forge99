use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use ut99_bsp::TacticalCoverRating;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BotNavTab {
    GraphNodes,
    JumpPads,
    LiftSync,
    TacticalCover,
}

/// Interactive Bot AI Navigation Graph, Reachability Network & JumpPad Studio
pub fn render_bot_nav_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_bot_nav_studio_dialog {
        return;
    }

    let mut is_open = state.show_bot_nav_studio_dialog;
    let tab_id = egui::Id::new("bot_nav_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<BotNavTab>(tab_id)
            .unwrap_or(BotNavTab::GraphNodes)
    });

    Window::new("Bot AI Navigation Graph & Tactical Cover Studio")
        .open(&mut is_open)
        .default_width(820.0)
        .default_height(560.0)
        .min_width(640.0)
        .min_height(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header bar
            ui.horizontal(|ui| {
                ui.heading("🤖 Bot Navigation Studio");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚡ Build Auto-Paths").clicked() {
                        let max_d = state.auto_path_max_distance;
                        let max_s = state.auto_path_max_step;
                        let max_j = state.auto_path_max_jump;
                        state.build_auto_paths(max_d, max_s, max_j);
                    }
                    if let Some(sel_id) = state.bot_nav_settings.selected_node_id {
                        if ui.button("📍 Focus Camera on Node").clicked() {
                            if let Some(node) = state.nav_graph.nodes.get(sel_id) {
                                state.camera.position = node.location - state.camera.forward() * 200.0;
                            }
                        }
                    }
                });
            });

            ui.separator();

            // Topology Network Summary Bar
            let node_count = state.nav_graph.nodes.len();
            let spec_count = state.nav_graph.reach_specs.len();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(100, 220, 255),
                        format!("Nodes: {} | ReachSpecs: {}", node_count, spec_count),
                    );
                    ui.separator();
                    ui.checkbox(&mut state.show_bot_paths, "Show 3D Paths");
                    ui.checkbox(&mut state.bot_nav_settings.show_jumppad_arcs, "JumpPad Arcs");
                    ui.checkbox(&mut state.bot_nav_settings.show_lift_linkages, "Lift Links");
                    ui.checkbox(&mut state.bot_nav_settings.show_tactical_cover, "Tactical Cover Rings");
                });
            });

            ui.add_space(4.0);

            // Tab Navigation
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == BotNavTab::GraphNodes, "🌐 Navigation Graph & Nodes").clicked() {
                    current_tab = BotNavTab::GraphNodes;
                }
                if ui.selectable_label(current_tab == BotNavTab::JumpPads, "🚀 JumpPad & Kicker Balistics").clicked() {
                    current_tab = BotNavTab::JumpPads;
                }
                if ui.selectable_label(current_tab == BotNavTab::LiftSync, "🛗 Lift Mover Synchronization").clicked() {
                    current_tab = BotNavTab::LiftSync;
                }
                if ui.selectable_label(current_tab == BotNavTab::TacticalCover, "🛡 Tactical Cover & Sightlines").clicked() {
                    current_tab = BotNavTab::TacticalCover;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            if state.nav_graph.nodes.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(RichText::new("No navigation graph loaded or constructed.").color(Color32::GRAY));
                    ui.add_space(10.0);
                    if ui.button("⚡ Scan Level & Build Auto-Paths").clicked() {
                        let max_d = state.auto_path_max_distance;
                        let max_s = state.auto_path_max_step;
                        let max_j = state.auto_path_max_jump;
                        state.build_auto_paths(max_d, max_s, max_j);
                    }
                });
                return;
            }

            match current_tab {
                BotNavTab::GraphNodes => render_graph_nodes_tab(ui, state),
                BotNavTab::JumpPads => render_jumppads_tab(ui, state),
                BotNavTab::LiftSync => render_lift_sync_tab(ui, state),
                BotNavTab::TacticalCover => render_tactical_cover_tab(ui, state),
            }
        });

    state.show_bot_nav_studio_dialog = is_open;
}

fn render_graph_nodes_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Navigation Nodes Directory & ReachSpec Topology").strong().size(13.0));
    ui.label(RichText::new("Inspect pathfinding waypoints, directional locomotion constraints, and vertical thresholds.").size(11.0).color(Color32::GRAY));
    ui.add_space(4.0);

    let graph = &state.nav_graph;

    let sel_id = state.bot_nav_settings.selected_node_id;

    ui.horizontal(|ui| {
        // Left Column: Nodes Table
        ui.vertical(|ui| {
            ui.set_width(380.0);
            egui::ScrollArea::vertical().id_salt("nodes_list_scroll").max_height(340.0).show(ui, |ui| {
                egui::Grid::new("nodes_list_grid")
                    .striped(true)
                    .min_col_width(70.0)
                    .spacing([8.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("#").strong());
                        ui.label(RichText::new("Name").strong());
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Z Height").strong());
                        ui.label(RichText::new("Action").strong());
                        ui.end_row();

                        let mut to_select = None;
                        for (idx, node) in graph.nodes.iter().enumerate() {
                            let is_sel = sel_id == Some(idx);
                            let name_text = if is_sel {
                                RichText::new(&node.name).strong().color(Color32::from_rgb(255, 210, 40))
                            } else {
                                RichText::new(&node.name)
                            };

                            ui.label(format!("#{idx}"));
                            ui.label(name_text);
                            ui.label(&node.node_type);
                            ui.label(format!("{:.0}", node.location.z));

                            if ui.button(if is_sel { "Selected" } else { "Select" }).clicked() {
                                to_select = Some(idx);
                            }
                            ui.end_row();
                        }

                        if let Some(s) = to_select {
                            state.bot_nav_settings.selected_node_id = Some(s);
                        }
                    });
            });
        });

        ui.separator();

        // Right Column: Node Details & Connected Edges
        ui.vertical(|ui| {
            if let Some(sel) = sel_id {
                if let Some(node) = graph.nodes.get(sel) {
                    ui.label(RichText::new(format!("Node #{sel}: '{}'", node.name)).strong().size(14.0));
                    ui.label(format!("Class: {}", node.node_type));
                    ui.label(format!("Location: ({:.1}, {:.1}, {:.1})", node.location.x, node.location.y, node.location.z));
                    ui.separator();

                    ui.label(RichText::new("Outgoing ReachSpecs (Destinations):").strong());
                    let outgoing: Vec<&ut99_bsp::NavReachSpec> = graph.reach_specs.iter()
                        .filter(|s| s.from == sel)
                        .collect();

                    if outgoing.is_empty() {
                        ui.colored_label(Color32::from_rgb(255, 100, 100), "No outgoing reachspecs (dead end node).");
                    } else {
                        egui::ScrollArea::vertical().id_salt("outgoing_specs_scroll").max_height(200.0).show(ui, |ui| {
                            egui::Grid::new("out_specs_grid").striped(true).spacing([8.0, 4.0]).show(ui, |ui| {
                                ui.label(RichText::new("To Node").strong());
                                ui.label(RichText::new("Distance").strong());
                                ui.label(RichText::new("Flags").strong());
                                ui.end_row();

                                for spec in outgoing {
                                    let target_name = graph.nodes.get(spec.to).map(|n| n.name.as_str()).unwrap_or("Unknown");
                                    ui.label(format!("#{} ({})", spec.to, target_name));
                                    ui.label(format!("{:.0} uu", spec.distance));
                                    let flags_desc = if (spec.reach_flags & ut99_bsp::R_SPECIAL) != 0 {
                                        "SPECIAL/LIFT"
                                    } else if (spec.reach_flags & ut99_bsp::R_JUMP) != 0 {
                                        "JUMP"
                                    } else {
                                        "WALK"
                                    };
                                    ui.label(flags_desc);
                                    ui.end_row();
                                }
                            });
                        });
                    }
                }
            } else {
                ui.label(RichText::new("Select a navigation node from the left list to inspect ReachSpecs.").color(Color32::GRAY));
            }
        });
    });
}

fn render_jumppads_tab(ui: &mut Ui, state: &EditorState) {
    ui.label(RichText::new("UT_JumpPad & Kicker 3D Ballistic Trajectory Simulator").strong().size(13.0));
    ui.label(RichText::new("Evaluates kick velocity vectors against UT99 gravity (-950 uu/s²) to prevent overshoot and void hazards.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let arcs = state.compute_jumppad_trajectories();
    if arcs.is_empty() {
        ui.colored_label(Color32::GRAY, "No JumpPad or Kicker actors detected in current map.");
        return;
    }

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("jumppad_arcs_grid")
            .striped(true)
            .min_col_width(90.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("JumpPad").strong());
                ui.label(RichText::new("Kick Velocity (XYZ)").strong());
                ui.label(RichText::new("Apex Z").strong());
                ui.label(RichText::new("Flight Time").strong());
                ui.label(RichText::new("Landing Error").strong());
                ui.label(RichText::new("Hazard Status").strong());
                ui.end_row();

                for arc in &arcs {
                    ui.label(RichText::new(&arc.source_name).strong());
                    ui.label(format!("({:.0}, {:.0}, {:.0})", arc.kick_velocity.x, arc.kick_velocity.y, arc.kick_velocity.z));
                    ui.label(format!("{:.0} uu", arc.apex_height_z));
                    ui.label(format!("{:.2} s", arc.flight_time_secs));
                    ui.label(format!("{:.1} uu", arc.landing_distance_error));

                    if arc.has_overshoot_hazard {
                        ui.colored_label(Color32::from_rgb(255, 90, 90), "⚠ OVERSHOOT HAZARD");
                    } else if arc.has_ceiling_hazard {
                        ui.colored_label(Color32::from_rgb(255, 180, 50), "⚠ CEILING CEILING");
                    } else {
                        ui.colored_label(Color32::from_rgb(90, 220, 100), "✔ TRAJECTORY OPTIMAL");
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_lift_sync_tab(ui: &mut Ui, state: &EditorState) {
    ui.label(RichText::new("LiftCenter & LiftExit Pathway Synchronization").strong().size(13.0));
    ui.label(RichText::new("Verifies mover keyframe stroke matching, exit waypoint tagging, and elevator door safety.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let reports = state.validate_lift_synchronization();
    if reports.is_empty() {
        ui.colored_label(Color32::GRAY, "No LiftCenter elevator actors found in current map.");
        return;
    }

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("lift_sync_grid")
            .striped(true)
            .min_col_width(100.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("LiftCenter").strong());
                ui.label(RichText::new("LiftTag").strong());
                ui.label(RichText::new("Mover Actor").strong());
                ui.label(RichText::new("Exits Linked").strong());
                ui.label(RichText::new("Synchronization").strong());
                ui.end_row();

                for r in &reports {
                    ui.label(RichText::new(&r.lift_center_name).strong());
                    ui.label(&r.lift_tag);
                    ui.label(r.associated_mover_name.as_deref().unwrap_or("❌ Missing Mover"));
                    ui.label(format!("{} exits", r.associated_exits.len()));

                    if r.is_synchronized {
                        ui.colored_label(Color32::from_rgb(90, 220, 100), "✔ SYNCHRONIZED");
                    } else {
                        let err_text = r.issues.first().cloned().unwrap_or_else(|| "Error".to_string());
                        ui.colored_label(Color32::from_rgb(255, 90, 90), format!("⚠ {err_text}"));
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_tactical_cover_tab(ui: &mut Ui, state: &EditorState) {
    ui.label(RichText::new("AI Bot Tactical Cover & Sightline Exposure Profiler").strong().size(13.0));
    ui.label(RichText::new("Analyzes line-of-fire vulnerability, crossfire angles, and strategic vantage points for bots.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let entries = state.compute_tactical_cover_ratings();
    if entries.is_empty() {
        ui.colored_label(Color32::GRAY, "No tactical cover evaluation available. Build navigation paths first.");
        return;
    }

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("tactical_cover_grid")
            .striped(true)
            .min_col_width(90.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Node").strong());
                ui.label(RichText::new("Position (X, Y, Z)").strong());
                ui.label(RichText::new("Cover Rating").strong());
                ui.label(RichText::new("Visible Enemies").strong());
                ui.label(RichText::new("Strategic Role").strong());
                ui.end_row();

                for e in &entries {
                    ui.label(RichText::new(&e.node_name).strong());
                    ui.label(format!("({:.0}, {:.0}, {:.0})", e.location.x, e.location.y, e.location.z));

                    match e.cover_rating {
                        TacticalCoverRating::SolidCover => {
                            ui.colored_label(Color32::from_rgb(80, 220, 100), "🛡 Solid Cover");
                        }
                        TacticalCoverRating::PartialCover => {
                            ui.colored_label(Color32::from_rgb(240, 200, 60), "⚔ Partial Cover");
                        }
                        TacticalCoverRating::OpenHazard => {
                            ui.colored_label(Color32::from_rgb(255, 80, 80), "☠ Open Hazard");
                        }
                    }

                    ui.label(format!("{} nodes", e.visible_nodes_count));

                    let role = if e.is_high_vantage_perch {
                        "🔭 Sniper Vantage Perch"
                    } else if e.strategic_item_bonus {
                        "⭐ Powerup Strategic Spot"
                    } else {
                        "🏃 Standard Transit"
                    };
                    ui.label(role);
                    ui.end_row();
                }
            });
    });
}
