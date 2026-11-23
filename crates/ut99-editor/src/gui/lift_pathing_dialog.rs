//! Interactive LiftCenter, LiftExit & Dynamic Elevator Pathing Studio
//!
//! Provides interactive diagnostics, mover synchronization, keyframe floor matching,
//! 1-click navigation setup generation, and 2D/3D real-time bot riding simulation.

use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{LiftBotSimulationState, LiftDiagnosticSeverity};
use crate::editor_state::EditorState;

pub fn render_lift_pathing_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_lift_pathing_studio_dialog {
        return;
    }

    let mut is_open = state.show_lift_pathing_studio_dialog;
    Window::new("LiftCenter, LiftExit & Elevator Pathing Studio")
        .open(&mut is_open)
        .default_width(940.0)
        .default_height(680.0)
        .min_width(780.0)
        .min_height(540.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_lift_pathing_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🛗 LiftCenter, LiftExit & Dynamic Elevator Pathing Studio");
    ui.label(
        RichText::new(
            "Synchronize mover keyframes, audit LiftCenter and LiftExit placement, verify bot reachability, and eliminate lethal plunge hazards.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 160, 175)),
    );
    ui.separator();

    // Ensure audit is available
    if state.lift_pathing_studio_settings.cached_audit.is_none() {
        state.collect_elevator_audit();
    }

    let audit = match state.lift_pathing_studio_settings.cached_audit.clone() {
        Some(a) => a,
        None => return,
    };

    // 1. Top Audit Summary Bar
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "Level Movers: {}  |  LiftCenters: {}  |  LiftExits: {}",
                audit.total_movers, audit.total_lift_centers, audit.total_lift_exits
            ))
            .strong(),
        );

        ui.separator();

        if audit.critical_hazard_count > 0 {
            ui.label(
                RichText::new(format!("⛔ {} Critical Hazards", audit.critical_hazard_count))
                    .color(Color32::from_rgb(255, 80, 80))
                    .strong(),
            );
        } else if audit.warning_count > 0 {
            ui.label(
                RichText::new(format!("⚠ {} Warnings", audit.warning_count))
                    .color(Color32::from_rgb(255, 180, 50))
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("✅ All Elevators Synchronized")
                    .color(Color32::from_rgb(80, 240, 120))
                    .strong(),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Re-Audit").clicked() {
                state.collect_elevator_audit();
            }
            if ui
                .button("🪜 Spawn Climbable WaterZone Ladder")
                .on_hover_text("Spawn a canonical UT99 climbable ladder volume (invisible ZonePortal + WaterZone + bot LiftExit/LiftCenter nodes) at the Builder Brush")
                .clicked()
            {
                let pos = state.builder_brush.location;
                state.spawn_ut99_climbable_ladder_volume(pos, 64.0, 32.0, 256.0);
                state.collect_elevator_audit();
            }
        });
    });

    ui.separator();

    if audit.systems.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(30.0);
            ui.label(RichText::new("No Movers or Elevators Found in Map").size(16.0).color(Color32::GRAY));
            ui.label(
                RichText::new("Add a Mover using the Builder Brush or create an elevator platform to begin pathing synchronization.")
                    .color(Color32::DARK_GRAY),
            );
            ui.add_space(20.0);
        });
        return;
    }

    // 2. Elevator Selection and Transport Bar
    let selected_idx = state.lift_pathing_studio_settings.selected_elevator_idx.min(audit.systems.len().saturating_sub(1));
    state.lift_pathing_studio_settings.selected_elevator_idx = selected_idx;
    let current_sys = audit.systems[selected_idx].clone();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Select Elevator:").strong());

        let mut changed_selection = false;
        egui::ComboBox::from_id_salt("elevator_selector_combo")
            .selected_text(format!(
                "{} [{}] ({} keys, {:.1}s) - {}",
                current_sys.mover_name,
                current_sys.mover_tag,
                current_sys.num_keys,
                current_sys.total_cycle_time,
                if current_sys.is_fully_synchronized { "✅ Synced" } else { "⚠ Issues" }
            ))
            .show_ui(ui, |ui| {
                for (idx, sys) in audit.systems.iter().enumerate() {
                    let status = if sys.is_fully_synchronized { "✅" } else { "⚠" };
                    let label = format!("{status} {} [{}] ({} keys, {:.1}s)", sys.mover_name, sys.mover_tag, sys.num_keys, sys.total_cycle_time);
                    if ui.selectable_value(&mut state.lift_pathing_studio_settings.selected_elevator_idx, idx, label).clicked() {
                        changed_selection = true;
                    }
                }
            });

        if changed_selection {
            state.lift_pathing_studio_settings.playback_time = 0.0;
            state.calculate_active_lift_simulation();
        }

        ui.separator();

        if ui.button("⚡ 1-Click Sync Setup").on_hover_text("Generate coordinated LiftCenter and LiftExit actors for all floors of this mover").clicked() {
            state.create_synchronized_lift_setup_for_mover(selected_idx);
        }

        if ui.button("🔧 Auto-Fix Tags").on_hover_text("Repair mismatched LiftTag values to match this mover").clicked() {
            state.auto_fix_elevator_issues(&current_sys.mover_tag);
        }
    });

    // 3. Playback Transport Bar
    let total_cycle = current_sys.total_cycle_time.max(0.5);
    ui.horizontal(|ui| {
        let play_btn_text = if state.lift_pathing_studio_settings.is_playing { "⏸ Pause" } else { "▶ Play" };
        if ui.button(play_btn_text).clicked() {
            state.lift_pathing_studio_settings.is_playing = !state.lift_pathing_studio_settings.is_playing;
        }

        if ui.button("⏮ Reset").clicked() {
            state.lift_pathing_studio_settings.playback_time = 0.0;
        }

        ui.label(RichText::new("Time:").strong());
        let mut time_val = state.lift_pathing_studio_settings.playback_time;
        if ui.add(egui::Slider::new(&mut time_val, 0.0..=total_cycle).text("s").suffix("s")).changed() {
            state.lift_pathing_studio_settings.playback_time = time_val;
        }

        ui.checkbox(&mut state.lift_pathing_studio_settings.auto_loop, "🔁 Loop");

        ui.separator();
        ui.label(RichText::new("Speed:").size(11.0));
        ui.selectable_value(&mut state.lift_pathing_studio_settings.playback_speed, 0.5, "0.5x");
        ui.selectable_value(&mut state.lift_pathing_studio_settings.playback_speed, 1.0, "1.0x");
        ui.selectable_value(&mut state.lift_pathing_studio_settings.playback_speed, 2.0, "2.0x");
    });

    ui.separator();

    // Advance simulation timer if playing
    let dt = ui.input(|i| i.stable_dt).min(0.1);
    state.step_elevator_simulation(dt);

    // Compute current simulation step interpolated state
    let cur_step = if !state.lift_pathing_studio_settings.cached_simulation.is_empty() {
        let sim_len = state.lift_pathing_studio_settings.cached_simulation.len();
        let norm_t = (state.lift_pathing_studio_settings.playback_time / total_cycle).clamp(0.0, 1.0);
        let idx = ((norm_t * (sim_len - 1) as f32).round() as usize).min(sim_len - 1);
        Some(state.lift_pathing_studio_settings.cached_simulation[idx].clone())
    } else {
        None
    };

    // 4. Main Body: Left 2D Canvas Profile + Right Inspection Tabs
    ui.horizontal(|ui| {
        // Left Column: 2D Shaft Canvas
        ui.vertical(|ui| {
            ui.label(RichText::new("📐 2D Elevator Shaft & Bot Trajectory Profile").strong());

            let canvas_width = 380.0;
            let canvas_height = 360.0;
            let (response, painter) = ui.allocate_painter(vec2(canvas_width, canvas_height), egui::Sense::hover());
            let rect = response.rect;

            // Canvas Background
            painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 60, 75)), StrokeKind::Middle);

            if !current_sys.keyframes.is_empty() {
                // Calculate vertical bounds
                let z_min = current_sys.keyframes.iter().map(|k| k.position.z).fold(f32::INFINITY, f32::min);
                let z_max = current_sys.keyframes.iter().map(|k| k.position.z).fold(f32::NEG_INFINITY, f32::max);
                let span_z = (z_max - z_min).max(128.0);
                let margin_y = 40.0;

                let to_screen_y = |z: f32| -> f32 {
                    let norm = (z - z_min) / span_z;
                    rect.bottom() - margin_y - norm * (rect.height() - margin_y * 2.0)
                };

                let center_x = rect.center().x;
                let shaft_half_w = 70.0;

                // Shaft Vertical Rails
                let rail_stroke = Stroke::new(1.5_f32, Color32::from_rgb(60, 70, 90));
                painter.line_segment([pos2(center_x - shaft_half_w, rect.top() + 20.0), pos2(center_x - shaft_half_w, rect.bottom() - 20.0)], rail_stroke);
                painter.line_segment([pos2(center_x + shaft_half_w, rect.top() + 20.0), pos2(center_x + shaft_half_w, rect.bottom() - 20.0)], rail_stroke);

                // Draw each Keyframe Floor
                for floor in &current_sys.floors {
                    let y = to_screen_y(floor.mover_position.z);

                    // Floor line
                    let floor_stroke = if floor.is_aligned {
                        Stroke::new(1.5_f32, Color32::from_rgb(60, 180, 120))
                    } else {
                        Stroke::new(1.5_f32, Color32::from_rgb(240, 80, 80))
                    };
                    painter.line_segment([pos2(center_x - shaft_half_w - 40.0, y), pos2(center_x + shaft_half_w + 40.0, y)], floor_stroke);

                    // Floor label
                    let label = format!("Floor {} (Key {}): Z={:.0}", floor.keyframe_index + 1, floor.keyframe_index, floor.mover_position.z);
                    painter.text(pos2(rect.left() + 8.0, y - 6.0), egui::Align2::LEFT_BOTTOM, label, egui::FontId::proportional(10.0), Color32::from_rgb(180, 190, 205));

                    // LiftExit marker if exits exist
                    if !floor.associated_exits.is_empty() {
                        let exit_pos = pos2(center_x + shaft_half_w + 24.0, y - 10.0);
                        painter.circle_filled(exit_pos, 7.0, Color32::from_rgb(40, 190, 255));
                        painter.text(exit_pos + vec2(12.0, 0.0), egui::Align2::LEFT_CENTER, "🚪 LiftExit", egui::FontId::proportional(9.0), Color32::from_rgb(100, 210, 255));
                    } else {
                        let warn_pos = pos2(center_x + shaft_half_w + 24.0, y - 10.0);
                        painter.text(warn_pos, egui::Align2::LEFT_CENTER, "⚠ Missing Exit", egui::FontId::proportional(9.0), Color32::from_rgb(255, 90, 90));
                    }
                }

                // Draw Elevator Platform Box at current position
                let cur_z = cur_step.as_ref().map(|s| s.mover_pos.z).unwrap_or(current_sys.keyframes[0].position.z);
                let plat_y = to_screen_y(cur_z);
                let plat_rect = egui::Rect::from_center_size(pos2(center_x, plat_y), vec2(shaft_half_w * 1.8, 14.0));

                painter.rect_filled(plat_rect, 2.0, Color32::from_rgb(35, 60, 95));
                painter.rect_stroke(plat_rect, 2.0, Stroke::new(2.0_f32, Color32::from_rgb(245, 180, 45)), StrokeKind::Middle);

                // LiftCenter icon on the platform
                if current_sys.lift_center.is_some() {
                    let center_node_pos = pos2(center_x, plat_y - 8.0);
                    painter.circle_filled(center_node_pos, 5.0, Color32::from_rgb(60, 240, 100));
                    painter.circle_stroke(center_node_pos, 7.0, Stroke::new(1.0_f32, Color32::WHITE));
                }

                // Draw Bot Riding Avatar
                if let Some(step) = cur_step {
                    let bot_y = to_screen_y(step.bot_pos.z);
                    let bot_x = match step.bot_state {
                        LiftBotSimulationState::WaitingAtBottom | LiftBotSimulationState::BoardingPlatform => {
                            center_x + shaft_half_w * 0.3
                        }
                        LiftBotSimulationState::Ascending | LiftBotSimulationState::Descending => center_x,
                        LiftBotSimulationState::ArrivedAtFloor | LiftBotSimulationState::ExitingPlatform | LiftBotSimulationState::WaitingAtTop | LiftBotSimulationState::BoardingReturn => {
                            center_x + shaft_half_w * 0.5
                        }
                    };

                    let bot_pos = pos2(bot_x, bot_y - 12.0);
                    let bot_color = match step.bot_state {
                        LiftBotSimulationState::WaitingAtBottom => Color32::from_rgb(180, 180, 200),
                        LiftBotSimulationState::BoardingPlatform => Color32::from_rgb(100, 220, 255),
                        LiftBotSimulationState::Ascending => Color32::from_rgb(255, 210, 60),
                        LiftBotSimulationState::ArrivedAtFloor => Color32::from_rgb(60, 255, 120),
                        LiftBotSimulationState::ExitingPlatform => Color32::from_rgb(220, 120, 255),
                        LiftBotSimulationState::WaitingAtTop => Color32::from_rgb(100, 220, 255),
                        LiftBotSimulationState::BoardingReturn => Color32::from_rgb(255, 180, 60),
                        LiftBotSimulationState::Descending => Color32::from_rgb(255, 140, 40),
                    };

                    // Draw bot avatar marker
                    painter.circle_filled(bot_pos, 7.0, bot_color);
                    painter.circle_stroke(bot_pos, 9.0, Stroke::new(1.5_f32, Color32::WHITE));

                    // Bot state badge below canvas
                    painter.text(
                        pos2(rect.left() + 8.0, rect.bottom() - 14.0),
                        egui::Align2::LEFT_BOTTOM,
                        format!("🤖 Bot: {}", step.bot_state.label()),
                        egui::FontId::proportional(11.0),
                        bot_color,
                    );
                }
            }
        });

        ui.add_space(10.0);

        // Right Column: Tabbed Controls & Diagnostics
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Inspector & Configuration").strong());
            });

            egui::ScrollArea::vertical().max_height(480.0).show(ui, |ui| {
                // Section 1: Keyframe Floor Alignment Table
                ui.group(|ui| {
                    ui.label(RichText::new("🏢 Keyframe Floor Alignment").strong().size(13.0));

                    egui::Grid::new("lift_floors_grid")
                        .striped(true)
                        .min_col_width(70.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Key").strong());
                            ui.label(RichText::new("Elevation (Z)").strong());
                            ui.label(RichText::new("Associated Exit").strong());
                            ui.label(RichText::new("Delta Z").strong());
                            ui.label(RichText::new("Status").strong());
                            ui.end_row();

                            for floor in &current_sys.floors {
                                ui.label(format!("Key {}", floor.keyframe_index));
                                ui.label(format!("{:.0} uu", floor.mover_position.z));

                                if floor.associated_exits.is_empty() {
                                    ui.colored_label(Color32::from_rgb(255, 90, 90), "None (Missing)");
                                } else {
                                    ui.label(floor.associated_exits.join(", "));
                                }

                                let delta_str = if floor.is_aligned {
                                    format!("{:.1} uu", floor.nearest_exit_distance)
                                } else {
                                    "-".to_string()
                                };
                                ui.label(delta_str);

                                if floor.is_aligned {
                                    ui.colored_label(Color32::from_rgb(60, 220, 100), "✅ Aligned");
                                } else {
                                    ui.colored_label(Color32::from_rgb(255, 70, 70), "⛔ Missing Exit");
                                }
                                ui.end_row();
                            }
                        });
                });

                ui.add_space(6.0);

                // Section 2: Diagnostics & Safety Audit
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔍 Diagnostics & Bot Safety Audit").strong().size(13.0));
                        ui.label(RichText::new(format!("({} findings)", current_sys.diagnostics.len())).size(11.0).color(Color32::GRAY));
                    });

                    if current_sys.diagnostics.is_empty() {
                        ui.colored_label(Color32::from_rgb(80, 220, 120), "✅ No pathing defects or plunge hazards detected.");
                    } else {
                        for diag in &current_sys.diagnostics {
                            ui.horizontal(|ui| {
                                let (badge_col, badge_text) = match diag.severity {
                                    LiftDiagnosticSeverity::CriticalHazard => (Color32::from_rgb(255, 70, 70), "HAZARD"),
                                    LiftDiagnosticSeverity::Warning => (Color32::from_rgb(255, 180, 50), "WARN"),
                                    LiftDiagnosticSeverity::Info => (Color32::from_rgb(80, 180, 255), "INFO"),
                                };

                                ui.colored_label(badge_col, format!("[{}]", badge_text));
                                ui.label(RichText::new(diag.category.title()).strong());
                            });

                            ui.label(RichText::new(&diag.message).size(11.0).color(Color32::from_rgb(210, 210, 220)));
                            ui.label(RichText::new(format!("💡 Fix: {}", diag.suggested_fix)).size(10.0).color(Color32::from_rgb(140, 190, 240)));
                            ui.separator();
                        }
                    }
                });

                ui.add_space(6.0);

                // Section 3: Wizard Settings
                ui.group(|ui| {
                    ui.label(RichText::new("⚙ Navigation Setup Wizard Parameters").strong().size(13.0));

                    ui.horizontal(|ui| {
                        ui.label("Platform Radius:");
                        ui.add(egui::Slider::new(&mut state.lift_pathing_studio_settings.wizard_platform_radius, 48.0..=256.0).suffix(" uu"));
                    });

                    ui.horizontal(|ui| {
                        ui.label("Exit Direction Yaw:");
                        ui.add(egui::Slider::new(&mut state.lift_pathing_studio_settings.wizard_exit_yaw_deg, 0.0..=360.0).suffix("°"));
                    });

                    ui.horizontal(|ui| {
                        if ui.button("✨ Generate Coordinated Navigation Set").clicked() {
                            state.create_synchronized_lift_setup_for_mover(selected_idx);
                        }
                    });
                });

                ui.add_space(6.0);

                // Section 4: Viewport WGPU Display Options
                ui.group(|ui| {
                    ui.label(RichText::new("👁 Viewport WGPU Visualizers").strong().size(13.0));
                    ui.checkbox(&mut state.lift_pathing_studio_settings.show_visualizer_in_viewport, "Show Elevator Visualizers in Viewport");
                    ui.checkbox(&mut state.lift_pathing_studio_settings.show_shaft_lines_viewport, "Draw Shaft Travel Guide Lines");
                    ui.checkbox(&mut state.lift_pathing_studio_settings.show_keyframe_floors_viewport, "Draw Keyframe Floor Outlines");
                    ui.checkbox(&mut state.lift_pathing_studio_settings.show_bot_avatar_viewport, "Draw Animated Bot Riding Avatar in 3D");
                });
            });
        });
    });
}
