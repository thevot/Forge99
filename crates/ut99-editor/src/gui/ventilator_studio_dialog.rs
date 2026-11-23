// crates/ut99-editor/src/gui/ventilator_studio_dialog.rs
//
// Interactive Environmental Ventilation Fan, Air Current & Wind Tunnel Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{AirFlowDirection, VentilatorArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_ventilator_dialog {
        return;
    }

    let mut open = state.show_ventilator_dialog;

    // Handle fan rotation animation playback
    if state.ventilator_settings.is_spinning {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        state.ventilator_settings.simulated_time_seconds += dt;
        state.recalculate_ventilator_preview();
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Environmental Ventilation Fan, Air Current & Wind Tunnel Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_ventilator_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Fan Archetypes:").strong());
        for &arch in VentilatorArchetype::all() {
            let is_sel = state.ventilator_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_ventilator_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.ventilator_settings.show_fan_viewport, "Rotating Fan Rotor");
        ui.checkbox(&mut state.ventilator_settings.show_housing_viewport, "Duct Housing Ring");
        ui.checkbox(&mut state.ventilator_settings.show_streamlines_viewport, "Air Current Streamlines");
        ui.checkbox(&mut state.ventilator_settings.show_hazard_zone_viewport, "Blade Danger Zone");
        ui.checkbox(&mut state.ventilator_settings.show_hover_height_viewport, "Hover Height Plane");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_ventilator_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Aerodynamic Canvas, Spin Timeline, Commit Action
        columns[0].vertical(|ui| {
            ui.heading("2D Airflow Streamlines & Duct Cross-Section");
            render_airflow_canvas(ui, state);

            ui.add_space(8.0);
            render_spin_timeline(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Ventilator Setup to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Geometry, Kinematics, Aerodynamics, Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Fan Geometry, Pitch & Aerodynamics");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Aerodynamic Lift & Fan Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_airflow_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let cfg = &state.ventilator_settings.active_config;
    let duct_r = cfg.air_current.duct_radius.max(32.0);
    let range = cfg.air_current.effective_range.max(128.0);

    let avail_w = ui.available_width().max(200.0);
    let target_h = (avail_w * (duct_r * 2.0 / range)).clamp(180.0, 280.0);
    let target_w = avail_w;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Duct background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(55, 65, 80)),
        StrokeKind::Inside,
    );

    // Coordinate mapping: Left is fan plane (x=0), Right is duct exit (x=range)
    let margin_x = 30.0;
    let margin_y = 15.0;
    let usable_w = rect.width() - margin_x * 2.0;
    let usable_h = rect.height() - margin_y * 2.0;

    let to_canvas = |dist_along: f32, radial_offset: f32| -> Pos2 {
        let u = dist_along / range;
        let v = (radial_offset + duct_r) / (duct_r * 2.0);
        Pos2::new(
            rect.min.x + margin_x + u * usable_w,
            rect.min.y + margin_y + (1.0 - v) * usable_h,
        )
    };

    // Duct Wall Borders
    let wall_top_start = to_canvas(0.0, duct_r);
    let wall_top_end = to_canvas(range, duct_r);
    let wall_bot_start = to_canvas(0.0, -duct_r);
    let wall_bot_end = to_canvas(range, -duct_r);

    let duct_stroke = Stroke::new(3.0_f32, Color32::from_rgb(90, 110, 140));
    painter.line_segment([wall_top_start, wall_top_end], duct_stroke);
    painter.line_segment([wall_bot_start, wall_bot_end], duct_stroke);

    // Fan Rotor Blade Plane (at dist_along = 0)
    let hub_r = cfg.geometry.hub_radius;
    let bl_len = cfg.geometry.blade_length;
    let fan_top = to_canvas(0.0, hub_r + bl_len);
    let fan_bot = to_canvas(0.0, -(hub_r + bl_len));
    painter.line_segment([fan_top, fan_bot], Stroke::new(6.0_f32, Color32::from_rgb(220, 180, 50)));

    // Hub circle in center
    let hub_center = to_canvas(0.0, 0.0);
    let hub_canvas_r = (hub_r / (duct_r * 2.0)) * usable_h;
    painter.circle_filled(hub_center, hub_canvas_r, Color32::from_rgb(70, 80, 95));
    painter.circle_stroke(hub_center, hub_canvas_r, Stroke::new(2.0_f32, Color32::from_rgb(180, 195, 215)));

    // Lethal blade danger stripe
    if cfg.lethality.is_lethal_contact {
        let p_top_left = to_canvas(0.0, duct_r * 0.95);
        let p_bot_right = to_canvas(16.0, -duct_r * 0.95);
        painter.rect_filled(
            egui::Rect::from_min_max(p_top_left, p_bot_right),
            2.0,
            Color32::from_rgba_unmultiplied(255, 40, 40, 45),
        );
    }

    // Dynamic Airflow Streamlines
    let stream_col = match cfg.air_current.flow_direction {
        AirFlowDirection::Blowing => Color32::from_rgb(80, 200, 255),
        AirFlowDirection::Sucking => Color32::from_rgb(255, 140, 50),
        AirFlowDirection::Neutral => Color32::from_rgb(140, 150, 160),
    };

    let stream_lines = 7;
    for s in 0..stream_lines {
        let frac = (s as f32 / (stream_lines - 1) as f32) * 2.0 - 1.0;
        let r_pos = frac * (duct_r * 0.75);

        let speed = cfg.air_current.zone_velocity_strength;
        let t_phase = (state.ventilator_settings.simulated_time_seconds * (speed / range) + s as f32 * 0.2).fract();

        let x_curr = t_phase * range;
        let x_tail = (x_curr - 40.0).max(0.0);

        let p_start = to_canvas(x_tail, r_pos);
        let p_end = to_canvas(x_curr, r_pos);

        painter.line_segment([p_start, p_end], Stroke::new(2.0_f32, stream_col));

        // Arrow head
        let arrow_len = 6.0;
        let p_arrow1 = Pos2::new(p_end.x - arrow_len, p_end.y - 3.0);
        let p_arrow2 = Pos2::new(p_end.x - arrow_len, p_end.y + 3.0);
        painter.line_segment([p_end, p_arrow1], Stroke::new(1.5_f32, stream_col));
        painter.line_segment([p_end, p_arrow2], Stroke::new(1.5_f32, stream_col));
    }

    // Player Silhouette reference standing in the duct
    let p_dist = range * 0.4;
    let p_base = to_canvas(p_dist, -duct_r + 4.0);
    let p_head = Pos2::new(p_base.x, p_base.y - (78.0 / (duct_r * 2.0)) * usable_h);
    painter.line_segment([p_base, p_head], Stroke::new(4.0_f32, Color32::from_rgba_unmultiplied(100, 180, 255, 180)));
    painter.circle_filled(p_head, 5.0, Color32::from_rgba_unmultiplied(100, 180, 255, 200));

    // Hover equilibrium line (if buoyant)
    if state.ventilator_settings.cached_lift.can_lift_player {
        let hover_d = state.ventilator_settings.cached_lift.equilibrium_hover_height;
        let h_top = to_canvas(hover_d, duct_r * 0.9);
        let h_bot = to_canvas(hover_d, -duct_r * 0.9);
        painter.line_segment([h_top, h_bot], Stroke::new(2.0_f32, Color32::from_rgb(60, 240, 120)));
        painter.text(
            Pos2::new(h_top.x + 4.0, h_top.y),
            egui::Align2::LEFT_TOP,
            format!("Hover Equil: {:.0} uu", hover_d),
            egui::FontId::monospace(9.0),
            Color32::from_rgb(60, 240, 120),
        );
    }

    // OSD Telemetry
    let telemetry = format!("Rotor: {:.0} RPM | Flow: {} | Velocity: {:.0} uu/s",
        cfg.spin_rpm, cfg.air_current.flow_direction.display_name(), cfg.air_current.zone_velocity_strength);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        telemetry,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(200, 220, 245),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Environmental Ventilation Air Duct").strong());
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Blades: {} x {:.0} uu @ {:.0}° pitch", cfg.geometry.blade_count, cfg.geometry.blade_length, cfg.geometry.blade_pitch_deg));
        ui.label(format!("Lift: {:.0} uu/s (Can lift: {})", state.ventilator_settings.cached_lift.net_vertical_lift, state.ventilator_settings.cached_lift.can_lift_player));
    });
}

fn render_spin_timeline(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.ventilator_settings.is_spinning { "⏸ Pause" } else { "▶ Spin Rotor" };
            if ui.button(anim_txt).clicked() {
                state.ventilator_settings.is_spinning = !state.ventilator_settings.is_spinning;
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_ventilator_time(0.0);
            }

            ui.label("Simulation Time:");
            let mut t = state.ventilator_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=10.0).text("s")).changed() {
                state.scrub_ventilator_time(t);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.ventilator_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Mover: RotatingMover | Zone: ZoneInfo ({:.0} uu/s) | Lethal: {}",
            cfg.air_current.zone_velocity_strength, cfg.lethality.is_lethal_contact));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Ventilator, ZoneInfo & Hazard Trigger to Level").strong())
            .on_hover_text("Spawns semi-solid rotating fan brush, outer duct ring brush, Engine.RotatingMover, Engine.ZoneInfo, and hazard trigger (100% Undoable)")
            .clicked()
        {
            state.commit_ventilator_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_ventilator_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("ventilator_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Spin RPM
        ui.label("Spin Speed (RPM):");
        if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.spin_rpm).speed(5.0).range(0.0..=600.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Blade Count
        ui.label("Blade Count:");
        if ui.add(egui::Slider::new(&mut state.ventilator_settings.active_config.geometry.blade_count, 2..=16)).changed() {
            changed = true;
        }
        ui.end_row();

        // Blade Length
        ui.label("Blade Length (uu):");
        if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.geometry.blade_length).speed(4.0).range(32.0..=512.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Blade Pitch Angle
        ui.label("Blade Pitch Angle (°):");
        if ui.add(egui::Slider::new(&mut state.ventilator_settings.active_config.geometry.blade_pitch_deg, 10.0..=60.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Hub Radius
        ui.label("Hub Radius (uu):");
        if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.geometry.hub_radius).speed(2.0).range(8.0..=64.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Outer Ring Housing
        ui.label("Outer Duct Ring:");
        if ui.checkbox(&mut state.ventilator_settings.active_config.geometry.has_outer_ring, "Generate Casing").changed() {
            changed = true;
        }
        ui.end_row();

        // Airflow Direction
        ui.label("Air Flow Direction:");
        egui::ComboBox::from_id_salt("airflow_dir_combo")
            .selected_text(state.ventilator_settings.active_config.air_current.flow_direction.display_name())
            .show_ui(ui, |ui| {
                for &dir in AirFlowDirection::all() {
                    let is_sel = state.ventilator_settings.active_config.air_current.flow_direction == dir;
                    if ui.selectable_label(is_sel, dir.display_name()).clicked() {
                        state.ventilator_settings.active_config.air_current.flow_direction = dir;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // ZoneVelocity Strength
        ui.label("ZoneVelocity Strength (uu/s):");
        if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.air_current.zone_velocity_strength).speed(20.0).range(0.0..=3000.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Effective Range
        ui.label("Duct Effective Range (uu):");
        if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.air_current.effective_range).speed(32.0).range(64.0..=2048.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Lethal Contact Hazard
        ui.label("Lethal Blade Contact:");
        if ui.checkbox(&mut state.ventilator_settings.active_config.lethality.is_lethal_contact, "Contact Damage").changed() {
            changed = true;
        }
        ui.end_row();

        if state.ventilator_settings.active_config.lethality.is_lethal_contact {
            ui.label("Blade Damage (HP/s):");
            if ui.add(egui::DragValue::new(&mut state.ventilator_settings.active_config.lethality.blade_damage_per_sec).speed(25.0).range(0.0..=1000.0)).changed() {
                changed = true;
            }
            ui.end_row();
        }
    });

    if changed {
        state.recalculate_ventilator_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.ventilator_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL FAN HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Aerodynamic Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Aerodynamic Lift & Kinematic Clearance Valid").strong());
            }
        });

        let lift = &state.ventilator_settings.cached_lift;
        ui.horizontal(|ui| {
            if lift.can_lift_player {
                ui.colored_label(Color32::from_rgb(80, 220, 120), format!("⬆ BUOYANT LIFT: {:.0} uu/s net (Hover at {:.0} uu)", lift.net_vertical_lift, lift.equilibrium_hover_height));
            } else {
                ui.colored_label(Color32::from_rgb(180, 190, 200), "⬇ Non-Buoyant Airflow (Will not lift against UT99 gravity)");
            }
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
