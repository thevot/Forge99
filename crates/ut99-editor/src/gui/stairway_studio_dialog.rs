// crates/ut99-editor/src/gui/stairway_studio_dialog.rs
//
// Interactive Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{CollisionHullSolidity, CsgOperation, StairArchetype, StairHazardSeverity};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_stairway_studio_dialog {
        return;
    }

    let mut open = state.show_stairway_studio_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Curved Stairway, Spiral Ramp & Smooth Collision Hull Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_stairway_studio_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Stair Archetypes:").strong());
        for &arch in StairArchetype::all() {
            let is_sel = state.stairway_studio_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_stair_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.stairway_studio_settings.show_stair_ghost_viewport, "Step Wireframe");
        ui.checkbox(&mut state.stairway_studio_settings.show_collision_ramp_ghost_viewport, "Smooth Ramp Wedge (Cyan)");
        ui.checkbox(&mut state.stairway_studio_settings.show_handrail_ghost_viewport, "Handrail Banister");
        ui.checkbox(&mut state.stairway_studio_settings.show_bot_waypoints_viewport, "Bot Centerline Path");
        ui.separator();
        if ui.button("🔄 Recalculate Geometry").clicked() {
            state.recalculate_stair_geometry();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Elevation Canvas, Metrics & Commit Actions
        columns[0].vertical(|ui| {
            ui.heading("2D Elevation & Slope Profile");
            render_elevation_canvas(ui, state);

            ui.add_space(8.0);
            render_architectural_metrics_card(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Geometry to Level");
            render_commit_buttons(ui, state);
        });

        // Right Column: Parameters, Safety Audit & Bot Pathing
        columns[1].vertical(|ui| {
            ui.heading("Stairway Engineering Parameters");
            render_parameters_grid(ui, state);

            ui.add_space(8.0);
            ui.heading("UT99 Physics & Ergonomics Safety Audit");
            render_safety_audit_card(ui, state);

            ui.add_space(8.0);
            ui.heading("Bot Navigation & ReachSpec Connectivity");
            render_bot_pathing_card(ui, state);
        });
    });
}

fn render_elevation_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width().max(200.0), 240.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)), StrokeKind::Inside);

    let cfg = &state.stairway_studio_settings.active_config;
    let metrics = &state.stairway_studio_settings.cached_metrics;

    let steps = cfg.step_count.max(1);
    let total_w = metrics.total_horizontal_run.max(32.0);
    let total_h = metrics.total_height.max(16.0);

    // Padding & coordinate scaling
    let pad = 24.0;
    let draw_w = (rect.width() - pad * 2.0).max(10.0);
    let draw_h = (rect.height() - pad * 2.0).max(10.0);

    let scale_x = draw_w / total_w;
    let scale_y = draw_h / total_h;
    let scale = scale_x.min(scale_y);

    let origin = Pos2::new(rect.min.x + pad, rect.max.y - pad);

    let to_canvas = |x: f32, z: f32| -> Pos2 {
        Pos2::new(origin.x + x * scale, origin.y - z * scale)
    };

    // Draw baseline
    painter.line_segment(
        [origin, Pos2::new(rect.max.x - pad, origin.y)],
        Stroke::new(1.0_f32, Color32::from_rgb(70, 80, 95)),
    );

    // Draw steps profile (Amber / Gold)
    let step_col = Color32::from_rgb(240, 180, 50);
    for i in 0..steps {
        let x0 = (i as f32 / steps as f32) * total_w;
        let x1 = ((i + 1) as f32 / steps as f32) * total_w;
        let z_prev = i as f32 * cfg.step_height;
        let z_curr = (i + 1) as f32 * cfg.step_height;

        let p_riser_bot = to_canvas(x0, z_prev);
        let p_riser_top = to_canvas(x0, z_curr);
        let p_tread_end = to_canvas(x1, z_curr);

        // Vertical riser
        painter.line_segment([p_riser_bot, p_riser_top], Stroke::new(2.0_f32, step_col));
        // Horizontal tread
        painter.line_segment([p_riser_top, p_tread_end], Stroke::new(2.0_f32, step_col));
    }

    // Draw smooth collision ramp wedge slope line (Cyan dashed/glow)
    if cfg.add_smooth_collision_hull {
        let ramp_p0 = to_canvas(0.0, 0.0);
        let ramp_p1 = to_canvas(total_w, total_h);
        painter.line_segment(
            [ramp_p0, ramp_p1],
            Stroke::new(2.5_f32, Color32::from_rgb(50, 220, 240)),
        );
    }

    // Draw MaxStepHeight limit reference marker (25 uu)
    let max_step_y = to_canvas(0.0, 25.0).y;
    if max_step_y >= rect.min.y && max_step_y <= rect.max.y {
        painter.line_segment(
            [Pos2::new(rect.min.x + 8.0, max_step_y), Pos2::new(rect.min.x + 40.0, max_step_y)],
            Stroke::new(1.5_f32, Color32::from_rgb(255, 60, 60)),
        );
        painter.text(
            Pos2::new(rect.min.x + 44.0, max_step_y),
            egui::Align2::LEFT_CENTER,
            "25.0 uu MaxStepHeight Limit",
            egui::FontId::proportional(10.0),
            Color32::from_rgb(255, 90, 90),
        );
    }

    // Tooltip with slope details
    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Stairway Incline Elevation").strong());
        ui.label(format!("Total Rise: {:.1} uu | Total Run: {:.1} uu", total_h, total_w));
        ui.label(format!("Slope Pitch Angle: {:.1}°", metrics.slope_pitch_degrees));
        ui.label(format!("Riser Height: {:.1} uu | Tread Run: {:.1} uu", cfg.step_height, metrics.step_run));
    });
}

fn render_architectural_metrics_card(ui: &mut egui::Ui, state: &EditorState) {
    let metrics = &state.stairway_studio_settings.cached_metrics;
    let cfg = &state.stairway_studio_settings.active_config;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Architectural Cadence:").strong());
            let blondel = metrics.blondel_score;
            let (col, eval) = if blondel >= 60.0 && blondel <= 68.0 {
                (Color32::from_rgb(50, 220, 100), "Optimal (Blondel Stride ~64 uu)")
            } else if blondel >= 50.0 && blondel <= 78.0 {
                (Color32::from_rgb(240, 200, 50), "Acceptable Cadence")
            } else {
                (Color32::from_rgb(255, 100, 100), "Awkward Stride Pitch")
            };
            ui.colored_label(col, format!("{:.1} uu ({})", blondel, eval));
        });

        ui.horizontal(|ui| {
            ui.label(format!("Slope Angle: {:.1}°", metrics.slope_pitch_degrees));
            ui.separator();
            ui.label(format!("Total Height: {:.0} uu", metrics.total_height));
            ui.separator();
            ui.label(format!("Total Run: {:.0} uu", metrics.total_horizontal_run));
            ui.separator();
            ui.label(format!("Flight Width: {:.0} uu", cfg.step_width));
        });
    });
}

fn render_commit_buttons(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal_wrapped(|ui| {
        if ui.button("🔴 Set Red Builder Brush")
            .on_hover_text("Assigns this staircase geometry to the Red Active Builder Brush")
            .clicked()
        {
            state.commit_stair_to_builder_brush();
        }

        if ui.button("✨ Commit Stairs + Smooth Ramp")
            .on_hover_text("Competitive Combo: Creates Additive visual steps + matching Invisible SemiSolid collision ramp")
            .clicked()
        {
            state.commit_stair_and_ramp_to_world(CsgOperation::Add);
        }

        if ui.button("➕ Add Solid Brush")
            .on_hover_text("Adds a permanent solid world brush without ramp")
            .clicked()
        {
            state.commit_stair_and_ramp_to_world(CsgOperation::Add);
        }

        if ui.button("🛡 Make SemiSolid")
            .on_hover_text("Creates an intricate non-BSP-cutting SemiSolid staircase brush")
            .clicked()
        {
            let (stair_idx, _) = state.commit_stair_and_ramp_to_world(CsgOperation::Add);
            if let Some(b) = state.world_brushes.get_mut(stair_idx) {
                for p in &mut b.polys {
                    p.flags |= ut99_bsp::PF_SEMISOLID;
                }
            }
        }

        if ui.button("➖ Subtract Shaft")
            .on_hover_text("Subtracts this stair volume to carve out a staircase well")
            .clicked()
        {
            state.commit_stair_and_ramp_to_world(CsgOperation::Subtract);
        }
    });
}

fn render_parameters_grid(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("stair_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Step Count:");
        if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.step_count).speed(1).range(2..=64)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Step Riser Height (uu):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.step_height).speed(0.5).range(4.0..=48.0)).changed() {
                changed = true;
            }
            if state.stairway_studio_settings.active_config.step_height > 24.0 {
                ui.colored_label(Color32::from_rgb(255, 80, 80), "⚠ >24 uu (Exceeds MaxStepHeight)");
            } else {
                ui.colored_label(Color32::from_rgb(80, 220, 100), "✓ Safe");
            }
        });
        ui.end_row();

        ui.label("Step Tread Run / Length (uu):");
        if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.step_length).speed(1.0).range(8.0..=128.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Step Flight Width (uu):");
        if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.step_width).speed(2.0).range(48.0..=512.0)).changed() {
            changed = true;
        }
        ui.end_row();

        let is_curved = matches!(
            state.stairway_studio_settings.active_config.archetype,
            StairArchetype::Curved90QuarterTurn
                | StairArchetype::Curved180HalfTurn
                | StairArchetype::SpiralTower
        );

        if is_curved {
            ui.label("Inner Well Radius (uu):");
            if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.inner_radius).speed(2.0).range(16.0..=512.0)).changed() {
                changed = true;
            }
            ui.end_row();

            ui.label("Total Sweep Angle (°):");
            if ui.add(egui::DragValue::new(&mut state.stairway_studio_settings.active_config.sweep_angle_deg).speed(5.0).range(15.0..=720.0)).changed() {
                changed = true;
            }
            ui.end_row();

            ui.label("Counter-Clockwise:");
            if ui.checkbox(&mut state.stairway_studio_settings.active_config.counter_clockwise, "Winding Direction").changed() {
                changed = true;
            }
            ui.end_row();
        }

        ui.label("Solid Ground Fill:");
        if ui.checkbox(&mut state.stairway_studio_settings.active_config.has_subgrade_solid_fill, "Extend to Floor (Z=0)").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Smooth Collision Hull:");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut state.stairway_studio_settings.active_config.add_smooth_collision_hull, "Generate Wedge Ramp").changed() {
                changed = true;
            }
            egui::ComboBox::from_id_salt("ramp_solidity_combo")
                .selected_text(state.stairway_studio_settings.active_config.collision_hull_solidity.display_name())
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut state.stairway_studio_settings.active_config.collision_hull_solidity, CollisionHullSolidity::InvisibleSemiSolid, "Invisible Semi-Solid").clicked() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut state.stairway_studio_settings.active_config.collision_hull_solidity, CollisionHullSolidity::InvisibleSolid, "Invisible Solid").clicked() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut state.stairway_studio_settings.active_config.collision_hull_solidity, CollisionHullSolidity::NonSolid, "Non-Solid (Guide)").clicked() {
                        changed = true;
                    }
                });
        });
        ui.end_row();

        ui.label("Outer Handrail Banister:");
        if ui.checkbox(&mut state.stairway_studio_settings.active_config.add_handrail_banister, "Include Guard Rail").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Surface Texture:");
        if ui.text_edit_singleline(&mut state.stairway_studio_settings.active_config.texture_name).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_stair_geometry();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.stairway_studio_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.severity == StairHazardSeverity::Critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} CRITICAL HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Ergonomic Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% UT99 Tournament Compliant").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("All stair dimensions comply with Unreal Tournament 99 physics standards and bot navigation guidelines.");
        } else {
            egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                for hazard in hazards {
                    ui.horizontal(|ui| {
                        let (badge_col, badge_txt) = match hazard.severity {
                            StairHazardSeverity::Critical => (Color32::from_rgb(255, 60, 60), "CRITICAL"),
                            StairHazardSeverity::Warning => (Color32::from_rgb(240, 180, 50), "WARN"),
                            StairHazardSeverity::Info => (Color32::from_rgb(100, 180, 255), "INFO"),
                        };
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

fn render_bot_pathing_card(ui: &mut egui::Ui, state: &EditorState) {
    let waypoints = &state.stairway_studio_settings.cached_waypoints;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(format!("Waypoints ({} stations):", waypoints.len())).strong());
            let all_valid = waypoints.iter().all(|wp| wp.reachspec_valid);
            if all_valid {
                ui.colored_label(Color32::from_rgb(60, 220, 100), "✓ ReachSpec Valid");
            } else {
                ui.colored_label(Color32::from_rgb(255, 80, 80), "⛔ ReachSpec Blocked");
            }
        });

        egui::ScrollArea::vertical().max_height(90.0).show(ui, |ui| {
            for wp in waypoints {
                ui.horizontal(|ui| {
                    ui.label(&wp.name);
                    ui.label(format!("({:.0}, {:.0}, {:.0})", wp.position.x, wp.position.y, wp.position.z));
                    if wp.reachspec_valid {
                        ui.colored_label(Color32::from_rgb(60, 220, 100), "OK");
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 80, 80), "BLOCKED");
                    }
                });
            }
        });
    });
}
