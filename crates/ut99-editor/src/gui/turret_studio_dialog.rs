// crates/ut99-editor/src/gui/turret_studio_dialog.rs
//
// Interactive Automated Defense Turret, Sentry Cannon & Emplacement Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{TurretArchetype, TurretMountType, TurretTeamOwnership, TurretWeaponType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_turret_dialog {
        return;
    }

    let mut open = state.show_turret_dialog;

    // Handle animated swivel tracking playback
    if state.turret_settings.is_tracking {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        state.turret_settings.simulated_time_seconds += dt;
        state.recalculate_turret_preview();
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Automated Defense Turret, Sentry Cannon & Emplacement Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_turret_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Turret Archetypes:").strong());
        for &arch in TurretArchetype::all() {
            let is_sel = state.turret_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_turret_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.turret_settings.show_mount_viewport, "Mount Brush");
        ui.checkbox(&mut state.turret_settings.show_shield_viewport, "Shield Visor");
        ui.checkbox(&mut state.turret_settings.show_tracking_cone_viewport, "Tracking Arc Cone");
        ui.checkbox(&mut state.turret_settings.show_line_of_fire_viewport, "Line-of-Fire Beam");
        ui.checkbox(&mut state.turret_settings.show_waypoints_viewport, "Bot Evasion Waypoints");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_turret_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Polar Traversal Radar Canvas, Scrubber, Commit
        columns[0].vertical(|ui| {
            ui.heading("2D Polar Traversal Radar & Aiming Elevation");
            render_radar_canvas(ui, state);

            ui.add_space(8.0);
            render_tracking_timeline(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Defense Emplacement to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Mount, Kinematics, Weapons, Diagnostics
        columns[1].vertical(|ui| {
            ui.heading("Mount Geometry & Swivel Kinematics");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Ballistic Threat & Spatial Clearance Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_radar_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let cfg = &state.turret_settings.active_config;
    let metrics = &state.turret_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 24));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(45, 58, 75)),
        StrokeKind::Inside,
    );

    let center = Pos2::new(rect.min.x + rect.width() * 0.45, rect.min.y + rect.height() * 0.52);
    let radar_radius = (rect.height() * 0.44).min(rect.width() * 0.38);

    // Concentric range rings (500, 1000, 2000, max range)
    let ring_cols = Color32::from_rgba_unmultiplied(60, 85, 115, 90);
    painter.circle_stroke(center, radar_radius * 0.33, Stroke::new(1.0_f32, ring_cols));
    painter.circle_stroke(center, radar_radius * 0.66, Stroke::new(1.0_f32, ring_cols));
    painter.circle_stroke(center, radar_radius, Stroke::new(1.5_f32, Color32::from_rgb(70, 105, 145)));

    // Polar crosshairs
    painter.line_segment([Pos2::new(center.x - radar_radius, center.y), Pos2::new(center.x + radar_radius, center.y)], Stroke::new(1.0_f32, ring_cols));
    painter.line_segment([Pos2::new(center.x, center.y - radar_radius), Pos2::new(center.x, center.y + radar_radius)], Stroke::new(1.0_f32, ring_cols));

    // Swept Traversal Fan Arc
    let base_rad = cfg.base_rotation_yaw_deg.to_radians();
    let min_rad = (cfg.base_rotation_yaw_deg + cfg.tracking.yaw_min_deg).to_radians();
    let max_rad = (cfg.base_rotation_yaw_deg + cfg.tracking.yaw_max_deg).to_radians();

    let arc_steps = 24;
    let mut fan_poly = vec![center];
    for s in 0..=arc_steps {
        let frac = s as f32 / arc_steps as f32;
        let ang = min_rad + frac * (max_rad - min_rad);
        let pt = Pos2::new(center.x + ang.cos() * radar_radius, center.y - ang.sin() * radar_radius);
        fan_poly.push(pt);
    }
    let fan_col = match cfg.team {
        TurretTeamOwnership::RedTeam => Color32::from_rgba_unmultiplied(230, 60, 60, 45),
        TurretTeamOwnership::BlueTeam => Color32::from_rgba_unmultiplied(60, 120, 240, 45),
        _ => Color32::from_rgba_unmultiplied(240, 180, 50, 45),
    };
    painter.add(egui::Shape::convex_polygon(fan_poly, fan_col, Stroke::new(1.5_f32, Color32::from_rgb(100, 160, 220))));

    // Central Turret Mount Pedestal & Shield
    painter.circle_filled(center, 12.0, Color32::from_rgb(40, 50, 65));
    painter.circle_stroke(center, 12.0, Stroke::new(2.0_f32, Color32::from_rgb(180, 200, 230)));

    if cfg.mount.has_shield_plate {
        let shield_half = 14.0;
        let normal_ang = base_rad + std::f32::consts::FRAC_PI_2;
        let p1 = Pos2::new(center.x + normal_ang.cos() * shield_half, center.y - normal_ang.sin() * shield_half);
        let p2 = Pos2::new(center.x - normal_ang.cos() * shield_half, center.y + normal_ang.sin() * shield_half);
        painter.line_segment([p1, p2], Stroke::new(4.0_f32, Color32::from_rgb(220, 190, 80)));
    }

    // Live Aiming Line-of-Fire Raycast Beam
    let curr_yaw_rad = metrics.current_yaw_deg.to_radians();
    let aim_tip = Pos2::new(
        center.x + curr_yaw_rad.cos() * radar_radius,
        center.y - curr_yaw_rad.sin() * radar_radius,
    );
    let beam_col = match cfg.combat.weapon_type {
        TurretWeaponType::ShockBeam => Color32::from_rgb(180, 60, 255),
        TurretWeaponType::RocketSalvo => Color32::from_rgb(255, 120, 30),
        TurretWeaponType::PlasmaBolt => Color32::from_rgb(80, 240, 255),
        _ => Color32::from_rgb(255, 215, 60),
    };
    painter.line_segment([center, aim_tip], Stroke::new(2.5_f32, beam_col));

    // Animated Burst Tracers along Beam
    let burst_count = 3;
    let t = state.turret_settings.simulated_time_seconds;
    for b in 0..burst_count {
        let u = ((t * 2.0) + (b as f32 / burst_count as f32)).rem_euclid(1.0);
        let tracer_pos = Pos2::new(
            center.x + curr_yaw_rad.cos() * (u * radar_radius),
            center.y - curr_yaw_rad.sin() * (u * radar_radius),
        );
        painter.circle_filled(tracer_pos, 3.5, Color32::WHITE);
    }

    // Muzzle Aiming Crosshair
    painter.circle_stroke(aim_tip, 6.0, Stroke::new(1.5_f32, beam_col));
    painter.line_segment([Pos2::new(aim_tip.x - 9.0, aim_tip.y), Pos2::new(aim_tip.x + 9.0, aim_tip.y)], Stroke::new(1.0_f32, beam_col));
    painter.line_segment([Pos2::new(aim_tip.x, aim_tip.y - 9.0), Pos2::new(aim_tip.x, aim_tip.y + 9.0)], Stroke::new(1.0_f32, beam_col));

    // Bot Evasion Waypoint Badges
    let wp_scale = radar_radius / cfg.tracking.tracking_range.max(500.0);
    for wp in &state.turret_settings.cached_waypoints {
        let rel_x = wp.location.x - cfg.position.x;
        let rel_y = wp.location.y - cfg.position.y;
        let p_wp = Pos2::new(center.x + rel_x * wp_scale, center.y - rel_y * wp_scale);

        let wp_col = if wp.has_direct_line_of_sight {
            Color32::from_rgb(255, 80, 80) // Red hazard
        } else {
            Color32::from_rgb(60, 220, 120) // Green flank cover
        };
        painter.circle_filled(p_wp, 4.0, wp_col);
        painter.circle_stroke(p_wp, 6.0, Stroke::new(1.0_f32, wp_col));
    }

    // Elevation Inset Gauge on the Right
    let gauge_x = rect.max.x - 45.0;
    let gauge_top = rect.min.y + 35.0;
    let gauge_bot = rect.max.y - 35.0;
    let gauge_h = gauge_bot - gauge_top;

    painter.line_segment([Pos2::new(gauge_x, gauge_top), Pos2::new(gauge_x, gauge_bot)], Stroke::new(2.0_f32, Color32::from_rgb(60, 80, 110)));
    painter.text(Pos2::new(gauge_x, gauge_top - 12.0), egui::Align2::CENTER_BOTTOM, "Pitch Arc", egui::FontId::monospace(9.0), Color32::from_rgb(160, 190, 220));

    let min_p = cfg.tracking.pitch_min_deg.clamp(-90.0, 90.0);
    let max_p = cfg.tracking.pitch_max_deg.clamp(-90.0, 90.0);
    let curr_p = metrics.current_pitch_deg.clamp(-90.0, 90.0);

    let to_gauge_y = |p: f32| -> f32 {
        let frac = (p - (-90.0)) / 180.0;
        gauge_bot - frac * gauge_h
    };

    let y_min = to_gauge_y(min_p);
    let y_max = to_gauge_y(max_p);
    let y_curr = to_gauge_y(curr_p);

    painter.line_segment([Pos2::new(gauge_x - 5.0, y_min), Pos2::new(gauge_x + 5.0, y_min)], Stroke::new(1.5_f32, Color32::from_rgb(220, 160, 40)));
    painter.line_segment([Pos2::new(gauge_x - 5.0, y_max), Pos2::new(gauge_x + 5.0, y_max)], Stroke::new(1.5_f32, Color32::from_rgb(220, 160, 40)));
    painter.line_segment([Pos2::new(gauge_x, y_min), Pos2::new(gauge_x, y_max)], Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(220, 160, 40, 120)));

    // Indicator needle
    painter.line_segment([Pos2::new(gauge_x - 10.0, y_curr), Pos2::new(gauge_x + 10.0, y_curr)], Stroke::new(2.5_f32, Color32::WHITE));
    painter.text(Pos2::new(gauge_x - 14.0, y_curr), egui::Align2::RIGHT_CENTER, format!("{:.0}°", curr_p), egui::FontId::monospace(10.0), Color32::WHITE);

    // OSD Telemetry
    let dps = cfg.combat.damage_per_shot * cfg.combat.fire_rate_hz;
    let osd = format!("Yaw: {:.1}° | Pitch: {:.1}° | Range: {:.0} uu | DPS: {:.0} HP/s | Health: {:.0}",
        metrics.current_yaw_deg, metrics.current_pitch_deg, cfg.tracking.tracking_range, dps, cfg.combat.health);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        osd,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(210, 225, 245),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Automated Defense Cannon Radar").strong());
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Weapon: {}", cfg.combat.weapon_type.display_name()));
        ui.label(format!("Team: {}", cfg.team.display_name()));
    });
}

fn render_tracking_timeline(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.turret_settings.is_tracking { "⏸ Pause" } else { "▶ Patrol Swivel" };
            if ui.button(anim_txt).clicked() {
                state.turret_settings.is_tracking = !state.turret_settings.is_tracking;
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_turret_time(0.0);
            }

            ui.label("Patrol Time:");
            let mut t = state.turret_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=10.0).text("s")).changed() {
                state.scrub_turret_time(t);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.turret_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Mount: Brush | Shield: Brush | Actor: TeamCannon ({}) | Cover Nodes: {}",
            cfg.team.display_name(), state.turret_settings.cached_waypoints.len()));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Turret, Mount, Shield & Bot Evasion Nodes to Level").strong())
            .on_hover_text("Spawns semi-solid pedestal, shield visor, Botpack.TeamCannon actor and bot evasion PathNodes (100% Undoable)")
            .clicked()
        {
            state.commit_turret_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_turret_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("turret_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Mount Type
        ui.label("Mount Style:");
        egui::ComboBox::from_id_salt("turret_mount_combo")
            .selected_text(state.turret_settings.active_config.mount.mount_type.display_name())
            .show_ui(ui, |ui| {
                for &m in TurretMountType::all() {
                    let is_sel = state.turret_settings.active_config.mount.mount_type == m;
                    if ui.selectable_label(is_sel, m.display_name()).clicked() {
                        state.turret_settings.active_config.mount.mount_type = m;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Weapon Type
        ui.label("Weapon System:");
        egui::ComboBox::from_id_salt("turret_weapon_combo")
            .selected_text(state.turret_settings.active_config.combat.weapon_type.display_name())
            .show_ui(ui, |ui| {
                for &w in TurretWeaponType::all() {
                    let is_sel = state.turret_settings.active_config.combat.weapon_type == w;
                    if ui.selectable_label(is_sel, w.display_name()).clicked() {
                        state.turret_settings.active_config.combat.weapon_type = w;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Team Ownership
        ui.label("Team Ownership:");
        egui::ComboBox::from_id_salt("turret_team_combo")
            .selected_text(state.turret_settings.active_config.team.display_name())
            .show_ui(ui, |ui| {
                for &t in TurretTeamOwnership::all() {
                    let is_sel = state.turret_settings.active_config.team == t;
                    if ui.selectable_label(is_sel, t.display_name()).clicked() {
                        state.turret_settings.active_config.team = t;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Tracking Range
        ui.label("Tracking Range (uu):");
        if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.tracking.tracking_range).speed(32.0).range(500.0..=4000.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Yaw Min / Max
        ui.label("Yaw Traversal Limits (°):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.tracking.yaw_min_deg).speed(2.0).range(-180.0..=0.0)).changed() {
                changed = true;
            }
            ui.label("to");
            if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.tracking.yaw_max_deg).speed(2.0).range(0.0..=180.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        // Pitch Min / Max
        ui.label("Pitch Elevation Limits (°):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.tracking.pitch_min_deg).speed(2.0).range(-85.0..=30.0)).changed() {
                changed = true;
            }
            ui.label("to");
            if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.tracking.pitch_max_deg).speed(2.0).range(-30.0..=85.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        // Fire Rate
        ui.label("Fire Rate (Hz):");
        if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.combat.fire_rate_hz).speed(0.2).range(0.5..=20.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Damage Per Shot
        ui.label("Damage Per Shot (HP):");
        if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.combat.damage_per_shot).speed(5.0).range(5.0..=250.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Turret Health
        ui.label("Emplacement Health (HP):");
        if ui.add(egui::DragValue::new(&mut state.turret_settings.active_config.combat.health).speed(20.0).range(50.0..=1000.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Armored Shield Plate
        ui.label("Shield Visor Plate:");
        if ui.checkbox(&mut state.turret_settings.active_config.mount.has_shield_plate, "Generate Visor Brush").changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_turret_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.turret_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL TURRET HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Ballistic / Balance Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Defense Turret Setup & Bot Evasion Valid").strong());
            }
        });

        let cfg = &state.turret_settings.active_config;
        let dps = cfg.combat.damage_per_shot * cfg.combat.fire_rate_hz;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("DPS: {:.0} HP/s | TTK vs 100 HP: {:.2}s | Cover Nodes: {}",
                dps, if dps > 0.0 { 100.0 / dps } else { 99.0 }, state.turret_settings.cached_waypoints.len()));
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
