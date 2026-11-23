use egui::{self, Color32, RichText};
use crate::editor_state::{EditorState, ShearAxis};

pub fn render_brush_shear_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_shear_brush_dialog {
        return;
    }

    let mut open = state.show_shear_brush_dialog;
    let mut close_requested = false;

    crate::gui::modal::compact_dialog_window(ctx, RichText::new("Shear Brush (UnrealEd 2.0 Parity)").strong(), &mut open)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Target Info
            let target_desc = if state.has_multi_selection() {
                format!("Target: {} Multi-Selected Brush(es)", state.multi_selected_brushes.len())
            } else if let Some(b_idx) = state.selected_brush_index {
                let name = state.world_brushes.get(b_idx).map(|b| b.name.as_str()).unwrap_or("Brush");
                format!("Target: Brush #{} ('{}')", b_idx, name)
            } else {
                "Target: Active Red Builder Brush".to_string()
            };

            ui.label(RichText::new(target_desc).strong().color(Color32::from_rgb(245, 182, 45)));
            ui.separator();

            // Shear Axis
            ui.label(RichText::new("Shear Transformation Axis:").strong());
            let axes = [
                (ShearAxis::XZ, "X along Z (Ramp along X)"),
                (ShearAxis::YZ, "Y along Z (Ramp along Y)"),
                (ShearAxis::ZX, "Z along X (Vertical slope on X)"),
                (ShearAxis::ZY, "Z along Y (Vertical slope on Y)"),
                (ShearAxis::XY, "X along Y (Horizontal skew X)"),
                (ShearAxis::YX, "Y along X (Horizontal skew Y)"),
            ];

            for (ax, label) in axes {
                ui.radio_value(&mut state.shear_brush_axis, ax, label);
            }

            ui.separator();

            // Factor & Anchor
            ui.horizontal(|ui| {
                ui.label(RichText::new("Shear Factor:").strong());
                ui.add(egui::DragValue::new(&mut state.shear_brush_factor).speed(0.05).range(-10.0..=10.0));
            });

            ui.checkbox(&mut state.shear_brush_anchor_base, "Anchor to Base (Keep bottom surface resting on floor)");

            ui.add_space(6.0);
            ui.label(RichText::new("Slope & Ramp Presets:").size(11.0).color(Color32::from_rgb(170, 180, 195)));
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("45° Ramp (1.0)").clicked() {
                    state.shear_brush_factor = 1.0;
                }
                if ui.small_button("2:1 Slope (0.5)").clicked() {
                    state.shear_brush_factor = 0.5;
                }
                if ui.small_button("4:1 Gentle (0.25)").clicked() {
                    state.shear_brush_factor = 0.25;
                }
                if ui.small_button("Steep (2.0)").clicked() {
                    state.shear_brush_factor = 2.0;
                }
                if ui.small_button("Reverse (-1.0)").clicked() {
                    state.shear_brush_factor = -1.0;
                }
                if ui.small_button("Reverse (-0.5)").clicked() {
                    state.shear_brush_factor = -0.5;
                }
                if ui.small_button("Reset (0.0)").clicked() {
                    state.shear_brush_factor = 0.0;
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                let apply_btn = egui::Button::new(RichText::new("⚡ Apply Shear").strong().color(Color32::BLACK))
                    .fill(Color32::from_rgb(245, 182, 45));

                if ui.add(apply_btn).clicked() {
                    let ax = state.shear_brush_axis;
                    let f = state.shear_brush_factor;
                    let anchor = state.shear_brush_anchor_base;
                    state.shear_selected(ax, f, anchor);
                }

                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
        });

    if !open || close_requested {
        state.show_shear_brush_dialog = false;
    }
}
