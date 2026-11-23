use crate::editor_state::{EditorState, SaveOperationKind, SavePhase};
use egui::{Color32, RichText, Stroke};

/// Renders a high-contrast, professional visual feedback modal overlay during
/// map saving and export operations, ensuring the user has instant, non-blocking feedback
/// with transparent file location and real-time elapsed timer.
pub fn render_save_feedback_overlay(ctx: &egui::Context, state: &mut EditorState) {
    let feedback = match state.save_feedback.as_ref() {
        Some(fb) => fb.clone(),
        None => return,
    };

    let mut dismiss_requested = false;

    // Keep active operation context until the worker has completed.
    let finished = matches!(feedback.phase, SavePhase::Completed | SavePhase::Failed);
    if finished && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        dismiss_requested = true;
    }
    if (feedback.phase == SavePhase::Completed || feedback.phase == SavePhase::Failed)
        && ctx.input(|i| i.key_pressed(egui::Key::Enter))
    {
        dismiss_requested = true;
    }

    let lang = state.language;

    // Semi-transparent backdrop overlay in Order::Middle so the Order::Foreground card stays bright
    let backdrop_color = match feedback.phase {
        SavePhase::PendingPreparation | SavePhase::Executing => Color32::from_rgba_premultiplied(10, 14, 20, 175),
        SavePhase::Completed => Color32::from_rgba_premultiplied(10, 24, 15, 130),
        SavePhase::Failed => Color32::from_rgba_premultiplied(30, 10, 10, 160),
    };

    crate::gui::modal::render_modal_backdrop(ctx, "save_feedback_backdrop", backdrop_color);

    // Centered modal card
    egui::Area::new(egui::Id::new("save_feedback_card_area"))
        .order(egui::Order::Foreground)
        .fade_in(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            let border_stroke = match feedback.phase {
                SavePhase::Completed => Stroke::new(1.8_f32, Color32::from_rgb(80, 235, 120)),
                SavePhase::Failed => Stroke::new(1.8_f32, Color32::from_rgb(255, 80, 80)),
                _ => Stroke::new(1.8_f32, Color32::from_rgb(0, 195, 255)),
            };

            egui::Frame::window(ui.style())
                .fill(Color32::from_rgba_premultiplied(20, 24, 32, 252))
                .stroke(border_stroke)
                .corner_radius(10.0_f32)
                .inner_margin(24.0)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_rgba_premultiplied(0, 0, 0, 230),
                })
                .show(ui, |ui| {
                    ui.set_width(520.0);
                    ui.vertical_centered(|ui| {
                        match feedback.phase {
                            SavePhase::PendingPreparation | SavePhase::Executing => {
                                ui.add_space(4.0);
                                ui.add(egui::Spinner::new().size(40.0).color(Color32::from_rgb(0, 215, 255)));
                                ui.add_space(12.0);

                                let op_title = match feedback.kind {
                                    SaveOperationKind::SaveUnr(_) => format!("💾 {}", crate::i18n::tr(lang, "save.unr_title")),
                                    SaveOperationKind::SaveUnrNative(_) => format!("💾 {}", crate::i18n::tr(lang, "save.unr_native_title")),
                                    SaveOperationKind::SaveT3d(_) => format!("💾 {}", crate::i18n::tr(lang, "save.t3d_document_title")),
                                    SaveOperationKind::ExportT3d(_) => format!("📦 {}", crate::i18n::tr(lang, "save.t3d_title")),
                                    SaveOperationKind::ExportGltf(_) => format!("🌐 {}", crate::i18n::tr(lang, "save.gltf_title")),
                                    SaveOperationKind::ExportObj(_) => format!("📐 {}", crate::i18n::tr(lang, "save.obj_title")),
                                };

                                ui.label(RichText::new(op_title).size(16.0).strong().color(Color32::WHITE));
                                ui.add_space(10.0);

                                // Destination box (Cartella & File)
                                let folder_path = feedback.target_path.parent()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| ".".to_string());

                                let folder_label = format!("📁 {}", crate::i18n::tr(lang, "save.folder"));
                                let file_label = format!("📄 {}", crate::i18n::tr(lang, "save.file"));

                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgba_premultiplied(14, 18, 26, 220))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 60, 80)))
                                    .corner_radius(6.0_f32)
                                    .inner_margin(egui::Margin::symmetric(14, 10))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&folder_label).size(11.0).color(Color32::from_rgb(140, 160, 185)));
                                            ui.add(egui::Label::new(
                                                RichText::new(&folder_path).size(11.0).monospace().color(Color32::from_rgb(180, 210, 240))
                                            ).truncate()).on_hover_text(&folder_path);
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&file_label).size(11.0).color(Color32::from_rgb(140, 160, 185)));
                                            ui.label(RichText::new(&feedback.filename).size(12.0).strong().monospace().color(Color32::from_rgb(100, 220, 255)));
                                        });
                                    });

                                ui.add_space(10.0);

                                // Stage info & animated progress bar
                                if feedback.total_stages > 0 {
                                    let current = feedback.current_stage.clamp(1, feedback.total_stages);
                                    let progress_ratio = (current as f32) / (feedback.total_stages as f32);
                                    let stage_prefix = crate::i18n::tr(lang, "save.stage_prefix");
                                    let stage_of = crate::i18n::tr(lang, "save.stage_of");
                                    ui.label(
                                        RichText::new(format!("{stage_prefix} {current} {stage_of} {}: {}", feedback.total_stages, feedback.stage_message))
                                            .size(12.0)
                                            .color(Color32::from_rgb(215, 230, 245)),
                                    );
                                    ui.add_space(6.0);

                                    let (bar_rect, _) = ui.allocate_exact_size(egui::vec2(380.0, 6.0), egui::Sense::hover());
                                    ui.painter().rect_filled(bar_rect, 3.0, Color32::from_rgb(35, 45, 60));
                                    let filled_width = (bar_rect.width() * progress_ratio).clamp(10.0, bar_rect.width());
                                    let fill_rect = egui::Rect::from_min_size(bar_rect.min, egui::vec2(filled_width, 6.0));
                                    ui.painter().rect_filled(fill_rect, 3.0, Color32::from_rgb(0, 215, 255));
                                } else {
                                    ui.label(
                                        RichText::new(&feedback.stage_message)
                                            .size(12.0)
                                            .color(Color32::from_rgb(200, 215, 230)),
                                    );
                                }

                                ui.add_space(8.0);
                                let elapsed = feedback.started_at.elapsed().as_secs_f32();
                                let elapsed_lbl = crate::i18n::tr(lang, "save.elapsed");
                                ui.label(
                                    RichText::new(format!("⏱ {elapsed_lbl} {:.1}s", elapsed))
                                        .size(11.0)
                                        .color(Color32::from_rgb(130, 160, 190)),
                                );
                            }
                            SavePhase::Completed => {
                                ui.add_space(4.0);
                                ui.label(RichText::new("✅").size(38.0).color(Color32::from_rgb(80, 235, 120)));
                                ui.add_space(6.0);
                                ui.label(RichText::new(crate::i18n::tr(lang, if feedback.kind.saves_document() { "save.success_title" } else { "export.success_title" })).size(17.0).strong().color(Color32::WHITE));
                                ui.add_space(10.0);

                                // Destination box
                                let folder_path = feedback.target_path.parent()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| ".".to_string());

                                let dest_label = format!("📁 {}", crate::i18n::tr(lang, "save.destination"));
                                let file_label = format!("📄 {}", crate::i18n::tr(lang, "save.file"));

                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgba_premultiplied(16, 26, 20, 220))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 100, 60)))
                                    .corner_radius(6.0_f32)
                                    .inner_margin(egui::Margin::symmetric(14, 10))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&dest_label).size(11.0).color(Color32::from_rgb(140, 190, 155)));
                                            ui.add(egui::Label::new(
                                                RichText::new(&folder_path).size(11.0).monospace().color(Color32::from_rgb(180, 240, 200))
                                            ).truncate()).on_hover_text(&folder_path);
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&file_label).size(11.0).color(Color32::from_rgb(140, 190, 155)));
                                            ui.label(RichText::new(&feedback.filename).size(12.0).strong().monospace().color(Color32::from_rgb(100, 255, 160)));
                                        });
                                        if let Some(bytes) = feedback.file_size_bytes {
                                            let size_str = if bytes >= 1024 * 1024 {
                                                format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
                                            } else {
                                                format!("{:.1} KB", bytes as f64 / 1024.0)
                                            };
                                            let dur = feedback.completed_at
                                                .map(|c| c.duration_since(feedback.started_at).as_secs_f32())
                                                .unwrap_or(0.0);
                                            let size_lbl = crate::i18n::tr(lang, "save.size");
                                            let total_lbl = crate::i18n::tr(lang, "save.total_time");
                                            ui.add_space(2.0);
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(format!("💾 {size_lbl} {size_str}   •   ⏱ {total_lbl} {:.2}s", dur)).size(11.0).color(Color32::from_rgb(150, 210, 170)));
                                            });
                                        }
                                    });

                                ui.add_space(14.0);

                                let open_folder_text = format!("📂 {}", crate::i18n::tr(lang, "action.open_folder"));
                                let open_folder_desc = crate::i18n::tr(lang, "action.open_folder_desc");
                                let copy_path_text = format!("📋 {}", crate::i18n::tr(lang, "action.copy_path"));
                                let copy_path_desc = crate::i18n::tr(lang, "action.copy_path_desc");
                                let close_text = format!("✓ {}", crate::i18n::tr(lang, "action.close"));

                                ui.horizontal(|ui| {
                                    if ui.button(RichText::new(&open_folder_text).size(12.0).strong())
                                        .on_hover_text(open_folder_desc)
                                        .clicked()
                                    {
                                        state.reveal_in_file_explorer(&feedback.target_path);
                                    }
                                    if ui.button(RichText::new(&copy_path_text).size(12.0))
                                        .on_hover_text(copy_path_desc)
                                        .clicked()
                                    {
                                        ctx.copy_text(feedback.target_path.display().to_string());
                                        state.toast_info(crate::i18n::tr(lang, "action.path_copied"));
                                    }
                                    if ui.button(RichText::new(&close_text).size(12.0)).clicked() {
                                        dismiss_requested = true;
                                    }
                                });
                            }
                            SavePhase::Failed => {
                                ui.add_space(4.0);
                                ui.label(RichText::new("❌").size(38.0).color(Color32::from_rgb(255, 80, 80)));
                                ui.add_space(6.0);
                                ui.label(RichText::new(crate::i18n::tr(lang, if feedback.kind.saves_document() { "save.failed_title" } else { "export.failed_title" })).size(17.0).strong().color(Color32::WHITE));
                                ui.add_space(10.0);

                                let folder_path = feedback.target_path.parent()
                                    .map(|p| p.display().to_string())
                                    .unwrap_or_else(|| ".".to_string());

                                let expected_path_label = format!("📁 {}", crate::i18n::tr(lang, "save.expected_path"));
                                let file_label = format!("📄 {}", crate::i18n::tr(lang, "save.file"));

                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgba_premultiplied(28, 16, 16, 220))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(110, 45, 45)))
                                    .corner_radius(6.0_f32)
                                    .inner_margin(egui::Margin::symmetric(14, 10))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&expected_path_label).size(11.0).color(Color32::from_rgb(200, 140, 140)));
                                            ui.add(egui::Label::new(
                                                RichText::new(&folder_path).size(11.0).monospace().color(Color32::from_rgb(240, 180, 180))
                                            ).truncate()).on_hover_text(&folder_path);
                                        });
                                        ui.add_space(2.0);
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new(&file_label).size(11.0).color(Color32::from_rgb(200, 140, 140)));
                                            ui.label(RichText::new(&feedback.filename).size(12.0).strong().monospace().color(Color32::from_rgb(255, 140, 140)));
                                        });
                                        if let Some(ref err) = feedback.error_message {
                                            let err_label = crate::i18n::tr(lang, "save.error");
                                            ui.add_space(4.0);
                                            ui.label(RichText::new(format!("⚠️ {err_label} {err}")).size(11.0).color(Color32::from_rgb(255, 170, 170)));
                                        }
                                    });

                                ui.add_space(14.0);
                                let open_folder_text = format!("📂 {}", crate::i18n::tr(lang, "action.open_folder"));
                                let open_folder_desc = crate::i18n::tr(lang, "action.open_folder_desc");
                                let close_text = crate::i18n::tr(lang, "action.close");
                                ui.horizontal(|ui| {
                                    if ui.button(RichText::new(&open_folder_text).size(12.0))
                                        .on_hover_text(open_folder_desc)
                                        .clicked()
                                    {
                                        state.reveal_in_file_explorer(&feedback.target_path);
                                    }
                                    if ui.button(RichText::new(close_text).size(12.0)).clicked() {
                                        dismiss_requested = true;
                                    }
                                });
                            }
                        }
                    });
                });
        });

    if dismiss_requested {
        state.save_feedback = None;
        state.pending_save_op = None;
    }
}
