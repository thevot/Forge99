// crates/ut99-editor/src/gui/hazard_chamber_studio_dialog.rs
//
// Interactive Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ChamberDoorStyle, HazardChamberArchetype, HazardDamageType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_hazard_chamber_dialog {
        return;
    }

    let mut open = state.show_hazard_chamber_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Environmental Hazard Chamber & Pressure Airlock Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_hazard_chamber_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Animation tick
    if state.hazard_chamber_settings.is_animating {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let total = state.hazard_chamber_settings.cached_metrics.total_cycle_time_secs.max(1.0);
        let new_t = (state.hazard_chamber_settings.simulated_time_seconds + dt) % total;
        state.hazard_chamber_settings.simulated_time_seconds = new_t;
        ui.ctx().request_repaint();
    }

    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Chamber Archetypes:").strong());
        for &arch in HazardChamberArchetype::all() {
            let is_sel = state.hazard_chamber_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_hazard_chamber_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.hazard_chamber_settings.show_chamber_viewport, "Chamber Shell");
        ui.checkbox(&mut state.hazard_chamber_settings.show_doors_viewport, "Blast Doors");
        ui.checkbox(&mut state.hazard_chamber_settings.show_window_viewport, "Observation Window");
        ui.checkbox(&mut state.hazard_chamber_settings.show_console_viewport, "Console Activator");
        ui.checkbox(&mut state.hazard_chamber_settings.show_gas_volume_viewport, "Hazard Atmosphere");
        ui.checkbox(&mut state.hazard_chamber_settings.show_waypoints_viewport, "Bot Bait/Trap");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_hazard_chamber_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Floorplan & Timeline Animation, Commit
        columns[0].vertical(|ui| {
            ui.heading("2D Chamber Floorplan & Dynamic Vent Cycle");
            render_floorplan_canvas(ui, state);

            ui.add_space(8.0);
            render_timeline_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Hazard Chamber to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Geometry, Timing, Hazard Atmosphere, Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Airlock Geometry & Vent Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Trap Safety & Gameplay Softlock Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_floorplan_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.hazard_chamber_settings.active_config.clone();
    let metrics = state.hazard_chamber_settings.cached_metrics.clone();
    let waypoints = state.hazard_chamber_settings.cached_waypoints.clone();
    let sim_time = state.hazard_chamber_settings.simulated_time_seconds;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 24));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(45, 58, 75)),
        StrokeKind::Inside,
    );

    let center = rect.center();
    let max_dim = (cfg.chamber_width.max(cfg.chamber_length) + 200.0).max(400.0);
    let scale = (rect.height() * 0.75 / max_dim).min(rect.width() * 0.75 / max_dim);

    let to_canvas = |pos_xy: Vec2| -> Pos2 {
        Pos2::new(center.x + pos_xy.x * scale, center.y - pos_xy.y * scale)
    };

    let hw = cfg.chamber_width * 0.5;
    let hl = cfg.chamber_length * 0.5;

    // Chamber Room Outer Bounds
    let p_min = to_canvas(Vec2::new(-hw, -hl));
    let p_max = to_canvas(Vec2::new(hw, hl));
    let chamber_rect = Rect::from_two_pos(p_min, p_max);

    // Door State Calculation
    // t < door_close_delay -> Open (0.0)
    // door_close_delay <= t < door_close_delay + 1.0 -> Closing (0.0 -> 1.0)
    // door_close_delay + 1.0 <= t < total - 1.5 -> Sealed / Venting (1.0)
    // t >= total - 1.5 -> Reopening (1.0 -> 0.0)
    let close_start = cfg.door_close_delay_secs;
    let close_end = close_start + 1.0;
    let reopen_start = metrics.total_cycle_time_secs - 1.5;
    let reopen_end = metrics.total_cycle_time_secs;

    let (door_closed_factor, is_venting, status_str) = if sim_time < close_start {
        (0.0, false, "Doors Open (Awaiting Victim)")
    } else if sim_time < close_end {
        let frac = (sim_time - close_start) / 1.0;
        (frac, false, "Doors Closing (Trap Triggered)")
    } else if sim_time < reopen_start {
        (1.0, true, "SEALED HAZARD ACTIVE (VENTING)")
    } else {
        let frac = 1.0 - (sim_time - reopen_start) / (reopen_end - reopen_start).max(0.1);
        (frac, false, "Doors Reopening (Purge Complete)")
    };

    // Chamber Interior Atmosphere (Gas/Decompression tint)
    if is_venting {
        let gas_col = match cfg.damage_type {
            HazardDamageType::Asphyxiated => Color32::from_rgba_unmultiplied(255, 40, 40, 60), // Red vacuum
            HazardDamageType::Corroded => Color32::from_rgba_unmultiplied(80, 255, 40, 75), // Toxic green
            HazardDamageType::Drowned => Color32::from_rgba_unmultiplied(40, 150, 255, 90), // Water surge
            HazardDamageType::Frozen => Color32::from_rgba_unmultiplied(100, 230, 255, 80), // Cryo cyan
            HazardDamageType::Electrocuted => Color32::from_rgba_unmultiplied(220, 240, 255, 100), // Plasma white
            HazardDamageType::Burned => Color32::from_rgba_unmultiplied(255, 140, 20, 80), // Fire amber
        };
        painter.rect_filled(chamber_rect, 0.0, gas_col);
    } else {
        painter.rect_filled(chamber_rect, 0.0, Color32::from_rgba_unmultiplied(25, 32, 45, 180));
    }

    // Chamber Wall Outlines (Steel)
    painter.rect_stroke(chamber_rect, 0.0, Stroke::new(3.0_f32, Color32::from_rgb(130, 155, 185)), StrokeKind::Middle);

    // Entrance Doorway Cutout (Bottom wall, -Y)
    let dw_half = cfg.door_width * 0.5;
    let door_center_x = 0.0;
    let d_left_x = door_center_x - dw_half;
    let d_right_x = door_center_x + dw_half;
    let p_d_left = to_canvas(Vec2::new(d_left_x, -hl));
    let p_d_right = to_canvas(Vec2::new(d_right_x, -hl));

    // Clear door opening
    painter.line_segment([p_d_left, p_d_right], Stroke::new(4.0_f32, Color32::from_rgb(14, 18, 24)));

    // Sliding Blast Doors
    let slide_travel = dw_half * (1.0 - door_closed_factor);
    let dl_cur_x = d_left_x + dw_half * 0.5 - slide_travel;
    let dr_cur_x = d_right_x - dw_half * 0.5 + slide_travel;

    let p_dl1 = to_canvas(Vec2::new(dl_cur_x - dw_half * 0.5, -hl));
    let p_dl2 = to_canvas(Vec2::new(dl_cur_x + dw_half * 0.5, -hl));
    let p_dr1 = to_canvas(Vec2::new(dr_cur_x - dw_half * 0.5, -hl));
    let p_dr2 = to_canvas(Vec2::new(dr_cur_x + dw_half * 0.5, -hl));

    let door_col = Color32::from_rgb(255, 180, 20); // Hazard Amber
    painter.line_segment([p_dl1, p_dl2], Stroke::new(5.0_f32, door_col));
    painter.line_segment([p_dr1, p_dr2], Stroke::new(5.0_f32, door_col));

    // Observation Window (Right wall, +X)
    if cfg.observation_window && cfg.window_width > 16.0 {
        let win_half = cfg.window_width * 0.5;
        let p_w1 = to_canvas(Vec2::new(hw, -win_half));
        let p_w2 = to_canvas(Vec2::new(hw, win_half));
        painter.line_segment([p_w1, p_w2], Stroke::new(3.5_f32, Color32::from_rgb(80, 220, 255)));
    }

    // Control Console (Outside chamber)
    let p_con = to_canvas(Vec2::new(cfg.console_offset.x, cfg.console_offset.y));
    painter.circle_filled(p_con, 7.0, Color32::from_rgb(255, 70, 70));
    painter.circle_stroke(p_con, 9.0, Stroke::new(1.5_f32, Color32::WHITE));
    painter.text(Pos2::new(p_con.x, p_con.y + 12.0), egui::Align2::CENTER_TOP, "Console Button", egui::FontId::monospace(9.0), Color32::from_rgb(255, 160, 160));

    // Bot Waypoints
    let p_bait = to_canvas(Vec2::new(waypoints.bait_inside.x - cfg.position.x, waypoints.bait_inside.y - cfg.position.y));
    let p_exit = to_canvas(Vec2::new(waypoints.safe_outside_door.x - cfg.position.x, waypoints.safe_outside_door.y - cfg.position.y));

    // Path Line Bait -> Exit
    painter.line_segment([p_bait, p_exit], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(200, 200, 200, 100)));
    painter.line_segment([p_exit, p_con], Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(200, 200, 200, 100)));

    // Bait Node (Gold diamond)
    painter.circle_filled(p_bait, 5.0, Color32::from_rgb(255, 215, 0));
    painter.text(Pos2::new(p_bait.x, p_bait.y - 10.0), egui::Align2::CENTER_BOTTOM, "Bait Node", egui::FontId::monospace(9.0), Color32::from_rgb(255, 215, 0));

    // Safe Exit Node (Cyan)
    painter.circle_filled(p_exit, 5.0, Color32::from_rgb(40, 200, 255));
    painter.text(Pos2::new(p_exit.x, p_exit.y + 10.0), egui::Align2::CENTER_TOP, "Safe Escape", egui::FontId::monospace(9.0), Color32::from_rgb(40, 200, 255));

    // Telemetry OSD
    let osd = format!("Time: {:.1}s / {:.1}s | State: {} | Damage: {:.0} HP/s ({}) | TTK: {:.1}s",
        sim_time, metrics.total_cycle_time_secs, status_str, cfg.damage_per_sec, cfg.damage_type.display_name(), metrics.time_to_kill_secs);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        osd,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(220, 235, 255),
    );
}

fn render_timeline_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    let total_time = state.hazard_chamber_settings.cached_metrics.total_cycle_time_secs;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim = &mut state.hazard_chamber_settings.is_animating;
            let icon = if *anim { "⏸ Pause" } else { "▶ Play" };
            if ui.button(icon).clicked() {
                *anim = !*anim;
            }

            ui.label(egui::RichText::new("Airlock Vent Cycle Timeline:").strong());
            let mut t = state.hazard_chamber_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=total_time).text("s")).changed() {
                state.scrub_hazard_chamber_time(t);
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_hazard_chamber_time(0.0);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.hazard_chamber_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Size: {:.0}x{:.0}x{:.0} uu | Doors: {} | Tag: {}",
            cfg.chamber_width, cfg.chamber_length, cfg.chamber_height, cfg.door_style.display_name(), cfg.tag));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Hazard Chamber, Blast Doors, Zone & Trigger to Level").strong())
            .on_hover_text("Spawns room shell, sliding doors, window, console, ZoneInfo hazard, console Trigger, and bot waypoints (100% Undoable)")
            .clicked()
        {
            state.commit_hazard_chamber_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_hazard_chamber_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("hazard_chamber_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Dimensions
        ui.label("Chamber Width x Length:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hazard_chamber_settings.active_config.chamber_width).range(256.0..=1024.0).speed(16.0)).changed() {
                changed = true;
            }
            ui.label("x");
            if ui.add(egui::DragValue::new(&mut state.hazard_chamber_settings.active_config.chamber_length).range(256.0..=1024.0).speed(16.0)).changed() {
                changed = true;
            }
            ui.label("uu");
        });
        ui.end_row();

        ui.label("Chamber Height:");
        if ui.add(egui::Slider::new(&mut state.hazard_chamber_settings.active_config.chamber_height, 128.0..=512.0).text("uu")).changed() {
            changed = true;
        }
        ui.end_row();

        // Door Style
        ui.label("Door Mechanism:");
        egui::ComboBox::from_id_salt("chamber_door_style_combo")
            .selected_text(state.hazard_chamber_settings.active_config.door_style.display_name())
            .show_ui(ui, |ui| {
                for &d in ChamberDoorStyle::all() {
                    let is_sel = state.hazard_chamber_settings.active_config.door_style == d;
                    if ui.selectable_label(is_sel, d.display_name()).clicked() {
                        state.hazard_chamber_settings.active_config.door_style = d;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Door Width & Height
        ui.label("Door Width & Height:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.hazard_chamber_settings.active_config.door_width).range(64.0..=256.0).speed(8.0)).changed() {
                changed = true;
            }
            ui.label("x");
            if ui.add(egui::DragValue::new(&mut state.hazard_chamber_settings.active_config.door_height).range(128.0..=384.0).speed(8.0)).changed() {
                changed = true;
            }
            ui.label("uu");
        });
        ui.end_row();

        // Observation Window
        ui.label("Observation Window:");
        if ui.checkbox(&mut state.hazard_chamber_settings.active_config.observation_window, "Enable Glass Pane").changed() {
            changed = true;
        }
        ui.end_row();

        // Cycle Timing
        ui.label("Door Close Delay:");
        if ui.add(egui::Slider::new(&mut state.hazard_chamber_settings.active_config.door_close_delay_secs, 0.2..=4.0).text("s")).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Vent Cycle Duration:");
        if ui.add(egui::Slider::new(&mut state.hazard_chamber_settings.active_config.vent_cycle_duration_secs, 2.0..=15.0).text("s")).changed() {
            changed = true;
        }
        ui.end_row();

        // Damage Type
        ui.label("Hazard Damage Type:");
        egui::ComboBox::from_id_salt("hazard_dmg_type_combo")
            .selected_text(state.hazard_chamber_settings.active_config.damage_type.display_name())
            .show_ui(ui, |ui| {
                for &dmg in HazardDamageType::all() {
                    let is_sel = state.hazard_chamber_settings.active_config.damage_type == dmg;
                    if ui.selectable_label(is_sel, dmg.display_name()).clicked() {
                        state.hazard_chamber_settings.active_config.damage_type = dmg;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Damage Per Sec
        ui.label("Damage Per Second:");
        ui.horizontal(|ui| {
            if ui.add(egui::Slider::new(&mut state.hazard_chamber_settings.active_config.damage_per_sec, 0.0..=500.0).text("HP/s")).changed() {
                changed = true;
            }
            if ui.checkbox(&mut state.hazard_chamber_settings.active_config.b_kill_zone, "Instant KillZone").changed() {
                changed = true;
            }
        });
        ui.end_row();

        // Console Trigger Event Name
        ui.label("Trigger Event Name:");
        if ui.text_edit_singleline(&mut state.hazard_chamber_settings.active_config.event_name).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_hazard_chamber_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.hazard_chamber_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL CHAMBER HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Gameplay Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Airlock & Hazard Chamber Valid").strong());
            }
        });

        let metrics = &state.hazard_chamber_settings.cached_metrics;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("Cycle: {:.1}s | Evac Speed: {:.0} uu/s | TTK: {:.2}s | Instant Kill: {}",
                metrics.total_cycle_time_secs, metrics.effective_air_evacuation_speed_uu_s, metrics.time_to_kill_secs, metrics.is_instant_death));
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
