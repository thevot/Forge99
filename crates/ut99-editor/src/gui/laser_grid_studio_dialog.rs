// crates/ut99-editor/src/gui/laser_grid_studio_dialog.rs
//
// Interactive Security Laser Grid, Tripwire Alarm & Energy Barrier Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{LaserArchetype, LaserLayoutMode};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_laser_grid_dialog {
        return;
    }

    let mut open = state.show_laser_grid_dialog;

    // Handle sweep / pulse animation playback
    if state.laser_grid_settings.is_pulsing {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        state.laser_grid_settings.simulated_time_seconds += dt;
        state.recalculate_laser_preview();
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Security Laser Grid, Tripwire Alarm & Energy Barrier Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_laser_grid_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Laser Archetypes:").strong());
        for &arch in LaserArchetype::all() {
            let is_sel = state.laser_grid_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_laser_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.laser_grid_settings.show_beams_viewport, "Glowing Laser Beams");
        ui.checkbox(&mut state.laser_grid_settings.show_diodes_viewport, "Diode Wall Mounts");
        ui.checkbox(&mut state.laser_grid_settings.show_triggers_viewport, "Trigger Touch Capsules");
        ui.checkbox(&mut state.laser_grid_settings.show_sweep_arcs_viewport, "Kinematic Sweep Range");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_laser_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Elevation Canvas, Sweep Timeline, Commit Action
        columns[0].vertical(|ui| {
            ui.heading("2D Corridor Elevation & Beam Clearance Canvas");
            render_corridor_canvas(ui, state);

            ui.add_space(8.0);
            render_sweep_timeline(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Laser Grid Setup to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Grid Dimensions, Optics, Kinematics, Triggers, Hazards
        columns[1].vertical(|ui| {
            ui.heading("Laser Grid Optics, Layout & Kinematics");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Laser Safety, Clearance & Alarm Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_corridor_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let cfg = &state.laser_grid_settings.active_config;
    let span_w = cfg.span_width.max(64.0);
    let span_h = cfg.span_height.max(64.0);

    let avail_w = ui.available_width().max(200.0);
    let target_h = (avail_w * (span_h / span_w)).clamp(160.0, 260.0);
    let target_w = target_h * (span_w / span_h);

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Corridor background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 30));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(60, 70, 85)),
        StrokeKind::Inside,
    );

    // Wall jambs (left and right metallic strips)
    let jamb_w = 12.0;
    painter.rect_filled(
        egui::Rect::from_min_size(rect.min, Vec2::new(jamb_w, rect.height())),
        2.0,
        Color32::from_rgb(45, 52, 62),
    );
    painter.rect_filled(
        egui::Rect::from_min_size(Pos2::new(rect.max.x - jamb_w, rect.min.y), Vec2::new(jamb_w, rect.height())),
        2.0,
        Color32::from_rgb(45, 52, 62),
    );

    // Coordinate mapping from 3D level coordinates to canvas
    let to_canvas = |pos_x: f32, pos_z: f32| -> Pos2 {
        let u = (pos_x + span_w * 0.5) / span_w;
        let v = 1.0 - (pos_z / span_h); // Invert Z so up is canvas top
        Pos2::new(
            rect.min.x + jamb_w + u * (rect.width() - jamb_w * 2.0),
            rect.min.y + v * rect.height(),
        )
    };

    // Draw active beams
    let beam_col = Color32::from_rgba_unmultiplied(
        (cfg.beam_color[0] * 255.0) as u8,
        (cfg.beam_color[1] * 255.0) as u8,
        (cfg.beam_color[2] * 255.0) as u8,
        (cfg.beam_color[3] * 255.0) as u8,
    );
    let halo_col = Color32::from_rgba_unmultiplied(
        (cfg.beam_color[0] * 255.0) as u8,
        (cfg.beam_color[1] * 255.0) as u8,
        (cfg.beam_color[2] * 255.0) as u8,
        50,
    );

    for beam in &state.laser_grid_settings.cached_beams {
        let p_start = to_canvas(beam.start_point.x - cfg.origin.x, beam.start_point.z);
        let p_end = to_canvas(beam.end_point.x - cfg.origin.x, beam.end_point.z);

        // Outer glow halo
        painter.line_segment([p_start, p_end], Stroke::new(6.0_f32, halo_col));
        // Bright core laser line
        painter.line_segment([p_start, p_end], Stroke::new(2.5_f32, beam_col));

        // Draw Diode caps at endpoints
        painter.circle_filled(p_start, 4.0, Color32::from_rgb(220, 230, 245));
        painter.circle_filled(p_end, 4.0, Color32::from_rgb(220, 230, 245));
    }

    // Draw Player Silhouette Reference (Standard UT99 Player: radius 17 uu, height 78 uu, crouch height 48 uu)
    let player_x = 0.0;
    let player_base = to_canvas(player_x, 0.0);
    let player_top = to_canvas(player_x, 78.0);
    let player_crouch = to_canvas(player_x, 48.0);
    let half_player_w = (17.0 / span_w) * (rect.width() - jamb_w * 2.0);

    // Standing outline
    let player_box = egui::Rect::from_min_max(
        Pos2::new(player_base.x - half_player_w, player_top.y),
        Pos2::new(player_base.x + half_player_w, player_base.y),
    );
    painter.rect_stroke(
        player_box,
        3.0,
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(100, 160, 255, 120)),
        StrokeKind::Inside,
    );

    // Crouch line
    painter.line_segment(
        [Pos2::new(player_base.x - half_player_w, player_crouch.y), Pos2::new(player_base.x + half_player_w, player_crouch.y)],
        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 200, 50, 160)),
    );

    painter.text(
        Pos2::new(player_base.x, player_base.y - 6.0),
        egui::Align2::CENTER_BOTTOM,
        "Player Hitbox",
        egui::FontId::proportional(9.0),
        Color32::from_rgba_unmultiplied(160, 190, 240, 180),
    );

    // OSD Info
    let info_txt = format!("Span: {:.0} x {:.0} uu | Beams: {}", span_w, span_h, state.laser_grid_settings.cached_beams.len());
    painter.text(
        Pos2::new(rect.min.x + jamb_w + 6.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        info_txt,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(200, 215, 235),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Security Laser Grid 2D Elevation").strong());
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Layout: {}", cfg.layout_mode.display_name()));
        ui.label(format!("Event: {}", cfg.trigger.alarm_event));
        ui.label(format!("Damage: {:.0} HP/s", cfg.trigger.damage_per_second));
    });
}

fn render_sweep_timeline(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.laser_grid_settings.is_pulsing { "⏸ Pause" } else { "▶ Pulse / Sweep" };
            if ui.button(anim_txt).clicked() {
                state.laser_grid_settings.is_pulsing = !state.laser_grid_settings.is_pulsing;
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_laser_time(0.0);
            }

            ui.label("Simulation Time:");
            let mut t = state.laser_grid_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=10.0).text("s")).changed() {
                state.scrub_laser_time(t);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.laser_grid_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Alarm Event: '{}' | Damage: {:.0} HP/s", cfg.trigger.alarm_event, cfg.trigger.damage_per_second));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Laser Grid, Diodes & Alarm Triggers to Level").strong())
            .on_hover_text("Spawns translucent laser ribbon brushes, semi-solid diode housing brushes, and Engine.Trigger actors into the level (100% Undoable)")
            .clicked()
        {
            state.commit_laser_grid_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_laser_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("laser_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Span Dimensions
        ui.label("Span Width (uu):");
        if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.span_width).speed(16.0).range(64.0..=2048.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Span Height (uu):");
        if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.span_height).speed(16.0).range(32.0..=1024.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Origin Height (Z)
        ui.label("Mount Height (Z):");
        if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.origin.z).speed(4.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Layout Mode
        ui.label("Beam Layout Pattern:");
        egui::ComboBox::from_id_salt("laser_layout_combo")
            .selected_text(state.laser_grid_settings.active_config.layout_mode.display_name())
            .show_ui(ui, |ui| {
                for &mode in LaserLayoutMode::all() {
                    let is_sel = state.laser_grid_settings.active_config.layout_mode == mode;
                    if ui.selectable_label(is_sel, mode.display_name()).clicked() {
                        state.laser_grid_settings.active_config.layout_mode = mode;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Beam Count
        ui.label("Beam Count:");
        if ui.add(egui::Slider::new(&mut state.laser_grid_settings.active_config.beam_count, 1..=16)).changed() {
            changed = true;
        }
        ui.end_row();

        // Beam Radius
        ui.label("Beam Radius (uu):");
        if ui.add(egui::Slider::new(&mut state.laser_grid_settings.active_config.beam_radius, 1.0..=8.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Beam Color Presets
        ui.label("Color Preset:");
        ui.horizontal(|ui| {
            if ui.button("🔴 Crimson").clicked() {
                state.laser_grid_settings.active_config.beam_color = [1.0, 0.15, 0.15, 0.95];
                changed = true;
            }
            if ui.button("🟢 Emerald").clicked() {
                state.laser_grid_settings.active_config.beam_color = [0.15, 0.95, 0.35, 0.95];
                changed = true;
            }
            if ui.button("🟡 Amber").clicked() {
                state.laser_grid_settings.active_config.beam_color = [1.0, 0.85, 0.15, 0.95];
                changed = true;
            }
            if ui.button("🔵 Cyan").clicked() {
                state.laser_grid_settings.active_config.beam_color = [0.20, 0.85, 1.00, 0.95];
                changed = true;
            }
            if ui.button("🟣 Violet").clicked() {
                state.laser_grid_settings.active_config.beam_color = [0.95, 0.10, 0.90, 0.95];
                changed = true;
            }
        });
        ui.end_row();

        // Kinematic Sweep
        ui.label("Kinematic Sweeping:");
        if ui.checkbox(&mut state.laser_grid_settings.active_config.sweep.is_sweeping, "Enable Oscillation").changed() {
            changed = true;
        }
        ui.end_row();

        if state.laser_grid_settings.active_config.sweep.is_sweeping {
            ui.label("Sweep Amplitude (uu):");
            if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.sweep.sweep_amplitude).speed(2.0).range(0.0..=256.0)).changed() {
                changed = true;
            }
            ui.end_row();

            ui.label("Sweep Period (sec):");
            if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.sweep.sweep_period_sec).speed(0.2).range(0.5..=15.0)).changed() {
                changed = true;
            }
            ui.end_row();
        }

        // Trigger Settings
        ui.label("Trigger Radius (uu):");
        if ui.add(egui::Slider::new(&mut state.laser_grid_settings.active_config.trigger.trigger_radius, 8.0..=48.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Alarm Event Name:");
        if ui.text_edit_singleline(&mut state.laser_grid_settings.active_config.trigger.alarm_event).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Damage (HP / sec):");
        if ui.add(egui::DragValue::new(&mut state.laser_grid_settings.active_config.trigger.damage_per_second).speed(5.0).range(0.0..=1000.0)).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_laser_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.laser_grid_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL LASER HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Laser Grid Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Laser Grid & Trigger Clearance Valid").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("Laser beams terminate properly, alarm events are bound, and trigger detection margin is secure.");
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
