// crates/ut99-editor/src/gui/hologram_studio_dialog.rs
//
// Interactive Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{HoloDisplayShape, HologramArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_hologram_dialog {
        return;
    }

    let mut open = state.show_hologram_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Holographic Signage & Sci-Fi Billboard Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_hologram_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Optical animation tick
    if state.hologram_settings.is_animating {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        state.hologram_settings.simulated_time_seconds =
            (state.hologram_settings.simulated_time_seconds + dt) % 60.0;
        state.hologram_settings.cached_metrics = ut99_bsp::evaluate_hologram_kinematics(
            &state.hologram_settings.active_config,
            state.hologram_settings.simulated_time_seconds,
        );
        ui.ctx().request_repaint();
    }

    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Hologram Presets:").strong());
        for &arch in HologramArchetype::all() {
            let is_sel = state.hologram_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_hologram_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.hologram_settings.show_display_viewport, "Display Surface");
        ui.checkbox(&mut state.hologram_settings.show_pedestal_viewport, "Emitter Pedestal");
        ui.checkbox(&mut state.hologram_settings.show_projection_beam_viewport, "Projection Frustum");
        ui.checkbox(&mut state.hologram_settings.show_scanlines_viewport, "Scanline Wave");
        ui.checkbox(&mut state.hologram_settings.show_rotation_axis_viewport, "Rotation Axis");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_hologram_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Hologram Optics & Projection, Commit
        columns[0].vertical(|ui| {
            ui.heading("2D Volumetric Hologram & Optical Projection");
            render_optical_canvas(ui, state);

            ui.add_space(8.0);
            render_timeline_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Holographic Signage to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Dimensions, Optics, Emitter, Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Hologram Display Dimensions & Optical Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Holographic Clarity & Placement Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_optical_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.hologram_settings.active_config.clone();
    let metrics = state.hologram_settings.cached_metrics.clone();
    let time = state.hologram_settings.simulated_time_seconds;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(10, 14, 22)); // Sci-fi deep void
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(35, 48, 68)),
        StrokeKind::Inside,
    );

    // Scale calculation
    let max_span_x = (cfg.display_width * 1.5).max(180.0);
    let max_span_z = (cfg.elevation_offset_uu + cfg.display_height * 0.7 + 40.0).max(200.0);

    let scale_x = (rect.width() - 80.0) / max_span_x;
    let scale_z = (rect.height() - 70.0) / max_span_z;
    let scale = scale_x.min(scale_z).clamp(0.2, 1.2);

    let floor_y = rect.max.y - 25.0;
    let center_x = rect.center().x;

    // 1. Emitter Pedestal
    let ped_r = cfg.pedestal_radius * scale;
    let ped_h = cfg.pedestal_height * scale;
    if cfg.has_emitter_pedestal {
        let ped_rect = Rect::from_min_size(
            Pos2::new(center_x - ped_r, floor_y - ped_h),
            Vec2::new(ped_r * 2.0, ped_h),
        );
        painter.rect_filled(ped_rect, 2.0, Color32::from_rgb(50, 55, 65));
        painter.rect_stroke(ped_rect, 2.0, Stroke::new(1.5_f32, Color32::from_rgb(140, 160, 190)), StrokeKind::Inside);

        // Projector emitter lens glow
        let lens_rect = Rect::from_min_size(
            Pos2::new(center_x - ped_r * 0.4, floor_y - ped_h - 2.0),
            Vec2::new(ped_r * 0.8, 3.0),
        );
        painter.rect_filled(lens_rect, 1.0, Color32::from_rgb(180, 240, 255));
    }

    // 2. Volumetric Projection Beam Frustum (Translucent gradient cone)
    let display_center_y = floor_y - (cfg.elevation_offset_uu + metrics.current_bob_offset_uu) * scale;
    let disp_hw = (cfg.display_width * 0.5) * scale;
    let disp_hh = (cfg.display_height * 0.5) * scale;
    let display_bottom_y = display_center_y + disp_hh;

    if cfg.has_projection_beam {
        let lens_top = Pos2::new(center_x, floor_y - ped_h);
        let beam_left = Pos2::new(center_x - disp_hw * 0.9, display_bottom_y);
        let beam_right = Pos2::new(center_x + disp_hw * 0.9, display_bottom_y);

        let cone_col = Color32::from_rgba_unmultiplied(
            (cfg.primary_color[0] * 255.0) as u8,
            (cfg.primary_color[1] * 255.0) as u8,
            (cfg.primary_color[2] * 255.0) as u8,
            (25.0 * metrics.effective_luminosity) as u8,
        );
        painter.add(egui::Shape::convex_polygon(vec![lens_top, beam_left, beam_right], cone_col, Stroke::NONE));

        // Beam edge boundary rays
        let ray_col = Color32::from_rgba_unmultiplied(
            (cfg.primary_color[0] * 255.0) as u8,
            (cfg.primary_color[1] * 255.0) as u8,
            (cfg.primary_color[2] * 255.0) as u8,
            120,
        );
        painter.line_segment([lens_top, beam_left], Stroke::new(1.0_f32, ray_col));
        painter.line_segment([lens_top, beam_right], Stroke::new(1.0_f32, ray_col));
    }

    // 3. Hologram Display Canvas (With chromatic aberration and scanlines)
    let ca = cfg.chromatic_aberration_uu * scale;
    let glitch_jitter = if metrics.is_glitching_now { (time * 100.0).sin() * 8.0 } else { 0.0 };

    // Primary & Chromatic Fringe Display Rects
    let base_rect = Rect::from_min_size(
        Pos2::new(center_x - disp_hw + glitch_jitter, display_center_y - disp_hh),
        Vec2::new(disp_hw * 2.0, disp_hh * 2.0),
    );

    // Red fringe (aberration offset left)
    if ca > 0.5 {
        let red_rect = base_rect.translate(Vec2::new(-ca, 0.0));
        let red_fringe = Color32::from_rgba_unmultiplied(255, 30, 30, 40);
        painter.rect_filled(red_rect, 0.0, red_fringe);
    }
    // Cyan fringe (aberration offset right)
    if ca > 0.5 {
        let cyan_rect = base_rect.translate(Vec2::new(ca, 0.0));
        let cyan_fringe = Color32::from_rgba_unmultiplied(30, 240, 255, 40);
        painter.rect_filled(cyan_rect, 0.0, cyan_fringe);
    }

    // Main Hologram Screen Background
    let holo_alpha = (75.0 * metrics.effective_luminosity) as u8;
    let holo_bg = Color32::from_rgba_unmultiplied(
        (cfg.primary_color[0] * 255.0) as u8,
        (cfg.primary_color[1] * 255.0) as u8,
        (cfg.primary_color[2] * 255.0) as u8,
        holo_alpha,
    );
    painter.rect_filled(base_rect, 0.0, holo_bg);

    // Outer Neon Border
    let border_col = Color32::from_rgba_unmultiplied(
        (cfg.secondary_color[0] * 255.0) as u8,
        (cfg.secondary_color[1] * 255.0) as u8,
        (cfg.secondary_color[2] * 255.0) as u8,
        (220.0 * metrics.effective_luminosity) as u8,
    );
    painter.rect_stroke(base_rect, 0.0, Stroke::new(2.0_f32, border_col), StrokeKind::Inside);

    // 4. Scanlines Wave (Drifting horizontal lines)
    let scanline_step = (base_rect.height() / cfg.scanline_count.max(4) as f32).max(2.0);
    let scan_phase = (time * cfg.scanline_drift_speed * 20.0) % scanline_step;
    let mut scan_y = base_rect.min.y + scan_phase;

    let scan_col = Color32::from_rgba_unmultiplied(255, 255, 255, 45);
    while scan_y < base_rect.max.y {
        painter.line_segment(
            [Pos2::new(base_rect.min.x, scan_y), Pos2::new(base_rect.max.x, scan_y)],
            Stroke::new(1.0_f32, scan_col),
        );
        scan_y += scanline_step;
    }

    // 5. Archetype Holographic Emblem in center
    let center_pos = base_rect.center();
    match cfg.archetype {
        HologramArchetype::LiandriCorporationEmblem => {
            // Rotating chevron / triangle emblem
            let emblem_r = disp_hh * 0.55;
            let ang = metrics.current_yaw_deg.to_radians();
            let p1 = Pos2::new(center_pos.x + ang.sin() * emblem_r, center_pos.y - ang.cos() * emblem_r);
            let p2 = Pos2::new(center_pos.x + (ang + 2.094).sin() * emblem_r, center_pos.y - (ang + 2.094).cos() * emblem_r);
            let p3 = Pos2::new(center_pos.x + (ang + 4.188).sin() * emblem_r, center_pos.y - (ang + 4.188).cos() * emblem_r);

            painter.line_segment([p1, p2], Stroke::new(2.5_f32, border_col));
            painter.line_segment([p2, p3], Stroke::new(2.5_f32, border_col));
            painter.line_segment([p3, p1], Stroke::new(2.5_f32, border_col));
            painter.text(
                Pos2::new(center_pos.x, base_rect.max.y - 14.0),
                egui::Align2::CENTER_BOTTOM,
                "LIANDRI CORP",
                egui::FontId::monospace(10.0),
                border_col,
            );
        }
        HologramArchetype::TournamentLiveScoreboard => {
            painter.text(
                Pos2::new(center_pos.x, base_rect.min.y + 16.0),
                egui::Align2::CENTER_TOP,
                "TOURNAMENT RANKINGS",
                egui::FontId::monospace(9.0),
                border_col,
            );
            painter.text(
                Pos2::new(center_pos.x, center_pos.y),
                egui::Align2::CENTER_CENTER,
                "1. MALCOLM [25] | 2. BROCK [21]",
                egui::FontId::monospace(9.0),
                Color32::WHITE,
            );
        }
        HologramArchetype::DirectionalWaypointArrow => {
            let arr_len = disp_hw * 0.6;
            let arr_y = center_pos.y + ((time * 3.0).sin() * 4.0);
            painter.line_segment([Pos2::new(center_pos.x - arr_len, arr_y), Pos2::new(center_pos.x + arr_len, arr_y)], Stroke::new(4.0_f32, border_col));
            painter.line_segment([Pos2::new(center_pos.x + arr_len - 12.0, arr_y - 10.0), Pos2::new(center_pos.x + arr_len, arr_y)], Stroke::new(4.0_f32, border_col));
            painter.line_segment([Pos2::new(center_pos.x + arr_len - 12.0, arr_y + 10.0), Pos2::new(center_pos.x + arr_len, arr_y)], Stroke::new(4.0_f32, border_col));
        }
        HologramArchetype::HazardWarningHoloAlert => {
            let warn_alpha = ((time * 6.0).sin().abs() * 255.0) as u8;
            let flash_col = Color32::from_rgba_unmultiplied(255, 60, 40, warn_alpha);
            painter.text(
                center_pos,
                egui::Align2::CENTER_CENTER,
                "⚠ DANGER: CONTAINMENT BREACH",
                egui::FontId::monospace(10.0),
                flash_col,
            );
        }
        HologramArchetype::TacticalTargetingReticle => {
            let ret_r = disp_hh * 0.65;
            painter.circle_stroke(center_pos, ret_r, Stroke::new(2.0_f32, border_col));
            painter.circle_stroke(center_pos, ret_r * 0.4, Stroke::new(1.5_f32, border_col));
            painter.line_segment([Pos2::new(center_pos.x - ret_r - 6.0, center_pos.y), Pos2::new(center_pos.x + ret_r + 6.0, center_pos.y)], Stroke::new(1.0_f32, border_col));
            painter.line_segment([Pos2::new(center_pos.x, center_pos.y - ret_r - 6.0), Pos2::new(center_pos.x, center_pos.y + ret_r + 6.0)], Stroke::new(1.0_f32, border_col));
        }
    }

    // Telemetry OSD
    let glitch_txt = if metrics.is_glitching_now { " [GLITCH STROBE]" } else { "" };
    let osd = format!("Size: {:.0}x{:.0} uu | Yaw: {:.1}° | Bob: {:+.1} uu | Lum: {:.0}%{}",
        cfg.display_width, cfg.display_height, metrics.current_yaw_deg, metrics.current_bob_offset_uu, metrics.effective_luminosity * 100.0, glitch_txt);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        osd,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(180, 225, 255),
    );
}

fn render_timeline_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim = &mut state.hologram_settings.is_animating;
            let icon = if *anim { "⏸ Pause" } else { "▶ Play" };
            if ui.button(icon).clicked() {
                *anim = !*anim;
            }

            ui.label(egui::RichText::new("Optical Time:").strong());
            let mut t = state.hologram_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=60.0).text("sec")).changed() {
                state.scrub_hologram_time(t);
            }

            if ui.button("Reset").clicked() {
                state.scrub_hologram_time(0.0);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.hologram_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Holo: {} ({:.0}x{:.0} uu) | Rot: {:.0}°/s | Pedestal: {} | Tag: {}",
            cfg.display_shape.display_name(), cfg.display_width, cfg.display_height, cfg.rotation_speed_deg_s, cfg.has_emitter_pedestal, cfg.tag));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Hologram Display, Emitter, Frustum & Mover to Level").strong())
            .on_hover_text("Spawns translucent unlit hologram surface, solid emitter pedestal, projection beam cone, rotating mover actor, and projector light actor (100% Undoable)")
            .clicked()
        {
            state.commit_hologram_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_hologram_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("hologram_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Display Shape
        ui.label("Display Geometry Shape:");
        egui::ComboBox::from_id_salt("holo_shape_combo")
            .selected_text(state.hologram_settings.active_config.display_shape.display_name())
            .show_ui(ui, |ui| {
                for &s in HoloDisplayShape::all() {
                    if ui.selectable_value(&mut state.hologram_settings.active_config.display_shape, s, s.display_name()).clicked() {
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Dimensions
        ui.label("Display Dimensions:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.display_width).range(32.0..=512.0).speed(4.0)).changed() {
                changed = true;
            }
            ui.label("W x");
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.display_height).range(32.0..=512.0).speed(4.0)).changed() {
                changed = true;
            }
            ui.label("H (uu)");
        });
        ui.end_row();

        // Curvature Sweep (for Curved Arc)
        if state.hologram_settings.active_config.display_shape == HoloDisplayShape::CurvedCylinderArc {
            ui.label("Curvature Sweep Angle:");
            if ui.add(egui::Slider::new(&mut state.hologram_settings.active_config.curvature_sweep_deg, 15.0..=120.0).text("deg")).changed() {
                changed = true;
            }
            ui.end_row();
        }

        // Elevation Offset
        ui.label("Elevation Above Emitter:");
        if ui.add(egui::Slider::new(&mut state.hologram_settings.active_config.elevation_offset_uu, 32.0..=384.0).text("uu")).changed() {
            changed = true;
        }
        ui.end_row();

        // Rotation Speed
        ui.label("Continuous Yaw Rotation:");
        ui.horizontal(|ui| {
            if ui.add(egui::Slider::new(&mut state.hologram_settings.active_config.rotation_speed_deg_s, -60.0..=60.0).text("deg/s")).changed() {
                changed = true;
            }
            if ui.button("Halt").clicked() {
                state.hologram_settings.active_config.rotation_speed_deg_s = 0.0;
                changed = true;
            }
        });
        ui.end_row();

        // Floating Bob
        ui.label("Vertical Floating Bob:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.bob_amplitude_uu).range(0.0..=24.0).speed(0.5)).changed() {
                changed = true;
            }
            ui.label("amp |");
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.bob_frequency_hz).range(0.1..=3.0).speed(0.1)).changed() {
                changed = true;
            }
            ui.label("Hz");
        });
        ui.end_row();

        // Scanlines
        ui.label("Scanline Wave & Drift:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.scanline_count).range(8..=64).speed(1)).changed() {
                changed = true;
            }
            ui.label("lines |");
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.scanline_drift_speed).range(0.0..=5.0).speed(0.2)).changed() {
                changed = true;
            }
            ui.label("spd");
        });
        ui.end_row();

        // Chromatic Aberration & Glitch
        ui.label("Aberration & Glitch Strobe:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.chromatic_aberration_uu).range(0.0..=10.0).speed(0.5)).changed() {
                changed = true;
            }
            ui.label("uu fringe |");
            let mut glitch_pct = state.hologram_settings.active_config.glitch_flicker_probability * 100.0;
            if ui.add(egui::DragValue::new(&mut glitch_pct).range(0.0..=25.0).speed(1.0)).changed() {
                state.hologram_settings.active_config.glitch_flicker_probability = glitch_pct / 100.0;
                changed = true;
            }
            ui.label("% glitch");
        });
        ui.end_row();

        // Emitter Pedestal
        ui.label("Emitter Pedestal Base:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.hologram_settings.active_config.has_emitter_pedestal, "Base Pedestal").changed() {
                changed = true;
            }
            if state.hologram_settings.active_config.has_emitter_pedestal {
                ui.label("R:");
                if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.pedestal_radius).range(16.0..=96.0).speed(2.0)).changed() {
                    changed = true;
                }
                ui.label("H:");
                if ui.add(egui::DragValue::new(&mut state.hologram_settings.active_config.pedestal_height).range(4.0..=32.0).speed(1.0)).changed() {
                    changed = true;
                }
            }
        });
        ui.end_row();

        // Projection Frustum Cone
        ui.label("Projection Beam Frustum:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.hologram_settings.active_config.has_projection_beam, "Light Cone").changed() {
                changed = true;
            }
            if state.hologram_settings.active_config.has_projection_beam {
                ui.label("Angle:");
                if ui.add(egui::Slider::new(&mut state.hologram_settings.active_config.beam_cone_angle_deg, 15.0..=60.0).text("deg")).changed() {
                    changed = true;
                }
            }
        });
        ui.end_row();

        // Texture Name
        ui.label("Hologram Texture:");
        if ui.text_edit_singleline(&mut state.hologram_settings.active_config.texture_name).changed() {
            changed = true;
        }
        ui.end_row();

        // Event & Tag
        ui.label("Rotating Mover Tag:");
        if ui.text_edit_singleline(&mut state.hologram_settings.active_config.tag).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_hologram_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.hologram_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL HOLOGRAM HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Optical Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Hologram Optical & Spatial Clarity Valid").strong());
            }
        });

        let metrics = &state.hologram_settings.cached_metrics;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("Area: {:.0} sq uu | Vol: {:.0} uu³ | Yaw: {:.1}° | Lum: {:.0}%",
                metrics.surface_area_sq_uu, metrics.projection_volume_uu3, metrics.current_yaw_deg, metrics.effective_luminosity * 100.0));
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
