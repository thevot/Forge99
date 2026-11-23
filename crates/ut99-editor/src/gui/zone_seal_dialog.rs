use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};

/// Zone Boundary Leak Detector & Watertight Seal Tester Dialog
pub fn show_zone_seal_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_zone_seal_dialog {
        return;
    }

    let mut is_open = state.show_zone_seal_dialog;
    let mut close_requested = false;
    Window::new("Zone Boundary Leak Detector & Seal Tester")
        .open(&mut is_open)
        .default_width(620.0)
        .default_height(400.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_zone_seal_content(ui, state, &mut close_requested);
        });

    state.show_zone_seal_dialog = is_open && !close_requested;
}

fn render_zone_seal_content(ui: &mut Ui, state: &mut EditorState, close_requested: &mut bool) {
    ui.horizontal(|ui| {
        ui.heading("🌊 Zone Containment & Leak Audit");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("✖ Close").clicked() {
                *close_requested = true;
            }
        });
    });

    ui.separator();

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.show_zone_leaks_visual, "Show 3D Red Escape Beams in Viewport");
        if ui.button("🎨 Switch to Zone/Portal 3D View (Alt+2)").clicked() {
            state.viewport_shading = crate::camera::ViewportShading::ZonePortal;
        }
    });

    ui.add_space(4.0);

    let flood = state.compute_3d_zone_flood_fill_report();
    let mut focus_leak_pt = None;
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🧭 3D BSP Zone Flood-Fill & Leak Path Tracer:").strong());
            ui.colored_label(
                Color32::from_rgb(100, 210, 255),
                format!("{} Distinct Sealed Zone(s)", flood.distinct_zone_count),
            );
            if flood.leak_paths.is_empty() {
                ui.colored_label(Color32::from_rgb(80, 240, 120), "✔ Zero Inter-Zone Portal Leaks");
            } else {
                ui.colored_label(
                    Color32::from_rgb(255, 80, 80),
                    format!("⚠ {} Inter-Zone Leak Path(s)!", flood.leak_paths.len()),
                );
            }
        });
        for lp in &flood.leak_paths {
            ui.horizontal(|ui| {
                ui.colored_label(
                    Color32::from_rgb(255, 110, 80),
                    format!("Leak: {} ↔ {}", lp.zone_a_name, lp.zone_b_name),
                );
                ui.label(format!(
                    "Gap at ({:.0}, {:.0}, {:.0})",
                    lp.leak_gap_point.x, lp.leak_gap_point.y, lp.leak_gap_point.z
                ));
                if ui.small_button("📍 Focus Camera on Leak Gap").clicked() {
                    focus_leak_pt = Some(lp.leak_gap_point);
                }
            });
        }
    });
    if let Some(pt) = focus_leak_pt {
        state.viewport_shading = crate::camera::ViewportShading::ZonePortal;
        state.camera.position = pt - state.camera.forward() * 220.0;
        state.log(format!(
            "Camera focused on 3D Zone Leak gap at ({:.1}, {:.1}, {:.1}).",
            pt.x, pt.y, pt.z
        ));
    }

    ui.add_space(4.0);

    let reports = state.analyze_zone_seals();
    if reports.is_empty() {
        ui.label("No zone actors found in map.");
        return;
    }

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        for report in &reports {
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
                            format!("❌ LEAK DETECTED ({} Openings)", report.detected_leaks.len()),
                        );
                    }
                });

                ui.label(format!("Location: ({:.0}, {:.0}, {:.0})", report.location.x, report.location.y, report.location.z));
                ui.label(RichText::new(&report.recommended_action).italics());

                if !report.is_watertight {
                    ui.collapsing(format!("Detailed Leak Rays ({})", report.detected_leaks.len()), |ui| {
                        for (idx, leak) in report.detected_leaks.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.colored_label(Color32::from_rgb(255, 100, 100), format!("Leak #{}", idx + 1));
                                ui.label(format!("At ({:.0}, {:.0}, {:.0}) | Dist: {:.0} uu", leak.leak_location.x, leak.leak_location.y, leak.leak_location.z, leak.leak_distance));
                            });
                        }
                    });
                }
            });
            ui.add_space(4.0);
        }
    });

    ui.add_space(8.0);
    if ui.button("📋 Log Leak Audit to Console").clicked() {
        state.log("=== ZONE BOUNDARY LEAK AUDIT ===");
        for r in &reports {
            state.log(format!("Zone '{}': sealed={} (leaks={})", r.zone_name, r.is_watertight, r.detected_leaks.len()));
        }
    }
}
