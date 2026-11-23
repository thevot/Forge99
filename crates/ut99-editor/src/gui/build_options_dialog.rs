use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};
use ut99_bsp::BspOptimizationLevel;

pub fn render_build_options_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_build_options_dialog {
        return;
    }

    let mut open = state.show_build_options_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        "Build Options & Map Rebuild",
        &mut open,
        [480.0, 580.0],
        [400.0, 360.0],
    )
    .show(ctx, |ui| {
            // Header Info & Status
            ui.horizontal(|ui| {
                ui.label(RichText::new("UnrealEd 2.0 Compilers & Rebuild Pipeline").strong().color(Color32::from_rgb(100, 200, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        close_requested = true;
                    }
                });
            });
            ui.separator();

            // 1. Geometry & BSP Optimization Panel
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(60, 75, 95)))
                .fill(Color32::from_rgba_premultiplied(20, 25, 32, 220))
                .show(ui, |ui| {
                    ui.label(RichText::new("🏗 Geometry & BSP Tree Configuration").strong().size(12.0));
                    ui.add_space(4.0);

                    // Optimization Level
                    ui.horizontal(|ui| {
                        ui.label("Optimization:");
                        let cur_opt = state.bsp_config.optimization;
                        if ui.selectable_label(cur_opt == BspOptimizationLevel::Lame, "Lame (Fast)").clicked() {
                            state.bsp_config.optimization = BspOptimizationLevel::Lame;
                        }
                        if ui.selectable_label(cur_opt == BspOptimizationLevel::Good, "Good (Recommended)").clicked() {
                            state.bsp_config.optimization = BspOptimizationLevel::Good;
                        }
                        if ui.selectable_label(cur_opt == BspOptimizationLevel::Optimal, "Optimal (Thorough)").clicked() {
                            state.bsp_config.optimization = BspOptimizationLevel::Optimal;
                        }
                    });

                    ui.add_space(4.0);

                    // Balance vs Cuts Slider (Canonical UnrealEd slider)
                    ui.horizontal(|ui| {
                        ui.label("BSP Cuts vs Balance:");
                        ui.add(egui::Slider::new(&mut state.bsp_config.balance_factor, 0..=100).suffix("%"));
                    });
                    ui.label(
                        RichText::new("0% = Minimize cuts (Fewer fragments)  <--->  100% = Minimize depth (Balanced tree)")
                            .size(10.0)
                            .color(Color32::from_rgb(140, 150, 165))
                    );

                    ui.add_space(4.0);

                    // Portal Bias Slider
                    ui.horizontal(|ui| {
                        ui.label("Portal Bias:");
                        ui.add(egui::Slider::new(&mut state.bsp_config.portal_bias, 0..=100).suffix("%"));
                    });

                    ui.add_space(4.0);

                    // Checkbox: Only Visible Brushes
                    ui.checkbox(&mut state.bsp_config.only_visible_brushes, "Only Build Visible Brushes (Respect Group Filters)");

                    ui.add_space(6.0);

                    if ui.button(RichText::new("🔨 Rebuild Geometry & BSP Tree").color(Color32::from_rgb(120, 200, 255)).strong()).clicked() {
                        state.build_geometry();
                    }
                });

            ui.add_space(6.0);

            // 2. Lighting & AI Paths Pipeline Flags
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(60, 75, 95)))
                .fill(Color32::from_rgba_premultiplied(20, 25, 32, 220))
                .show(ui, |ui| {
                    ui.label(RichText::new("⚡ Subsystem Compilers for 'Build All'").strong().size(12.0));
                    ui.add_space(4.0);

                    // Lighting section
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.bsp_config.build_lighting, "💡 Lighting (Rayon Raytracing)");
                        if ui.small_button("Bake Lighting Now").clicked() {
                            state.build_lighting();
                        }
                        if ui
                            .small_button("💡 Selected Surfaces Only")
                            .on_hover_text("UnrealEd 2.0 'Apply Lights to Selected' (REQ-F07): Bakes lighting strictly onto currently selected surfaces/brushes")
                            .clicked()
                        {
                            state.build_lighting_for_selected_surfaces();
                        }
                        if ui.small_button("Configure...").clicked() {
                            state.show_lighting_dialog = true;
                        }
                    });

                    ui.add_space(4.0);

                    // AI Paths section
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.bsp_config.build_paths, "🔷 Bot Navigation Paths (ReachSpecs)");
                        if ui.small_button("Build Paths Now").clicked() {
                            state.build_ai_paths();
                        }
                    });

                    ui.add_space(4.0);

                    // Decals section
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut state.bsp_config.build_decals, "🎯 Projector Decals");
                        if ui.small_button("Bake Decals Now").clicked() {
                            state.bake_projector_decals();
                        }
                    });
                });

            ui.add_space(8.0);

            // 3. Master Build All Button
            ui.vertical_centered(|ui| {
                let build_all_btn = egui::Button::new(
                    RichText::new("⚡ BUILD ALL (Geometry + Lighting + Paths + Decals)")
                        .color(Color32::from_rgb(255, 230, 80))
                        .strong()
                        .size(13.0)
                );
                if ui.add_sized([ui.available_width() - 20.0, 32.0], build_all_btn).clicked() {
                    state.build_all_complete();
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // 4. Compilation Results & Metrics Card (UnrealEd 2.0 Stats Tab Parity — REQ-I02 & REQ-I03)
            ui.horizontal(|ui| {
                ui.label(RichText::new("📊 Compilation Metrics & BSP Tree Report (Stats)").strong().size(12.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("🔄 Refresh").on_hover_text("Recalculate Geometry, BSP, Zone, and Lighting statistics without modifying scene actors").clicked() {
                        state.refresh_build_stats();
                    }
                });
            });
            ui.add_space(4.0);

            let brush_count = state.world_brushes.len();
            let zone_count = state.compute_3d_zone_flood_fill_report().distinct_zone_count;
            let light_count = state
                .loaded_unr
                .as_ref()
                .map(|u| u.actors.iter().filter(|a| a.class_name.contains("Light")).count())
                .unwrap_or(0);

            if let Some(ref report) = state.last_bsp_report.clone() {
                let ratio = report.nodes_to_polys_ratio();
                let ratio_color = if report.is_optimal_ratio() {
                    Color32::from_rgb(100, 230, 140)
                } else if ratio <= 2.5 {
                    Color32::from_rgb(255, 210, 80)
                } else {
                    Color32::from_rgb(255, 120, 100)
                };
                egui::Frame::group(ui.style())
                    .stroke(Stroke::new(1.0f32, Color32::from_rgb(45, 55, 70)))
                    .fill(Color32::from_rgba_premultiplied(16, 20, 26, 240))
                    .show(ui, |ui| {
                        egui::Grid::new("bsp_report_grid")
                            .num_columns(4)
                            .spacing([14.0, 6.0])
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Brushes:");
                                ui.label(RichText::new(format!("{}", brush_count)).strong());

                                ui.label("Zones:");
                                ui.label(RichText::new(format!("{}", zone_count)).strong().color(Color32::from_rgb(140, 220, 255)));
                                ui.end_row();

                                ui.label("BSP Nodes:");
                                ui.label(RichText::new(format!("{}", report.node_count)).strong().color(Color32::from_rgb(120, 220, 255)));

                                ui.label("World Polygons:");
                                ui.label(RichText::new(format!("{}", report.poly_count)).strong());
                                ui.end_row();

                                ui.label("Ratio (Nodes:Polys):");
                                ui.label(
                                    RichText::new(format!("{:.2}:1", ratio))
                                        .strong()
                                        .color(ratio_color),
                                ).on_hover_text("Canonical UnrealEd 2.0 target is <= 2.00:1");

                                ui.label("Lights:");
                                ui.label(RichText::new(format!("{}", light_count)).strong().color(Color32::from_rgb(255, 230, 110)));
                                ui.end_row();

                                ui.label("Polygon Cuts:");
                                let cut_color = if report.cut_count == 0 {
                                    Color32::from_rgb(100, 230, 140)
                                } else if report.cut_count < 20 {
                                    Color32::from_rgb(255, 210, 80)
                                } else {
                                    Color32::from_rgb(255, 120, 100)
                                };
                                ui.label(RichText::new(format!("{}", report.cut_count)).strong().color(cut_color));

                                ui.label("Coplanar Polys:");
                                ui.label(RichText::new(format!("{}", report.coplanar_count)).strong());
                                ui.end_row();

                                ui.label("Max Tree Depth:");
                                ui.label(RichText::new(format!("{}", report.max_depth)).strong());

                                ui.label("Balance Efficiency:");
                                ui.label(RichText::new(format!("{:.1}%", report.balance_ratio_pct)).strong().color(Color32::from_rgb(140, 240, 160)));
                                ui.end_row();

                                ui.label("BSP Build Time:");
                                ui.label(RichText::new(format!("{} ms", report.elapsed_ms)).strong().color(Color32::from_rgb(200, 160, 255)));

                                ui.label("Status:");
                                ui.label(RichText::new("VALID").strong().color(Color32::from_rgb(100, 240, 120)));
                                ui.end_row();
                            });
                    });
            } else {
                ui.label(RichText::new(format!("Brushes: {} | Zones: {} | Lights: {} (Click '🔄 Refresh' or 'Build' to compute BSP Tree stats)", brush_count, zone_count, light_count)).italics().color(Color32::GRAY));
            }

            // Lighting Stats preview
            if let Some(ref lstats) = state.last_lighting_stats.clone() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Last Lighting Bake:").size(11.0).color(Color32::GRAY));
                    ui.label(RichText::new(format!("{} luxels, {} rays in {} ms ({} lights)", lstats.total_luxels, lstats.shadow_rays_cast, lstats.elapsed_ms, light_count))
                        .size(11.0)
                        .color(Color32::from_rgb(255, 230, 100)));
                });
            }
        });

    if close_requested {
        open = false;
    }
    state.show_build_options_dialog = open;
}
