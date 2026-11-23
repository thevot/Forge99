//! Interactive Dynamic Zone Water Surface & Wave Reflection Studio
//!
//! Provides real-time 2D discrete wave equation simulation, ripple propagation,
//! surface normal and caustic focusing computation, and automated watertight
//! water sheet brush generation for Unreal Engine 1 water and fluid zones.

use egui::{pos2, vec2, Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::WaterPreset;
use crate::editor_state::EditorState;

pub fn render_water_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_water_surface_studio_dialog {
        return;
    }

    let mut is_open = state.show_water_surface_studio_dialog;
    Window::new("Dynamic Zone Water Surface & Wave Reflection Studio")
        .open(&mut is_open)
        .default_width(960.0)
        .default_height(720.0)
        .min_width(800.0)
        .min_height(580.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_water_surface_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    // 0. Step simulation if playing
    let dt = ui.input(|i| i.stable_dt).min(0.05);
    state.step_water_surface_simulation(dt);
    if state.water_surface_studio_settings.is_playing {
        ui.ctx().request_repaint();
    }

    // 1. Header Banner
    ui.heading("🌊 Dynamic Zone Water Surface & Wave Reflection Studio");
    ui.label(
        RichText::new(
            "Simulate 2D wave equations, caustic light focusing, and generate watertight tessellated water sheet brushes for Unreal Engine 1 fluid zones.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 175, 195)),
    );
    ui.separator();

    // 2. Preset Selection Ribbon
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset:").strong());
        for preset in &WaterPreset::ALL {
            let is_selected = state.water_surface_studio_settings.selected_preset == *preset;
            let label = match preset {
                WaterPreset::CrystalClearPool => "🌊 Clear Pool",
                WaterPreset::MurkySwampVat => "🍃 Swamp Vat",
                WaterPreset::TorrentialRainLake => "🌧 Rain Lake",
                WaterPreset::FastRiverRapids => "🛶 River Rapids",
                WaterPreset::ToxicVatBubbles => "🧪 Toxic Bubbles",
                WaterPreset::LavaMagmaCrust => "🌋 Lava Crust",
            };

            let btn = if is_selected {
                ui.button(RichText::new(label).strong().color(Color32::from_rgb(100, 220, 255)))
            } else {
                ui.button(label)
            };

            if btn.clicked() {
                state.apply_water_preset(*preset);
            }
        }
    });

    ui.separator();

    // 3. Transport & Ripple Controls Bar
    ui.horizontal(|ui| {
        let play_text = if state.water_surface_studio_settings.is_playing {
            "⏸ Pause"
        } else {
            "▶ Play"
        };
        if ui.button(play_text).clicked() {
            state.water_surface_studio_settings.is_playing = !state.water_surface_studio_settings.is_playing;
        }

        if ui.button("⏭ Step").clicked() {
            let config = state.water_surface_studio_settings.active_config.clone();
            state.water_surface_studio_settings.simulation.step(0.016, &config);
        }

        if ui.button("💧 Drop Pebble").clicked() {
            let u = 0.3 + ((ui.input(|i| i.time * 3.7) as f32).sin() * 0.3);
            let v = 0.5 + ((ui.input(|i| i.time * 2.3) as f32).cos() * 0.3);
            let amp = state.water_surface_studio_settings.ripple_brush_amplitude;
            state.inject_water_ripple_uv(u, v, amp);
        }

        if ui.button("🔄 Clear Tank").clicked() {
            state.reset_water_simulation();
        }

        ui.separator();

        ui.label("Speed:");
        ui.selectable_value(&mut state.water_surface_studio_settings.simulation_speed, 0.5, "0.5x");
        ui.selectable_value(&mut state.water_surface_studio_settings.simulation_speed, 1.0, "1.0x");
        ui.selectable_value(&mut state.water_surface_studio_settings.simulation_speed, 2.0, "2.0x");

        ui.separator();

        ui.label("Ripple Radius:");
        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.ripple_brush_radius, 1..=5).suffix(" cells"));

        ui.label("Amplitude:");
        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.ripple_brush_amplitude, 5.0..=50.0).suffix(" uu"));
    });

    ui.separator();

    // 4. Interactive 2D Wave Tank Canvas & Caustics Visualizer
    ui.columns(2, |cols| {
        // --- Column 1: 2D Wave Tank Canvas ---
        cols[0].vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("🌊 Interactive Wave Tank (Click & Drag to Ripple)").strong());
            });

            let canvas_width = ui.available_width().max(320.0);
            let canvas_height = 280.0;
            let (rect, response) = ui.allocate_exact_size(vec2(canvas_width, canvas_height), Sense::click_and_drag());

            // Handle mouse interaction
            if response.clicked() || response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    let u = ((pos.x - rect.left()) / rect.width()).clamp(0.0, 1.0);
                    let v = ((pos.y - rect.top()) / rect.height()).clamp(0.0, 1.0);
                    let amp = state.water_surface_studio_settings.ripple_brush_amplitude;
                    state.inject_water_ripple_uv(u, v, amp);
                }
            }

            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, Color32::from_rgb(12, 20, 32));
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 70, 100)), StrokeKind::Inside);

            let sim = &state.water_surface_studio_settings.simulation;
            let w = sim.width;
            let h = sim.height;

            if w > 0 && h > 0 {
                let cell_w = rect.width() / w as f32;
                let cell_h = rect.height() / h as f32;

                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let height_val = sim.current_heights[idx];
                        let cell_rect = Rect::from_min_size(
                            pos2(rect.left() + x as f32 * cell_w, rect.top() + y as f32 * cell_h),
                            vec2(cell_w + 0.5, cell_h + 0.5),
                        );

                        // Color mapping based on wave displacement
                        let color = if height_val >= 0.0 {
                            let t = (height_val / 20.0).clamp(0.0, 1.0);
                            let r = (18.0 + t * 200.0) as u8;
                            let g = (48.0 + t * 195.0) as u8;
                            let b = (95.0 + t * 160.0) as u8;
                            Color32::from_rgb(r, g, b)
                        } else {
                            let t = (-height_val / 20.0).clamp(0.0, 1.0);
                            let r = (18.0 * (1.0 - t * 0.7)) as u8;
                            let g = (48.0 * (1.0 - t * 0.7)) as u8;
                            let b = (95.0 * (1.0 - t * 0.6)) as u8;
                            Color32::from_rgb(r, g, b)
                        };

                        painter.rect_filled(cell_rect, 0.0, color);
                    }
                }

                // Draw flow vector arrow overlay if flow is active
                let flow = state.water_surface_studio_settings.active_config.flow_velocity;
                if flow.x.abs() > 1.0 || flow.y.abs() > 1.0 {
                    let center = rect.center();
                    let dir = vec2(flow.x, flow.y).normalized();
                    let arrow_end = center + dir * 35.0;
                    painter.arrow(center, arrow_end - center, Stroke::new(2.0_f32, Color32::from_rgb(255, 220, 80)));
                    painter.text(
                        arrow_end + vec2(8.0, 0.0),
                        egui::Align2::LEFT_CENTER,
                        format!("Flow ({:.0}, {:.0})", flow.x, flow.y),
                        egui::FontId::monospace(10.0),
                        Color32::from_rgb(255, 220, 80),
                    );
                }
            }
        });

        // --- Column 2: Real-time Caustics Intensity Preview ---
        cols[1].vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("✨ Real-time Underwater Caustics & Reflection Map").strong());
            });

            let canvas_width = ui.available_width().max(320.0);
            let canvas_height = 280.0;
            let (rect, _) = ui.allocate_exact_size(vec2(canvas_width, canvas_height), Sense::hover());

            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, Color32::from_rgb(8, 14, 24));
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 80, 110)), StrokeKind::Inside);

            let sim = &state.water_surface_studio_settings.simulation;
            let w = sim.width;
            let h = sim.height;

            if w > 0 && h > 0 {
                let cell_w = rect.width() / w as f32;
                let cell_h = rect.height() / h as f32;

                for y in 0..h {
                    for x in 0..w {
                        let idx = y * w + x;
                        let caustic = sim.caustics_intensity_map[idx];
                        let cell_rect = Rect::from_min_size(
                            pos2(rect.left() + x as f32 * cell_w, rect.top() + y as f32 * cell_h),
                            vec2(cell_w + 0.5, cell_h + 0.5),
                        );

                        // Caustic light focusing: 1.0 is neutral, > 1.0 is focused rays (golden/white), < 1.0 is dispersed
                        let color = if caustic >= 1.0 {
                            let t = ((caustic - 1.0) / 1.5).clamp(0.0, 1.0);
                            let r = (15.0 + t * 240.0) as u8;
                            let g = (35.0 + t * 220.0) as u8;
                            let b = (70.0 + t * 185.0) as u8;
                            Color32::from_rgb(r, g, b)
                        } else {
                            let t = ((1.0 - caustic) / 0.9).clamp(0.0, 1.0);
                            let r = (15.0 * (1.0 - t * 0.8)) as u8;
                            let g = (35.0 * (1.0 - t * 0.8)) as u8;
                            let b = (70.0 * (1.0 - t * 0.7)) as u8;
                            Color32::from_rgb(r, g, b)
                        };

                        painter.rect_filled(cell_rect, 0.0, color);
                    }
                }

                // Caustics info overlay
                painter.text(
                    rect.left_bottom() + vec2(10.0, -10.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("Intensity Mult: {:.2}x | Scale: {:.2}", state.water_surface_studio_settings.active_config.caustics_intensity, state.water_surface_studio_settings.active_config.caustics_scale),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(220, 240, 255),
                );
            }
        });
    });

    ui.separator();

    // 5. Lower Settings Tabs: [Physics & Optics Inspector] and [Watertight Water Sheet Generator]
    ui.horizontal(|ui| {
        ui.columns(2, |cols| {
            // Left Column: Physical & Optical Parameters
            cols[0].vertical(|ui| {
                ui.label(RichText::new("⚙ Physical & Optical Parameters").strong());

                egui::Grid::new("water_physics_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Wave Speed (c):");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.wave_speed, 0.2..=2.5));
                        ui.end_row();

                        ui.label("Viscosity Damping:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.damping, 0.88..=0.999));
                        ui.end_row();

                        ui.label("Surface Tension:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.tension, 0.1..=1.0));
                        ui.end_row();

                        ui.label("Caustics Intensity:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.caustics_intensity, 0.0..=3.0));
                        ui.end_row();

                        ui.label("Refraction Index (n):");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.refraction_index, 1.0..=1.6).suffix(" (Water: 1.33)"));
                        ui.end_row();

                        ui.label("Flow Velocity X:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.flow_velocity.x, -200.0..=200.0).suffix(" uu/s"));
                        ui.end_row();

                        ui.label("Flow Velocity Y:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.flow_velocity.y, -200.0..=200.0).suffix(" uu/s"));
                        ui.end_row();

                        ui.label("Ambient Rain Droplets:");
                        ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.active_config.ambient_rain_intensity, 0.0..=1.0));
                        ui.end_row();
                    });
            });

            // Right Column: Watertight Water Sheet Generator & Viewport Options
            cols[1].vertical(|ui| {
                ui.label(RichText::new("📐 Watertight Water Sheet Generator").strong());

                egui::Grid::new("water_sheet_grid")
                    .num_columns(2)
                    .spacing([12.0, 6.0])
                    .show(ui, |ui| {
                        ui.label("Center (X, Y, Z):");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut state.water_surface_studio_settings.sheet_center.x).speed(1.0).prefix("X:"));
                            ui.add(egui::DragValue::new(&mut state.water_surface_studio_settings.sheet_center.y).speed(1.0).prefix("Y:"));
                            ui.add(egui::DragValue::new(&mut state.water_surface_studio_settings.sheet_center.z).speed(1.0).prefix("Z:"));
                        });
                        ui.end_row();

                        ui.label("Sheet Size (X x Y):");
                        ui.horizontal(|ui| {
                            ui.add(egui::DragValue::new(&mut state.water_surface_studio_settings.sheet_size_x).speed(16.0).range(32.0..=8192.0).prefix("W:"));
                            ui.add(egui::DragValue::new(&mut state.water_surface_studio_settings.sheet_size_y).speed(16.0).range(32.0..=8192.0).prefix("H:"));
                        });
                        ui.end_row();

                        ui.label("Tessellation Subdivisions:");
                        ui.horizontal(|ui| {
                            ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.subdivisions_x, 1..=32).prefix("X:"));
                            ui.add(egui::Slider::new(&mut state.water_surface_studio_settings.subdivisions_y, 1..=32).prefix("Y:"));
                        });
                        ui.end_row();

                        ui.label("Texture:");
                        ui.text_edit_singleline(&mut state.water_surface_studio_settings.sheet_texture_name);
                        ui.end_row();
                    });

                ui.add_space(6.0);

                if ui.button(RichText::new("🌊 Generate Watertight Water Sheet Brush").strong().color(Color32::from_rgb(255, 230, 100))).clicked() {
                    state.generate_water_sheet_brush();
                }

                ui.separator();
                ui.label(RichText::new("👁 Viewport Overlay Settings").strong());
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.water_surface_studio_settings.show_wave_wireframe_viewport, "Wave Mesh Wireframe");
                    ui.checkbox(&mut state.water_surface_studio_settings.show_caustics_viewport, "Caustics Light Rays");
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.water_surface_studio_settings.show_flow_vectors_viewport, "Flow Direction Vectors");
                    ui.checkbox(&mut state.water_surface_studio_settings.show_boundary_box_viewport, "Water Sheet Boundary Box");
                });
            });
        });
    });
}
