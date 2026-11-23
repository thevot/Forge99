use crate::editor_state::EditorState;
use crate::gui::modal::compact_dialog_window;
use egui::Color32;

pub fn render_bookmarks_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_camera_bookmarks_dialog {
        return;
    }

    let mut open = state.show_camera_bookmarks_dialog;
    let mut close_dialog = false;

    compact_dialog_window(ctx, "Camera Bookmarks (0..9)", &mut open)
        .show(ctx, |ui| {
            ui.label(egui::RichText::new("Quickly save and fly to up to 10 points of interest in your map.").italics());
            ui.label(egui::RichText::new("Hotkeys: Ctrl+1..0 to set bookmark, Alt+1..0 to jump to bookmark.").size(11.0).color(egui::Color32::from_rgb(180, 200, 240)));
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                for slot in 0..10 {
                    let display_num = if slot == 9 { 0 } else { slot + 1 };
                    ui.group(|ui| {
                        let has_bm = state.camera_bookmarks[slot].is_some();
                        ui.horizontal(|ui| {
                            let badge_text = format!("[{}]", display_num);
                            ui.label(egui::RichText::new(badge_text).strong().color(egui::Color32::from_rgb(255, 215, 0)));

                            if let Some(bm) = state.camera_bookmarks[slot].as_mut() {
                                ui.text_edit_singleline(&mut bm.name);
                                ui.label(egui::RichText::new(format!(
                                    "({:.0}, {:.0}, {:.0})",
                                    bm.position.x, bm.position.y, bm.position.z
                                )).monospace().size(11.0));
                            } else {
                                ui.label(egui::RichText::new("<Empty Bookmark Slot>").italics().color(egui::Color32::GRAY));
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if has_bm {
                                    if ui.button(egui::RichText::new("✖").color(Color32::from_rgb(255, 115, 115))).on_hover_text("Clear Bookmark").clicked() {
                                        state.clear_camera_bookmark(slot);
                                    }
                                    if ui.button(egui::RichText::new("🚀 Jump").color(Color32::from_rgb(110, 240, 160))).on_hover_text(format!("Jump Camera (Alt+{})", display_num)).clicked() {
                                        state.goto_camera_bookmark(slot);
                                    }
                                }
                                if ui.button(egui::RichText::new("💾 Set").color(Color32::from_rgb(255, 215, 80))).on_hover_text(format!("Save Current Camera (Ctrl+{})", display_num)).clicked() {
                                    state.set_camera_bookmark(slot, None);
                                }
                            });
                        });
                    });
                    ui.add_space(2.0);
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        open = false;
    }
    state.show_camera_bookmarks_dialog = open;
}
