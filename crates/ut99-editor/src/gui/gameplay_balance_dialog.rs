use crate::editor_state::EditorState;
use egui::{Color32, RichText};
use ut99_bsp::BalanceSeverity;

pub fn render_gameplay_balance_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_gameplay_balance_dialog {
        return;
    }

    let mut open = state.show_gameplay_balance_dialog;

    egui::Window::new("Tournament Item Balance & Timing Matrix")
        .open(&mut open)
        .resizable(true)
        .default_width(620.0)
        .default_height(560.0)
        .show(ctx, |ui| {
            ui.heading("Competitive Gameplay & Route Timing Analyzer");
            ui.label(
                RichText::new("Evaluates item placement, running travel times (400 uu/s), super item stacking hazards, and spawn starvation.")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            let report = state.analyze_map_gameplay_balance();

            // Certification Banner
            ui.horizontal(|ui| {
                if report.is_tournament_certified {
                    ui.label(
                        RichText::new("🏆 TOURNAMENT CERTIFIED (BALANCED)")
                            .color(Color32::from_rgb(80, 230, 120))
                            .strong(),
                    );
                    ui.label("Map meets competitive standards for duel and team deathmatch.");
                } else {
                    ui.label(
                        RichText::new("⚠ COMPETITIVE BALANCE WARNINGS")
                            .color(Color32::from_rgb(255, 170, 40))
                            .strong(),
                    );
                    ui.label("Review stacking hazards or spawn travel times below.");
                }
            });

            ui.add_space(4.0);

            // Metrics Grid
            egui::Grid::new("balance_metrics_grid").num_columns(4).striped(true).show(ui, |ui| {
                ui.label(RichText::new("Total Pickups:").strong());
                ui.label(format!("{}", report.pickups.len()));

                ui.label(RichText::new("Avg Spawn to Weapon:").strong());
                let avg_col = if report.average_spawn_to_weapon_secs < 4.0 {
                    Color32::from_rgb(100, 230, 120)
                } else if report.average_spawn_to_weapon_secs < 8.0 {
                    Color32::from_rgb(230, 200, 80)
                } else {
                    Color32::from_rgb(255, 100, 100)
                };
                ui.colored_label(avg_col, format!("{:.1}s", report.average_spawn_to_weapon_secs));
                ui.end_row();

                ui.label(RichText::new("Super Pickups:").strong());
                let super_count = report.pickups.iter().filter(|p| matches!(p.tier, ut99_bsp::ItemTier::SuperArmor | ut99_bsp::ItemTier::SuperHealth | ut99_bsp::ItemTier::PowerUp)).count();
                ui.label(format!("{}", super_count));

                ui.label(RichText::new("Active Spawns:").strong());
                ui.label(format!("{}", report.spawn_times_to_weapon.len()));
                ui.end_row();
            });

            ui.add_space(8.0);
            ui.separator();

            // Detected Issues
            if !report.issues.is_empty() {
                ui.label(RichText::new("Balance & Ergonomics Alerts").strong());
                egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                    for issue in &report.issues {
                        let (icon, col) = match issue.severity {
                            BalanceSeverity::Critical => ("🚨", Color32::from_rgb(255, 80, 80)),
                            BalanceSeverity::Warning => ("⚠", Color32::from_rgb(255, 190, 40)),
                            BalanceSeverity::Info => ("ℹ", Color32::from_rgb(120, 200, 255)),
                        };

                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(col, icon);
                                ui.label(RichText::new(&issue.title).strong().color(col));
                            });
                            ui.label(&issue.description);
                            ui.label(RichText::new(format!("Fix: {}", issue.suggested_fix)).italics().color(Color32::from_rgb(170, 220, 170)));
                        });
                        ui.add_space(3.0);
                    }
                });
                ui.add_space(6.0);
                ui.separator();
            }

            // Major Pickup Pair Timings Table
            ui.collapsing(format!("Super Item Travel Times & Routes ({} pairs)", report.pair_timings.len()), |ui| {
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    egui::Grid::new("pair_timings_grid").num_columns(4).striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Item A").strong());
                        ui.label(RichText::new("Item B").strong());
                        ui.label(RichText::new("Distance").strong());
                        ui.label(RichText::new("Run Time (400 uu/s)").strong());
                        ui.end_row();

                        for pair in &report.pair_timings {
                            ui.label(&pair.item_a);
                            ui.label(&pair.item_b);
                            ui.label(format!("{:.0} uu", pair.distance_uu));

                            if pair.is_stacking_hazard {
                                ui.colored_label(Color32::from_rgb(255, 70, 70), format!("{:.1}s [STACKING RISK]", pair.run_time_secs));
                            } else {
                                ui.colored_label(Color32::from_rgb(100, 220, 120), format!("{:.1}s", pair.run_time_secs));
                            }
                            ui.end_row();
                        }
                    });
                });
            });

            // Player Spawns to Nearest Weapon
            ui.collapsing(format!("Player Spawns to First Weapon ({} starts)", report.spawn_times_to_weapon.len()), |ui| {
                egui::ScrollArea::vertical().max_height(130.0).show(ui, |ui| {
                    egui::Grid::new("spawns_grid").num_columns(3).striped(true).show(ui, |ui| {
                        ui.label(RichText::new("PlayerStart").strong());
                        ui.label(RichText::new("Time to Weapon").strong());
                        ui.label(RichText::new("Safety Status").strong());
                        ui.end_row();

                        for (spawn, time_s) in &report.spawn_times_to_weapon {
                            ui.label(spawn);
                            ui.label(format!("{:.1} s", time_s));

                            if *time_s <= 4.0 {
                                ui.colored_label(Color32::from_rgb(80, 220, 100), "Optimal (< 4s)");
                            } else if *time_s <= 8.0 {
                                ui.colored_label(Color32::from_rgb(220, 200, 50), "Acceptable (4-8s)");
                            } else {
                                ui.colored_label(Color32::from_rgb(255, 80, 80), "Starvation Hazard (> 8s)");
                            }
                            ui.end_row();
                        }
                    });
                });
            });
        });

    state.show_gameplay_balance_dialog = open;
}
