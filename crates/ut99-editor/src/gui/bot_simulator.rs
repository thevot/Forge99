use crate::editor_state::EditorState;
use egui::{Color32, RichText};

pub fn render_bot_simulator_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_bot_simulator_window {
        return;
    }

    let mut open = state.show_bot_simulator_window;

    egui::Window::new("Bot AI Navigation & Pathfinding Simulator")
        .open(&mut open)
        .default_size([420.0, 480.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Interactive Bot Navigation, Reachability & A* Path Simulation")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            // Navigation Network Stats
            ui.horizontal(|ui| {
                ui.label(RichText::new("Nav Nodes:").strong());
                ui.label(format!("{}", state.nav_graph.nodes.len()));

                ui.add_space(20.0);
                ui.label(RichText::new("Reach Specs:").strong());
                ui.label(format!("{}", state.nav_graph.reach_specs.len()));
            });

            if ui.button("⚡ Rebuild Navigation Network (Paths)").clicked() {
                state.rebuild_navigation_paths();
            }

            ui.add_space(10.0);
            ui.separator();

            // Simulation Controls
            ui.heading("A* Route Simulation");

            if state.nav_graph.nodes.len() < 2 {
                ui.colored_label(
                    Color32::from_rgb(255, 180, 50),
                    "Add at least 2 PathNodes/PlayerStarts or click 'Rebuild' to initialize test nodes.",
                );
            } else {
                if state.sim_start_node >= state.nav_graph.nodes.len() {
                    state.sim_start_node = 0;
                }
                if state.sim_target_node >= state.nav_graph.nodes.len() {
                    state.sim_target_node = state.nav_graph.nodes.len().saturating_sub(1);
                }

                let start_idx = state.sim_start_node;
                let target_idx = state.sim_target_node;

                ui.horizontal(|ui| {
                    ui.label("Start Node:");
                    egui::ComboBox::from_id_salt("bot_start_node_combo")
                        .selected_text(format!(
                            "#{}: {}",
                            start_idx, state.nav_graph.nodes[start_idx].name
                        ))
                        .show_ui(ui, |ui| {
                            for (idx, n) in state.nav_graph.nodes.iter().enumerate() {
                                ui.selectable_value(
                                    &mut state.sim_start_node,
                                    idx,
                                    format!("#{}: {} ({})", idx, n.name, n.node_type),
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Target Node:");
                    egui::ComboBox::from_id_salt("bot_target_node_combo")
                        .selected_text(format!(
                            "#{}: {}",
                            target_idx, state.nav_graph.nodes[target_idx].name
                        ))
                        .show_ui(ui, |ui| {
                            for (idx, n) in state.nav_graph.nodes.iter().enumerate() {
                                ui.selectable_value(
                                    &mut state.sim_target_node,
                                    idx,
                                    format!("#{}: {} ({})", idx, n.name, n.node_type),
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    if ui
                        .button(
                            RichText::new("▶ Start Simulation")
                                .color(Color32::from_rgb(100, 255, 100))
                                .strong(),
                        )
                        .clicked()
                    {
                        let s = state.sim_start_node;
                        let t = state.sim_target_node;
                        state.start_bot_simulation(s, t);
                    }

                    if ui.button("⏹ Stop / Clear").clicked() {
                        state.bot_simulator = None;
                    }
                });
            }

            ui.add_space(10.0);
            ui.separator();

            // Real-time Bot Agent Status
            ui.heading("Live Simulated Agent");
            if let Some(ref bot) = state.bot_simulator {
                ui.horizontal(|ui| {
                    ui.label("Bot Name:");
                    ui.label(
                        RichText::new(&bot.name)
                            .color(Color32::from_rgb(100, 200, 255))
                            .strong(),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.label(format!(
                        "({:.1}, {:.1}, {:.1})",
                        bot.location.x, bot.location.y, bot.location.z
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.label(format!("{:.0} uu/s", bot.speed));
                });
                ui.horizontal(|ui| {
                    ui.label("Waypoint Route:");
                    ui.label(format!(
                        "Step {} of {} (Node #{:?})",
                        bot.current_waypoint_idx,
                        bot.path.len(),
                        bot.path.get(bot.current_waypoint_idx)
                    ));
                });

                if bot.reached_destination {
                    ui.colored_label(
                        Color32::from_rgb(80, 255, 120),
                        "✔ Destination successfully reached!",
                    );
                } else if bot.is_stuck {
                    ui.colored_label(
                        Color32::from_rgb(255, 80, 80),
                        "⚠ Bot is stuck or route is blocked!",
                    );
                } else {
                    ui.colored_label(
                        Color32::from_rgb(100, 200, 255),
                        "🏃 Navigating towards next waypoint...",
                    );
                }
            } else {
                ui.label(
                    RichText::new("No active simulated bot. Select start and target to launch.")
                        .italics()
                        .color(Color32::from_rgb(160, 160, 160)),
                );
            }
        });

    state.show_bot_simulator_window = open;
}
