use crate::editor_state::EditorState;
use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{calculate_corona_occlusion, CoronaPreset, FlareShape};

/// Interactive Sun, Corona & Anamorphic Lens Flare Studio
pub fn render_corona_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_corona_studio_dialog {
        return;
    }

    let mut is_open = state.show_corona_studio_dialog;

    Window::new("Sun, Corona & Lens Flare Studio")
        .open(&mut is_open)
        .default_width(880.0)
        .default_height(640.0)
        .min_width(720.0)
        .min_height(500.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_corona_studio_content(ui, state);
        });

    state.show_corona_studio_dialog = is_open;
}

fn render_corona_studio_content(ui: &mut Ui, state: &mut EditorState) {
    // Header Bar with Quick Action Buttons
    ui.horizontal(|ui| {
        ui.heading("☀ Sun, Corona & Lens Flare Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📍 Place Corona Light at Pivot").clicked() {
                let preset = state.corona_studio_settings.selected_preset;
                state.place_corona_light_at_pivot(preset);
            }

            if ui.button("⚡ Apply Preset to Selected Lights").clicked() {
                let preset = state.corona_studio_settings.selected_preset;
                state.apply_corona_preset_to_selected(preset);
            }
        });
    });

    ui.label(
        RichText::new("Configure glowing billboard coronas (bCorona), multi-element lens flare chains, and raycast occlusion testing.")
            .size(11.0)
            .color(Color32::from_rgb(170, 170, 180)),
    );

    ui.separator();

    // 1. Presets Selector Bar
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Atmosphere Presets:").strong());
        for &preset in CoronaPreset::all() {
            let is_sel = state.corona_studio_settings.selected_preset == preset;
            let btn_text = match preset {
                CoronaPreset::BlazingDesertSun => "☀ Desert Sun (Face)",
                CoronaPreset::CyberpunkSodiumStreetlamp => "💡 Sodium Streetlamp",
                CoronaPreset::SecurityBeaconSiren => "🚨 Security Siren",
                CoronaPreset::HighVoltagePlasmaArc => "⚡ Plasma Arc (Deck16)",
                CoronaPreset::AlienBioluminescentOrb => "🔮 Bioluminescent Orb",
                CoronaPreset::AnamorphicSciFiStarburst => "🌟 Anamorphic Starburst",
            };

            let resp = ui.selectable_label(is_sel, btn_text);
            if resp.clicked() {
                state.corona_studio_settings.selected_preset = preset;
                state.corona_studio_settings.preview_config = preset.to_config();
            }
        }
    });

    ui.add_space(4.0);

    let preset = state.corona_studio_settings.selected_preset;
    let config = state.corona_studio_settings.preview_config.clone();

    // 2. Interactive Real-Time Optical Canvas
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Optical Viewport Lens Flare Preview").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut state.corona_studio_settings.simulate_camera_orbit, "Simulate Orbiting Camera");
            });
        });

        if state.corona_studio_settings.simulate_camera_orbit {
            state.corona_studio_settings.orbit_angle_deg += 0.8;
            if state.corona_studio_settings.orbit_angle_deg >= 360.0 {
                state.corona_studio_settings.orbit_angle_deg -= 360.0;
            }
            ui.ctx().request_repaint();
        }

        ui.horizontal(|ui| {
            ui.label("Orbit Angle:");
            ui.add(egui::Slider::new(&mut state.corona_studio_settings.orbit_angle_deg, 0.0..=360.0).suffix("°"));
        });

        // Canvas Area
        let (canvas_rect, _response) = ui.allocate_exact_size(vec2(ui.available_width(), 170.0), egui::Sense::drag());
        let painter = ui.painter_at(canvas_rect);

        // Dark celestial viewport background
        painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(10, 12, 18));
        painter.rect_stroke(
            canvas_rect,
            4.0,
            Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 70)),
            StrokeKind::Outside,
        );

        let screen_center = canvas_rect.center();

        // Calculate source pos on canvas based on orbit angle or mouse drag
        let orbit_rad = state.corona_studio_settings.orbit_angle_deg.to_radians();
        let max_radius_x = canvas_rect.width() * 0.40;
        let max_radius_y = canvas_rect.height() * 0.35;
        let source_pos = pos2(
            screen_center.x + orbit_rad.cos() * max_radius_x,
            screen_center.y + orbit_rad.sin() * max_radius_y,
        );

        // Flare chain vector: from source through center
        let flare_dir = screen_center - source_pos;
        let elements = preset.generate_flare_chain();

        let base_rgb = config.color_rgb;

        // Draw Flare Elements along axis
        for elem in &elements {
            let elem_pos = source_pos + flare_dir * elem.distance_factor;
            let alpha = ((elem.color_rgba[3] * config.intensity).clamp(0.0, 1.0) * 255.0) as u8;
            let elem_col = Color32::from_rgba_unmultiplied(
                (elem.color_rgba[0] * 255.0) as u8,
                (elem.color_rgba[1] * 255.0) as u8,
                (elem.color_rgba[2] * 255.0) as u8,
                alpha,
            );

            let r = elem.size_scale * config.draw_scale * 16.0;

            match elem.shape {
                FlareShape::CircularGlow => {
                    painter.circle_filled(elem_pos, r * 1.5, Color32::from_rgba_unmultiplied(elem_col.r(), elem_col.g(), elem_col.b(), alpha / 3));
                    painter.circle_filled(elem_pos, r * 0.7, elem_col);
                }
                FlareShape::HexagonGhost => {
                    // Draw 6-sided polygon ghost
                    let mut pts = Vec::with_capacity(6);
                    for k in 0..6 {
                        let a = (k as f32) * std::f32::consts::PI / 3.0;
                        pts.push(pos2(elem_pos.x + a.cos() * r, elem_pos.y + a.sin() * r));
                    }
                    painter.add(egui::Shape::convex_polygon(pts, elem_col, Stroke::new(1.0_f32, Color32::from_white_alpha(alpha))));
                }
                FlareShape::AnamorphicStreak => {
                    // Wide horizontal streak
                    let streak_w = canvas_rect.width() * 0.8;
                    let p_left = pos2(elem_pos.x - streak_w * 0.5, elem_pos.y);
                    let p_right = pos2(elem_pos.x + streak_w * 0.5, elem_pos.y);
                    painter.line_segment([p_left, p_right], Stroke::new(3.0_f32, elem_col));
                    painter.line_segment([pos2(elem_pos.x - streak_w * 0.25, elem_pos.y), pos2(elem_pos.x + streak_w * 0.25, elem_pos.y)], Stroke::new(5.0_f32, Color32::WHITE));
                }
                FlareShape::RingHalo => {
                    painter.circle_stroke(elem_pos, r * 1.8, Stroke::new(2.0_f32, elem_col));
                }
                FlareShape::ChromaSpike => {
                    for spike in 0..4 {
                        let a = (spike as f32) * std::f32::consts::PI / 4.0;
                        let d = r * 2.5;
                        painter.line_segment(
                            [pos2(elem_pos.x - a.cos() * d, elem_pos.y - a.sin() * d),
                             pos2(elem_pos.x + a.cos() * d, elem_pos.y + a.sin() * d)],
                            Stroke::new(1.5_f32, elem_col),
                        );
                    }
                }
            }
        }

        // Primary Core Glow
        painter.circle_filled(source_pos, 14.0 * config.draw_scale, Color32::from_rgba_unmultiplied(255, 255, 255, 240));
        painter.circle_stroke(source_pos, 16.0 * config.draw_scale, Stroke::new(2.0_f32, Color32::from_rgb((base_rgb.x * 255.0) as u8, (base_rgb.y * 255.0) as u8, (base_rgb.z * 255.0) as u8)));
    });

    ui.add_space(8.0);

    let mut do_update_coronas = false;
    let mut enabled = state.corona_studio_settings.preview_config.is_enabled;
    let mut sun_mode = state.corona_studio_settings.preview_config.use_sun_mode;
    let mut tex_name = state.corona_studio_settings.preview_config.texture_name.clone();
    let mut draw_scale = state.corona_studio_settings.preview_config.draw_scale;
    let mut intensity = state.corona_studio_settings.preview_config.intensity;

    // 3. Corona Parameters & Canonical UT99 Properties
    ui.columns(2, |cols| {
        // Left Column: Corona Light Parameters
        cols[0].group(|ui| {
            ui.heading("⚙ Corona Actor Properties");
            ui.label(
                RichText::new("Authoritative Unreal Engine 1 corona properties on Engine.Light.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            ui.checkbox(&mut enabled, "bCorona (Enable Visible Corona)");
            ui.checkbox(&mut sun_mode, "bSunlight (Celestial Directional Sun)");

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Corona Texture:");
                ui.text_edit_singleline(&mut tex_name);
            });

            ui.horizontal(|ui| {
                ui.label("DrawScale:");
                ui.add(egui::Slider::new(&mut draw_scale, 0.2..=5.0).step_by(0.1));
            });

            ui.horizontal(|ui| {
                ui.label("Intensity:");
                ui.add(egui::Slider::new(&mut intensity, 0.1..=3.0).step_by(0.1));
            });

            ui.add_space(6.0);
            if ui.button("⚡ Update Selected Light Coronas").clicked() {
                do_update_coronas = true;
            }
        });

        // Right Column: Raycast Occlusion Diagnostic & Testing
        cols[1].group(|ui| {
            ui.heading("🔬 BSP Line-of-Sight Occlusion");
            ui.label(
                RichText::new("Tests whether solid world brushes block corona visibility from editor camera.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            let camera_pos = state.camera.position;
            let pivot_pos = state.builder_brush.location;

            let occlusion = calculate_corona_occlusion(
                pivot_pos,
                camera_pos,
                &state.world_brushes,
                config.draw_scale * 32.0,
            );

            ui.horizontal(|ui| {
                ui.label("Visibility Ratio:");
                let (col, verdict) = if occlusion >= 0.99 {
                    (Color32::from_rgb(80, 240, 120), "100% Unobstructed (Full Glare)")
                } else if occlusion > 0.01 {
                    (Color32::from_rgb(255, 200, 80), "Partial Penumbra Edge Occlusion")
                } else {
                    (Color32::from_rgb(255, 90, 90), "0% Fully Occluded Behind Brush")
                };
                ui.colored_label(col, RichText::new(verdict).strong());
            });

            ui.horizontal(|ui| {
                ui.label("Ray Length:");
                ui.label(format!("{:.1} uu", camera_pos.distance(pivot_pos)));
            });

            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("Active Preset: {}", preset.display_name()))
                    .size(11.0)
                    .color(Color32::from_rgb(140, 210, 255)),
            );
            ui.label(
                RichText::new(preset.description())
                    .size(10.5)
                    .italics()
                    .color(Color32::from_rgb(180, 180, 190)),
            );
        });
    });

    state.corona_studio_settings.preview_config.is_enabled = enabled;
    state.corona_studio_settings.preview_config.use_sun_mode = sun_mode;
    state.corona_studio_settings.preview_config.texture_name = tex_name.clone();
    state.corona_studio_settings.preview_config.draw_scale = draw_scale;
    state.corona_studio_settings.preview_config.intensity = intensity;

    if do_update_coronas {
        state.update_selected_light_corona(enabled, &tex_name, draw_scale);
    }
}
