use crate::editor_state::EditorState;
use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{
    evaluate_dynamic_light, sample_light_waveform, DynamicLightPreset,
    LightEffect, LightType,
};

/// Interactive Dynamic Light FX & Animating Strobe/Spotlight Studio
pub fn render_light_effects_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_light_effects_studio_dialog {
        return;
    }

    let mut is_open = state.show_light_effects_studio_dialog;

    Window::new("Dynamic Light FX & Animating Strobe/Spotlight Studio")
        .open(&mut is_open)
        .default_width(860.0)
        .default_height(600.0)
        .min_width(700.0)
        .min_height(480.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_light_studio_content(ui, state);
        });

    state.show_light_effects_studio_dialog = is_open;
}

fn render_light_studio_content(ui: &mut Ui, state: &mut EditorState) {
    // Header Bar
    ui.horizontal(|ui| {
        ui.heading("💡 Dynamic Light FX & Spotlight Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📍 Place Animated Light at Pivot").clicked() {
                let preset = state.light_effects_settings.selected_preset;
                state.place_dynamic_light_at_pivot(preset);
            }
            if ui.button("⚡ Apply to Selected Lights").clicked() {
                let p = &state.light_effects_settings.preview_light;
                state.update_selected_light_effects(
                    p.light_type,
                    p.light_effect,
                    p.period,
                    p.phase,
                    p.cone,
                    p.brightness,
                    p.hue,
                    p.saturation,
                    p.radius,
                );
            }
        });
    });

    ui.separator();

    // Presets Bar
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Presets:").strong());
        for &preset in DynamicLightPreset::all() {
            let is_sel = state.light_effects_settings.selected_preset == preset;
            if ui.selectable_label(is_sel, preset.display_name()).clicked() {
                state.light_effects_settings.selected_preset = preset;
                preset.apply_to_light(&mut state.light_effects_settings.preview_light);
                state.light_effects_settings.waveform_samples = sample_light_waveform(
                    &state.light_effects_settings.preview_light,
                    2.0,
                    120,
                );
            }
        }
    });

    // Preset Description Badge
    ui.add_space(2.0);
    ui.group(|ui| {
        ui.label(
            RichText::new(state.light_effects_settings.selected_preset.description())
                .size(11.0)
                .italics()
                .color(Color32::from_rgb(180, 220, 255)),
        );
    });

    ui.add_space(4.0);

    // Transport & Playback Controls
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    if state.light_effects_settings.is_animating {
        let speed = state.light_effects_settings.anim_speed;
        state.light_effects_settings.current_time_sec += dt * speed;
        if state.light_effects_settings.current_time_sec > 4.0 {
            state.light_effects_settings.current_time_sec -= 4.0;
        }
    }

    let cur_time = state.light_effects_settings.current_time_sec;

    ui.group(|ui| {
        ui.horizontal(|ui| {
            let play_label = if state.light_effects_settings.is_animating {
                "⏸ Pause"
            } else {
                "▶ Play"
            };
            if ui.button(play_label).clicked() {
                state.light_effects_settings.is_animating = !state.light_effects_settings.is_animating;
            }
            if ui.button("⏪ Reset Time").clicked() {
                state.light_effects_settings.current_time_sec = 0.0;
            }

            ui.separator();

            ui.label(format!("{:.2}s", cur_time));
            if ui.add(
                egui::Slider::new(&mut state.light_effects_settings.current_time_sec, 0.0..=4.0)
                    .show_value(false)
                    .text("Time (s)"),
            ).changed() {
                state.light_effects_settings.is_animating = false;
            }

            ui.separator();

            // Speed selectors
            let cur_speed = state.light_effects_settings.anim_speed;
            if ui.selectable_label((cur_speed - 0.5).abs() < 1e-2, "0.5x").clicked() {
                state.light_effects_settings.anim_speed = 0.5;
            }
            if ui.selectable_label((cur_speed - 1.0).abs() < 1e-2, "1.0x").clicked() {
                state.light_effects_settings.anim_speed = 1.0;
            }
            if ui.selectable_label((cur_speed - 2.0).abs() < 1e-2, "2.0x").clicked() {
                state.light_effects_settings.anim_speed = 2.0;
            }

            ui.separator();

            ui.checkbox(
                &mut state.light_effects_settings.show_animated_beams_viewport,
                "3D Beams",
            );
            ui.checkbox(
                &mut state.light_effects_settings.show_spotlight_cones_viewport,
                "Spotlight Cones",
            );
        });
    });

    ui.add_space(4.0);

    // Dynamic State at Current Time
    let dyn_state = evaluate_dynamic_light(&state.light_effects_settings.preview_light, cur_time);

    // Waveform Graph Canvas
    ui.label(RichText::new("📊 Live Intensity Waveform I(t)").strong());
    render_waveform_canvas(ui, &state.light_effects_settings.waveform_samples, cur_time, &dyn_state);

    ui.add_space(6.0);
    ui.separator();

    // Interactive Tuning Panel
    egui::ScrollArea::vertical().max_height(220.0).show(ui, |ui| {
        ui.columns(2, |cols| {
            // Column 1: Types & Waveforms
            cols[0].group(|ui| {
                ui.label(RichText::new("Waveform & Animation Parameters").strong());

                // LightType ComboBox
                let cur_type = LightType::from(state.light_effects_settings.preview_light.light_type);
                egui::ComboBox::from_label("LightType (Style)")
                    .selected_text(format!("{:?}", cur_type))
                    .show_ui(ui, |ui| {
                        let types = [
                            LightType::Steady,
                            LightType::Pulse,
                            LightType::Blink,
                            LightType::Flicker,
                            LightType::Strobe,
                            LightType::SubtlePulse,
                        ];
                        for &lt in &types {
                            if ui.selectable_label(cur_type == lt, format!("{:?}", lt)).clicked() {
                                state.light_effects_settings.preview_light.light_type = lt as u8;
                                state.light_effects_settings.waveform_samples = sample_light_waveform(
                                    &state.light_effects_settings.preview_light,
                                    2.0,
                                    120,
                                );
                            }
                        }
                    });

                // LightEffect ComboBox
                let cur_eff = LightEffect::from(state.light_effects_settings.preview_light.light_effect);
                egui::ComboBox::from_label("LightEffect (Spatial/Color)")
                    .selected_text(format!("{:?}", cur_eff))
                    .show_ui(ui, |ui| {
                        let effects = [
                            LightEffect::None,
                            LightEffect::TorchWaver,
                            LightEffect::FireWaver,
                            LightEffect::WateryShimmer,
                            LightEffect::Searchlight,
                            LightEffect::SlowWave,
                            LightEffect::FastWave,
                            LightEffect::CloudCast,
                            LightEffect::StaticSpot,
                            LightEffect::Shock,
                            LightEffect::Disco,
                            LightEffect::Cylinder,
                        ];
                        for &le in &effects {
                            if ui.selectable_label(cur_eff == le, format!("{:?}", le)).clicked() {
                                state.light_effects_settings.preview_light.light_effect = le as u8;
                                state.light_effects_settings.waveform_samples = sample_light_waveform(
                                    &state.light_effects_settings.preview_light,
                                    2.0,
                                    120,
                                );
                            }
                        }
                    });

                let period_sec = state.light_effects_settings.preview_light.period as f32 / 32.0;
                if ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.period, 1..=255)
                        .text(format!("Period ({:.2}s/cycle)", period_sec)),
                ).changed() {
                    state.light_effects_settings.waveform_samples = sample_light_waveform(
                        &state.light_effects_settings.preview_light,
                        2.0,
                        120,
                    );
                }

                if ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.phase, 0..=255)
                        .text("Phase Shift"),
                ).changed() {
                    state.light_effects_settings.waveform_samples = sample_light_waveform(
                        &state.light_effects_settings.preview_light,
                        2.0,
                        120,
                    );
                }

                let cone_deg = (state.light_effects_settings.preview_light.cone as f32 / 255.0) * 80.0;
                ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.cone, 1..=255)
                        .text(format!("Cone ({:.1}°)", cone_deg)),
                );
            });

            // Column 2: Color, Brightness & Telemetry
            cols[1].group(|ui| {
                ui.label(RichText::new("Color, Attenuation & Telemetry").strong());

                ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.brightness, 0..=255)
                        .text("Base Brightness"),
                );

                ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.hue, 0..=255)
                        .text("Hue (0=Red, 85=Green, 170=Blue)"),
                );

                ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.saturation, 0..=255)
                        .text("Saturation (0=Vivid, 255=White)"),
                );

                let world_r = state.light_effects_settings.preview_light.radius as f32 * 25.0;
                ui.add(
                    egui::Slider::new(&mut state.light_effects_settings.preview_light.radius, 1..=255)
                        .text(format!("Radius ({:.0} uu)", world_r)),
                );

                ui.separator();

                // Live dynamic output
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Live State:").strong());
                    ui.label(format!("I = {:.2}x", dyn_state.intensity_multiplier));
                    ui.label(format!("Radius = {:.0} uu", dyn_state.current_radius_uu));

                    // Color swatch
                    let rgb_col = Color32::from_rgb(
                        (dyn_state.effective_rgb.x.clamp(0.0, 1.0) * 255.0) as u8,
                        (dyn_state.effective_rgb.y.clamp(0.0, 1.0) * 255.0) as u8,
                        (dyn_state.effective_rgb.z.clamp(0.0, 1.0) * 255.0) as u8,
                    );
                    let (rect, _) = ui.allocate_exact_size(vec2(24.0, 16.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 2.0, rgb_col);
                    ui.painter().rect_stroke(rect, 2.0, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Outside);
                });

                if dyn_state.is_directional {
                    ui.label(format!(
                        "Spot Beam: ({:.2}, {:.2}, {:.2}) | Cone: {:.1}°",
                        dyn_state.spot_direction.x,
                        dyn_state.spot_direction.y,
                        dyn_state.spot_direction.z,
                        dyn_state.spot_cone_angle_rad.to_degrees()
                    ));
                }
            });
        });
    });
}

fn render_waveform_canvas(
    ui: &mut Ui,
    samples: &[(f32, f32)],
    current_time: f32,
    dyn_state: &ut99_bsp::DynamicLightState,
) {
    let desired_size = vec2(ui.available_width(), 90.0);
    let (rect, _) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(15, 18, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 60, 75)), StrokeKind::Outside);

    // Grid lines (horizontal at 0.5x, 1.0x, 1.5x)
    for mult in [0.5, 1.0, 1.5] {
        let norm_y = 1.0 - (mult / 2.0_f32).clamp(0.0, 1.0);
        let y = rect.min.y + norm_y * rect.height();
        painter.line_segment(
            [pos2(rect.min.x, y), pos2(rect.max.x, y)],
            Stroke::new(0.5_f32, Color32::from_rgb(35, 45, 55)),
        );
    }

    if samples.len() < 2 {
        return;
    }

    let duration = samples.last().map(|s| s.0).unwrap_or(2.0);

    // Waveform line
    let mut points = Vec::with_capacity(samples.len());
    for &(t, val) in samples {
        let norm_x = (t / duration).clamp(0.0, 1.0);
        let norm_y = 1.0 - (val / 2.0_f32).clamp(0.0, 1.0);

        let x = rect.min.x + norm_x * rect.width();
        let y = rect.min.y + norm_y * rect.height();
        points.push(pos2(x, y));
    }

    let wave_color = Color32::from_rgb(
        (dyn_state.effective_rgb.x.clamp(0.2, 1.0) * 255.0) as u8,
        (dyn_state.effective_rgb.y.clamp(0.2, 1.0) * 255.0) as u8,
        (dyn_state.effective_rgb.z.clamp(0.2, 1.0) * 255.0) as u8,
    );

    for pair in points.windows(2) {
        painter.line_segment([pair[0], pair[1]], Stroke::new(2.0_f32, wave_color));
    }

    // Playhead vertical line
    let cycle_time = current_time % duration;
    let norm_playhead = (cycle_time / duration).clamp(0.0, 1.0);
    let playhead_x = rect.min.x + norm_playhead * rect.width();

    painter.line_segment(
        [pos2(playhead_x, rect.min.y), pos2(playhead_x, rect.max.y)],
        Stroke::new(2.0_f32, Color32::WHITE),
    );

    // Intensity dot on playhead
    let current_norm_y = 1.0 - (dyn_state.intensity_multiplier / 2.0_f32).clamp(0.0, 1.0);
    let dot_y = rect.min.y + current_norm_y * rect.height();
    painter.circle_filled(pos2(playhead_x, dot_y), 4.0, Color32::YELLOW);
}
