use egui::{self, Color32, RichText};
use glam::Vec3;
use crate::editor_state::EditorState;

pub fn render_scale_map_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_scale_map_dialog {
        return;
    }

    let mut open = state.show_scale_map_dialog;
    let mut close_requested = false;
    let mut execute_scale = false;

    crate::gui::modal::standard_dialog_window(
        ctx,
        RichText::new("Scale Entire Map (Tools Parity)").strong(),
        &mut open,
        [440.0, 480.0],
        [360.0, 320.0],
    )
    .show(ctx, |ui| {
            ui.add_space(4.0);

            // Summary Header
            let brush_count = state.world_brushes.len();
            let mover_count = state.movers.len();
            let actor_count = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "Map Content: {} Brushes | {} Movers | {} Actors",
                        brush_count, mover_count, actor_count
                    ))
                    .strong()
                    .color(Color32::from_rgb(255, 200, 80)),
                );
            });
            ui.separator();

            ui.label(RichText::new("Scaling Factor:").strong());

            // Quick Preset Buttons
            ui.horizontal(|ui| {
                ui.label("Presets:");
                if ui.small_button("0.5x").clicked() {
                    state.scale_map_factor = Vec3::splat(0.5);
                }
                if ui.small_button("0.75x").clicked() {
                    state.scale_map_factor = Vec3::splat(0.75);
                }
                if ui.small_button("1.25x").clicked() {
                    state.scale_map_factor = Vec3::splat(1.25);
                }
                if ui.small_button("1.5x").clicked() {
                    state.scale_map_factor = Vec3::splat(1.5);
                }
                if ui.small_button("2.0x").clicked() {
                    state.scale_map_factor = Vec3::splat(2.0);
                }
                if ui.small_button("Reset (1.0x)").clicked() {
                    state.scale_map_factor = Vec3::ONE;
                }
            });

            ui.add_space(6.0);

            // Per-Axis / Uniform Controls
            let mut factor = state.scale_map_factor;
            egui::Grid::new("scale_map_grid")
                .num_columns(2)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label("X Axis Factor:");
                    ui.add(egui::DragValue::new(&mut factor.x).speed(0.02).range(0.01..=50.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                    ui.end_row();

                    ui.label("Y Axis Factor:");
                    ui.add(egui::DragValue::new(&mut factor.y).speed(0.02).range(0.01..=50.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                    ui.end_row();

                    ui.label("Z Axis Factor:");
                    ui.add(egui::DragValue::new(&mut factor.z).speed(0.02).range(0.01..=50.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                    ui.end_row();
                });
            state.scale_map_factor = factor;

            ui.add_space(6.0);
            ui.separator();
            ui.label(RichText::new("Target Entities:").strong());

            ui.checkbox(&mut state.scale_map_actors, "Scale Actor World Coordinates");
            ui.checkbox(&mut state.scale_map_lights, "Scale Light Radii Proportionally");

            ui.add_space(8.0);
            ui.label(
                RichText::new("ℹ Note: All brush vertices, mover paths and actor positions will be scaled around world origin (0, 0, 0). Undo is supported.")
                    .italics()
                    .color(Color32::from_rgb(160, 160, 160)),
            );

            ui.add_space(10.0);
            ui.separator();

            let mut cancel_clicked = false;
            // Footer Buttons
            ui.horizontal(|ui| {
                if ui.button(RichText::new("🚀 Scale Entire Map").strong().color(Color32::from_rgb(255, 255, 255))).clicked() {
                    execute_scale = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
            });
            if cancel_clicked {
                close_requested = true;
            }
        });

    if close_requested {
        open = false;
    }

    if execute_scale {
        let f = state.scale_map_factor;
        let scale_actors = state.scale_map_actors;
        let scale_lights = state.scale_map_lights;
        if state.scale_entire_map(f, scale_actors, scale_lights) {
            open = false;
        }
    }

    state.show_scale_map_dialog = open;
}
