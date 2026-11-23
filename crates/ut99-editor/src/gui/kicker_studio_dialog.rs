// crates/ut99-editor/src/gui/kicker_studio_dialog.rs
//
// Interactive Jump Pad, Aerial Kicker Trajectory & Low-Gravity Aerodynamics Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{KickerArchetype, KickerHazardSeverity};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_kicker_studio_dialog {
        return;
    }

    let mut open = state.show_kicker_studio_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_kicker_studio_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Presets:").strong());
        for &arch in KickerArchetype::all() {
            let is_sel = state.kicker_studio_settings.selected_archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_kicker_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Main Two-Column Layout (Canvas + Controls)
    ui.columns(2, |columns| {
        // Left Column: 2D Flight Profile Canvas & Timeline
        columns[0].vertical(|ui| {
            ui.heading("2D Aerial Flight Profile (Distance vs Height)");
            render_flight_canvas(ui, state);

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(if state.kicker_studio_settings.is_animating { "⏸ Pause" } else { "▶ Play" }).clicked() {
                    state.kicker_studio_settings.is_animating = !state.kicker_studio_settings.is_animating;
                }
                let total_t = state.kicker_studio_settings.trajectory_analysis.total_flight_time_secs.max(0.1);
                ui.add(egui::Slider::new(
                    &mut state.kicker_studio_settings.flight_scrubber_secs,
                    0.0..=total_t,
                ).text("Flight Time (s)"));
            });

            // Animate flight scrubber
            if state.kicker_studio_settings.is_animating {
                let dt = ui.input(|i| i.stable_dt).min(0.1);
                state.kicker_studio_settings.flight_scrubber_secs += dt;
                let total_t = state.kicker_studio_settings.trajectory_analysis.total_flight_time_secs.max(0.1);
                if state.kicker_studio_settings.flight_scrubber_secs > total_t {
                    state.kicker_studio_settings.flight_scrubber_secs = 0.0;
                }
                ui.ctx().request_repaint();
            }

            ui.add_space(8.0);
            render_trajectory_metrics_summary(ui, state);
        });

        // Right Column: Parameters, Inverse Solver & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Kicker Setup & Ballistic Inversion");

            let mut cfg_changed = false;

            egui::Grid::new("kicker_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut state.kicker_studio_settings.active_config.name);
                ui.end_row();

                ui.label("Actor Class:");
                let cur_class = &mut state.kicker_studio_settings.active_config.actor_class;
                egui::ComboBox::from_id_salt("kicker_actor_class")
                    .selected_text(cur_class.as_str())
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(cur_class, "UT_JumpPad".to_string(), "UT_JumpPad (Tournament Pad)").clicked() {
                            cfg_changed = true;
                        }
                        if ui.selectable_value(cur_class, "Kicker".to_string(), "Kicker (Unreal Kicker)").clicked() {
                            cfg_changed = true;
                        }
                    });
                ui.end_row();

                ui.label("Gravity Z (uu/s²):");
                if ui.add(egui::DragValue::new(&mut state.kicker_studio_settings.active_config.gravity_z).speed(10.0).range(-2000.0..=-50.0)).changed() {
                    cfg_changed = true;
                }
                ui.end_row();

                ui.label("Ceiling Height Z:");
                if ui.add(egui::DragValue::new(&mut state.kicker_studio_settings.active_config.ceiling_height_z).speed(16.0)).changed() {
                    cfg_changed = true;
                }
                ui.end_row();

                ui.label("Air Control (0..1):");
                if ui.add(egui::DragValue::new(&mut state.kicker_studio_settings.active_config.air_control).speed(0.01).range(0.0..=1.0)).changed() {
                    cfg_changed = true;
                }
                ui.end_row();

                ui.label("Kick Velocity (uu/s):");
                ui.horizontal(|ui| {
                    let v = &mut state.kicker_studio_settings.active_config.kick_velocity;
                    ui.label("X");
                    if ui.add(egui::DragValue::new(&mut v.x).speed(10.0)).changed() { cfg_changed = true; }
                    ui.label("Y");
                    if ui.add(egui::DragValue::new(&mut v.y).speed(10.0)).changed() { cfg_changed = true; }
                    ui.label("Z");
                    if ui.add(egui::DragValue::new(&mut v.z).speed(10.0)).changed() { cfg_changed = true; }
                });
                ui.end_row();

                ui.label("Target Apex Clearance:");
                ui.add(egui::DragValue::new(&mut state.kicker_studio_settings.target_apex_clearance_z).speed(10.0).range(32.0..=1000.0));
                ui.end_row();
            });

            ui.add_space(8.0);
            if ui.button("⚡ Auto-Solve Kick Velocity for Target Platform").on_hover_text("Analytically computes exact KickVelocity vector to cleanly land on target").clicked() {
                state.solve_kicker_velocity_to_target();
            }

            ui.checkbox(&mut state.kicker_studio_settings.active_config.auto_place_jump_spot, "Auto-Place Companion JumpSpot (Bot AI)");

            ui.add_space(8.0);
            if ui.button("🚀 Generate JumpPad / Kicker Setup & JumpSpot").clicked() {
                state.generate_kicker_setup();
            }
            if ui
                .button("🤖 Spawn Native UT99 Bot Triad (Kicker + LiftExit + JumpSpot)")
                .on_hover_text(
                    "Spawns a 100% UT99 v436/v469 compatible Kicker + LiftExit + JumpSpot bot navigation chain with solved ballistic velocity (BL-P3-04)",
                )
                .clicked()
            {
                let cfg = state.kicker_studio_settings.active_config.clone();
                let flight_time = state
                    .kicker_studio_settings
                    .trajectory_analysis
                    .total_flight_time_secs
                    .max(0.8);
                let _ = state.spawn_ut99_kicker_bot_chain(
                    cfg.pad_location,
                    cfg.target_location,
                    flight_time,
                );
                state.show_trajectories = true;
                state.show_event_links = true;
            }

            if cfg_changed {
                state.recalculate_kicker_trajectory();
            }

            ui.separator();
            render_hazards_table(ui, state);
        });
    });
}

fn render_flight_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width(), 280.0);
    let (response, painter) = ui.allocate_painter(desired_size, egui::Sense::hover());
    let rect = response.rect;

    // Background dark field
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 30));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 60, 80)), StrokeKind::Inside);

    let analysis = &state.kicker_studio_settings.trajectory_analysis;
    if analysis.samples.is_empty() {
        return;
    }

    let origin = state.kicker_studio_settings.active_config.pad_location;
    let target = state.kicker_studio_settings.active_config.target_location;
    let ceiling_z = state.kicker_studio_settings.active_config.ceiling_height_z;

    // Bounds in (dist, z) space
    let max_dist = analysis.horizontal_distance_uu.max(target.truncate().distance(origin.truncate())).max(400.0) * 1.15;
    let min_z = origin.z.min(target.z).min(analysis.landing_position.z) - 100.0;
    let max_z = analysis.apex_position.z.max(ceiling_z).max(target.z + 100.0) + 150.0;
    let range_z = (max_z - min_z).max(200.0);

    let to_canvas = |dist: f32, z: f32| -> Pos2 {
        let u = (dist / max_dist).clamp(0.0, 1.0);
        let v = 1.0 - ((z - min_z) / range_z).clamp(0.0, 1.0);
        Pos2::new(
            rect.min.x + 35.0 + u * (rect.width() - 70.0),
            rect.min.y + 25.0 + v * (rect.height() - 50.0),
        )
    };

    // 1. Grid & reference baselines
    let p_ground_start = to_canvas(0.0, origin.z);
    let p_ground_end = to_canvas(max_dist, origin.z);
    painter.line_segment([p_ground_start, p_ground_end], Stroke::new(1.0_f32, Color32::from_rgb(40, 55, 75)));

    // 2. Ceiling obstacle line
    let p_ceil_start = to_canvas(0.0, ceiling_z);
    let p_ceil_end = to_canvas(max_dist, ceiling_z);
    let ceil_col = if analysis.hit_ceiling { Color32::from_rgb(255, 60, 60) } else { Color32::from_rgb(120, 80, 80) };
    painter.line_segment([p_ceil_start, p_ceil_end], Stroke::new(1.5_f32, ceil_col));
    painter.text(p_ceil_start + Vec2::new(4.0, -10.0), egui::Align2::LEFT_BOTTOM, "Ceiling Clearance", egui::FontId::monospace(10.0), ceil_col);

    // 3. Launch Pad Marker
    let p_pad = to_canvas(0.0, origin.z);
    painter.rect_filled(Rect::from_center_size(p_pad, Vec2::new(28.0, 10.0)), 2.0, Color32::from_rgb(60, 220, 100));
    painter.text(p_pad + Vec2::new(0.0, 14.0), egui::Align2::CENTER_TOP, "JumpPad", egui::FontId::proportional(11.0), Color32::from_rgb(60, 220, 100));

    // 4. Target Landing Platform Marker
    let target_dist = target.truncate().distance(origin.truncate());
    let p_target = to_canvas(target_dist, target.z);
    painter.rect_filled(Rect::from_center_size(p_target, Vec2::new(32.0, 10.0)), 2.0, Color32::from_rgb(60, 160, 255));
    painter.circle_stroke(p_target, 8.0, Stroke::new(1.5_f32, Color32::from_rgb(80, 200, 255)));
    painter.text(p_target + Vec2::new(0.0, 14.0), egui::Align2::CENTER_TOP, "Target JumpSpot", egui::FontId::proportional(11.0), Color32::from_rgb(80, 160, 255));

    // 5. Parabolic Trajectory Ribbon
    for i in 0..analysis.samples.len() - 1 {
        let s1 = &analysis.samples[i];
        let s2 = &analysis.samples[i + 1];

        let d1 = s1.position.truncate().distance(origin.truncate());
        let d2 = s2.position.truncate().distance(origin.truncate());

        let p1 = to_canvas(d1, s1.position.z);
        let p2 = to_canvas(d2, s2.position.z);

        // Color transitions: launch green -> apex gold -> landing cyan
        let arc_factor = (s1.time_secs / analysis.total_flight_time_secs).clamp(0.0, 1.0);
        let seg_col = if arc_factor < 0.5 {
            Color32::from_rgb(
                (60.0 + arc_factor * 2.0 * 180.0) as u8,
                (220.0 + arc_factor * 2.0 * 20.0) as u8,
                80,
            )
        } else {
            Color32::from_rgb(
                (240.0 - (arc_factor - 0.5) * 2.0 * 180.0) as u8,
                (240.0 - (arc_factor - 0.5) * 2.0 * 60.0) as u8,
                (80.0 + (arc_factor - 0.5) * 2.0 * 175.0) as u8,
            )
        };

        painter.line_segment([p1, p2], Stroke::new(2.5_f32, seg_col));
    }

    // 6. Apex Badge
    let apex_dist = analysis.apex_position.truncate().distance(origin.truncate());
    let p_apex = to_canvas(apex_dist, analysis.apex_position.z);
    painter.circle_filled(p_apex, 5.0, Color32::from_rgb(255, 215, 0));
    painter.text(
        p_apex + Vec2::new(0.0, -12.0),
        egui::Align2::CENTER_BOTTOM,
        format!("Apex: Z={:.0} ({:.2}s)", analysis.apex_position.z, analysis.apex_time_secs),
        egui::FontId::proportional(11.0),
        Color32::from_rgb(255, 215, 0),
    );

    // 7. Ceiling Collision Warning Star
    if analysis.hit_ceiling {
        if let Some(ceil_pt) = analysis.ceiling_impact_point {
            let d_c = ceil_pt.truncate().distance(origin.truncate());
            let p_c = to_canvas(d_c, ceil_pt.z);
            painter.circle_filled(p_c, 8.0, Color32::from_rgb(255, 40, 40));
            painter.text(p_c + Vec2::new(0.0, -14.0), egui::Align2::CENTER_BOTTOM, "💥 CEILING IMPACT", egui::FontId::proportional(11.0), Color32::from_rgb(255, 80, 80));
        }
    }

    // 8. Animated Flight Scrubber Player Avatar
    let scrub_t = state.kicker_studio_settings.flight_scrubber_secs;
    let mut scrub_pos = analysis.samples[0].position;
    for i in 0..analysis.samples.len() - 1 {
        if scrub_t >= analysis.samples[i].time_secs && scrub_t <= analysis.samples[i + 1].time_secs {
            let seg_dt = analysis.samples[i + 1].time_secs - analysis.samples[i].time_secs;
            let alpha = if seg_dt > 0.0001 { (scrub_t - analysis.samples[i].time_secs) / seg_dt } else { 0.0 };
            scrub_pos = analysis.samples[i].position.lerp(analysis.samples[i + 1].position, alpha);
            break;
        }
    }
    let p_avatar = to_canvas(scrub_pos.truncate().distance(origin.truncate()), scrub_pos.z);
    painter.circle_filled(p_avatar, 7.0, Color32::from_rgb(255, 255, 255));
    painter.circle_stroke(p_avatar, 9.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 180, 40)));
}

fn render_trajectory_metrics_summary(ui: &mut egui::Ui, state: &EditorState) {
    let a = &state.kicker_studio_settings.trajectory_analysis;

    ui.horizontal_wrapped(|ui| {
        ui.label(egui::RichText::new("⏱ Flight Time:").strong());
        ui.label(format!("{:.2} s", a.total_flight_time_secs));

        ui.label(egui::RichText::new("🏔 Apex Z:").strong());
        ui.label(format!("{:.0} uu", a.apex_position.z));

        ui.label(egui::RichText::new("🎯 Miss:").strong());
        let miss_col = if a.target_miss_distance_uu < 50.0 { Color32::from_rgb(80, 220, 100) } else { Color32::from_rgb(240, 160, 60) };
        ui.colored_label(miss_col, format!("{:.1} uu", a.target_miss_distance_uu));

        ui.label(egui::RichText::new("💥 Fall Dmg:").strong());
        let dmg_col = if a.estimated_fall_damage < 1.0 {
            Color32::from_rgb(80, 220, 100)
        } else if a.estimated_fall_damage < 50.0 {
            Color32::from_rgb(240, 160, 60)
        } else {
            Color32::from_rgb(255, 50, 50)
        };
        ui.colored_label(dmg_col, format!("{:.0} HP", a.estimated_fall_damage));
    });
}

fn render_hazards_table(ui: &mut egui::Ui, state: &EditorState) {
    ui.label(egui::RichText::new("Trajectory Safety Audit:").strong());

    if state.kicker_studio_settings.cached_hazards.is_empty() {
        ui.colored_label(Color32::from_rgb(60, 220, 100), "✔ Safe Ballistic Trajectory: Zero hazards detected.");
        return;
    }

    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for h in &state.kicker_studio_settings.cached_hazards {
            let (badge_text, col) = match h.severity {
                KickerHazardSeverity::Critical => ("CRITICAL", Color32::from_rgb(255, 60, 60)),
                KickerHazardSeverity::Warning => ("WARNING", Color32::from_rgb(255, 180, 40)),
                KickerHazardSeverity::Info => ("INFO", Color32::from_rgb(80, 180, 255)),
            };

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(col, badge_text);
                    ui.label(egui::RichText::new(&h.title).strong());
                });
                ui.label(&h.description);
                ui.label(egui::RichText::new(format!("💡 Fix: {}", h.suggested_fix)).italics().color(Color32::LIGHT_GRAY));
            });
        }
    });
}
