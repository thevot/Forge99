use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{evaluate_player_survival_time, FluidZonePreset};
use crate::editor_state::EditorState;

pub fn render_fluid_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_fluid_studio_dialog {
        return;
    }
    let mut is_open = state.show_fluid_studio_dialog;
    Window::new("Fluid Physics, Hazard Volumes & Environmental Dynamics Studio")
        .open(&mut is_open)
        .default_width(860.0)
        .default_height(620.0)
        .min_width(700.0)
        .min_height(480.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_fluid_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🏊 Fluid Physics, Hazard Volumes & Environmental Dynamics Studio");
    ui.label(
        RichText::new(
            "Simulate water buoyancy, fluid currents, lava/slime hazards, ground friction and terminal velocity.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 160, 175)),
    );
    ui.separator();

    let preset = state.fluid_studio_settings.selected_preset;

    // 1. Environmental Preset Selector
    ui.horizontal(|ui| {
        ui.label(RichText::new("Preset:").strong());
        let current_display = preset.display_name();

        let mut changed_preset = None;
        egui::ComboBox::from_id_salt("fluid_preset_combo")
            .selected_text(current_display)
            .width(360.0)
            .show_ui(ui, |ui| {
                for &p in FluidZonePreset::all() {
                    let is_sel = p == preset;
                    if ui.selectable_label(is_sel, p.display_name()).clicked() {
                        changed_preset = Some(p);
                    }
                }
            });

        if let Some(p) = changed_preset {
            state.fluid_studio_settings.selected_preset = p;
            state.fluid_studio_settings.preview_physics = p.to_physics();
        }

        if ui.button("➕ Place New Zone at Pivot").clicked() {
            state.place_physics_zone_at_pivot(preset);
        }

        if ui.button("⚡ Apply Preset to Selected").clicked() {
            state.apply_fluid_preset_to_selected(preset);
        }
    });

    ui.add_space(4.0);
    ui.label(
        RichText::new(preset.description())
            .size(10.5)
            .italics()
            .color(Color32::from_rgb(170, 175, 190)),
    );
    ui.add_space(8.0);

    // Advance simulation time
    if state.fluid_studio_settings.is_simulating {
        state.fluid_studio_settings.simulation_time += ui.input(|i| i.stable_dt);
    }
    let sim_time = state.fluid_studio_settings.simulation_time;
    let physics = &state.fluid_studio_settings.preview_physics;
    let hazard_color = preset.hazard_color_rgba();

    // 2. Interactive 2D Fluid Simulation Canvas
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("🌊 Dynamic Fluid & Buoyancy Simulation").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔄 Reset").clicked() {
                    state.fluid_studio_settings.simulation_time = 0.0;
                }
                if ui
                    .button(if state.fluid_studio_settings.is_simulating {
                        "⏸ Pause"
                    } else {
                        "▶ Play"
                    })
                    .clicked()
                {
                    state.fluid_studio_settings.is_simulating = !state.fluid_studio_settings.is_simulating;
                }
                ui.label(format!("Time: {:.2}s", sim_time));
            });
        });

        let canvas_size = vec2(ui.available_width(), 160.0);
        let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
        let rect = response.rect;

        // Background with fluid tint
        let bg_color = Color32::from_rgb(
            (20.0 + hazard_color[0] * 35.0) as u8,
            (25.0 + hazard_color[1] * 40.0) as u8,
            (32.0 + hazard_color[2] * 45.0) as u8,
        );
        painter.rect_filled(rect, 4.0, bg_color);
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)), StrokeKind::Inside);

        // Animated current flow streamers
        let flow_speed_x = physics.zone_velocity.x * 0.05;
        let _flow_speed_y = physics.zone_velocity.y * 0.05;
        for line in 0..6 {
            let y = rect.top() + (line as f32 + 1.0) * (rect.height() / 7.0);
            let offset_x = (sim_time * flow_speed_x + line as f32 * 60.0) % rect.width();

            let p0 = pos2(rect.left() + offset_x, y);
            let p1 = pos2(rect.left() + offset_x + 35.0, y);

            if p1.x <= rect.right() {
                let stream_col = Color32::from_rgba_unmultiplied(
                    (hazard_color[0] * 255.0) as u8,
                    (hazard_color[1] * 255.0) as u8,
                    (hazard_color[2] * 255.0) as u8,
                    120,
                );
                painter.line_segment([p0, p1], Stroke::new(2.0_f32, stream_col));
                // Streamer arrowhead
                painter.line_segment([p1, pos2(p1.x - 5.0, p1.y - 3.0)], Stroke::new(1.5_f32, stream_col));
                painter.line_segment([p1, pos2(p1.x - 5.0, p1.y + 3.0)], Stroke::new(1.5_f32, stream_col));
            }
        }

        // Water surface line (if water zone)
        if physics.b_water_zone {
            let water_y = rect.top() + 35.0;
            painter.line_segment(
                [pos2(rect.left(), water_y), pos2(rect.right(), water_y)],
                Stroke::new(2.0_f32, Color32::from_rgb(100, 200, 255)),
            );
            painter.text(
                pos2(rect.left() + 8.0, water_y - 12.0),
                egui::Align2::LEFT_TOP,
                "Water Surface",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(140, 210, 255),
            );
        }

        // Test pawn avatar (showing buoyancy bobbing or sinking)
        let center_x = rect.center().x;
        let base_y = if physics.b_water_zone {
            if physics.buoyancy_factor >= 1.0 {
                // Bobbing at water surface
                rect.top() + 45.0 + (sim_time * 2.5).sin() * 6.0
            } else {
                // Sinking down to floor
                let sink_depth = ((sim_time * 15.0) % 90.0).min(90.0);
                rect.top() + 45.0 + sink_depth
            }
        } else {
            // In air / space / gravity
            rect.center().y
        };

        let avatar_pos = pos2(center_x, base_y);
        let pawn_col = if physics.b_pain_zone {
            let pulse = ((sim_time * 6.0).sin() * 0.5 + 0.5) * 100.0;
            Color32::from_rgb(255, (100.0 + pulse) as u8, 80)
        } else {
            Color32::from_rgb(80, 220, 140)
        };

        // Draw avatar capsule
        painter.circle_filled(avatar_pos, 12.0, pawn_col);
        painter.circle_stroke(avatar_pos, 14.0, Stroke::new(1.5_f32, Color32::WHITE));
        painter.text(
            pos2(avatar_pos.x, avatar_pos.y + 18.0),
            egui::Align2::CENTER_TOP,
            if physics.b_pain_zone { "⚠ Pawn (Hazard!)" } else { "Pawn (100 HP)" },
            egui::FontId::proportional(10.5),
            Color32::WHITE,
        );

        // Survival meter if pain zone
        if let Some(surv) = evaluate_player_survival_time(physics, state.fluid_studio_settings.test_player_health) {
            let timer_str = format!("⏱ Survival Time: {:.1}s (Damage: {} DPS {})", surv, physics.damage_per_sec, physics.damage_type);
            painter.text(
                pos2(rect.right() - 10.0, rect.top() + 10.0),
                egui::Align2::RIGHT_TOP,
                timer_str,
                egui::FontId::proportional(11.0),
                Color32::from_rgb(255, 100, 100),
            );
        }
    });

    ui.add_space(8.0);

    // Extract local properties to avoid borrow conflicts in columns
    let mut b_water = state.fluid_studio_settings.preview_physics.b_water_zone;
    let mut b_pain = state.fluid_studio_settings.preview_physics.b_pain_zone;
    let mut vel = state.fluid_studio_settings.preview_physics.zone_velocity;
    let mut ground_fric = state.fluid_studio_settings.preview_physics.zone_ground_friction;
    let mut fluid_fric = state.fluid_studio_settings.preview_physics.zone_fluid_friction;
    let mut term_vel = state.fluid_studio_settings.preview_physics.zone_terminal_velocity;
    let mut buoyancy = state.fluid_studio_settings.preview_physics.buoyancy_factor;
    let mut dps = state.fluid_studio_settings.preview_physics.damage_per_sec;
    let mut dmg_type = state.fluid_studio_settings.preview_physics.damage_type.clone();
    let mut show_streamers = state.fluid_studio_settings.show_current_streamers_viewport;
    let mut show_boxes = state.fluid_studio_settings.show_hazard_boxes_viewport;
    let mut do_update_selected = false;

    // 3. Properties and Parameter Tuning Columns
    ui.columns(2, |cols| {
        // Left Column: Fluid Mechanics & Flow Vectors
        cols[0].group(|ui| {
            ui.heading("⚙ Fluid Mechanics & Current Vectors");
            ui.label(
                RichText::new("Authoritative Unreal Engine 1 properties on ZoneInfo.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            ui.checkbox(&mut b_water, "bWaterZone (Swimmable Fluid)");
            ui.checkbox(&mut b_pain, "bPainZone (Damaging Environmental Hazard)");

            ui.add_space(4.0);
            ui.label(RichText::new("ZoneVelocity (Flow / Draft Vector):").strong());
            ui.horizontal(|ui| {
                ui.label("X:");
                ui.add(egui::DragValue::new(&mut vel.x).speed(10.0));
                ui.label("Y:");
                ui.add(egui::DragValue::new(&mut vel.y).speed(10.0));
                ui.label("Z:");
                ui.add(egui::DragValue::new(&mut vel.z).speed(10.0));
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Ground Friction:");
                ui.add(egui::Slider::new(&mut ground_fric, 0.05..=10.0).step_by(0.05));
            });

            ui.horizontal(|ui| {
                ui.label("Fluid Friction:");
                ui.add(egui::Slider::new(&mut fluid_fric, 0.01..=5.0).step_by(0.05));
            });

            ui.horizontal(|ui| {
                ui.label("Terminal Velocity:");
                ui.add(egui::Slider::new(&mut term_vel, 200.0..=4000.0).step_by(50.0));
            });

            ui.horizontal(|ui| {
                ui.label("Buoyancy Factor:");
                ui.add(egui::Slider::new(&mut buoyancy, 0.5..=2.0).step_by(0.05));
            });
        });

        // Right Column: Hazard Damage & Viewport Overlays
        cols[1].group(|ui| {
            ui.heading("🔥 Environmental Hazard & Damage Rating");
            ui.label(
                RichText::new("Periodic damage and visualizer overlays.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("DamagePerSec:");
                ui.add(egui::Slider::new(&mut dps, 0..=100).step_by(5.0));
            });

            ui.horizontal(|ui| {
                ui.label("DamageType:");
                ui.text_edit_singleline(&mut dmg_type);
            });

            ui.add_space(6.0);
            ui.label(RichText::new("Viewport Visualizer Overlays:").strong());
            ui.checkbox(&mut show_streamers, "Show 3D Current Streamer Vectors in Viewport");
            ui.checkbox(&mut show_boxes, "Show Hazard Wireframe Boxes in Viewport");

            ui.add_space(8.0);
            if ui.button("⚡ Update Selected Zone(s)").clicked() {
                do_update_selected = true;
            }
        });
    });

    // Write back modified properties
    state.fluid_studio_settings.preview_physics.b_water_zone = b_water;
    state.fluid_studio_settings.preview_physics.b_pain_zone = b_pain;
    state.fluid_studio_settings.preview_physics.zone_velocity = vel;
    state.fluid_studio_settings.preview_physics.zone_ground_friction = ground_fric;
    state.fluid_studio_settings.preview_physics.zone_fluid_friction = fluid_fric;
    state.fluid_studio_settings.preview_physics.zone_terminal_velocity = term_vel;
    state.fluid_studio_settings.preview_physics.buoyancy_factor = buoyancy;
    state.fluid_studio_settings.preview_physics.damage_per_sec = dps;
    state.fluid_studio_settings.preview_physics.damage_type = dmg_type;
    state.fluid_studio_settings.show_current_streamers_viewport = show_streamers;
    state.fluid_studio_settings.show_hazard_boxes_viewport = show_boxes;

    if do_update_selected {
        let p = state.fluid_studio_settings.preview_physics.clone();
        state.update_selected_zone_physics(&p);
    }

    ui.add_space(8.0);

    // 4. Level ZoneInfo Table
    ui.group(|ui| {
        ui.heading("📋 ZoneInfo Actors in Level");
        let zones = state.collect_level_zone_physics();

        if zones.is_empty() {
            ui.label(
                RichText::new("No ZoneInfo or specialized hazard actors found in active level.")
                    .italics()
                    .color(Color32::from_rgb(180, 180, 190)),
            );
        } else {
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("zone_physics_table")
                        .striped(true)
                        .min_col_width(90.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Actor").strong());
                            ui.label(RichText::new("Water").strong());
                            ui.label(RichText::new("Hazard / DPS").strong());
                            ui.label(RichText::new("Velocity").strong());
                            ui.label(RichText::new("Actions").strong());
                            ui.end_row();

                            for (idx, name, phys) in &zones {
                                ui.label(name);
                                ui.label(if phys.b_water_zone { "💧 Water" } else { "Air" });
                                if phys.b_pain_zone {
                                    ui.colored_label(
                                        Color32::from_rgb(255, 100, 100),
                                        format!("⚠ {} DPS ({})", phys.damage_per_sec, phys.damage_type),
                                    );
                                } else {
                                    ui.colored_label(Color32::from_rgb(80, 220, 120), "Safe");
                                }
                                ui.label(format!("({:.0}, {:.0}, {:.0})", phys.zone_velocity.x, phys.zone_velocity.y, phys.zone_velocity.z));

                                if ui.button("🎯 Focus 3D").clicked() {
                                    if let Some(ref unr) = state.loaded_unr {
                                        if let Some(a) = unr.actors.get(*idx) {
                                            state.camera.position = a.location + glam::Vec3::new(0.0, -150.0, 50.0);
                                            state.selected_actor_index = Some(*idx);
                                        }
                                    }
                                }
                                ui.end_row();
                            }
                        });
                });
        }
    });
}
