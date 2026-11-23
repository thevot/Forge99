// crates/ut99-editor/src/gui/soundscape_studio_dialog.rs
//
// Interactive Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{AcousticReverbPreset, AudioFalloffCurve, SoundscapeArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_soundscape_dialog {
        return;
    }

    let mut open = state.show_soundscape_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_soundscape_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Soundscape Archetypes:").strong());
        for &arch in SoundscapeArchetype::all() {
            let is_sel = state.soundscape_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_soundscape_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.soundscape_settings.show_emitter_viewport, "Emitter Icon");
        ui.checkbox(&mut state.soundscape_settings.show_inner_radius_viewport, "Inner 100% Shell");
        ui.checkbox(&mut state.soundscape_settings.show_outer_radius_viewport, "Outer Threshold Shell");
        ui.checkbox(&mut state.soundscape_settings.show_soundwaves_viewport, "Acoustic Waves");
        ui.checkbox(&mut state.soundscape_settings.show_zone_bounds_viewport, "Acoustic Room Volume");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_soundscape_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D SPL Decay Curve, Listener Scrubber, Commit
        columns[0].vertical(|ui| {
            ui.heading("2D Spatial Attenuation & Decibel Falloff Curve");
            render_attenuation_canvas(ui, state);

            ui.add_space(8.0);
            render_listener_probe_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Soundscape & Reverb Zone to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Emitter Params, EAX Reverb Zone, Diagnostics
        columns[1].vertical(|ui| {
            ui.heading("Ambient Emitter & EAX Reverb Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Acoustic Safety & Intelligibility Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_attenuation_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let falloff_curve = state.soundscape_settings.active_config.emitter.falloff_curve;
    let reverb_name = state.soundscape_settings.active_config.acoustics.reverb_preset.display_name();
    let metrics = state.soundscape_settings.cached_metrics.clone();
    let probe = state.soundscape_settings.cached_probe_sample.clone();

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 24));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(45, 58, 75)),
        StrokeKind::Inside,
    );

    let graph_margin_left = 55.0;
    let graph_margin_right = 25.0;
    let graph_margin_top = 35.0;
    let graph_margin_bottom = 40.0;

    let g_left = rect.min.x + graph_margin_left;
    let g_right = rect.max.x - graph_margin_right;
    let g_top = rect.min.y + graph_margin_top;
    let g_bottom = rect.max.y - graph_margin_bottom;
    let g_width = g_right - g_left;
    let g_height = g_bottom - g_top;

    let max_dist = (metrics.outer_radius_uu * 1.25).max(1000.0);

    // Grid lines (horizontal for volume %, vertical for distance)
    let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(60, 80, 110, 80));
    for v in &[0.0, 0.25, 0.50, 0.75, 1.0] {
        let y = g_bottom - v * g_height;
        painter.line_segment([Pos2::new(g_left, y), Pos2::new(g_right, y)], grid_stroke);
        painter.text(
            Pos2::new(g_left - 8.0, y),
            egui::Align2::RIGHT_CENTER,
            format!("{:.0}%", v * 100.0),
            egui::FontId::monospace(9.0),
            Color32::from_rgb(160, 185, 215),
        );
    }

    let to_canvas_pt = |dist: f32, vol: f32| -> Pos2 {
        let frac_x = (dist / max_dist).clamp(0.0, 1.0);
        let frac_y = vol.clamp(0.0, 1.0);
        Pos2::new(g_left + frac_x * g_width, g_bottom - frac_y * g_height)
    };

    // Shaded Inner Radius Zone (100% Volume plateau)
    let p_inner_top = to_canvas_pt(metrics.inner_radius_uu, metrics.volume_fraction);
    let p_origin_top = to_canvas_pt(0.0, metrics.volume_fraction);
    let p_origin_bot = to_canvas_pt(0.0, 0.0);
    let p_inner_bot = to_canvas_pt(metrics.inner_radius_uu, 0.0);

    painter.add(egui::Shape::convex_polygon(
        vec![p_origin_bot, p_origin_top, p_inner_top, p_inner_bot],
        Color32::from_rgba_unmultiplied(240, 200, 60, 35),
        Stroke::NONE,
    ));

    // Falloff Curve (step-by-step evaluation)
    let steps = 40;
    let mut curve_pts = Vec::new();
    curve_pts.push(p_origin_top);
    curve_pts.push(p_inner_top);

    for s in 0..=steps {
        let frac = s as f32 / steps as f32;
        let d = metrics.inner_radius_uu + frac * (metrics.outer_radius_uu - metrics.inner_radius_uu);
        let t = (d - metrics.inner_radius_uu) / (metrics.outer_radius_uu - metrics.inner_radius_uu).max(1.0);
        let v = match falloff_curve {
            AudioFalloffCurve::Linear => metrics.volume_fraction * (1.0 - t),
            AudioFalloffCurve::InverseSquare => {
                let r_norm = (d / metrics.inner_radius_uu.max(1.0)).max(1.0);
                (metrics.volume_fraction / (r_norm * r_norm)).clamp(0.0, 1.0)
            }
            AudioFalloffCurve::LogarithmicDecibel => {
                let db_loss = t * 40.0;
                metrics.volume_fraction * 10.0_f32.powf(-db_loss / 20.0)
            }
        };
        curve_pts.push(to_canvas_pt(d, v));
    }
    curve_pts.push(to_canvas_pt(max_dist, 0.0));

    // Draw curve
    for i in 0..(curve_pts.len() - 1) {
        painter.line_segment([curve_pts[i], curve_pts[i + 1]], Stroke::new(2.5_f32, Color32::from_rgb(80, 220, 255)));
    }

    // Outer threshold line (dashed cyan)
    let p_outer_top = to_canvas_pt(metrics.outer_radius_uu, 1.0);
    let p_outer_bot = to_canvas_pt(metrics.outer_radius_uu, 0.0);
    painter.line_segment([p_outer_top, p_outer_bot], Stroke::new(1.5_f32, Color32::from_rgb(60, 180, 240)));
    painter.text(Pos2::new(p_outer_bot.x, p_outer_bot.y + 12.0), egui::Align2::CENTER_TOP, format!("R_out: {:.0} uu", metrics.outer_radius_uu), egui::FontId::monospace(9.0), Color32::from_rgb(80, 200, 255));

    // Inner radius boundary line (dashed gold)
    painter.line_segment([p_inner_top, p_inner_bot], Stroke::new(1.5_f32, Color32::from_rgb(240, 200, 60)));
    painter.text(Pos2::new(p_inner_bot.x, p_inner_bot.y + 12.0), egui::Align2::CENTER_TOP, format!("R_in: {:.0} uu", metrics.inner_radius_uu), egui::FontId::monospace(9.0), Color32::from_rgb(240, 200, 60));

    // Listener Probe Indicator (Green Stalk + Handle)
    let probe_dist = state.soundscape_settings.probe_listener_distance_uu;
    let p_probe_top = to_canvas_pt(probe_dist, probe.raw_volume_fraction);
    let p_probe_bot = to_canvas_pt(probe_dist, 0.0);

    painter.line_segment([p_probe_top, p_probe_bot], Stroke::new(2.0_f32, Color32::from_rgb(40, 240, 120)));
    painter.circle_filled(p_probe_top, 6.0, Color32::from_rgb(40, 240, 120));
    painter.circle_stroke(p_probe_top, 8.0, Stroke::new(1.5_f32, Color32::WHITE));

    // Handle Drag to move Listener Probe
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let rel_x = ((pos.x - g_left) / g_width).clamp(0.0, 1.0);
            let new_dist = rel_x * max_dist;
            state.scrub_soundscape_probe(new_dist);
        }
    }

    // Telemetry OSD
    let osd = format!("Listener: {:.0} uu | Volume: {:.0}% | SPL: {:.1} dB | Low-Pass Cutoff: {:.0} Hz | Reverb: {}",
        probe_dist, probe.raw_volume_fraction * 100.0, probe.effective_spl_db, probe.low_pass_cutoff_hz, reverb_name);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        osd,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(215, 235, 255),
    );
}

fn render_listener_probe_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("👂 Virtual Listener Probe:").strong());
            let max_r = (state.soundscape_settings.cached_metrics.outer_radius_uu * 1.25).max(1000.0);
            let mut d = state.soundscape_settings.probe_listener_distance_uu;
            if ui.add(egui::Slider::new(&mut d, 0.0..=max_r).text("Distance (uu)")).changed() {
                state.scrub_soundscape_probe(d);
            }
            if ui.button("⏮ Reset (Pivot)").clicked() {
                state.scrub_soundscape_probe(0.0);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.soundscape_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Sound: {}.{} | Radius: {} ({} uu) | EAX: {}",
            cfg.emitter.sound_package, cfg.emitter.sound_name, cfg.emitter.sound_radius,
            cfg.emitter.sound_radius as usize * 40, cfg.acoustics.reverb_preset.display_name()));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Soundscape, Zone Brush & AmbientSound Actor to Level").strong())
            .on_hover_text("Spawns semi-solid room volume, Engine.AmbientSound actor and Engine.ZoneInfo actor (100% Undoable)")
            .clicked()
        {
            state.commit_soundscape_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_soundscape_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("soundscape_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Sound Package
        ui.label("Sound Package:");
        if ui.text_edit_singleline(&mut state.soundscape_settings.active_config.emitter.sound_package).changed() {
            changed = true;
        }
        ui.end_row();

        // Sound Name
        ui.label("Sound Name:");
        if ui.text_edit_singleline(&mut state.soundscape_settings.active_config.emitter.sound_name).changed() {
            changed = true;
        }
        ui.end_row();

        // SoundRadius (byte, 1 = 40 uu)
        ui.label("Sound Radius:");
        ui.horizontal(|ui| {
            let mut r = state.soundscape_settings.active_config.emitter.sound_radius;
            if ui.add(egui::Slider::new(&mut r, 1..=128).text("units")).changed() {
                state.soundscape_settings.active_config.emitter.sound_radius = r;
                changed = true;
            }
            ui.label(format!("({} uu)", r as usize * 40));
        });
        ui.end_row();

        // SoundVolume (0..255)
        ui.label("Sound Volume:");
        ui.horizontal(|ui| {
            let mut v = state.soundscape_settings.active_config.emitter.sound_volume;
            if ui.add(egui::Slider::new(&mut v, 0..=255).text("")).changed() {
                state.soundscape_settings.active_config.emitter.sound_volume = v;
                changed = true;
            }
            ui.label(format!("{:.0}%", v as f32 / 2.55));
        });
        ui.end_row();

        // SoundPitch (32..128, 64 = 1.0x)
        ui.label("Sound Pitch:");
        ui.horizontal(|ui| {
            let mut p = state.soundscape_settings.active_config.emitter.sound_pitch;
            if ui.add(egui::Slider::new(&mut p, 32..=128).text("")).changed() {
                state.soundscape_settings.active_config.emitter.sound_pitch = p;
                changed = true;
            }
            ui.label(format!("{:.2}x", p as f32 / 64.0));
        });
        ui.end_row();

        // Falloff Curve
        ui.label("Falloff Curve Model:");
        egui::ComboBox::from_id_salt("sound_falloff_combo")
            .selected_text(state.soundscape_settings.active_config.emitter.falloff_curve.display_name())
            .show_ui(ui, |ui| {
                for &f in AudioFalloffCurve::all() {
                    let is_sel = state.soundscape_settings.active_config.emitter.falloff_curve == f;
                    if ui.selectable_label(is_sel, f.display_name()).clicked() {
                        state.soundscape_settings.active_config.emitter.falloff_curve = f;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Reverb Preset
        ui.label("EAX Environmental Reverb:");
        egui::ComboBox::from_id_salt("sound_reverb_combo")
            .selected_text(state.soundscape_settings.active_config.acoustics.reverb_preset.display_name())
            .show_ui(ui, |ui| {
                for &p in AcousticReverbPreset::all() {
                    let is_sel = state.soundscape_settings.active_config.acoustics.reverb_preset == p;
                    if ui.selectable_label(is_sel, p.display_name()).clicked() {
                        state.soundscape_settings.active_config.acoustics.reverb_preset = p;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Reverb Decay Time (s)
        ui.label("Reverb Decay Time:");
        if ui.add(egui::Slider::new(&mut state.soundscape_settings.active_config.acoustics.reverb_decay_time_secs, 0.1..=8.0).text("s")).changed() {
            changed = true;
        }
        ui.end_row();

        // Solid Wall Occlusion
        ui.label("Solid BSP Wall Occlusion:");
        if ui.checkbox(&mut state.soundscape_settings.active_config.emitter.enable_wall_occlusion, "Enable -12 dB & Muffling").changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_soundscape_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.soundscape_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL AUDIO HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Acoustic Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Spatial Audio & Reverb Setup Valid").strong());
            }
        });

        let metrics = &state.soundscape_settings.cached_metrics;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("SPL at 1m: {:.1} dB | Inner Radius: {:.0} uu | Outer Radius: {:.0} uu",
                metrics.spl_decibels_at_1m, metrics.inner_radius_uu, metrics.outer_radius_uu));
        });

        if !hazards.is_empty() {
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                for hazard in hazards {
                    ui.horizontal(|ui| {
                        let badge_col = if hazard.is_critical {
                            Color32::from_rgb(255, 60, 60)
                        } else {
                            Color32::from_rgb(240, 180, 50)
                        };
                        let badge_txt = if hazard.is_critical { "CRITICAL" } else { "WARN" };
                        ui.colored_label(badge_col, format!("[{}]", badge_txt));
                        ui.label(egui::RichText::new(&hazard.title).strong());
                    });
                    ui.label(&hazard.description);
                    ui.colored_label(Color32::from_rgb(140, 200, 255), format!("💡 {}", hazard.recommendation));
                    ui.separator();
                }
            });
        }
    });
}
