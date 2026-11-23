use crate::editor_state::EditorState;
use egui::{vec2, Color32, RichText, Ui, Window};
use glam::Vec3;
use ut99_bsp::ZoneOcclusionPreset;

/// Interactive Zone Portals, Antiportals & PVS Occlusion Diagnostic Studio
pub fn render_zone_occlusion_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_zone_occlusion_studio_dialog {
        return;
    }

    let mut is_open = state.show_zone_occlusion_studio_dialog;

    Window::new("Zone Portals, Antiportals & PVS Occlusion Studio")
        .open(&mut is_open)
        .default_width(880.0)
        .default_height(620.0)
        .min_width(720.0)
        .min_height(500.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_zone_occlusion_content(ui, state);
        });

    state.show_zone_occlusion_studio_dialog = is_open;
}

fn render_zone_occlusion_content(ui: &mut Ui, state: &mut EditorState) {
    // Header Bar
    ui.horizontal(|ui| {
        ui.heading("🌊 Zone Portals & PVS Occlusion Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Recompute PVS & Leak Audit").clicked() {
                state.analyze_zone_occlusion();
            }
        });
    });

    ui.separator();

    // Presets Bar
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Presets:").strong());
        for &preset in ZoneOcclusionPreset::all() {
            let is_sel = state.zone_occlusion_settings.selected_preset == preset;
            if ui.selectable_label(is_sel, preset.display_name()).clicked() {
                state.zone_occlusion_settings.selected_preset = preset;
                match preset {
                    ZoneOcclusionPreset::StandardDoorwayPortal => {
                        state.zone_occlusion_settings.portal_sheet_width = 128.0;
                        state.zone_occlusion_settings.portal_sheet_height = 256.0;
                        state.zone_occlusion_settings.portal_sheet_normal = Vec3::X;
                    }
                    ZoneOcclusionPreset::CorridorAirlockDual => {
                        state.zone_occlusion_settings.portal_sheet_width = 256.0;
                        state.zone_occlusion_settings.portal_sheet_height = 256.0;
                        state.zone_occlusion_settings.portal_sheet_normal = Vec3::X;
                    }
                    ZoneOcclusionPreset::OutdoorOcclusionShield => {
                        state.zone_occlusion_settings.antiportal_size = Vec3::new(1024.0, 512.0, 512.0);
                    }
                    ZoneOcclusionPreset::SniperTowerVisGate => {
                        state.zone_occlusion_settings.portal_sheet_width = 64.0;
                        state.zone_occlusion_settings.portal_sheet_height = 128.0;
                        state.zone_occlusion_settings.portal_sheet_normal = Vec3::Y;
                    }
                }
            }
        }
    });

    // Preset Description Badge
    ui.add_space(2.0);
    ui.group(|ui| {
        ui.label(
            RichText::new(state.zone_occlusion_settings.selected_preset.description())
                .size(11.0)
                .italics()
                .color(Color32::from_rgb(180, 230, 255)),
        );
    });

    ui.add_space(4.0);

    // Audit Summary Banner
    if let Some(ref audit) = state.zone_occlusion_settings.last_audit_report {
        let (banner_col, icon) = if audit.is_tournament_ready {
            (Color32::from_rgb(80, 230, 120), "🏆")
        } else {
            (Color32::from_rgb(255, 120, 60), "⚠")
        };

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(18.0));
                ui.label(
                    RichText::new(&audit.summary_verdict)
                        .color(banner_col)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!(
                        "PVS Culling: {:.1}% | Portals: {} | AntiPortals: {}",
                        audit.occlusion_efficiency_pct, audit.total_portals, audit.active_antiportals
                    ));
                });
            });
        });
    } else {
        // Initial auto-audit if none run yet
        state.analyze_zone_occlusion();
    }

    ui.add_space(4.0);

    // Main Interactive Panes: PVS Matrix & Tools
    ui.columns(2, |cols| {
        // Column 1: PVS Visibility Grid & Matrix
        cols[0].group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("📊 PVS Mutual Visibility Matrix").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.checkbox(&mut state.zone_occlusion_settings.show_pvs_matrix_grid, "Show Matrix");
                });
            });

            if let Some(ref pvs) = state.zone_pvs_matrix {
                let n = pvs.zone_count.min(12); // Limit visual matrix to 12x12
                ui.label(format!("Zones Count: {} | Occluded Pairs: {:.1}%", pvs.zone_count, pvs.occlusion_efficiency_pct()));

                if state.zone_occlusion_settings.show_pvs_matrix_grid {
                    egui::Grid::new("pvs_vis_grid")
                        .spacing(vec2(2.0, 2.0))
                        .show(ui, |ui| {
                            // Column headers
                            ui.label("Z");
                            for j in 0..n {
                                ui.label(RichText::new(format!("{}", j)).size(10.0));
                            }
                            ui.end_row();

                            // Rows
                            for i in 0..n {
                                ui.label(RichText::new(format!("{}", i)).size(10.0));
                                for j in 0..n {
                                    let is_vis = pvs.is_visible(i, j);
                                    let cell_col = if i == j {
                                        Color32::from_rgb(40, 100, 220) // Self
                                    } else if is_vis {
                                        Color32::from_rgb(60, 200, 100) // Mutually visible
                                    } else {
                                        Color32::from_rgb(40, 45, 55)   // Occluded (Culled)
                                    };

                                    let (rect, response) = ui.allocate_exact_size(vec2(16.0, 16.0), egui::Sense::click());
                                    ui.painter().rect_filled(rect, 2.0, cell_col);
                                    if response.clicked() {
                                        state.zone_occlusion_settings.selected_zone_idx = Some(i);
                                    }
                                }
                                ui.end_row();
                            }
                        });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Legend:").size(10.0));
                        ui.colored_label(Color32::from_rgb(60, 200, 100), "■ Visible");
                        ui.colored_label(Color32::from_rgb(40, 45, 55), "■ Culled (PVS)");
                        ui.colored_label(Color32::from_rgb(40, 100, 220), "■ Self");
                    });
                }
            } else {
                ui.label("No PVS Matrix computed yet.");
            }
        });

        // Column 2: 1-Click Portal Generator & Antiportals
        cols[1].group(|ui| {
            ui.label(RichText::new("🚪 1-Click Portal Sheet Generator").strong());

            ui.horizontal(|ui| {
                ui.label("Width (uu):");
                ui.add(egui::DragValue::new(&mut state.zone_occlusion_settings.portal_sheet_width).speed(1.0).range(32.0..=2048.0));
                ui.label("Height (uu):");
                ui.add(egui::DragValue::new(&mut state.zone_occlusion_settings.portal_sheet_height).speed(1.0).range(32.0..=2048.0));
            });

            ui.horizontal(|ui| {
                ui.label("Normal Axis:");
                let n = &mut state.zone_occlusion_settings.portal_sheet_normal;
                if ui.selectable_label(*n == Vec3::X, "Wall X").clicked() { *n = Vec3::X; }
                if ui.selectable_label(*n == Vec3::Y, "Wall Y").clicked() { *n = Vec3::Y; }
                if ui.selectable_label(*n == Vec3::Z, "Floor/Ceil Z").clicked() { *n = Vec3::Z; }
            });

            if ui.button("⚡ Create Portal Sheet at Pivot").clicked() {
                let w = state.zone_occlusion_settings.portal_sheet_width;
                let h = state.zone_occlusion_settings.portal_sheet_height;
                let n = state.zone_occlusion_settings.portal_sheet_normal;
                state.create_portal_sheet_at_pivot(w, h, n);
            }

            ui.separator();

            ui.label(RichText::new("🛡 AntiPortal Occluder Volume").strong());
            ui.horizontal(|ui| {
                ui.label("Extents:");
                ui.add(egui::DragValue::new(&mut state.zone_occlusion_settings.antiportal_size.x).speed(1.0).range(32.0..=4096.0));
                ui.add(egui::DragValue::new(&mut state.zone_occlusion_settings.antiportal_size.y).speed(1.0).range(32.0..=4096.0));
                ui.add(egui::DragValue::new(&mut state.zone_occlusion_settings.antiportal_size.z).speed(1.0).range(32.0..=4096.0));
            });

            if ui.button("➕ Add AntiPortal at Pivot").clicked() {
                let sz = state.zone_occlusion_settings.antiportal_size;
                state.add_antiportal_volume_at_pivot(sz);
            }

            if !state.antiportals.is_empty() {
                ui.label(format!("Active AntiPortals in Map: {}", state.antiportals.len()));
            }
        });
    });

    ui.add_space(4.0);
    ui.separator();

    // Zone Leak & Watertightness Audit List
    ui.label(RichText::new("🔍 Zone Leak & Boundary Containment Audit").strong());
    let seal_reports = state.analyze_zone_seals();

    egui::ScrollArea::vertical().max_height(180.0).show(ui, |ui| {
        for report in &seal_reports {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&report.zone_name).strong());
                    ui.label(format!("({})", report.class_name));
                    if report.is_watertight {
                        ui.colored_label(
                            Color32::from_rgb(50, 220, 100),
                            format!("✔ HERMETICALLY SEALED (Portals: {})", report.portal_surfaces_count),
                        );
                    } else {
                        ui.colored_label(
                            Color32::from_rgb(255, 60, 60),
                            format!("🚨 LEAK DETECTED ({} unsealed opening(s))", report.detected_leaks.len()),
                        );
                    }
                });

                if !report.detected_leaks.is_empty() {
                    for leak in &report.detected_leaks {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(255, 100, 100), "  ↳");
                            ui.label(format!(
                                "Breach at ({:.0}, {:.0}, {:.0}) -> Escaping {:.0} uu",
                                leak.leak_location.x, leak.leak_location.y, leak.leak_location.z, leak.leak_distance
                            ));
                            if ui.button("🎯 Focus Breach").clicked() {
                                state.camera.position = leak.leak_location - leak.escape_direction * 200.0;
                                state.log(format!("Focused camera on zone leak breach at ({:.0}, {:.0}, {:.0})", leak.leak_location.x, leak.leak_location.y, leak.leak_location.z));
                            }
                        });
                    }
                }
            });
            ui.add_space(2.0);
        }
    });
}
