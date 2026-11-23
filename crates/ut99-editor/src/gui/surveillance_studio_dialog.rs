// crates/ut99-editor/src/gui/surveillance_studio_dialog.rs
//
// Interactive Security CCTV Camera, Live Video Monitor & Surveillance Network Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{
    sample_patrol_orientation, SurveillanceArchetype, VideoFilterMode,
};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_surveillance_dialog {
        return;
    }

    let mut open = state.show_surveillance_dialog;

    // Handle patrol animation playback if active
    if state.surveillance_settings.is_patrolling {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        let new_t = state.surveillance_settings.simulated_time_seconds + dt;
        state.surveillance_settings.simulated_time_seconds = new_t;
        state.recalculate_surveillance_preview();
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Security CCTV Camera, Live Video Monitor & Surveillance Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_surveillance_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Surveillance Archetypes:").strong());
        for &arch in SurveillanceArchetype::all() {
            let is_sel = state.surveillance_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_surveillance_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.surveillance_settings.show_camera_frustum_viewport, "Camera Frustum (FOV)");
        ui.checkbox(&mut state.surveillance_settings.show_patrol_sweep_viewport, "Patrol Sweep Arc");
        ui.checkbox(&mut state.surveillance_settings.show_sightline_target_viewport, "Sightline & Reticle");
        ui.checkbox(&mut state.surveillance_settings.show_monitor_box_viewport, "CRT Monitor Housing");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_surveillance_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: Simulated CRT Video Screen & Patrol Timeline
        columns[0].vertical(|ui| {
            ui.heading("Simulated CRT / CCTV Live Video Monitor Feed");
            render_simulated_screen_canvas(ui, state);

            ui.add_space(8.0);
            render_patrol_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Surveillance Setup to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Camera Mount, Patrol Kinematics, Display & Hazards
        columns[1].vertical(|ui| {
            ui.heading("Camera Mount & Patrol Kinematics");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Sightline Occlusion & Lens Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_simulated_screen_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let cfg = &state.surveillance_settings.active_config;
    let aspect_w = cfg.monitor.aspect_ratio_w.max(1) as f32;
    let aspect_h = cfg.monitor.aspect_ratio_h.max(1) as f32;
    let aspect = aspect_w / aspect_h;

    let avail_w = ui.available_width().max(200.0);
    let target_h = (avail_w / aspect).min(260.0);
    let target_w = target_h * aspect;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Filter-specific base background
    let bg_color = match cfg.monitor.filter_mode {
        VideoFilterMode::CRTScanlinesMonochrome => Color32::from_rgb(22, 25, 30),
        VideoFilterMode::GreenNightVision => Color32::from_rgb(10, 32, 14),
        VideoFilterMode::ThermalInfrared => Color32::from_rgb(35, 12, 45),
        VideoFilterMode::ColorCCTVNoise => Color32::from_rgb(18, 22, 28),
        VideoFilterMode::HighDefClean => Color32::from_rgb(16, 20, 26),
    };

    painter.rect_filled(rect, 6.0, bg_color);
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(2.5_f32, Color32::from_rgb(60, 70, 85)),
        StrokeKind::Inside,
    );

    // Draw CRT scanlines
    let scanline_step = 4.0;
    let scan_count = (rect.height() / scanline_step) as usize;
    let scan_alpha = (cfg.monitor.scanline_intensity * 80.0) as u8;
    let scan_color = match cfg.monitor.filter_mode {
        VideoFilterMode::GreenNightVision => Color32::from_rgba_unmultiplied(60, 255, 80, scan_alpha),
        _ => Color32::from_rgba_unmultiplied(0, 0, 0, scan_alpha),
    };

    for i in 0..scan_count {
        let y = rect.min.y + i as f32 * scanline_step;
        painter.line_segment(
            [Pos2::new(rect.min.x + 2.0, y), Pos2::new(rect.max.x - 2.0, y)],
            Stroke::new(1.0_f32, scan_color),
        );
    }

    // Draw Center Crosshair Reticle
    let cx = rect.center().x;
    let cy = rect.center().y;
    let reticle_col = match cfg.monitor.filter_mode {
        VideoFilterMode::GreenNightVision => Color32::from_rgb(100, 255, 120),
        VideoFilterMode::ThermalInfrared => Color32::from_rgb(255, 180, 50),
        _ => Color32::from_rgb(220, 230, 245),
    };

    let cross_len = 16.0;
    let gap = 5.0;
    painter.line_segment([Pos2::new(cx - cross_len, cy), Pos2::new(cx - gap, cy)], Stroke::new(1.5_f32, reticle_col));
    painter.line_segment([Pos2::new(cx + gap, cy), Pos2::new(cx + cross_len, cy)], Stroke::new(1.5_f32, reticle_col));
    painter.line_segment([Pos2::new(cx, cy - cross_len), Pos2::new(cx, cy - gap)], Stroke::new(1.5_f32, reticle_col));
    painter.line_segment([Pos2::new(cx, cy + gap), Pos2::new(cx, cy + cross_len)], Stroke::new(1.5_f32, reticle_col));
    painter.circle_stroke(Pos2::new(cx, cy), 12.0, Stroke::new(1.0_f32, reticle_col));

    // Top-Left: Camera Tag / OSD Name
    painter.text(
        Pos2::new(rect.min.x + 12.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        &cfg.monitor.osd_camera_name,
        egui::FontId::monospace(11.0),
        reticle_col,
    );

    // Top-Right: [REC] Indicator with pulse
    if cfg.monitor.show_rec_indicator {
        let t = state.surveillance_settings.simulated_time_seconds;
        let is_blink = (t * 2.0).fract() > 0.3;
        if is_blink {
            painter.circle_filled(Pos2::new(rect.max.x - 52.0, rect.min.y + 18.0), 4.5, Color32::from_rgb(255, 40, 40));
            painter.text(
                Pos2::new(rect.max.x - 42.0, rect.min.y + 12.0),
                egui::Align2::LEFT_TOP,
                "REC",
                egui::FontId::monospace(11.0),
                Color32::from_rgb(255, 60, 60),
            );
        }
    }

    // Bottom-Left: Live Security Timestamp
    if cfg.monitor.show_timestamp_osd {
        let t = state.surveillance_settings.simulated_time_seconds;
        let total_sec = (t as u32) % 86400;
        let hours = total_sec / 3600;
        let mins = (total_sec % 3600) / 60;
        let secs = total_sec % 60;
        let time_str = format!("2026-10-15 {:02}:{:02}:{:02}", hours, mins, secs);

        painter.text(
            Pos2::new(rect.min.x + 12.0, rect.max.y - 14.0),
            egui::Align2::LEFT_BOTTOM,
            time_str,
            egui::FontId::monospace(10.0),
            reticle_col,
        );
    }

    // Bottom-Right: Pan / Tilt / FOV Telemetry
    let (cur_yaw, cur_pitch) = sample_patrol_orientation(
        &cfg.patrol,
        cfg.camera_mount.base_yaw_deg,
        cfg.camera_mount.base_pitch_deg,
        state.surveillance_settings.simulated_time_seconds,
    );
    let telem_str = format!("PAN:{:.1}° TLT:{:.1}° FOV:{:.0}°", cur_yaw, cur_pitch, cfg.camera_mount.fov_deg);
    painter.text(
        Pos2::new(rect.max.x - 12.0, rect.max.y - 14.0),
        egui::Align2::RIGHT_BOTTOM,
        telem_str,
        egui::FontId::monospace(10.0),
        reticle_col,
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("CCTV Security Monitor Feed").strong());
        ui.label(format!("Tag: {}", cfg.camera_tag));
        ui.label(format!("Screen Event: {}", cfg.screen_event));
        ui.label(format!("Filter: {}", cfg.monitor.filter_mode.display_name()));
        ui.label(format!("Aspect: {}:{}", cfg.monitor.aspect_ratio_w, cfg.monitor.aspect_ratio_h));
    });
}

fn render_patrol_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.surveillance_settings.is_patrolling { "⏸ Pause" } else { "▶ Patrol" };
            if ui.button(anim_txt).clicked() {
                state.surveillance_settings.is_patrolling = !state.surveillance_settings.is_patrolling;
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_surveillance_time(0.0);
            }

            ui.label("Patrol Time:");
            let mut t = state.surveillance_settings.simulated_time_seconds;
            let period = state.surveillance_settings.active_config.patrol.sweep_period_seconds.max(1.0);
            if ui.add(egui::Slider::new(&mut t, 0.0..=(period * 2.0)).text("s")).changed() {
                state.scrub_surveillance_time(t);
            }
        });

        let (cur_yaw, cur_pitch) = sample_patrol_orientation(
            &state.surveillance_settings.active_config.patrol,
            state.surveillance_settings.active_config.camera_mount.base_yaw_deg,
            state.surveillance_settings.active_config.camera_mount.base_pitch_deg,
            state.surveillance_settings.simulated_time_seconds,
        );

        ui.horizontal(|ui| {
            ui.label(format!("Current Pan (Yaw): {:.1}°", cur_yaw));
            ui.label(format!("Current Tilt (Pitch): {:.1}°", cur_pitch));
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.surveillance_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Binding: Camera '{}' ➜ Screen '{}'", cfg.camera_tag, cfg.screen_event));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Camera, Screen Actor & Monitor Brush to World").strong())
            .on_hover_text("Spawns an Engine.Camera actor, Screen.Screen scripted texture actor, and a 3D CRT Monitor box brush into the level (100% Undoable)")
            .clicked()
        {
            state.commit_surveillance_setup_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_surveillance_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("surveillance_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Camera Position
        ui.label("Camera Pos X:");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.camera_mount.position.x).speed(16.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Camera Pos Y:");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.camera_mount.position.y).speed(16.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Camera Pos Z:");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.camera_mount.position.z).speed(16.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Optics
        ui.label("Field of View FOV (°):");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.camera_mount.fov_deg).speed(1.0).range(30.0..=120.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Max Sight Range (uu):");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.camera_mount.max_range).speed(64.0).range(256.0..=4096.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Patrol Kinematics
        ui.label("Pan Sweep Angle (±°):");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.patrol.pan_sweep_deg).speed(1.0).range(0.0..=90.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Tilt Sweep Angle (±°):");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.patrol.tilt_sweep_deg).speed(1.0).range(0.0..=45.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Sweep Period (seconds):");
        if ui.add(egui::DragValue::new(&mut state.surveillance_settings.active_config.patrol.sweep_period_seconds).speed(0.5).range(1.0..=30.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("360° Continuous Spin:");
        if ui.checkbox(&mut state.surveillance_settings.active_config.patrol.is_continuous_spin, "Drone / Radar Mode").changed() {
            changed = true;
        }
        ui.end_row();

        // Filter Mode
        ui.label("Video Filter Effect:");
        egui::ComboBox::from_id_salt("video_filter_combo")
            .selected_text(state.surveillance_settings.active_config.monitor.filter_mode.display_name())
            .show_ui(ui, |ui| {
                for &mode in VideoFilterMode::all() {
                    let is_sel = state.surveillance_settings.active_config.monitor.filter_mode == mode;
                    if ui.selectable_label(is_sel, mode.display_name()).clicked() {
                        state.surveillance_settings.active_config.monitor.filter_mode = mode;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Scanlines
        ui.label("Scanline Intensity:");
        if ui.add(egui::Slider::new(&mut state.surveillance_settings.active_config.monitor.scanline_intensity, 0.0..=1.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Tags
        ui.label("Camera Tag:");
        if ui.text_edit_singleline(&mut state.surveillance_settings.active_config.camera_tag).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Screen Event:");
        if ui.text_edit_singleline(&mut state.surveillance_settings.active_config.screen_event).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_surveillance_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.surveillance_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CAMERA SIGHTLINE HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Surveillance Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Sightline & Feed Valid").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("Camera sightline is clear of solid geometry and event tags are properly bound.");
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
