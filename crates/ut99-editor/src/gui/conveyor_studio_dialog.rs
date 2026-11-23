// crates/ut99-editor/src/gui/conveyor_studio_dialog.rs
//
// Interactive Industrial Conveyor Belt, Treadmill & Surface Velocity Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ConveyorArchetype, ConveyorDirection};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_conveyor_dialog {
        return;
    }

    let mut open = state.show_conveyor_dialog;

    // Handle conveyor belt animation playback
    if state.conveyor_settings.is_running {
        let dt = ctx.input(|i| i.stable_dt).min(0.1);
        state.conveyor_settings.simulated_time_seconds += dt;
        state.recalculate_conveyor_preview();
        ctx.request_repaint();
    }

    crate::gui::modal::studio_dialog_window(ctx, "Industrial Conveyor Belt, Treadmill & Surface Velocity Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_conveyor_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Conveyor Archetypes:").strong());
        for &arch in ConveyorArchetype::all() {
            let is_sel = state.conveyor_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_conveyor_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.conveyor_settings.show_deck_viewport, "Conveyor Deck");
        ui.checkbox(&mut state.conveyor_settings.show_rollers_viewport, "End Rollers");
        ui.checkbox(&mut state.conveyor_settings.show_guardrails_viewport, "Side Guardrails & Legs");
        ui.checkbox(&mut state.conveyor_settings.show_velocity_vectors_viewport, "Surface Velocity Arrows");
        ui.checkbox(&mut state.conveyor_settings.show_waypoints_viewport, "Bot AI Waypoints");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_conveyor_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Elevation Canvas, Timeline, Commit Action
        columns[0].vertical(|ui| {
            ui.heading("2D Side Elevation & Moving Tread Surface");
            render_conveyor_canvas(ui, state);

            ui.add_space(8.0);
            render_running_timeline(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Conveyor Setup to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Geometry, Kinematics, Panning Sync, Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Belt Geometry, Kinematics & Sync");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Kinematic Safety & Bot Pathing Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_conveyor_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let cfg = &state.conveyor_settings.active_config;
    let deck_l = cfg.deck.length.max(128.0);
    let incline_deg = cfg.deck.incline_angle_deg;
    let rise = deck_l * incline_deg.to_radians().sin();
    let total_h = (rise.abs() + 120.0).max(180.0);

    let avail_w = ui.available_width().max(200.0);
    let target_h = (avail_w * (total_h / deck_l)).clamp(180.0, 280.0);
    let target_w = avail_w;

    let (rect, response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(55, 65, 80)),
        StrokeKind::Inside,
    );

    let margin_x = 45.0;
    let margin_y = 35.0;
    let usable_w = rect.width() - margin_x * 2.0;
    let usable_h = rect.height() - margin_y * 2.0;

    // Coordinate mapping: Left is intake (u=0), Right is discharge (u=1)
    let to_canvas = |u: f32, z_offset: f32| -> Pos2 {
        let elev_frac = if incline_deg >= 0.0 { u } else { 1.0 - u };
        let elev_y = elev_frac * (rise.abs() / total_h) * usable_h;
        let base_y = rect.max.y - margin_y - elev_y;
        Pos2::new(
            rect.min.x + margin_x + u * usable_w,
            base_y - (z_offset / total_h) * usable_h,
        )
    };

    let p_in_bot = to_canvas(0.0, 0.0);
    let p_out_bot = to_canvas(1.0, 0.0);
    let p_in_top = to_canvas(0.0, cfg.deck.thickness);
    let p_out_top = to_canvas(1.0, cfg.deck.thickness);

    // Support Legs underneath
    if cfg.guardrails.has_support_legs {
        let leg_count = 5;
        for i in 0..leg_count {
            let u = i as f32 / (leg_count - 1) as f32;
            let top_pt = to_canvas(u, 0.0);
            let bot_pt = Pos2::new(top_pt.x, rect.max.y - margin_y * 0.5);
            painter.line_segment([top_pt, bot_pt], Stroke::new(3.0_f32, Color32::from_rgb(70, 85, 105)));
        }
    }

    // Main Conveyor Deck Body
    let deck_poly = vec![p_in_bot, p_out_bot, p_out_top, p_in_top];
    painter.add(egui::Shape::convex_polygon(deck_poly, Color32::from_rgb(35, 42, 54), Stroke::new(2.0_f32, Color32::from_rgb(90, 110, 140))));

    // Belt Top Surface (amber/orange)
    painter.line_segment([p_in_top, p_out_top], Stroke::new(4.0_f32, Color32::from_rgb(230, 170, 40)));

    // Moving Surface Cleats / Tread Ribs
    let cleat_count = 10;
    let speed = cfg.kinematics.speed_uu_per_sec;
    let dir_sign = match cfg.kinematics.direction {
        ConveyorDirection::Forward => 1.0,
        ConveyorDirection::Backward => -1.0,
        ConveyorDirection::Stopped => 0.0,
    };
    let time = state.conveyor_settings.simulated_time_seconds;
    let phase_base = (time * (speed / deck_l) * dir_sign).rem_euclid(1.0);

    for c in 0..cleat_count {
        let u = ((c as f32 / cleat_count as f32) + phase_base).rem_euclid(1.0);
        let cleat_base = to_canvas(u, cfg.deck.thickness);
        let cleat_top = to_canvas(u, cfg.deck.thickness + 4.0);
        painter.line_segment([cleat_base, cleat_top], Stroke::new(2.0_f32, Color32::from_rgb(255, 215, 80)));
    }

    // Side Guardrails
    if cfg.guardrails.has_guardrails {
        let rail_h = cfg.guardrails.rail_height;
        let r_in_top = to_canvas(0.0, cfg.deck.thickness + rail_h);
        let r_out_top = to_canvas(1.0, cfg.deck.thickness + rail_h);
        painter.line_segment([r_in_top, r_out_top], Stroke::new(2.5_f32, Color32::from_rgb(120, 150, 190)));
        painter.line_segment([p_in_top, r_in_top], Stroke::new(2.0_f32, Color32::from_rgb(120, 150, 190)));
        painter.line_segment([p_out_top, r_out_top], Stroke::new(2.0_f32, Color32::from_rgb(120, 150, 190)));
    }

    // End Rollers (Cylinders at intake and discharge)
    if cfg.rollers.has_end_rollers {
        let r_screen = (cfg.rollers.roller_radius / total_h) * usable_h;
        let r_in_center = to_canvas(0.0, cfg.deck.thickness * 0.5);
        let r_out_center = to_canvas(1.0, cfg.deck.thickness * 0.5);

        painter.circle_filled(r_in_center, r_screen.clamp(4.0, 18.0), Color32::from_rgb(60, 75, 95));
        painter.circle_stroke(r_in_center, r_screen.clamp(4.0, 18.0), Stroke::new(2.0_f32, Color32::from_rgb(180, 200, 230)));

        painter.circle_filled(r_out_center, r_screen.clamp(4.0, 18.0), Color32::from_rgb(60, 75, 95));
        painter.circle_stroke(r_out_center, r_screen.clamp(4.0, 18.0), Stroke::new(2.0_f32, Color32::from_rgb(180, 200, 230)));
    }

    // Surface Velocity Flow Vector Arrows
    let arrow_count = 4;
    let arrow_col = Color32::from_rgb(60, 220, 255);
    for a in 0..arrow_count {
        let u_mid = (a as f32 + 0.5) / arrow_count as f32;
        let p_arrow_base = to_canvas(u_mid - 0.03 * dir_sign, cfg.deck.thickness + 12.0);
        let p_arrow_tip = to_canvas(u_mid + 0.03 * dir_sign, cfg.deck.thickness + 12.0);
        painter.line_segment([p_arrow_base, p_arrow_tip], Stroke::new(2.0_f32, arrow_col));
    }

    // Bot Waypoints (Intake & Discharge PathNodes)
    if cfg.bot_nav.generate_bot_waypoints {
        let p_intake_node = Pos2::new(p_in_top.x - 24.0, p_in_top.y - 15.0);
        let p_discharge_node = Pos2::new(p_out_top.x + 24.0, p_out_top.y - 15.0);

        painter.circle_filled(p_intake_node, 5.0, Color32::from_rgb(80, 220, 120));
        painter.line_segment([p_intake_node, p_in_top], Stroke::new(1.5_f32, Color32::from_rgb(80, 220, 120)));
        painter.text(Pos2::new(p_intake_node.x, p_intake_node.y - 10.0), egui::Align2::CENTER_BOTTOM, "Intake Node", egui::FontId::monospace(9.0), Color32::from_rgb(80, 220, 120));

        painter.circle_filled(p_discharge_node, 5.0, Color32::from_rgb(255, 140, 40));
        painter.line_segment([p_discharge_node, p_out_top], Stroke::new(1.5_f32, Color32::from_rgb(255, 140, 40)));
        painter.text(Pos2::new(p_discharge_node.x, p_discharge_node.y - 10.0), egui::Align2::CENTER_BOTTOM, "Discharge Node", egui::FontId::monospace(9.0), Color32::from_rgb(255, 140, 40));
    }

    // Player Silhouette reference standing on the belt
    let p_pos = to_canvas(0.5, cfg.deck.thickness);
    let p_head = Pos2::new(p_pos.x, p_pos.y - (78.0 / total_h) * usable_h);
    painter.line_segment([p_pos, p_head], Stroke::new(4.0_f32, Color32::from_rgba_unmultiplied(120, 190, 255, 180)));
    painter.circle_filled(p_head, 5.0, Color32::from_rgba_unmultiplied(120, 190, 255, 200));

    // Telemetry OSD
    let telemetry = format!("Speed: {:.0} uu/s | Pan Rate: {:.0} px/s | Incline: {:.1}° | Roller: {:.0} RPM",
        speed, state.conveyor_settings.cached_metrics.texture_pan_rate_pixels_sec, incline_deg, state.conveyor_settings.cached_metrics.roller_rpm);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        telemetry,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(210, 225, 245),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Industrial Conveyor Belt & Surface Velocity").strong());
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Dimensions: {:.0} x {:.0} x {:.0} uu", cfg.deck.length, cfg.deck.width, cfg.deck.thickness));
        ui.label(format!("Direction: {}", cfg.kinematics.direction.display_name()));
    });
}

fn render_running_timeline(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim_txt = if state.conveyor_settings.is_running { "⏸ Pause" } else { "▶ Run Belt" };
            if ui.button(anim_txt).clicked() {
                state.conveyor_settings.is_running = !state.conveyor_settings.is_running;
            }

            if ui.button("⏮ Reset").clicked() {
                state.scrub_conveyor_time(0.0);
            }

            ui.label("Simulation Time:");
            let mut t = state.conveyor_settings.simulated_time_seconds;
            if ui.add(egui::Slider::new(&mut t, 0.0..=10.0).text("s")).changed() {
                state.scrub_conveyor_time(t);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.conveyor_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Deck: Brush | Rollers: Brush | Frame: Brush | Zone: ZoneInfo ({:.0} uu/s) | Nodes: {}",
            cfg.kinematics.speed_uu_per_sec, if cfg.bot_nav.generate_bot_waypoints { 2 } else { 0 }));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Conveyor, Rollers, Frame, ZoneInfo & Bot Waypoints to Level").strong())
            .on_hover_text("Spawns semi-solid deck, rollers, guardrails, Engine.ZoneInfo and bot AI PathNodes (100% Undoable)")
            .clicked()
        {
            state.commit_conveyor_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_conveyor_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("conveyor_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Deck Length
        ui.label("Belt Length (uu):");
        if ui.add(egui::DragValue::new(&mut state.conveyor_settings.active_config.deck.length).speed(16.0).range(128.0..=2048.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Deck Width
        ui.label("Belt Width (uu):");
        if ui.add(egui::DragValue::new(&mut state.conveyor_settings.active_config.deck.width).speed(8.0).range(48.0..=512.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Deck Thickness
        ui.label("Belt Thickness (uu):");
        if ui.add(egui::DragValue::new(&mut state.conveyor_settings.active_config.deck.thickness).speed(2.0).range(8.0..=64.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Incline Angle
        ui.label("Incline Angle (°):");
        if ui.add(egui::Slider::new(&mut state.conveyor_settings.active_config.deck.incline_angle_deg, -35.0..=35.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Linear Speed
        ui.label("Surface Speed (uu/s):");
        if ui.add(egui::DragValue::new(&mut state.conveyor_settings.active_config.kinematics.speed_uu_per_sec).speed(16.0).range(0.0..=1500.0)).changed() {
            changed = true;
        }
        ui.end_row();

        // Operational Direction
        ui.label("Flow Direction:");
        egui::ComboBox::from_id_salt("conveyor_dir_combo")
            .selected_text(state.conveyor_settings.active_config.kinematics.direction.display_name())
            .show_ui(ui, |ui| {
                for &dir in ConveyorDirection::all() {
                    let is_sel = state.conveyor_settings.active_config.kinematics.direction == dir;
                    if ui.selectable_label(is_sel, dir.display_name()).clicked() {
                        state.conveyor_settings.active_config.kinematics.direction = dir;
                        changed = true;
                    }
                }
            });
        ui.end_row();

        // Synchronized Pan Rate readout
        ui.label("Synchronized Pan Rate:");
        ui.colored_label(
            Color32::from_rgb(80, 220, 120),
            format!("{:.1} px/s (1:1 lockstep)", state.conveyor_settings.cached_metrics.texture_pan_rate_pixels_sec),
        );
        ui.end_row();

        // End Rollers
        ui.label("End Rollers:");
        if ui.checkbox(&mut state.conveyor_settings.active_config.rollers.has_end_rollers, "Generate Cylinders").changed() {
            changed = true;
        }
        ui.end_row();

        // Guardrails
        ui.label("Side Guardrails:");
        if ui.checkbox(&mut state.conveyor_settings.active_config.guardrails.has_guardrails, "Safety Rails & Legs").changed() {
            changed = true;
        }
        ui.end_row();

        // Bot AI Waypoints
        ui.label("Bot AI Waypoints:");
        if ui.checkbox(&mut state.conveyor_settings.active_config.bot_nav.generate_bot_waypoints, "Intake/Discharge PathNodes").changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_conveyor_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.conveyor_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL CONVEYOR HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Kinematic Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Conveyor Kinematics & Bot Pathing Valid").strong());
            }
        });

        let metrics = &state.conveyor_settings.cached_metrics;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("Velocity: ({:.0}, {:.0}, {:.0}) uu/s | Rise: {:.0} uu",
                metrics.linear_velocity.x, metrics.linear_velocity.y, metrics.linear_velocity.z, metrics.incline_rise_height));
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
