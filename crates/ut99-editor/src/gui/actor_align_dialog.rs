use egui::{self, Color32, RichText};
use crate::editor_state::{AlignAxis, AlignMode, EditorState};

pub fn render_actor_align_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_actor_align_dialog {
        return;
    }

    let mut open = state.show_actor_align_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        RichText::new("Align & Distribute Actors (UnrealEd 2.0 Parity)").strong(),
        &mut open,
        [420.0, 480.0],
        [360.0, 300.0],
    )
    .show(ctx, |ui| {
            ui.add_space(4.0);

            let selected_count = if state.has_multi_selection() {
                state.multi_selected_actors.len()
            } else if state.selected_actor_index.is_some() {
                1
            } else {
                0
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Selected Actors: {}", selected_count)).strong().color(Color32::from_rgb(100, 200, 255)));
                if selected_count < 2 {
                    ui.label(RichText::new("(Select at least 2 actors with Shift+Click)").italics().size(11.0).color(Color32::from_rgb(240, 160, 60)));
                }
            });

            ui.separator();

            // SECTION 1: ALIGNMENT
            ui.label(RichText::new("1. Coordinate Alignment:").strong());
            ui.label(RichText::new("Align all selected actors to have matching coordinate:").size(11.0).color(Color32::from_rgb(170, 180, 195)));

            let enabled = selected_count >= 2;

            for axis in [AlignAxis::X, AlignAxis::Y, AlignAxis::Z] {
                let axis_name = match axis {
                    AlignAxis::X => "Axis X",
                    AlignAxis::Y => "Axis Y",
                    AlignAxis::Z => "Axis Z (Height)",
                };

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{}:", axis_name)).strong().color(Color32::WHITE));
                    if ui.add_enabled(enabled, egui::Button::new("◀ Min")).on_hover_text("Align to minimum coordinate").clicked() {
                        state.align_selected_actors(axis, AlignMode::Min);
                    }
                    if ui.add_enabled(enabled, egui::Button::new("● Center (Avg)")).on_hover_text("Align to center/average coordinate").clicked() {
                        state.align_selected_actors(axis, AlignMode::Center);
                    }
                    if ui.add_enabled(enabled, egui::Button::new("▶ Max")).on_hover_text("Align to maximum coordinate").clicked() {
                        state.align_selected_actors(axis, AlignMode::Max);
                    }
                });
            }

            ui.separator();

            // SECTION 2: DISTRIBUTION
            ui.label(RichText::new("2. Spatial Distribution:").strong());
            ui.label(RichText::new("Distribute selected actors evenly between endpoints:").size(11.0).color(Color32::from_rgb(170, 180, 195)));

            let dist_enabled = selected_count >= 3;

            ui.horizontal(|ui| {
                ui.label(RichText::new("Even Spacing:").strong());
                if ui.add_enabled(dist_enabled, egui::Button::new("↔ Along X")).clicked() {
                    state.distribute_selected_actors_evenly(AlignAxis::X);
                }
                if ui.add_enabled(dist_enabled, egui::Button::new("↔ Along Y")).clicked() {
                    state.distribute_selected_actors_evenly(AlignAxis::Y);
                }
                if ui.add_enabled(dist_enabled, egui::Button::new("↕ Along Z")).clicked() {
                    state.distribute_selected_actors_evenly(AlignAxis::Z);
                }
            });

            ui.add_space(4.0);

            // Fixed Spacing
            ui.horizontal(|ui| {
                ui.label(RichText::new("Fixed Interval:").strong());
                let spacing = state.grid_step.max(32.0);
                if ui.add_enabled(enabled, egui::Button::new(format!("Along X ({:.0}u)", spacing))).clicked() {
                    state.distribute_selected_actors_spacing(AlignAxis::X, spacing);
                }
                if ui.add_enabled(enabled, egui::Button::new(format!("Along Y ({:.0}u)", spacing))).clicked() {
                    state.distribute_selected_actors_spacing(AlignAxis::Y, spacing);
                }
                if ui.add_enabled(enabled, egui::Button::new(format!("Along Z ({:.0}u)", spacing))).clicked() {
                    state.distribute_selected_actors_spacing(AlignAxis::Z, spacing);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(RichText::new("💡 Full Undo/Redo supported (Ctrl+Z / Ctrl+Y).").size(11.0).italics().color(Color32::from_rgb(140, 150, 165)));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if !open || close_requested {
        state.show_actor_align_dialog = false;
    }
}
