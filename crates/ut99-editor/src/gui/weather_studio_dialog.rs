// crates/ut99-editor/src/gui/weather_studio_dialog.rs
//
// Interactive Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{PrecipitationType, WeatherArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_weather_dialog {
        return;
    }

    let mut open = state.show_weather_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Dynamic Weather & Lightning Storm Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_weather_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Dynamic physics / lightning simulation tick
    if state.weather_settings.is_animating {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        state.step_weather_simulation(dt);
        ui.ctx().request_repaint();
    }

    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Weather Presets:").strong());
        let archetypes = [
            WeatherArchetype::FrigateOceanicStorm,
            WeatherArchetype::ChizraMonsoonDeluge,
            WeatherArchetype::MorpheusBlizzardDrift,
            WeatherArchetype::NaPaliToxicAcidRain,
            WeatherArchetype::PhobosMeteorAshFallout,
        ];
        for arch in archetypes {
            let is_sel = state.weather_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_weather_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.weather_settings.show_weather_volume_viewport, "Volume Bounds");
        ui.checkbox(&mut state.weather_settings.show_lightning_viewport, "Lightning Bolts");
        ui.checkbox(&mut state.weather_settings.show_rain_streaks_viewport, "Precipitation Streaks");
        ui.separator();
        if ui.button("🔄 Recalculate Preview").clicked() {
            state.recalculate_weather_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Simulation Canvas & Controls
        columns[0].vertical(|ui| {
            ui.heading("2D Atmospheric Storm & Precipitation Simulation");
            render_simulation_canvas(ui, state);

            ui.add_space(8.0);
            render_timeline_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Weather Volume to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Physical Kinetics, Lightning, Acoustics & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Atmospheric Kinetics & Precipitation Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Atmospheric Health & Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_simulation_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.weather_settings.active_config.clone();
    let metrics = state.weather_settings.cached_metrics.clone();
    let time = state.weather_settings.simulated_time_seconds;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (response, painter) = ui.allocate_painter(Vec2::new(target_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background color: darkened stormy sky modulated by lightning flash
    let base_sky = if metrics.is_flashing_now {
        let flash = metrics.flash_brightness;
        Color32::from_rgb(
            (25.0 + flash * 180.0 * cfg.lightning_color_rgb[0]) as u8,
            (30.0 + flash * 190.0 * cfg.lightning_color_rgb[1]) as u8,
            (45.0 + flash * 210.0 * cfg.lightning_color_rgb[2]) as u8,
        )
    } else {
        Color32::from_rgb(
            (cfg.fog_color_rgb[0] * 80.0) as u8 + 10,
            (cfg.fog_color_rgb[1] * 80.0) as u8 + 12,
            (cfg.fog_color_rgb[2] * 80.0) as u8 + 18,
        )
    };

    painter.rect_filled(rect, 4.0, base_sky);
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 80, 110)), StrokeKind::Middle);

    // Weather Volume boundary box projection
    let cx = rect.center().x;
    let cy = rect.center().y;
    let scale_factor = (target_w * 0.75) / cfg.volume_extents.x.max(512.0);

    let half_w_px = (cfg.volume_extents.x * 0.5) * scale_factor;
    let half_h_px = (cfg.volume_extents.z * 0.5) * scale_factor;
    let vol_rect = Rect::from_center_size(Pos2::new(cx, cy), Vec2::new(half_w_px * 2.0, half_h_px * 2.0));

    if state.weather_settings.show_weather_volume_viewport {
        painter.rect_stroke(
            vol_rect,
            2.0,
            Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(80, 160, 240, 140)),
            StrokeKind::Middle,
        );
    }

    // Ground plane line
    let ground_y = vol_rect.max.y;
    painter.line_segment(
        [Pos2::new(rect.min.x + 8.0, ground_y), Pos2::new(rect.max.x - 8.0, ground_y)],
        Stroke::new(2.0_f32, Color32::from_rgb(90, 85, 80)),
    );

    // Particle streaks
    if state.weather_settings.show_rain_streaks_viewport {
        let p_color = cfg.precipitation_type.default_particle_color();
        let stroke_c = Color32::from_rgba_unmultiplied(
            (p_color[0] * 255.0) as u8,
            (p_color[1] * 255.0) as u8,
            (p_color[2] * 255.0) as u8,
            180,
        );

        let streak_count = metrics.total_simulated_particles.min(80);
        let wind_shift_x = (cfg.wind_vector.x * 0.05) * scale_factor;
        let streak_len = (cfg.fall_speed_uu_s * 0.03).clamp(4.0, 24.0);

        for i in 0..streak_count {
            let hash_x = ((i as f32 * 17.345 + time * 0.1).sin() * 0.5 + 0.5) * (vol_rect.width() - 10.0);
            let drop_y_norm = (i as f32 * 23.456 + time * (cfg.fall_speed_uu_s * 0.0015)).fract();
            let drop_x = vol_rect.min.x + 5.0 + hash_x;
            let drop_y = vol_rect.min.y + drop_y_norm * vol_rect.height();

            let p0 = Pos2::new(drop_x, drop_y);
            let p1 = Pos2::new(drop_x - wind_shift_x, drop_y - streak_len);
            painter.line_segment([p0, p1], Stroke::new(1.2_f32, stroke_c));
        }
    }

    // Lightning Bolt
    if metrics.is_flashing_now && state.weather_settings.show_lightning_viewport {
        let bolt_points = &state.weather_settings.cached_lightning_branch;
        if bolt_points.len() >= 2 {
            let mut screen_pts = Vec::with_capacity(bolt_points.len());
            for pt in bolt_points {
                let px = cx + (pt.x - cfg.position.x) * scale_factor;
                let py = cy - (pt.z - cfg.position.z) * scale_factor;
                screen_pts.push(Pos2::new(px, py));
            }

            let flash_c = Color32::from_rgba_unmultiplied(
                (cfg.lightning_color_rgb[0] * 255.0) as u8,
                (cfg.lightning_color_rgb[1] * 255.0) as u8,
                (cfg.lightning_color_rgb[2] * 255.0) as u8,
                (metrics.flash_brightness * 255.0) as u8,
            );

            for i in 0..screen_pts.len() - 1 {
                painter.line_segment([screen_pts[i], screen_pts[i + 1]], Stroke::new(2.5_f32, flash_c));
            }
        }
    }

    // Telemetry text overlay
    let info = format!(
        "Mode: {} | Fall: {:.0} uu/s | Wind: {:.0} uu/s | Delay: {:.2}s",
        cfg.precipitation_type.display_name(),
        cfg.fall_speed_uu_s,
        metrics.net_wind_speed_uu_s,
        metrics.thunder_delay_at_listener_secs
    );
    painter.text(
        Pos2::new(rect.min.x + 10.0, rect.max.y - 18.0),
        egui::Align2::LEFT_BOTTOM,
        info,
        egui::FontId::proportional(12.0),
        Color32::from_rgb(220, 230, 240),
    );
}

fn render_timeline_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        let play_btn_text = if state.weather_settings.is_animating { "⏸ Pause" } else { "▶ Play" };
        if ui.button(play_btn_text).clicked() {
            state.weather_settings.is_animating = !state.weather_settings.is_animating;
        }

        if ui.button("⏹ Reset").clicked() {
            state.weather_settings.simulated_time_seconds = 0.0;
            state.scrub_weather_time(0.0);
        }

        ui.label("Time (s):");
        let mut t = state.weather_settings.simulated_time_seconds;
        if ui.add(egui::Slider::new(&mut t, 0.0..=30.0).text("s")).changed() {
            state.scrub_weather_time(t);
        }

        if state.weather_settings.cached_metrics.is_flashing_now {
            ui.label(egui::RichText::new("⚡ LIGHTNING FLASH").color(Color32::YELLOW).strong());
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut cfg = state.weather_settings.active_config.clone();
    let mut changed = false;

    ui.group(|ui| {
        ui.label(egui::RichText::new("Precipitation Kinetics:").strong());

        egui::ComboBox::from_label("Type")
            .selected_text(cfg.precipitation_type.display_name())
            .show_ui(ui, |ui| {
                let types = [
                    PrecipitationType::TorrentialRain,
                    PrecipitationType::GentleSnowfall,
                    PrecipitationType::AcidHailstorm,
                    PrecipitationType::AshVolcanicFallout,
                    PrecipitationType::SandstormBlizzard,
                ];
                for t in types {
                    if ui.selectable_label(cfg.precipitation_type == t, t.display_name()).clicked() {
                        cfg.precipitation_type = t;
                        cfg.fall_speed_uu_s = t.default_fall_speed();
                        changed = true;
                    }
                }
            });

        if ui.add(egui::Slider::new(&mut cfg.fall_speed_uu_s, 50.0..=2500.0).text("Terminal Velocity (uu/s)")).changed() {
            changed = true;
        }

        if ui.add(egui::Slider::new(&mut cfg.particle_density, 10.0..=500.0).text("Particle Density")).changed() {
            changed = true;
        }

        ui.horizontal(|ui| {
            ui.label("Wind XYZ:");
            if ui.add(egui::DragValue::new(&mut cfg.wind_vector.x).prefix("X: ").speed(10.0)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.wind_vector.y).prefix("Y: ").speed(10.0)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.wind_vector.z).prefix("Z: ").speed(10.0)).changed() { changed = true; }
        });

        if ui.add(egui::Slider::new(&mut cfg.turbulence, 0.0..=1.0).text("Wind Turbulence")).changed() {
            changed = true;
        }
    });

    ui.group(|ui| {
        ui.label(egui::RichText::new("Lightning & Acoustics:").strong());

        if ui.checkbox(&mut cfg.lightning_enabled, "Lightning Strikes Enabled").changed() {
            changed = true;
        }

        if cfg.lightning_enabled {
            if ui.add(egui::Slider::new(&mut cfg.lightning_interval_secs, 1.0..=30.0).text("Strike Interval (s)")).changed() {
                changed = true;
            }
            if ui.add(egui::Slider::new(&mut cfg.lightning_flash_radius, 500.0..=4000.0).text("Flash Light Radius (uu)")).changed() {
                changed = true;
            }
        }

        ui.horizontal(|ui| {
            ui.label("Fog Density:");
            if ui.add(egui::DragValue::new(&mut cfg.fog_density).speed(0.0001).range(0.0001..=0.01)).changed() {
                changed = true;
            }
        });

        ui.horizontal(|ui| {
            ui.label("Volume Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() {
                changed = true;
            }
        });
    });

    if changed {
        state.weather_settings.active_config = cfg;
        state.recalculate_weather_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.weather_settings.cached_hazards;

    if hazards.is_empty() {
        ui.colored_label(Color32::from_rgb(80, 220, 100), "✔ 100% Atmospheric Integrity Certified - Zero Hazards Found.");
        return;
    }

    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for h in hazards {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if h.is_critical {
                        ui.colored_label(Color32::from_rgb(255, 90, 90), "CRITICAL");
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 180, 50), "WARNING");
                    }
                    ui.label(egui::RichText::new(&h.title).strong());
                });
                ui.label(&h.description);
            });
        }
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let arch_name = state.weather_settings.active_config.archetype.display_name();
    let prec_name = state.weather_settings.active_config.precipitation_type.display_name();

    ui.horizontal(|ui| {
        if ui.button(egui::RichText::new("🌧 Commit Weather Setup to World").strong()).clicked() {
            state.commit_weather_to_world();
        }

        ui.label(format!("Archetype: {} ({})", arch_name, prec_name));
    });
}
