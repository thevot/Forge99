// crates/ut99-editor/src/gui/track_mover_studio_dialog.rs
//
// Interactive Spline Rail, Kinetic Track Mover & Rollercoaster Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{sample_track_spline, TrackArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_track_studio_dialog {
        return;
    }

    let mut open = state.show_track_studio_dialog;

    // Handle timeline animation playback if playing
    if state.track_studio_settings.is_animating {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let speed = state.track_studio_settings.active_config.default_speed;
        let approx_len = 2000.0;
        let delta_t = (speed / approx_len) * dt * 0.25;
        let mut new_t = state.track_studio_settings.timeline_t + delta_t;
        if new_t > 1.0 {
            new_t = if state.track_studio_settings.active_config.is_closed_loop {
                0.0
            } else {
                state.track_studio_settings.is_animating = false;
                1.0
            };
        }
        state.track_studio_settings.timeline_t = new_t;
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Spline Rail, Kinetic Track Mover & Rollercoaster Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_track_studio_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Track Archetypes:").strong());
        for &arch in TrackArchetype::all() {
            let is_sel = state.track_studio_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_track_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.track_studio_settings.show_rails_viewport, "Dual Rails");
        ui.checkbox(&mut state.track_studio_settings.show_ties_viewport, "Cross-Ties");
        ui.checkbox(&mut state.track_studio_settings.show_keyframes_viewport, "Mover Keyframes (0..7)");
        ui.checkbox(&mut state.track_studio_settings.show_animated_cart_viewport, "Kinetic Cart");
        ui.checkbox(&mut state.track_studio_settings.show_banking_frames_viewport, "Banking Frames");
        ui.checkbox(&mut state.track_studio_settings.show_station_waypoints_viewport, "Bot AI Waypoints");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_track_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Elevation Canvas, Timeline Scrubber & Commit Panel
        columns[0].vertical(|ui| {
            ui.heading("2D Elevation & Physical Banking Profile");
            render_profile_canvas(ui, state);

            ui.add_space(8.0);
            render_timeline_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Track to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters, Control Points & Safety Hazards
        columns[1].vertical(|ui| {
            ui.heading("Track Geometry & Kinematic Parameters");
            render_parameters_grid(ui, state);

            ui.add_space(8.0);
            ui.heading("Kinetic Safety & Centrifugal G-Force Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_profile_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width().max(200.0), 220.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)), StrokeKind::Inside);

    let cfg = &state.track_studio_settings.active_config;
    let pad = 24.0;
    let plot_w = rect.width() - pad * 2.0;
    let plot_h = rect.height() - pad * 2.0;

    let steps = 50;
    let mut elev_pts = Vec::with_capacity(steps);
    let mut bank_pts = Vec::with_capacity(steps);

    let mut min_z = f32::MAX;
    let mut max_z = f32::MIN;

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let s = sample_track_spline(cfg, t);
        min_z = min_z.min(s.position.z);
        max_z = max_z.max(s.position.z);
    }
    let span_z = (max_z - min_z).max(64.0);

    for i in 0..steps {
        let t = i as f32 / (steps - 1) as f32;
        let s = sample_track_spline(cfg, t);
        let x = rect.min.x + pad + t * plot_w;

        // Elevation (cyan line)
        let norm_z = (s.position.z - min_z) / span_z;
        let y_elev = (rect.max.y - pad) - norm_z * (plot_h * 0.65);
        elev_pts.push(Pos2::new(x, y_elev));

        // Bank angle (magenta line, plotted relative to center)
        let cy = rect.min.y + pad + plot_h * 0.82;
        let norm_bank = s.bank_angle_deg / cfg.max_bank_angle_deg.max(1.0);
        let y_bank = cy - norm_bank * 20.0;
        bank_pts.push(Pos2::new(x, y_bank));
    }

    // Draw baseline
    let base_y = rect.min.y + pad + plot_h * 0.82;
    painter.line_segment(
        [Pos2::new(rect.min.x + pad, base_y), Pos2::new(rect.max.x - pad, base_y)],
        Stroke::new(1.0_f32, Color32::from_rgb(60, 70, 85)),
    );

    // Draw elevation curve
    let elev_stroke = Stroke::new(2.5_f32, Color32::from_rgb(60, 220, 220));
    for window in elev_pts.windows(2) {
        painter.line_segment([window[0], window[1]], elev_stroke);
    }

    // Draw banking curve
    let bank_stroke = Stroke::new(1.8_f32, Color32::from_rgb(255, 100, 200));
    for window in bank_pts.windows(2) {
        painter.line_segment([window[0], window[1]], bank_stroke);
    }

    // Draw baked keyframe dots
    let kf_count = state.track_studio_settings.cached_keyframes.len();
    for (i, _kf) in state.track_studio_settings.cached_keyframes.iter().enumerate() {
        let t = i as f32 / (kf_count - 1).max(1) as f32;
        let s = sample_track_spline(cfg, t);
        let x = rect.min.x + pad + t * plot_w;
        let norm_z = (s.position.z - min_z) / span_z;
        let y_elev = (rect.max.y - pad) - norm_z * (plot_h * 0.65);

        painter.circle_filled(Pos2::new(x, y_elev), 4.5, Color32::from_rgb(255, 215, 0));
        painter.text(
            Pos2::new(x, y_elev - 12.0),
            egui::Align2::CENTER_BOTTOM,
            format!("K{}", i),
            egui::FontId::proportional(10.0),
            Color32::from_rgb(255, 220, 100),
        );
    }

    // Draw current playhead dot
    let cur_t = state.track_studio_settings.timeline_t;
    let cur_s = sample_track_spline(cfg, cur_t);
    let cur_x = rect.min.x + pad + cur_t * plot_w;
    let norm_z = (cur_s.position.z - min_z) / span_z;
    let cur_y = (rect.max.y - pad) - norm_z * (plot_h * 0.65);
    painter.circle_filled(Pos2::new(cur_x, cur_y), 6.0, Color32::from_rgb(255, 60, 60));

    // Legend
    painter.text(
        Pos2::new(rect.min.x + pad + 8.0, rect.min.y + pad + 4.0),
        egui::Align2::LEFT_TOP,
        "Cyan: Elevation Z | Magenta: Bank Angle φ",
        egui::FontId::proportional(11.0),
        Color32::from_rgb(180, 190, 205),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Track Elevation Profile").strong());
        ui.label(format!("Z Range: {:.0} to {:.0} uu (Span: {:.0} uu)", min_z, max_z, span_z));
        ui.label(format!("Keyframes: {} (UE1 Mover max 8)", kf_count));
        ui.label(format!("Current Playhead: {:.1}%", cur_t * 100.0));
    });
}

fn render_timeline_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.track_studio_settings.is_animating { "⏸ Pause" } else { "▶ Play" };
            if ui.button(anim_txt).clicked() {
                state.track_studio_settings.is_animating = !state.track_studio_settings.is_animating;
            }

            if ui.button("⏮ Reset").clicked() {
                state.track_studio_settings.timeline_t = 0.0;
            }

            ui.label("Playhead:");
            let mut t = state.track_studio_settings.timeline_t;
            if ui.add(egui::Slider::new(&mut t, 0.0..=1.0).text("")).changed() {
                state.scrub_track_timeline(t);
            }
        });

        let cur_s = sample_track_spline(&state.track_studio_settings.active_config, state.track_studio_settings.timeline_t);
        ui.horizontal(|ui| {
            ui.label(format!("Pos: ({:.0}, {:.0}, {:.0})", cur_s.position.x, cur_s.position.y, cur_s.position.z));
            ui.label(format!("Speed: {:.0} uu/s", cur_s.speed));
            ui.label(format!("Bank: {:.1}°", cur_s.bank_angle_deg));
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let rails_polys = state.track_studio_settings.cached_rails_brush
        .as_ref()
        .map(|b| b.polys.len())
        .unwrap_or(0);
    let ties_polys = state.track_studio_settings.cached_ties_brush
        .as_ref()
        .map(|b| b.polys.len())
        .unwrap_or(0);
    let kf_count = state.track_studio_settings.cached_keyframes.len();

    ui.horizontal(|ui| {
        ui.label(format!("Mesh: {} Rail Polys + {} Tie Polys | {} Mover Keyframes", rails_polys, ties_polys, kf_count));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Track, Mover & Bot Waypoints to Level").strong())
            .on_hover_text("Spawns dual rails brush, cross-ties brush, Mover actor with baked keyframes 0..7, and station LiftCenter/LiftExit AI waypoints into the level (100% Undoable)")
            .clicked()
        {
            state.commit_track_setup();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_track_preview();
        }
    });
}

fn render_parameters_grid(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("track_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Gauge Width (uu):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.gauge_width).speed(2.0).range(16.0..=160.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Rail Thickness (uu):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.rail_thickness).speed(0.5).range(2.0..=24.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Tie Spacing (uu):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.tie_spacing).speed(2.0).range(16.0..=128.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Max Bank Angle (°):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.max_bank_angle_deg).speed(1.0).range(0.0..=65.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Default Speed (uu/s):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.default_speed).speed(20.0).range(100.0..=3000.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Mover Keyframe Count (2..8):");
        if ui.add(egui::DragValue::new(&mut state.track_studio_settings.active_config.keyframe_count).speed(1).range(2..=8)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Closed Loop Track:");
        if ui.checkbox(&mut state.track_studio_settings.active_config.is_closed_loop, "Continuous Circuit").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Support Pylons:");
        if ui.checkbox(&mut state.track_studio_settings.active_config.add_support_pylons, "Generate Ground Trestle").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Rail Texture:");
        if ui.text_edit_singleline(&mut state.track_studio_settings.active_config.rail_texture).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Tie Texture:");
        if ui.text_edit_singleline(&mut state.track_studio_settings.active_config.tie_texture).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_track_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.track_studio_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} KINETIC HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Track Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% G-Force & Mover Safe").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("Lateral G-forces and track grades are within comfortable passenger limits.");
        } else {
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
