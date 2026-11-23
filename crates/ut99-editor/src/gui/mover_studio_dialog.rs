//! Interactive Mover Encroachment, Door Sequencing & Crushing Trap Studio
//!
//! Provides real-time 3D mover timeline scrubbing, swept encroachment volume evaluation,
//! pinch-point collision safety audits, 1-click proximity trigger auto-wiring,
//! and bot crushing simulation for Unreal Engine 1 movers and doors.

use egui::{pos2, vec2, Color32, Rect, RichText, Sense, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{MoverEncroachType, MoverGlideType, MoverHazardSeverity, MoverTemplateArchetype, MoverTriggerType};
use crate::editor_state::EditorState;

pub fn render_mover_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_mover_studio_dialog {
        return;
    }

    let mut is_open = state.show_mover_studio_dialog;
    Window::new("Mover Encroachment, Door Sequencing & Crushing Trap Studio")
        .open(&mut is_open)
        .default_width(960.0)
        .default_height(720.0)
        .min_width(800.0)
        .min_height(580.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_mover_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    // 0. Advance simulation if playing
    let dt = ui.input(|i| i.stable_dt).min(0.05);
    state.step_mover_studio_simulation(dt);
    if state.mover_studio_settings.is_playing {
        ui.ctx().request_repaint();
    }

    // Ensure audit is fresh if empty
    if state.mover_studio_settings.cached_hazards.is_empty() {
        state.audit_active_mover_hazards();
    }

    // 1. Header Banner
    ui.heading("🚪 Mover Encroachment, Door Sequencing & Crushing Trap Studio");
    ui.label(
        RichText::new(
            "Simulate door kinematics, audit lethal pinch-points, configure encroachment behaviors (Crush/Stop/Return), and auto-wire proximity triggers.",
        )
        .size(11.0)
        .color(Color32::from_rgb(165, 175, 195)),
    );
    ui.separator();

    // 2. Mover Selector & Template Ribbon
    ui.horizontal(|ui| {
        ui.label(RichText::new("Select Mover:").strong());

        let mover_count = state.movers.len();
        if mover_count > 0 {
            let cur_idx = state.mover_studio_settings.selected_mover_idx.min(mover_count.saturating_sub(1));
            let cur_name = state.movers.get(cur_idx).map(|m| m.name.clone()).unwrap_or_else(|| "Mover".to_string());

            egui::ComboBox::from_id_salt("mover_studio_select_combo")
                .selected_text(format!("[{}] {}", cur_idx, cur_name))
                .show_ui(ui, |ui| {
                    for i in 0..mover_count {
                        let m_name = &state.movers[i].name;
                        let label = format!("[{}] {}", i, m_name);
                        if ui.selectable_value(&mut state.mover_studio_settings.selected_mover_idx, i, label).clicked() {
                            state.audit_active_mover_hazards();
                        }
                    }
                });
        } else {
            ui.label(RichText::new("No Movers in Level (Operating on Virtual Profile)").color(Color32::GRAY));
        }

        ui.separator();
        ui.label(RichText::new("Templates:").strong());

        for template in &MoverTemplateArchetype::ALL {
            let is_sel = state.mover_studio_settings.selected_template == *template;
            let label = match template {
                MoverTemplateArchetype::AutomaticSlidingDoor => "🚪 Slide Door",
                MoverTemplateArchetype::VerticalBlastBulkhead => "🏰 Blast Gate",
                MoverTemplateArchetype::RotatingSecurityGate => "🔑 Swing Gate",
                MoverTemplateArchetype::HydraulicCrusherTrap => "🪓 Crusher Trap",
                MoverTemplateArchetype::CastleDrawbridgeHinged => "🌉 Drawbridge",
                MoverTemplateArchetype::ContinuousRotatingFan => "⚙ Rotor Fan",
            };

            let btn = if is_sel {
                ui.button(RichText::new(label).strong().color(Color32::from_rgb(100, 220, 255)))
            } else {
                ui.button(label)
            };

            if btn.clicked() {
                state.apply_mover_template(*template);
            }
        }
    });

    ui.separator();

    // 3. Playback & Scrubbing Transport Bar
    ui.horizontal(|ui| {
        let play_text = if state.mover_studio_settings.is_playing {
            "⏸ Pause"
        } else {
            "▶ Play"
        };
        if ui.button(play_text).clicked() {
            state.mover_studio_settings.is_playing = !state.mover_studio_settings.is_playing;
        }

        if ui.button("⏮ Reset").clicked() {
            state.mover_studio_settings.playback_time = 0.0;
        }

        let move_t = state.mover_studio_settings.active_config.move_time.max(0.1);
        let stay_t = state.mover_studio_settings.active_config.stay_open_time.max(0.0);
        let total_cycle = move_t * 2.0 + stay_t;

        let mut time_val = state.mover_studio_settings.playback_time;
        ui.label("Timeline:");
        let slider = egui::Slider::new(&mut time_val, 0.0..=total_cycle)
            .suffix(format!(" / {:.2}s", total_cycle))
            .show_value(true);
        if ui.add(slider).changed() {
            state.scrub_mover_timeline(time_val);
        }

        ui.checkbox(&mut state.mover_studio_settings.auto_loop, "🔁 Loop");

        ui.separator();
        ui.label("Speed:");
        ui.selectable_value(&mut state.mover_studio_settings.playback_speed, 0.5, "0.5x");
        ui.selectable_value(&mut state.mover_studio_settings.playback_speed, 1.0, "1.0x");
        ui.selectable_value(&mut state.mover_studio_settings.playback_speed, 2.0, "2.0x");
    });

    ui.separator();

    // 4. Interactive 2D Motion Profile Canvas & Encroachment Audit
    ui.columns(2, |cols| {
        // --- Left: 2D Kinematics & Trajectory Canvas ---
        cols[0].vertical(|ui| {
            ui.label(RichText::new("📈 2D Kinematic Motion Profile & Bot Contact").strong());

            let canvas_width = ui.available_width().max(320.0);
            let canvas_height = 240.0;
            let (rect, _) = ui.allocate_exact_size(vec2(canvas_width, canvas_height), Sense::hover());

            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 20, 30));
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 70, 95)), StrokeKind::Inside);

            let move_t = state.mover_studio_settings.active_config.move_time.max(0.1);
            let stay_t = state.mover_studio_settings.active_config.stay_open_time.max(0.0);
            let total_cycle = move_t * 2.0 + stay_t;

            // Draw phase regions: [Opening], [StayOpen], [Closing]
            let x_open_end = rect.left() + (move_t / total_cycle) * rect.width();
            let x_stay_end = rect.left() + ((move_t + stay_t) / total_cycle) * rect.width();

            // Opening region
            painter.rect_filled(
                Rect::from_min_max(pos2(rect.left(), rect.top()), pos2(x_open_end, rect.bottom())),
                0.0,
                Color32::from_rgba_premultiplied(30, 60, 90, 40),
            );
            // Stay Open region
            painter.rect_filled(
                Rect::from_min_max(pos2(x_open_end, rect.top()), pos2(x_stay_end, rect.bottom())),
                0.0,
                Color32::from_rgba_premultiplied(40, 90, 60, 40),
            );

            // Draw motion curve
            let num_points = 64;
            let mut curve_pts = Vec::new();
            for i in 0..=num_points {
                let frac = i as f32 / num_points as f32;
                let cur_t = frac * total_cycle;
                let progress = if cur_t <= move_t {
                    cur_t / move_t
                } else if cur_t <= move_t + stay_t {
                    1.0
                } else {
                    1.0 - ((cur_t - move_t - stay_t) / move_t)
                };

                let eased = match state.mover_studio_settings.active_config.glide_type {
                    MoverGlideType::Linear => progress,
                    MoverGlideType::Smooth => 0.5 * (1.0 - (std::f32::consts::PI * progress).cos()),
                };

                let px = rect.left() + frac * rect.width();
                let py = rect.bottom() - 25.0 - eased * (rect.height() - 50.0);
                curve_pts.push(pos2(px, py));
            }

            for i in 0..(curve_pts.len() - 1) {
                painter.line_segment([curve_pts[i], curve_pts[i + 1]], Stroke::new(2.0_f32, Color32::from_rgb(100, 200, 255)));
            }

            // Current playhead line & moving platform box
            let playhead_frac = (state.mover_studio_settings.playback_time / total_cycle).clamp(0.0, 1.0);
            let play_x = rect.left() + playhead_frac * rect.width();
            painter.line_segment([pos2(play_x, rect.top()), pos2(play_x, rect.bottom())], Stroke::new(1.5_f32, Color32::from_rgb(255, 230, 80)));

            // Platform indicator on curve
            let cur_t = state.mover_studio_settings.playback_time;
            let cur_prog = if cur_t <= move_t {
                cur_t / move_t
            } else if cur_t <= move_t + stay_t {
                1.0
            } else {
                1.0 - ((cur_t - move_t - stay_t) / move_t)
            };
            let cur_eased = match state.mover_studio_settings.active_config.glide_type {
                MoverGlideType::Linear => cur_prog,
                MoverGlideType::Smooth => 0.5 * (1.0 - (std::f32::consts::PI * cur_prog).cos()),
            };
            let plat_y = rect.bottom() - 25.0 - cur_eased * (rect.height() - 50.0);
            let plat_box = Rect::from_center_size(pos2(play_x, plat_y), vec2(20.0, 12.0));
            painter.rect_filled(plat_box, 2.0, Color32::from_rgb(255, 200, 50));
            painter.rect_stroke(plat_box, 2.0, Stroke::new(1.0_f32, Color32::WHITE), StrokeKind::Inside);

            // Phase text labels
            painter.text(pos2(rect.left() + 10.0, rect.top() + 10.0), egui::Align2::LEFT_TOP, "Opening", egui::FontId::proportional(11.0), Color32::from_rgb(140, 180, 220));
            painter.text(pos2(x_open_end + 10.0, rect.top() + 10.0), egui::Align2::LEFT_TOP, "Stay Open", egui::FontId::proportional(11.0), Color32::from_rgb(140, 220, 160));
            painter.text(pos2(x_stay_end + 10.0, rect.top() + 10.0), egui::Align2::LEFT_TOP, "Closing", egui::FontId::proportional(11.0), Color32::from_rgb(220, 160, 140));
        });

        // --- Right: Encroachment & Bot Crushing Safety Audit ---
        cols[1].vertical(|ui| {
            ui.label(RichText::new("⚠ Encroachment & Pinch-Point Safety Audit").strong());

            let hazards = &state.mover_studio_settings.cached_hazards;
            if hazards.is_empty() {
                ui.label(RichText::new("✅ No Pinch Points or Glitch Timings Detected").color(Color32::from_rgb(80, 220, 120)).strong());
            } else {
                egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                    for h in hazards {
                        let (badge_col, badge_text) = match h.severity {
                            MoverHazardSeverity::CriticalLethalHazard => (Color32::from_rgb(255, 80, 80), "LETHAL"),
                            MoverHazardSeverity::Warning => (Color32::from_rgb(255, 180, 50), "WARNING"),
                            MoverHazardSeverity::Info => (Color32::from_rgb(100, 180, 255), "INFO"),
                        };

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("[{}]", badge_text)).color(badge_col).strong());
                            ui.label(RichText::new(&h.hazard_type).strong());
                        });
                        ui.label(RichText::new(&h.description).size(10.5).color(Color32::from_rgb(190, 190, 205)));
                        ui.separator();
                    }
                });
            }

            ui.add_space(4.0);
            ui.label(RichText::new("🤖 Bot Encroachment Test Result:").strong());

            if let Some(ref sim) = state.mover_studio_settings.cached_encroach_sim {
                ui.horizontal(|ui| {
                    ui.label("Outcome:");
                    ui.label(RichText::new(sim.mover_reaction).color(Color32::from_rgb(255, 220, 80)).strong());
                });
                ui.horizontal(|ui| {
                    ui.label(format!("Damage Dealt: {:.1} HP", sim.damage_dealt));
                    ui.label(format!("Impact Time: {:.0}%", sim.impact_time_norm * 100.0));
                });
            } else {
                ui.label(RichText::new("Bot position clear of swept mover volume.").color(Color32::from_rgb(120, 190, 255)));
            }

            ui.horizontal(|ui| {
                ui.label("Test Bot Pos X:");
                if ui.add(egui::DragValue::new(&mut state.mover_studio_settings.bot_test_position.x).speed(2.0)).changed() {
                    state.simulate_active_mover_bot_encroachment();
                }
                ui.label("Y:");
                if ui.add(egui::DragValue::new(&mut state.mover_studio_settings.bot_test_position.y).speed(2.0)).changed() {
                    state.simulate_active_mover_bot_encroachment();
                }
            });
        });
    });

    ui.separator();

    // 5. Lower Inspector Tabs: [Physical & Sound Parameters] and [Proximity Trigger Setup]
    ui.columns(2, |cols| {
        // Left Column: Kinematics & Sound Slots
        cols[0].vertical(|ui| {
            ui.label(RichText::new("⚙ Kinematics & Sound Profile").strong());

            egui::Grid::new("mover_kinematics_grid")
                .num_columns(2)
                .spacing([12.0, 5.0])
                .show(ui, |ui| {
                    ui.label("Encroach Handling:");
                    egui::ComboBox::from_id_salt("encroach_combo")
                        .selected_text(state.mover_studio_settings.active_config.encroach_type.display_name())
                        .show_ui(ui, |ui| {
                            for e in &MoverEncroachType::ALL {
                                if ui.selectable_value(&mut state.mover_studio_settings.active_config.encroach_type, *e, e.display_name()).clicked() {
                                    state.audit_active_mover_hazards();
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Activation Trigger:");
                    egui::ComboBox::from_id_salt("trigger_combo")
                        .selected_text(state.mover_studio_settings.active_config.trigger_type.display_name())
                        .show_ui(ui, |ui| {
                            for t in &MoverTriggerType::ALL {
                                if ui.selectable_value(&mut state.mover_studio_settings.active_config.trigger_type, *t, t.display_name()).clicked() {
                                    state.audit_active_mover_hazards();
                                }
                            }
                        });
                    ui.end_row();

                    ui.label("Glide Smoothing:");
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut state.mover_studio_settings.active_config.glide_type, MoverGlideType::Linear, "Linear");
                        ui.selectable_value(&mut state.mover_studio_settings.active_config.glide_type, MoverGlideType::Smooth, "Smooth (Cosine)");
                    });
                    ui.end_row();

                    ui.label("Move Time:");
                    ui.add(egui::Slider::new(&mut state.mover_studio_settings.active_config.move_time, 0.1..=10.0).suffix(" s"));
                    ui.end_row();

                    ui.label("Stay Open Time:");
                    ui.add(egui::Slider::new(&mut state.mover_studio_settings.active_config.stay_open_time, 0.0..=15.0).suffix(" s"));
                    ui.end_row();

                    ui.label("Crush Damage:");
                    ui.add(egui::Slider::new(&mut state.mover_studio_settings.active_config.crush_damage, 0.0..=1000.0).suffix(" HP"));
                    ui.end_row();

                    ui.label("Opening Sound:");
                    ui.text_edit_singleline(&mut state.mover_studio_settings.active_config.opening_sound);
                    ui.end_row();

                    ui.label("Closing Sound:");
                    ui.text_edit_singleline(&mut state.mover_studio_settings.active_config.closing_sound);
                    ui.end_row();
                });
        });

        // Right Column: Trigger Wiring Wizard & Viewport Overlays
        cols[1].vertical(|ui| {
            ui.label(RichText::new("⚡ Proximity Trigger Auto-Wiring Wizard").strong());

            egui::Grid::new("trigger_wizard_grid")
                .num_columns(2)
                .spacing([12.0, 5.0])
                .show(ui, |ui| {
                    ui.label("Approach Distance:");
                    ui.add(egui::Slider::new(&mut state.mover_studio_settings.trigger_approach_distance, 32.0..=384.0).suffix(" uu"));
                    ui.end_row();

                    ui.label("Trigger Radius:");
                    ui.add(egui::Slider::new(&mut state.mover_studio_settings.trigger_radius, 32.0..=256.0).suffix(" uu"));
                    ui.end_row();
                });

            ui.add_space(6.0);

            if ui.button(RichText::new("⚡ 1-Click Wire Proximity Trigger for Door").strong().color(Color32::from_rgb(255, 230, 90))).clicked() {
                state.wire_proximity_trigger_for_active_mover();
            }

            ui.separator();
            ui.label(RichText::new("👁 Viewport Overlay Settings").strong());
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.mover_studio_settings.show_swept_bounds_viewport, "Swept Bounds Box");
                ui.checkbox(&mut state.mover_studio_settings.show_keyframe_ghosts_viewport, "Keyframe Ghost Poses");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.mover_studio_settings.show_animated_brush_viewport, "Animated 3D Brush");
                ui.checkbox(&mut state.mover_studio_settings.show_pinch_hazards_viewport, "Pinch Hazard Highlights");
            });
            ui.checkbox(&mut state.mover_studio_settings.show_trigger_radius_viewport, "Sensor Trigger Radius Sphere");
        });
    });
}
