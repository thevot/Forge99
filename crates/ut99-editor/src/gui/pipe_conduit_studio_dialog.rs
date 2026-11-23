// crates/ut99-editor/src/gui/pipe_conduit_studio_dialog.rs
//
// Interactive Industrial Pipe Network, Steam Valve & Conduit Junction Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::PipeArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_pipe_conduit_dialog {
        return;
    }

    let mut open = state.show_pipe_conduit_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Industrial Pipe Network & Steam Valve Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_pipe_conduit_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Flow animation tick
    if state.pipe_conduit_settings.is_animating {
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        let speed = (state.pipe_conduit_settings.cached_metrics.flow_velocity_uu_s / 200.0).max(0.2);
        state.pipe_conduit_settings.simulated_flow_time_secs =
            (state.pipe_conduit_settings.simulated_flow_time_secs + dt * speed) % 10.0;
        ui.ctx().request_repaint();
    }

    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Pipe Archetypes:").strong());
        for &arch in PipeArchetype::all() {
            let is_sel = state.pipe_conduit_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_pipe_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.pipe_conduit_settings.show_pipe_viewport, "Straight Pipe");
        ui.checkbox(&mut state.pipe_conduit_settings.show_elbow_viewport, "Elbow Bends");
        ui.checkbox(&mut state.pipe_conduit_settings.show_flanges_viewport, "Bolt Flanges");
        ui.checkbox(&mut state.pipe_conduit_settings.show_valve_viewport, "Relief Valve");
        ui.checkbox(&mut state.pipe_conduit_settings.show_steam_hazard_viewport, "Steam Hazard Jet");
        ui.checkbox(&mut state.pipe_conduit_settings.show_flow_centerline_viewport, "Flow Spine");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_pipe_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Flow Schematic & Elevation, Commit
        columns[0].vertical(|ui| {
            ui.heading("2D Pipe Network Elevation & Kinetic Fluid Flow");
            render_flow_schematic_canvas(ui, state);

            ui.add_space(8.0);
            render_valve_scrubber(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Pipe Network to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Geometry, Elbows, Flanges, Valve Hazard, Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Hydraulic Geometry & Network Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Hydraulic Clearance & Pinch Hazard Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_flow_schematic_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.pipe_conduit_settings.active_config.clone();
    let metrics = state.pipe_conduit_settings.cached_metrics.clone();
    let flow_time = state.pipe_conduit_settings.simulated_flow_time_secs;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 280.0;
    let target_w = avail_w;

    let (rect, _response) = ui.allocate_exact_size(Vec2::new(target_w, target_h), egui::Sense::hover());
    let painter = ui.painter_at(rect);

    // Canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 20, 26));
    painter.rect_stroke(
        rect,
        4.0,
        Stroke::new(2.0_f32, Color32::from_rgb(45, 58, 75)),
        StrokeKind::Inside,
    );

    // Scale calculation: Fit total length + elbow in width
    let total_horiz = cfg.total_run_length_uu + if cfg.has_elbow_bend { cfg.elbow_radius_uu + 64.0 } else { 64.0 };
    let total_vert = (cfg.pipe_outer_radius * 4.0).max(if cfg.has_elbow_bend { cfg.elbow_radius_uu + 64.0 } else { 128.0 });

    let scale_x = (rect.width() - 80.0) / total_horiz.max(100.0);
    let scale_y = (rect.height() - 80.0) / total_vert.max(100.0);
    let scale = scale_x.min(scale_y).clamp(0.15, 1.2);

    let start_pos = Pos2::new(rect.min.x + 40.0, rect.center().y + 30.0);

    let r_scaled = cfg.pipe_outer_radius * scale;
    let len_scaled = cfg.total_run_length_uu * scale;

    // Pipe outer shell color based on archetype
    let pipe_col = match cfg.archetype {
        PipeArchetype::HighPressureSteamMain => Color32::from_rgb(140, 110, 80), // Rusty iron
        PipeArchetype::BiohazardCoolantConduit => Color32::from_rgb(40, 160, 60), // Toxic green
        PipeArchetype::SubterraneanSewerAqueduct => Color32::from_rgb(90, 95, 105), // Cast iron
        PipeArchetype::CryogenicFuelTransferLine => Color32::from_rgb(60, 150, 200), // Frost cyan
        PipeArchetype::HydraulicControlBundle => Color32::from_rgb(180, 120, 50), // Copper
    };

    // 1. Wall Pass-Through Collar at start
    if cfg.has_wall_collars {
        let col_r = r_scaled * 1.5;
        let p_col_top = Pos2::new(start_pos.x - 6.0, start_pos.y - col_r);
        let col_rect = Rect::from_min_size(p_col_top, Vec2::new(12.0, col_r * 2.0));
        painter.rect_filled(col_rect, 2.0, Color32::from_rgb(80, 85, 95));
        painter.rect_stroke(col_rect, 2.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);
    }

    // 2. Straight Run Body
    let straight_rect = Rect::from_min_size(
        Pos2::new(start_pos.x, start_pos.y - r_scaled),
        Vec2::new(len_scaled, r_scaled * 2.0),
    );
    painter.rect_filled(straight_rect, 0.0, pipe_col);
    painter.rect_stroke(straight_rect, 0.0, Stroke::new(2.5_f32, Color32::from_rgb(220, 230, 245)), StrokeKind::Inside);

    // Wall thickness inner core (fluid channel)
    let inner_r_scaled = ((cfg.pipe_outer_radius - cfg.pipe_wall_thickness).max(1.0)) * scale;
    let core_rect = Rect::from_min_size(
        Pos2::new(start_pos.x, start_pos.y - inner_r_scaled),
        Vec2::new(len_scaled, inner_r_scaled * 2.0),
    );
    let fluid_bg = Color32::from_rgba_unmultiplied(20, 25, 35, 200);
    painter.rect_filled(core_rect, 0.0, fluid_bg);

    // 3. Flanges
    if cfg.has_flanges && cfg.flange_spacing_uu > 32.0 {
        let fl_r_scaled = cfg.flange_outer_radius * scale;
        let fl_th_scaled = (cfg.flange_thickness_uu * scale).max(4.0);
        let count = (cfg.total_run_length_uu / cfg.flange_spacing_uu).floor() as usize + 1;

        for k in 0..count {
            let offset_x = (k as f32 * cfg.flange_spacing_uu * scale).min(len_scaled - fl_th_scaled);
            let f_pos = Pos2::new(start_pos.x + offset_x, start_pos.y - fl_r_scaled);
            let f_rect = Rect::from_min_size(f_pos, Vec2::new(fl_th_scaled, fl_r_scaled * 2.0));

            painter.rect_filled(f_rect, 1.0, Color32::from_rgb(240, 190, 40)); // Gold/Hazard Yellow
            painter.rect_stroke(f_rect, 1.0, Stroke::new(1.0_f32, Color32::BLACK), StrokeKind::Inside);

            // Bolt dots
            painter.circle_filled(Pos2::new(f_pos.x + fl_th_scaled * 0.5, f_pos.y + 4.0), 1.5, Color32::BLACK);
            painter.circle_filled(Pos2::new(f_pos.x + fl_th_scaled * 0.5, f_pos.y + fl_r_scaled * 2.0 - 4.0), 1.5, Color32::BLACK);
        }
    }

    // 4. Elbow Bend (if enabled)
    if cfg.has_elbow_bend && cfg.elbow_angle_deg > 0.0 {
        let elbow_end_x = start_pos.x + len_scaled;
        let arc_r = cfg.elbow_radius_uu * scale;
        let elbow_center = Pos2::new(elbow_end_x, start_pos.y - arc_r);

        let mut pts_outer = Vec::new();
        let mut pts_inner = Vec::new();
        let steps = 10;
        let total_ang = cfg.elbow_angle_deg.to_radians();

        for s in 0..=steps {
            let theta = (s as f32 / steps as f32) * total_ang;
            let ang = std::f32::consts::FRAC_PI_2 - theta;
            let cx = elbow_center.x + ang.cos() * arc_r;
            let cy = elbow_center.y + ang.sin() * arc_r;

            pts_outer.push(Pos2::new(cx, cy - r_scaled));
            pts_inner.push(Pos2::new(cx, cy + r_scaled));
        }

        for i in 0..steps {
            painter.line_segment([pts_outer[i], pts_outer[i + 1]], Stroke::new(2.5_f32, Color32::from_rgb(220, 230, 245)));
            painter.line_segment([pts_inner[i], pts_inner[i + 1]], Stroke::new(2.5_f32, Color32::from_rgb(220, 230, 245)));
        }
    }

    // 5. Relief Valve & Handwheel
    if cfg.has_relief_valve {
        let v_center_x = start_pos.x + len_scaled * 0.5;
        let v_hw = (r_scaled * 1.4).max(8.0);

        // Valve cast body
        let v_body_rect = Rect::from_min_size(
            Pos2::new(v_center_x - v_hw, start_pos.y - v_hw),
            Vec2::new(v_hw * 2.0, v_hw * 2.0),
        );
        painter.rect_filled(v_body_rect, 3.0, Color32::from_rgb(180, 140, 50)); // Brass
        painter.rect_stroke(v_body_rect, 3.0, Stroke::new(1.5_f32, Color32::WHITE), StrokeKind::Inside);

        // Valve stem stalk
        let stem_top = start_pos.y - r_scaled - 22.0 * scale;
        painter.line_segment(
            [Pos2::new(v_center_x, start_pos.y - r_scaled), Pos2::new(v_center_x, stem_top)],
            Stroke::new(3.0_f32, Color32::from_rgb(200, 200, 210)),
        );

        // Handwheel rim
        let wheel_r = cfg.valve_handwheel_radius * scale;
        let wheel_center = Pos2::new(v_center_x, stem_top);
        painter.circle_filled(wheel_center, wheel_r, Color32::from_rgb(220, 40, 40)); // Red wheel
        painter.circle_stroke(wheel_center, wheel_r, Stroke::new(2.0_f32, Color32::WHITE));

        // Spokes
        let spokes_rot = flow_time * 2.0 * cfg.valve_open_fraction;
        for i in 0..4 {
            let a = spokes_rot + (i as f32 / 4.0) * std::f32::consts::TAU;
            let sp_end = Pos2::new(wheel_center.x + a.cos() * wheel_r, wheel_center.y + a.sin() * wheel_r);
            painter.line_segment([wheel_center, sp_end], Stroke::new(2.0_f32, Color32::WHITE));
        }

        // Steam Hazard Relief Jet
        if cfg.steam_hazard_enabled {
            let jet_origin = Pos2::new(v_center_x, start_pos.y + r_scaled);
            let jet_len = 35.0 * scale;
            let jet_half_spread = 15.0 * scale;

            let jet_p1 = Pos2::new(jet_origin.x - jet_half_spread, jet_origin.y + jet_len);
            let jet_p2 = Pos2::new(jet_origin.x + jet_half_spread, jet_origin.y + jet_len);

            let jet_col = Color32::from_rgba_unmultiplied(220, 240, 255, 120);
            painter.add(egui::Shape::convex_polygon(vec![jet_origin, jet_p1, jet_p2], jet_col, Stroke::NONE));

            painter.text(
                Pos2::new(jet_origin.x, jet_origin.y + jet_len + 5.0),
                egui::Align2::CENTER_TOP,
                format!("STEAM RELIEF ({:.0} HP/s)", cfg.scald_damage_per_sec),
                egui::FontId::monospace(8.0),
                Color32::from_rgb(255, 160, 160),
            );
        }
    }

    // 6. Kinetic Flow Particles (traveling dots inside pipe core)
    let particle_count = 12;
    for i in 0..particle_count {
        let t = ((flow_time * 0.2 + (i as f32 / particle_count as f32)) % 1.0) * len_scaled;
        let px = start_pos.x + t;
        let py = start_pos.y;

        let dot_col = match cfg.archetype {
            PipeArchetype::BiohazardCoolantConduit => Color32::from_rgb(80, 255, 80),
            PipeArchetype::CryogenicFuelTransferLine => Color32::from_rgb(120, 220, 255),
            PipeArchetype::HighPressureSteamMain => Color32::from_rgb(240, 245, 255),
            _ => Color32::from_rgb(255, 200, 100),
        };
        painter.circle_filled(Pos2::new(px, py), 2.5, dot_col);
    }

    // Telemetry OSD
    let osd = format!("Length: {:.0} uu | Volume: {:.1} L | Velocity: {:.0} uu/s | Drop: {:.2} psi | Valve: {:.0}%",
        metrics.total_pipe_length_uu, metrics.total_fluid_volume_liters, metrics.flow_velocity_uu_s, metrics.pressure_drop_psi, cfg.valve_open_fraction * 100.0);
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.min.y + 6.0),
        egui::Align2::LEFT_TOP,
        osd,
        egui::FontId::monospace(10.0),
        Color32::from_rgb(220, 235, 255),
    );
}

fn render_valve_scrubber(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            let anim = &mut state.pipe_conduit_settings.is_animating;
            let icon = if *anim { "⏸ Pause" } else { "▶ Play" };
            if ui.button(icon).clicked() {
                *anim = !*anim;
            }

            ui.label(egui::RichText::new("Valve Handwheel Position:").strong());
            let mut open_pct = state.pipe_conduit_settings.active_config.valve_open_fraction * 100.0;
            if ui.add(egui::Slider::new(&mut open_pct, 0.0..=100.0).text("%")).changed() {
                state.scrub_pipe_valve_position(open_pct / 100.0);
            }

            if ui.button("Shut Off").clicked() {
                state.scrub_pipe_valve_position(0.0);
            }
            if ui.button("Full Open").clicked() {
                state.scrub_pipe_valve_position(1.0);
            }
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.pipe_conduit_settings.active_config;

    ui.horizontal(|ui| {
        ui.label(format!("Pipe: {:.0} uu dia | Length: {:.0} uu | Elbow: {} | Flanges: {} | Tag: {}",
            cfg.pipe_outer_radius * 2.0, cfg.total_run_length_uu, cfg.has_elbow_bend, cfg.has_flanges, cfg.tag));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("✨ Commit Pipe Run, Elbows, Flanges, Valve & Hazard to Level").strong())
            .on_hover_text("Spawns straight pipe, elbow bend, bolt flanges, wall collar, valve body, valve handwheel mover, and steam hazard trigger (100% Undoable)")
            .clicked()
        {
            state.commit_pipe_conduit_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_pipe_preview();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("pipe_conduit_params_grid").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
        // Outer Radius
        ui.label("Pipe Outer Radius:");
        ui.horizontal(|ui| {
            if ui.add(egui::Slider::new(&mut state.pipe_conduit_settings.active_config.pipe_outer_radius, 8.0..=64.0).text("uu")).changed() {
                changed = true;
            }
            ui.label(format!("(Dia: {:.0} uu)", state.pipe_conduit_settings.active_config.pipe_outer_radius * 2.0));
        });
        ui.end_row();

        // Wall Thickness & Facets
        ui.label("Wall Thickness & Facets:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.pipe_wall_thickness).range(1.0..=16.0).speed(0.5)).changed() {
                changed = true;
            }
            ui.label("uu |");
            if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.cylinder_facets).range(6..=16).speed(1)).changed() {
                changed = true;
            }
            ui.label("sides");
        });
        ui.end_row();

        // Total Run Length
        ui.label("Straight Run Length:");
        if ui.add(egui::Slider::new(&mut state.pipe_conduit_settings.active_config.total_run_length_uu, 128.0..=1024.0).text("uu")).changed() {
            changed = true;
        }
        ui.end_row();

        // Elbow Bends
        ui.label("Elbow Bend:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.pipe_conduit_settings.active_config.has_elbow_bend, "Enable").changed() {
                changed = true;
            }
            if state.pipe_conduit_settings.active_config.has_elbow_bend {
                if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.elbow_radius_uu).range(32.0..=256.0).speed(4.0)).changed() {
                    changed = true;
                }
                ui.label("R |");
                if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.elbow_angle_deg).range(15.0..=90.0).speed(5.0)).changed() {
                    changed = true;
                }
                ui.label("deg");
            }
        });
        ui.end_row();

        // Flanges
        ui.label("Bolt Flanges:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.pipe_conduit_settings.active_config.has_flanges, "Enable").changed() {
                changed = true;
            }
            if state.pipe_conduit_settings.active_config.has_flanges {
                ui.label("Spacing:");
                if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.flange_spacing_uu).range(64.0..=512.0).speed(16.0)).changed() {
                    changed = true;
                }
                ui.label("uu");
            }
        });
        ui.end_row();

        // Wall Collars
        ui.label("Wall Mounting Collars:");
        if ui.checkbox(&mut state.pipe_conduit_settings.active_config.has_wall_collars, "Wall Collar Plates").changed() {
            changed = true;
        }
        ui.end_row();

        // Relief Valve
        ui.label("Relief Valve & Mover:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.pipe_conduit_settings.active_config.has_relief_valve, "Handwheel Valve").changed() {
                changed = true;
            }
            if state.pipe_conduit_settings.active_config.has_relief_valve {
                ui.label("Wheel R:");
                if ui.add(egui::DragValue::new(&mut state.pipe_conduit_settings.active_config.valve_handwheel_radius).range(10.0..=48.0).speed(2.0)).changed() {
                    changed = true;
                }
                ui.label("uu");
            }
        });
        ui.end_row();

        // Steam Hazard Jet
        ui.label("Steam Scald Hazard:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.pipe_conduit_settings.active_config.steam_hazard_enabled, "Steam Relief Jet").changed() {
                changed = true;
            }
            if state.pipe_conduit_settings.active_config.steam_hazard_enabled {
                if ui.add(egui::Slider::new(&mut state.pipe_conduit_settings.active_config.scald_damage_per_sec, 0.0..=100.0).text("HP/s")).changed() {
                    changed = true;
                }
            }
        });
        ui.end_row();

        // Event & Tag
        ui.label("Trigger Event Name:");
        if ui.text_edit_singleline(&mut state.pipe_conduit_settings.active_config.event_name).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_pipe_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.pipe_conduit_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL PIPE HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Hydraulic Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Pipe Network & Fluid Fit Valid").strong());
            }
        });

        let metrics = &state.pipe_conduit_settings.cached_metrics;
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(100, 200, 255), format!("Length: {:.0} uu | Vol: {:.1} L | Velocity: {:.0} uu/s | Flanges: {}",
                metrics.total_pipe_length_uu, metrics.total_fluid_volume_liters, metrics.flow_velocity_uu_s, metrics.flange_count));
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
