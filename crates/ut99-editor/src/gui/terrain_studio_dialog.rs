use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use glam::Vec2;
use ut99_bsp::{CsgOperation, TerrainPreset, TerrainSculptMode};

/// Interactive Terrain & Heightmap Mesh Sculptor Studio
pub fn render_terrain_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_terrain_studio_dialog {
        return;
    }

    let mut is_open = state.show_terrain_studio_dialog;

    Window::new("Terrain & Heightmap Mesh Sculptor Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(580.0)
        .min_width(620.0)
        .min_height(440.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.heading("🏔 Terrain Sculptor");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➖ Bake Subtractive").clicked() {
                        state.bake_terrain_to_world(CsgOperation::Subtract);
                    }
                    if ui.button("➕ Bake Additive").clicked() {
                        state.bake_terrain_to_world(CsgOperation::Add);
                    }
                    if ui.button("⚡ Bake to Builder").clicked() {
                        state.synthesize_terrain_to_builder();
                    }
                });
            });

            ui.separator();

            // Terrain Metrics & Viewport Preview Controls
            let width_uu = (state.terrain_settings.grid.cols as f32) * state.terrain_settings.grid.cell_size;
            let length_uu = (state.terrain_settings.grid.rows as f32) * state.terrain_settings.grid.cell_size;
            let tri_count = state.terrain_settings.grid.rows * state.terrain_settings.grid.cols * 2;
            let (min_z, max_z) = state.terrain_settings.grid.min_max_elevation();

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(100, 220, 255),
                        format!("Grid: {}x{} ({:.0}x{:.0} uu)", state.terrain_settings.grid.cols, state.terrain_settings.grid.rows, width_uu, length_uu),
                    );
                    ui.separator();
                    ui.colored_label(
                        Color32::from_rgb(255, 200, 50),
                        format!("Triangles: {} | Elev: [{:.0}..{:.0}] uu", tri_count, min_z, max_z),
                    );
                    ui.separator();
                    ui.checkbox(&mut state.terrain_settings.show_preview, "Live 3D Viewport");
                    ui.checkbox(&mut state.terrain_settings.grid.solid_base, "Solid Base Skirt");
                });
            });

            ui.add_space(4.0);

            // Main Editor Layout: Left Column = Tools, Right Column = Presets
            ui.horizontal(|ui| {
                // Left Column: Sculpting Tools
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    egui::ScrollArea::vertical().id_salt("terrain_tools_scroll").max_height(400.0).show(ui, |ui| {
                        render_sculpting_controls(ui, state);
                    });
                });

                ui.separator();

                // Right Column: Topographic Presets Library
                ui.vertical(|ui| {
                    ui.label(RichText::new("⭐ Canonical Topographic Presets").strong().size(13.0));
                    ui.label(RichText::new("Calibrated orographic layouts for legendary arena types.").size(11.0).color(Color32::GRAY));
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical().id_salt("terrain_presets_scroll").max_height(400.0).show(ui, |ui| {
                        for preset in TerrainPreset::all() {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(preset.title()).strong().color(Color32::from_rgb(100, 220, 255)));
                                    ui.label(RichText::new(preset.description()).size(11.0).color(Color32::LIGHT_GRAY));
                                    ui.add_space(4.0);
                                    if ui.button("Apply Preset").clicked() {
                                        state.apply_terrain_preset(*preset);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                });
            });
        });

    state.show_terrain_studio_dialog = is_open;
}

fn render_sculpting_controls(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Interactive Sculpting Brush").strong().size(13.0));
    ui.add_space(4.0);

    // Sculpt Mode Selector
    ui.horizontal(|ui| {
        let cur_mode = state.terrain_settings.sculpt_mode;
        if ui.selectable_label(cur_mode == TerrainSculptMode::Raise, "⬆ Raise").clicked() {
            state.terrain_settings.sculpt_mode = TerrainSculptMode::Raise;
        }
        if ui.selectable_label(cur_mode == TerrainSculptMode::Lower, "⬇ Lower").clicked() {
            state.terrain_settings.sculpt_mode = TerrainSculptMode::Lower;
        }
        if ui.selectable_label(cur_mode == TerrainSculptMode::Smooth, "〰 Smooth").clicked() {
            state.terrain_settings.sculpt_mode = TerrainSculptMode::Smooth;
        }
        if ui.selectable_label(cur_mode == TerrainSculptMode::Flatten, "⎯ Flatten").clicked() {
            state.terrain_settings.sculpt_mode = TerrainSculptMode::Flatten;
        }
        if ui.selectable_label(cur_mode == TerrainSculptMode::Noise, "⚡ Noise").clicked() {
            state.terrain_settings.sculpt_mode = TerrainSculptMode::Noise;
        }
    });

    ui.add_space(6.0);

    // Brush Parameters
    ui.horizontal(|ui| {
        ui.label("Brush Radius:");
        ui.add(egui::Slider::new(&mut state.terrain_settings.brush_radius, 64.0..=1024.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Brush Strength:");
        ui.add(egui::Slider::new(&mut state.terrain_settings.brush_strength, 2.0..=128.0).text("uu"));
    });

    if state.terrain_settings.sculpt_mode == TerrainSculptMode::Flatten {
        ui.horizontal(|ui| {
            ui.label("Target Plateau Height:");
            ui.add(egui::Slider::new(&mut state.terrain_settings.target_plateau_height, -512.0..=512.0).text("uu"));
        });
    }

    ui.separator();

    // Brush Center Positioning
    ui.label(RichText::new("Brush Position on Terrain:").strong());
    ui.horizontal(|ui| {
        ui.label("Center X:");
        ui.add(egui::DragValue::new(&mut state.terrain_settings.brush_center.x).speed(8.0).suffix(" uu"));
        ui.label("Center Y:");
        ui.add(egui::DragValue::new(&mut state.terrain_settings.brush_center.y).speed(8.0).suffix(" uu"));
        if ui.button("Center (0, 0)").clicked() {
            state.terrain_settings.brush_center = Vec2::ZERO;
        }
    });

    ui.add_space(6.0);

    // Action Buttons
    ui.horizontal(|ui| {
        if ui.button("🖌 Apply Sculpt Stroke").clicked() {
            let center = state.terrain_settings.brush_center;
            let mode = state.terrain_settings.sculpt_mode;
            let radius = state.terrain_settings.brush_radius;
            let strength = state.terrain_settings.brush_strength;
            let target_h = state.terrain_settings.target_plateau_height;
            state.sculpt_terrain_at(center, mode, radius, strength, target_h);
        }

        if ui.button("⚡ Global Procedural Noise").clicked() {
            state.generate_terrain_noise(96.0, 0.22);
        }

        if ui.button("🔄 Reset to Flat Plane").clicked() {
            let len = state.terrain_settings.grid.heights.len();
            state.terrain_settings.grid.heights = vec![0.0; len];
        }
    });
}
