use egui::{self, Color32, RichText};
use glam::Vec3;
use crate::editor_state::{BrushScaleMode, EditorState};

pub fn render_brush_scale_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_scale_brush_dialog {
        return;
    }

    let mut open = state.show_scale_brush_dialog;
    let mut close_requested = false;

    crate::gui::modal::compact_dialog_window(ctx, RichText::new("Scale Brush").strong(), &mut open)
        .show(ctx, |ui| {
            ui.add_space(4.0);

            // Target Info Header
            let target_desc = if state.has_multi_selection() {
                format!("Target: {} Multi-Selected Brush(es)", state.multi_selected_brushes.len())
            } else if let Some(b_idx) = state.selected_brush_index {
                let name = state.world_brushes.get(b_idx).map(|b| b.name.as_str()).unwrap_or("Brush");
                format!("Target: Brush #{} ('{}')", b_idx, name)
            } else {
                "Target: Active Red Builder Brush".to_string()
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(target_desc).strong().color(Color32::from_rgb(245, 182, 45)));
            });
            ui.separator();

            // Mode Selector
            ui.label(RichText::new("Scale Operation Mode:").strong());
            ui.horizontal(|ui| {
                ui.radio_value(&mut state.scale_brush_mode, BrushScaleMode::GeometryVertices, "Geometry Vertices (CSG)");
                ui.radio_value(&mut state.scale_brush_mode, BrushScaleMode::ActorTransform, "Actor Transform (.scale)");
            });

            ui.add_space(4.0);
            ui.checkbox(&mut state.scale_brush_uniform, "Uniform Scaling (Lock Aspect Ratio)");
            if state.scale_brush_mode == BrushScaleMode::GeometryVertices {
                ui.checkbox(&mut state.scale_brush_adjust_textures, "Adjust Texture UVs with Geometry");
            }

            ui.separator();
            ui.label(RichText::new("Scaling Factors:").strong());

            let mut scale = state.scale_brush_factors;
            let mut changed = false;

            ui.horizontal(|ui| {
                ui.label("X Factor:");
                let drag = ui.add(egui::DragValue::new(&mut scale.x).speed(0.05).range(0.01..=100.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                if drag.changed() {
                    changed = true;
                    if state.scale_brush_uniform {
                        scale.y = scale.x;
                        scale.z = scale.x;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Y Factor:");
                let drag = ui.add(egui::DragValue::new(&mut scale.y).speed(0.05).range(0.01..=100.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                if drag.changed() {
                    changed = true;
                    if state.scale_brush_uniform {
                        scale.x = scale.y;
                        scale.z = scale.y;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Z Factor:");
                let drag = ui.add(egui::DragValue::new(&mut scale.z).speed(0.05).range(0.01..=100.0).custom_formatter(|n, _| format!("{:.2}x", n)));
                if drag.changed() {
                    changed = true;
                    if state.scale_brush_uniform {
                        scale.x = scale.z;
                        scale.y = scale.z;
                    }
                }
            });

            if changed {
                state.scale_brush_factors = scale;
            }

            ui.add_space(6.0);
            ui.label(RichText::new("Quick Factor Presets:").size(11.0).color(Color32::from_rgb(170, 180, 195)));
            ui.horizontal_wrapped(|ui| {
                if ui.small_button("0.5x Halve").clicked() {
                    state.scale_brush_factors = Vec3::splat(0.5);
                }
                if ui.small_button("2.0x Double").clicked() {
                    state.scale_brush_factors = Vec3::splat(2.0);
                }
                if ui.small_button("4.0x").clicked() {
                    state.scale_brush_factors = Vec3::splat(4.0);
                }
                if ui.small_button("0.25x").clicked() {
                    state.scale_brush_factors = Vec3::splat(0.25);
                }
                if ui.small_button("10.0x").clicked() {
                    state.scale_brush_factors = Vec3::splat(10.0);
                }
                if ui.small_button("0.1x").clicked() {
                    state.scale_brush_factors = Vec3::splat(0.1);
                }
                if ui.small_button("Reset 1.0").clicked() {
                    state.scale_brush_factors = Vec3::ONE;
                }
            });

            ui.add_space(4.0);
            ui.label(RichText::new("Coordinate Inversions (Mirrors):").size(11.0).color(Color32::from_rgb(170, 180, 195)));
            ui.horizontal(|ui| {
                if ui.small_button("Invert X (-1, 1, 1)").clicked() {
                    state.scale_brush_factors = Vec3::new(-1.0, 1.0, 1.0);
                    state.scale_brush_uniform = false;
                }
                if ui.small_button("Invert Y (1, -1, 1)").clicked() {
                    state.scale_brush_factors = Vec3::new(1.0, -1.0, 1.0);
                    state.scale_brush_uniform = false;
                }
                if ui.small_button("Invert Z (1, 1, -1)").clicked() {
                    state.scale_brush_factors = Vec3::new(1.0, 1.0, -1.0);
                    state.scale_brush_uniform = false;
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                let apply_btn = egui::Button::new(RichText::new("✓ Apply Scale").strong().color(Color32::BLACK))
                    .fill(Color32::from_rgb(245, 182, 45));

                if ui.add(apply_btn).clicked() {
                    let s = state.scale_brush_factors;
                    let m = state.scale_brush_mode;
                    let adj = state.scale_brush_adjust_textures;
                    state.scale_selected(s, m, adj);
                    state.toast_success(format!("Scaled selection by ({:.2}x, {:.2}x, {:.2}x)", s.x, s.y, s.z));
                }

                if ui.button("Close").clicked() {
                    close_requested = true;
                }
            });
        });

    if !open || close_requested {
        state.show_scale_brush_dialog = false;
    }
}
