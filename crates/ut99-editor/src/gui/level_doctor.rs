use crate::editor_state::{EditorState, LevelAuditFreshness};
use egui::{Color32, RichText};
use ut99_bsp::IssueSeverity;

pub fn render_level_doctor_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_level_doctor_window {
        return;
    }

    let mut open = state.show_level_doctor_window;

    crate::gui::modal::standard_dialog_window(ctx, "Level Doctor & Map Diagnostic Audit", &mut open, [560.0, 460.0], [400.0, 300.0])
        .show(ctx, |ui| {
            ui.label(
                RichText::new("Automated Map Diagnostics, Error Auditing & Geometry Auto-Fixer")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            match state.level_audit_freshness() {
                LevelAuditFreshness::NotRun => { ui.colored_label(Color32::LIGHT_GRAY, "Audit not yet run"); }
                LevelAuditFreshness::Stale => { ui.colored_label(Color32::YELLOW, "Report is out of date; run the audit again"); }
                LevelAuditFreshness::Current => { ui.label("Report matches the current static audit inputs; runtime behavior is not verified."); }
            }

            // Actions bar
            let mut focus_loc = None;
            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new("🔍 Run Map Audit")
                            .color(Color32::from_rgb(100, 200, 255))
                            .strong(),
                    )
                    .clicked()
                {
                    state.run_level_doctor_audit();
                }

                if ui
                    .button(
                        RichText::new("✨ Auto-Fix Degenerate & Missing Textures")
                            .color(Color32::from_rgb(100, 255, 100))
                            .strong(),
                    )
                    .clicked()
                {
                    state.autofix_level_issues();
                }

                if ui
                    .button(
                        RichText::new("🩺 Run BSP Doctor (Auto-Weld & Planar Fix)")
                            .color(Color32::from_rgb(255, 190, 80))
                            .strong(),
                    )
                    .on_hover_text(
                        "Repairs off-grid vertices, welds duplicate vertices, and triangulates non-planar >=4-vertex polygons (BL-P3-02)",
                    )
                    .clicked()
                {
                    let _ = state.run_bsp_doctor();
                    state.run_level_doctor_audit();
                }

                ui.add_space(20.0);
                let err_count = state
                    .map_audit_issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Error)
                    .count();
                let warn_count = state
                    .map_audit_issues
                    .iter()
                    .filter(|i| i.severity == IssueSeverity::Warning)
                    .count();

                if err_count > 0 {
                    ui.colored_label(Color32::from_rgb(255, 80, 80), format!("✖ {} Errors", err_count));
                }
                if warn_count > 0 {
                    ui.colored_label(Color32::from_rgb(255, 200, 50), format!("▲ {} Warnings", warn_count));
                }
            });

            ui.horizontal(|ui| {
                ui.checkbox(&mut state.show_diagnostic_markers, "📍 Viewport 3D Beacons");
                if state.selected_issue_index.is_some() {
                    if ui.small_button("Deselect Issue").clicked() {
                        state.selected_issue_index = None;
                    }
                }
            });

            ui.add_space(5.0);
            ui.separator();

            // Issues List
            egui::ScrollArea::vertical().show(ui, |ui| {
                if state.map_audit_issues.is_empty() {
                    ui.add_space(30.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new(match state.level_audit_freshness() {
                                LevelAuditFreshness::NotRun => "Audit not yet run.",
                                LevelAuditFreshness::Stale => "The previous report is out of date.",
                                LevelAuditFreshness::Current => "No issues in the current static report.",
                            })
                                .italics()
                                .color(Color32::from_rgb(160, 160, 160)),
                        );
                        ui.label("Click 'Run Map Audit' to inspect brushes, textures, and navigation.");
                    });
                } else {
                    for (idx, issue) in state.map_audit_issues.iter().enumerate() {
                        let is_selected = state.selected_issue_index == Some(idx);
                        let frame = if is_selected {
                            egui::Frame::group(ui.style())
                                .fill(Color32::from_rgba_unmultiplied(255, 215, 0, 22))
                                .stroke(egui::Stroke::new(1.5f32, Color32::from_rgb(255, 215, 0)))
                        } else {
                            egui::Frame::group(ui.style())
                        };

                        frame.show(ui, |ui| {
                            ui.horizontal(|ui| {
                                match issue.severity {
                                    IssueSeverity::Error => {
                                        ui.colored_label(Color32::from_rgb(255, 70, 70), "✖ ERROR");
                                    }
                                    IssueSeverity::Warning => {
                                        ui.colored_label(Color32::from_rgb(255, 200, 40), "▲ WARN");
                                    }
                                    IssueSeverity::Info => {
                                        ui.colored_label(Color32::from_rgb(100, 180, 255), "ℹ INFO");
                                    }
                                }

                                ui.label(RichText::new(&issue.category).strong());
                                ui.label(RichText::new(&issue.object_name).color(Color32::from_rgb(200, 200, 200)));

                                if let Some(loc) = issue.location {
                                    let btn_text = if is_selected { "★ Focused" } else { "📍 Focus" };
                                    if ui.button(btn_text).clicked() {
                                        if is_selected {
                                            state.selected_issue_index = None;
                                        } else {
                                            state.selected_issue_index = Some(idx);
                                            focus_loc = Some(loc);
                                        }
                                    }
                                }
                            });

                            ui.label(&issue.description);
                        });
                    }
                }
            });

            if let Some(loc) = focus_loc {
                state.camera.position = loc - state.camera.forward() * 300.0;
                state.log(format!("Camera focused on diagnostic issue at ({:.0}, {:.0}, {:.0}).", loc.x, loc.y, loc.z));
            }
        });

    state.show_level_doctor_window = open;
}
