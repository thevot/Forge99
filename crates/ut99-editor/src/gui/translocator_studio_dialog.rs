use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::TranslocatorPreset;
use crate::editor_state::EditorState;

pub fn render_translocator_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_translocator_studio_dialog {
        return;
    }
    let mut is_open = state.show_translocator_studio_dialog;
    Window::new("Translocator Trajectory & Teleport Beacon Studio")
        .open(&mut is_open)
        .default_width(880.0)
        .default_height(640.0)
        .min_width(720.0)
        .min_height(500.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_translocator_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🎯 Translocator Trajectory & Teleport Beacon Studio");
    ui.label(
        RichText::new(
            "Simulate disc toss physics, surface bounces, ledge clinging, aperture clearance, and transit timing vs walking paths.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 160, 175)),
    );
    ui.separator();

    let preset = state.translocator_studio_settings.selected_preset;

    // 1. Preset Selector
    let mut do_recalc = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Toss Maneuver:").strong());
        let current_display = preset.display_name();

        let mut changed_preset = None;
        egui::ComboBox::from_id_salt("translocator_preset_combo")
            .selected_text(current_display)
            .width(340.0)
            .show_ui(ui, |ui| {
                for &p in &TranslocatorPreset::ALL {
                    let is_sel = p == preset;
                    if ui.selectable_label(is_sel, p.display_name()).clicked() {
                        changed_preset = Some(p);
                    }
                }
            });

        if let Some(p) = changed_preset {
            state.translocator_studio_settings.selected_preset = p;
            let cfg = p.to_config();
            state.translocator_studio_settings.active_config = cfg.clone();
            state.translocator_studio_settings.pitch_deg = cfg.pitch_deg;
            state.translocator_studio_settings.yaw_deg = cfg.yaw_deg;
            state.translocator_studio_settings.toss_speed = cfg.toss_speed;
            state.translocator_studio_settings.elasticity = cfg.elasticity;
            state.translocator_studio_settings.friction = cfg.friction;
            do_recalc = true;
        }

        if ui.button("📍 Place Probe at Pivot").clicked() {
            state.place_translocator_probe_at_pivot();
            do_recalc = true;
        }

        if ui.button("🔄 Recalculate").clicked() {
            do_recalc = true;
        }
    });

    if do_recalc || state.translocator_studio_settings.active_simulation.is_none() {
        state.calculate_active_translocator_simulation();
    }

    ui.add_space(4.0);

    // 2. Interactive 2D Trajectory Profile Canvas
    let sim_opt = state.translocator_studio_settings.active_simulation.clone();
    if let Some(ref sim) = sim_opt {
        let canvas_size = vec2(ui.available_width(), 200.0);
        let (_response, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
        let rect = _response.rect;

        // Dark Blueprint background
        painter.rect_filled(rect, 6.0, Color32::from_rgb(14, 18, 26));
        painter.rect_stroke(
            rect,
            6.0,
            Stroke::new(1.0_f32, Color32::from_rgb(40, 50, 70)),
            StrokeKind::Inside,
        );

        if !sim.points.is_empty() {
            let start_pt = sim.points[0].position;

            // Compute distance and elevation bounds
            let mut max_dist: f32 = 100.0;
            let mut min_elev: f32 = 0.0;
            let mut max_elev: f32 = 100.0;

            for pt in &sim.points {
                let diff = pt.position - start_pt;
                let d = (diff.x * diff.x + diff.y * diff.y).sqrt();
                max_dist = max_dist.max(d);
                let elev = pt.position.z - start_pt.z;
                min_elev = min_elev.min(elev);
                max_elev = max_elev.max(elev);
            }

            max_dist = max_dist.max(600.0);
            let elev_span = (max_elev - min_elev).max(400.0);

            let map_pt = |pt: &ut99_bsp::TranslocatorTrajectoryPoint| -> egui::Pos2 {
                let diff = pt.position - start_pt;
                let d = (diff.x * diff.x + diff.y * diff.y).sqrt();
                let elev = pt.position.z - start_pt.z;

                let margin_x = 40.0;
                let margin_y = 30.0;

                let norm_x = (d / max_dist).clamp(0.0, 1.0);
                let norm_y = ((elev - min_elev) / elev_span).clamp(0.0, 1.0);

                let px = rect.min.x + margin_x + norm_x * (rect.width() - margin_x * 2.0);
                let py = rect.max.y - margin_y - norm_y * (rect.height() - margin_y * 2.0);
                pos2(px, py)
            };

            // Ground baseline
            let ground_y = rect.max.y - 30.0 - ((-min_elev) / elev_span).clamp(0.0, 1.0) * (rect.height() - 60.0);
            painter.line_segment(
                [pos2(rect.min.x + 30.0, ground_y), pos2(rect.max.x - 30.0, ground_y)],
                Stroke::new(1.0_f32, Color32::from_rgb(60, 75, 95)),
            );

            // Draw Trajectory Segments
            for i in 0..sim.points.len() - 1 {
                let p1 = map_pt(&sim.points[i]);
                let p2 = map_pt(&sim.points[i + 1]);

                let alpha = 255.max((255.0 * (1.0 - (i as f32 / sim.points.len() as f32) * 0.4)) as u8);
                let arc_color = Color32::from_rgba_premultiplied(255, 205, 50, alpha);
                painter.line_segment([p1, p2], Stroke::new(2.2_f32, arc_color));
            }

            // Draw Bounces
            for pt in &sim.points {
                if pt.is_bounce {
                    let bp = map_pt(pt);
                    painter.circle_filled(bp, 4.5, Color32::from_rgb(255, 130, 30));
                    painter.circle_stroke(bp, 6.0, Stroke::new(1.2_f32, Color32::from_rgb(255, 220, 120)));

                    if let Some(norm) = pt.surface_normal {
                        let tick_end = pos2(bp.x + norm.x * 12.0, bp.y - norm.z * 12.0);
                        painter.line_segment([bp, tick_end], Stroke::new(1.5_f32, Color32::from_rgb(255, 180, 50)));
                    }
                }
            }

            // Beacon Rest Point with animated pulse
            if let Some(last) = sim.points.last() {
                if last.is_at_rest {
                    let lp = map_pt(last);
                    let time = ui.input(|i| i.time) as f32;
                    let pulse_rad = 6.0 + (time * 4.0).sin().abs() * 5.0;

                    painter.circle_stroke(lp, pulse_rad, Stroke::new(1.2_f32, Color32::from_rgba_premultiplied(80, 240, 150, 140)));
                    painter.circle_filled(lp, 5.0, Color32::from_rgb(50, 255, 120));

                    painter.text(
                        pos2(lp.x, lp.y - 12.0),
                        egui::Align2::CENTER_BOTTOM,
                        "BEACON REST",
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(80, 255, 140),
                    );
                }
            }
        }
    }

    ui.add_space(4.0);

    // 3. Aiming & Physics Parameters
    ui.columns(2, |cols| {
        // Left Column: Toss Angles & Physical Restitution
        cols[0].group(|ui| {
            ui.label(RichText::new("⚙ Launch & Restitution Physics").strong().color(Color32::from_rgb(255, 215, 80)));

            let mut pitch = state.translocator_studio_settings.pitch_deg;
            let mut yaw = state.translocator_studio_settings.yaw_deg;
            let mut speed = state.translocator_studio_settings.toss_speed;
            let mut elasticity = state.translocator_studio_settings.elasticity;
            let mut friction = state.translocator_studio_settings.friction;

            let mut changed = false;

            ui.horizontal(|ui| {
                ui.label("Pitch (°):");
                changed |= ui.add(egui::Slider::new(&mut pitch, -89.0..=89.0).step_by(1.0)).changed();
            });

            ui.horizontal(|ui| {
                ui.label("Yaw (°):");
                changed |= ui.add(egui::Slider::new(&mut yaw, 0.0..=360.0).step_by(5.0)).changed();
            });

            ui.horizontal(|ui| {
                ui.label("Speed (uu/s):");
                changed |= ui.add(egui::Slider::new(&mut speed, 200.0..=1800.0).step_by(25.0)).changed();
            });

            ui.horizontal(|ui| {
                ui.label("Elasticity:");
                changed |= ui.add(egui::Slider::new(&mut elasticity, 0.05..=0.95).step_by(0.05)).changed();
            });

            ui.horizontal(|ui| {
                ui.label("Friction:");
                changed |= ui.add(egui::Slider::new(&mut friction, 0.1..=1.0).step_by(0.05)).changed();
            });

            if changed {
                state.translocator_studio_settings.pitch_deg = pitch;
                state.translocator_studio_settings.yaw_deg = yaw;
                state.translocator_studio_settings.toss_speed = speed;
                state.translocator_studio_settings.elasticity = elasticity;
                state.translocator_studio_settings.friction = friction;

                state.translocator_studio_settings.active_config.pitch_deg = pitch;
                state.translocator_studio_settings.active_config.yaw_deg = yaw;
                state.translocator_studio_settings.active_config.toss_speed = speed;
                state.translocator_studio_settings.active_config.elasticity = elasticity;
                state.translocator_studio_settings.active_config.friction = friction;

                state.calculate_active_translocator_simulation();
            }
        });

        // Right Column: Transit Timing Diagnostics & Walking Comparison
        cols[1].group(|ui| {
            ui.label(RichText::new("⏱ Transit Timing vs Walking Audit").strong().color(Color32::from_rgb(100, 220, 255)));

            if let Some(ref sim) = state.translocator_studio_settings.active_simulation {
                ui.horizontal(|ui| {
                    ui.label("Flight Time:");
                    ui.label(RichText::new(format!("{:.2} s ({} bounces)", sim.total_flight_time, sim.bounce_count)).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Teleport Transit Time:");
                    ui.label(RichText::new(format!("{:.2} s (+0.2s switch latency)", sim.teleport_transit_time)).strong().color(Color32::from_rgb(255, 220, 100)));
                });

                ui.horizontal(|ui| {
                    ui.label("Running Travel Time:");
                    ui.label(RichText::new(format!("{:.2} s (@ 400 uu/s)", sim.walking_travel_time)).strong());
                });

                ui.horizontal(|ui| {
                    ui.label("Advantage:");
                    if sim.time_saved_seconds > 0.0 {
                        ui.label(
                            RichText::new(format!("🚀 +{:.2} s FASTER THAN RUNNING", sim.time_saved_seconds))
                                .color(Color32::from_rgb(80, 255, 120))
                                .strong(),
                        );
                    } else {
                        ui.label(
                            RichText::new(format!("⚠ {:.2} s SLOWER (walking is faster)", sim.time_saved_seconds.abs()))
                                .color(Color32::from_rgb(255, 100, 80))
                                .strong(),
                        );
                    }
                });

                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Aperture Clearance:");
                    if sim.min_aperture_clearance >= 15.0 {
                        ui.label(RichText::new(format!("✅ {:.1} uu (Safe Window/Bars Clearance)", sim.min_aperture_clearance)).color(Color32::from_rgb(80, 255, 120)));
                    } else {
                        ui.label(RichText::new(format!("⚠ {:.1} uu (Risk of clipping narrow grate)", sim.min_aperture_clearance)).color(Color32::from_rgb(255, 160, 40)));
                    }
                });
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.translocator_studio_settings.show_arcs_viewport, "Toss Arcs (WGPU)");
                ui.checkbox(&mut state.translocator_studio_settings.show_beacons_viewport, "Landing Beacons");
                ui.checkbox(&mut state.translocator_studio_settings.show_bounce_rings_viewport, "Bounce Rings");
            });
        });
    });

    ui.add_space(4.0);

    // 4. Placed Probes List
    ui.label(RichText::new("📍 Placed Translocator Probes").strong());
    egui::ScrollArea::vertical()
        .max_height(120.0)
        .show(ui, |ui| {
            let probes = state.translocator_studio_settings.probes.clone();
            if probes.is_empty() {
                ui.label(RichText::new("No probes placed. Click 'Place Probe at Pivot' to test specific jump points.").italics().color(Color32::GRAY));
            } else {
                let mut to_delete = None;
                for (idx, probe) in probes.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("#{}: {}", probe.id, probe.name)).strong());
                        ui.label(format!("Pos: ({:.0}, {:.0}, {:.0})", probe.location.x, probe.location.y, probe.location.z));
                        ui.label(format!("Pitch: {:.0}°, Yaw: {:.0}°", probe.pitch_deg, probe.yaw_deg));
                        ui.label(RichText::new(probe.preset.display_name()).color(Color32::from_rgb(255, 215, 100)));

                        if ui.button("📍 Focus Viewport").clicked() {
                            state.builder_brush.location = probe.location;
                            state.log(format!("Focused 3D camera on probe '{}'.", probe.name));
                        }

                        if ui.button("🗑 Delete").clicked() {
                            to_delete = Some((idx, probe.clone()));
                        }
                    });
                }

                if let Some((del_idx, probe)) = to_delete {
                    if del_idx < state.translocator_studio_settings.probes.len() {
                        state.translocator_studio_settings.probes.remove(del_idx);
                        state.history.push(crate::history::Action::DeleteTranslocatorProbe {
                            probe_index: del_idx,
                            name: probe.name.clone(),
                            location: probe.location,
                            pitch: probe.pitch_deg,
                            yaw: probe.yaw_deg,
                            preset: probe.preset,
                        });
                        state.calculate_active_translocator_simulation();
                        state.log(format!("Deleted translocator probe '{}'.", probe.name));
                    }
                }
            }
        });
}
