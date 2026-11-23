use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};
use ut99_bsp::{ProceduralColorPalette, ProceduralTextureType};

pub fn render_procedural_texture_studio(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_procedural_texture_dialog {
        return;
    }

    let mut is_open = state.show_procedural_texture_dialog;
    let mut close_requested = false;

    // Advance simulation step if active
    if state.procedural_anim_playing {
        state.procedural_simulator.step();
        ctx.request_repaint();
    }

    egui::Window::new("FireEngine Procedural Texture Studio")
        .open(&mut is_open)
        .default_size([680.0, 520.0])
        .min_size([540.0, 400.0])
        .resizable(true)
        .show(ctx, |ui| {
            // 1. Top Controls Bar: Presets, Play/Pause, Step & Reset
            ui.horizontal(|ui| {
                ui.label(RichText::new("Preset:").strong());

                let presets = [
                    "🔥 Torch Flame (Fire)",
                    "🌋 Lava Pool (Fire)",
                    "⚡ Blue Plasma (Fire)",
                    "🧪 Toxic Slime (Fire)",
                    "💧 Azure Water (Water)",
                    "☣ Acid Lake (Water)",
                    "🧊 Glacial Frost (Water)",
                ];

                for &preset in &presets {
                    let is_active = state.procedural_texture_name == preset.split(' ').nth(1).unwrap_or("Custom");
                    if ui.selectable_label(is_active, preset).clicked() {
                        match preset {
                            "🔥 Torch Flame (Fire)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_fire(64, 64);
                                state.procedural_simulator.cooling_rate = 1.8;
                                state.procedural_simulator.damping = 0.98;
                                state.procedural_simulator.palette = ProceduralColorPalette::FireThermal;
                                state.procedural_texture_name = "TorchFlame".to_string();
                            }
                            "🌋 Lava Pool (Fire)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_fire(64, 64);
                                state.procedural_simulator.cooling_rate = 0.8;
                                state.procedural_simulator.damping = 0.95;
                                state.procedural_simulator.palette = ProceduralColorPalette::FireThermal;
                                state.procedural_texture_name = "LavaPool".to_string();
                            }
                            "⚡ Blue Plasma (Fire)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_fire(64, 64);
                                state.procedural_simulator.cooling_rate = 1.4;
                                state.procedural_simulator.damping = 0.98;
                                state.procedural_simulator.palette = ProceduralColorPalette::BluePlasma;
                                state.procedural_texture_name = "BluePlasma".to_string();
                            }
                            "🧪 Toxic Slime (Fire)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_fire(64, 64);
                                state.procedural_simulator.cooling_rate = 1.1;
                                state.procedural_simulator.damping = 0.97;
                                state.procedural_simulator.palette = ProceduralColorPalette::GreenToxic;
                                state.procedural_texture_name = "ToxicSlime".to_string();
                            }
                            "💧 Azure Water (Water)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_water(64, 64);
                                state.procedural_simulator.damping = 0.96;
                                state.procedural_simulator.palette = ProceduralColorPalette::WaterAzure;
                                state.procedural_texture_name = "AzureWater".to_string();
                            }
                            "☣ Acid Lake (Water)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_water(64, 64);
                                state.procedural_simulator.damping = 0.94;
                                state.procedural_simulator.palette = ProceduralColorPalette::GreenToxic;
                                state.procedural_texture_name = "AcidLake".to_string();
                            }
                            "🧊 Glacial Frost (Water)" => {
                                state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_water(64, 64);
                                state.procedural_simulator.damping = 0.98;
                                state.procedural_simulator.palette = ProceduralColorPalette::IceFrost;
                                state.procedural_texture_name = "GlacialFrost".to_string();
                            }
                            _ => {}
                        }
                    }
                }
            });

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                let play_icon = if state.procedural_anim_playing { "⏸ Pause" } else { "▶ Play" };
                if ui.button(play_icon).clicked() {
                    state.procedural_anim_playing = !state.procedural_anim_playing;
                }

                if ui.button("⏭ Single Step").clicked() {
                    state.procedural_simulator.step();
                }

                if ui.button("↺ Reset Buffers").clicked() {
                    let w = state.procedural_simulator.width;
                    let h = state.procedural_simulator.height;
                    let is_fire = state.procedural_simulator.tex_type == ProceduralTextureType::Fire;
                    let pal = state.procedural_simulator.palette;
                    state.procedural_simulator = if is_fire {
                        let mut s = ut99_bsp::ProceduralTextureSimulator::new_fire(w, h);
                        s.palette = pal;
                        s
                    } else {
                        let mut s = ut99_bsp::ProceduralTextureSimulator::new_water(w, h);
                        s.palette = pal;
                        s
                    };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!("Step: #{}", state.procedural_simulator.step_counter))
                            .monospace()
                            .color(Color32::from_rgb(180, 200, 220)),
                    );
                });
            });

            ui.separator();

            // 2. Main Content: Left Interactive Canvas / Right Parameter Inspector
            ui.columns(2, |columns| {
                // Left Column: Interactive Simulation Canvas
                columns[0].vertical(|ui| {
                    ui.label(
                        RichText::new("Interactive Viewport (Click/Drag to Disturb):")
                            .strong()
                            .color(Color32::from_rgb(255, 215, 100)),
                    );
                    ui.add_space(4.0);

                    let w = state.procedural_simulator.width;
                    let h = state.procedural_simulator.height;
                    let rgba = state.procedural_simulator.to_rgba8();

                    let color_img = egui::ColorImage::from_rgba_unmultiplied([w, h], &rgba);
                    let tex_handle = ctx.load_texture(
                        "procedural_live_texture",
                        color_img,
                        egui::TextureOptions::NEAREST,
                    );

                    let canvas_size = egui::vec2(256.0, 256.0);
                    let (rect, resp) = ui.allocate_exact_size(canvas_size, egui::Sense::click_and_drag());

                    // Render texture image
                    ui.painter().image(
                        tex_handle.id(),
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        Color32::WHITE,
                    );

                    // Border stroke
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        Stroke::new(1.5_f32, Color32::from_rgb(80, 100, 140)),
                        egui::StrokeKind::Outside,
                    );

                    // Handle interactive click and drag
                    if resp.dragged() || resp.clicked() {
                        if let Some(pos) = resp.interact_pointer_pos() {
                            let rel_x = ((pos.x - rect.min.x) / rect.width()).clamp(0.0, 0.99);
                            let rel_y = ((pos.y - rect.min.y) / rect.height()).clamp(0.0, 0.99);
                            let px = (rel_x * (w as f32)) as usize;
                            let py = (rel_y * (h as f32)) as usize;

                            match state.procedural_simulator.tex_type {
                                ProceduralTextureType::WaterRipples => {
                                    state.procedural_simulator.add_droplet(px, py, 140.0);
                                }
                                ProceduralTextureType::Fire => {
                                    state.procedural_simulator.add_heat_spark(px, py, 240.0);
                                }
                            }
                        }
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("💡 Click or drag on the viewport above to generate ripples or sparks in real time.")
                            .size(10.5)
                            .italics()
                            .color(Color32::LIGHT_GRAY),
                    );
                });

                // Right Column: Parameters & Material Actions
                columns[1].vertical(|ui| {
                    ui.label(RichText::new("Texture Configuration:").strong().color(Color32::from_rgb(170, 200, 240)));
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.add(egui::TextEdit::singleline(&mut state.procedural_texture_name).desired_width(160.0));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Engine Mode:");
                        let is_fire = state.procedural_simulator.tex_type == ProceduralTextureType::Fire;
                        if ui.selectable_label(is_fire, "🔥 Fire").clicked() {
                            let w = state.procedural_simulator.width;
                            let h = state.procedural_simulator.height;
                            state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_fire(w, h);
                        }
                        if ui.selectable_label(!is_fire, "💧 Water Ripples").clicked() {
                            let w = state.procedural_simulator.width;
                            let h = state.procedural_simulator.height;
                            state.procedural_simulator = ut99_bsp::ProceduralTextureSimulator::new_water(w, h);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Color Palette:");
                        let mut cur_pal = state.procedural_simulator.palette;
                        egui::ComboBox::from_id_salt("procedural_palette_combo")
                            .selected_text(match cur_pal {
                                ProceduralColorPalette::FireThermal => "🔥 Fire Thermal",
                                ProceduralColorPalette::BluePlasma => "⚡ Blue Plasma",
                                ProceduralColorPalette::GreenToxic => "🧪 Green Toxic / Acid",
                                ProceduralColorPalette::PurpleVoid => "🔮 Purple Void",
                                ProceduralColorPalette::WaterAzure => "💧 Water Azure",
                                ProceduralColorPalette::IceFrost => "🧊 Ice Frost",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::FireThermal, "🔥 Fire Thermal");
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::BluePlasma, "⚡ Blue Plasma");
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::GreenToxic, "🧪 Green Toxic / Acid");
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::PurpleVoid, "🔮 Purple Void");
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::WaterAzure, "💧 Water Azure");
                                ui.selectable_value(&mut cur_pal, ProceduralColorPalette::IceFrost, "🧊 Ice Frost");
                            });
                        state.procedural_simulator.palette = cur_pal;
                    });

                    ui.add_space(6.0);

                    // Physics Sliders
                    egui::Frame::group(ui.style())
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(60, 75, 95)))
                        .fill(Color32::from_rgba_premultiplied(22, 28, 38, 220))
                        .show(ui, |ui| {
                            ui.label(RichText::new("⚙ Simulation Physics").strong().size(12.0));
                            ui.separator();

                            if state.procedural_simulator.tex_type == ProceduralTextureType::Fire {
                                ui.horizontal(|ui| {
                                    ui.label("Cooling Rate:");
                                    ui.add(egui::Slider::new(&mut state.procedural_simulator.cooling_rate, 0.2..=4.0).suffix(" uu/step"));
                                });
                            }

                            ui.horizontal(|ui| {
                                ui.label("Damping (Friction):");
                                ui.add(egui::Slider::new(&mut state.procedural_simulator.damping, 0.88..=0.999));
                            });
                        });

                    ui.add_space(10.0);

                    // Action Buttons
                    ui.vertical(|ui| {
                        if ui.button(RichText::new("★ Set as Active Material").strong().color(Color32::from_rgb(255, 215, 0)))
                            .on_hover_text("Bakes the current procedural frame into the texture manager and sets it as active for painting surfaces")
                            .clicked()
                        {
                            state.apply_procedural_texture_as_active();
                        }

                        ui.add_space(4.0);

                        if ui.button("💾 Export Animation Frames (PNG Strip)...")
                            .on_hover_text("Bakes and exports a series of animated PNG frames to a folder on disk")
                            .clicked()
                        {
                            if let Some(dir) = rfd::FileDialog::new().set_title("Select Folder to Export Frames").pick_folder() {
                                let _ = state.export_procedural_animation_strip(&dir, 32);
                            }
                        }
                    });
                });
            });

            ui.add_space(8.0);
            ui.separator();

            // 3. Bottom Action Bar
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!(
                        "FireEngine: 64x64 dynamic grid | Buffer: {}",
                        if state.procedural_simulator.current_is_a { "A" } else { "B" }
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(140, 160, 180)),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if close_requested {
        state.show_procedural_texture_dialog = false;
    } else {
        state.show_procedural_texture_dialog = is_open;
    }
}
