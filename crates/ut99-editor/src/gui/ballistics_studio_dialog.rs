use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{audit_combat_sightline, WeaponCombatPreset};
use crate::editor_state::EditorState;

pub fn render_ballistics_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_ballistics_studio_dialog {
        return;
    }
    let mut is_open = state.show_ballistics_studio_dialog;
    Window::new("Weapon Ballistics, Ricochet & Combat Sightline Studio")
        .open(&mut is_open)
        .default_width(880.0)
        .default_height(640.0)
        .min_width(720.0)
        .min_height(500.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_ballistics_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🎯 Weapon Ballistics, Ricochet & Combat Sightline Studio");
    ui.label(
        RichText::new(
            "Simulate projectile arcs, surface ricochets, splash damage clearance and sniper sightlines.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 160, 175)),
    );
    ui.separator();

    let preset = state.ballistics_studio_settings.selected_preset;

    // 1. Weapon Combat Preset Selector
    let mut do_recalc = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new("Weapon:").strong());
        let current_display = preset.display_name();

        let mut changed_preset = None;
        egui::ComboBox::from_id_salt("weapon_preset_combo")
            .selected_text(current_display)
            .width(360.0)
            .show_ui(ui, |ui| {
                for &p in WeaponCombatPreset::all() {
                    let is_sel = p == preset;
                    if ui.selectable_label(is_sel, p.display_name()).clicked() {
                        changed_preset = Some(p);
                    }
                }
            });

        if let Some(p) = changed_preset {
            state.ballistics_studio_settings.selected_preset = p;
            state.ballistics_studio_settings.preview_projectile = p.to_projectile();
            do_recalc = true;
        }

        if ui.button("➕ Place Ballistic Probe at Pivot").clicked() {
            state.place_ballistic_probe_at_pivot(preset);
        }

        if ui.button("⚡ Recalculate Trajectory").clicked() {
            do_recalc = true;
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

    if do_recalc || state.ballistics_studio_settings.active_trajectory.is_empty() {
        state.calculate_active_weapon_trajectory();
    }

    let trajectory = &state.ballistics_studio_settings.active_trajectory;
    let proj = &state.ballistics_studio_settings.preview_projectile;
    let trail_col = Color32::from_rgba_unmultiplied(
        (proj.trail_color_rgba[0] * 255.0) as u8,
        (proj.trail_color_rgba[1] * 255.0) as u8,
        (proj.trail_color_rgba[2] * 255.0) as u8,
        (proj.trail_color_rgba[3] * 255.0) as u8,
    );

    // 2. Interactive 2D Elevation & Trajectory Canvas
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("📊 2D Ballistic Profile (Distance vs Elevation)").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(last) = trajectory.last() {
                    ui.label(format!("Flight Time: {:.2}s | Points: {}", last.time_sec, trajectory.len()));
                }
            });
        });

        let canvas_size = vec2(ui.available_width(), 160.0);
        let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::hover());
        let rect = response.rect;

        painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 20, 26));
        painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(55, 60, 75)), StrokeKind::Inside);

        // Grid lines (Distance markers every 500 uu, Height markers every 200 uu)
        let origin_x = rect.left() + 20.0;
        let ground_y = rect.bottom() - 25.0;

        painter.line_segment([pos2(origin_x, ground_y), pos2(rect.right() - 10.0, ground_y)], Stroke::new(1.0_f32, Color32::from_rgb(80, 85, 100)));
        painter.text(pos2(origin_x, ground_y + 4.0), egui::Align2::LEFT_TOP, "Muzzle (0 uu)", egui::FontId::proportional(10.0), Color32::from_rgb(150, 155, 170));

        if !trajectory.is_empty() {
            let start_pos = trajectory[0].position;

            // Map trajectory points into 2D profile view: (horizontal dist along launch vector, delta Z)
            let scale_x = (rect.width() - 40.0) / 2500.0; // 2500 uu scale
            let scale_y = (rect.height() - 40.0) / 800.0;  // 800 uu elevation scale

            let mut prev_screen_pt = pos2(origin_x, ground_y - 20.0);

            for (idx, pt) in trajectory.iter().enumerate() {
                let diff = pt.position - start_pos;
                let horiz_dist = (diff.x * diff.x + diff.y * diff.y).sqrt();
                let delta_z = pt.position.z - start_pos.z;

                let screen_x = (origin_x + horiz_dist * scale_x).min(rect.right() - 5.0);
                let screen_y = (ground_y - (delta_z + 32.0) * scale_y).clamp(rect.top() + 5.0, ground_y);
                let cur_pt = pos2(screen_x, screen_y);

                if idx > 0 {
                    painter.line_segment([prev_screen_pt, cur_pt], Stroke::new(2.5_f32, trail_col));
                }

                // Bounce marker
                if pt.is_bounce {
                    painter.circle_filled(cur_pt, 4.5, Color32::YELLOW);
                    painter.circle_stroke(cur_pt, 6.0, Stroke::new(1.5_f32, Color32::WHITE));
                }

                // Impact / Detonation marker
                if pt.is_impact {
                    painter.circle_filled(cur_pt, 6.0, Color32::RED);
                    let blast_r = (proj.damage_radius * scale_x).clamp(10.0, 50.0);
                    painter.circle_stroke(cur_pt, blast_r, Stroke::new(1.5_f32, Color32::from_rgba_unmultiplied(255, 100, 50, 180)));
                    painter.text(
                        pos2(cur_pt.x, cur_pt.y - 14.0),
                        egui::Align2::CENTER_BOTTOM,
                        format!("Explosion ({:.0} uu AOE)", proj.damage_radius),
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(255, 140, 60),
                    );
                }

                prev_screen_pt = cur_pt;
            }
        }
    });

    ui.add_space(8.0);

    // Extract local properties to avoid borrow conflicts in columns
    let mut pitch = state.ballistics_studio_settings.pitch_deg;
    let mut yaw = state.ballistics_studio_settings.yaw_deg;
    let mut speed = state.ballistics_studio_settings.preview_projectile.initial_speed;
    let mut gravity_scale = state.ballistics_studio_settings.preview_projectile.gravity_scale;
    let mut bounces = state.ballistics_studio_settings.preview_projectile.bounces_max;
    let mut elasticity = state.ballistics_studio_settings.preview_projectile.elasticity;
    let mut damage_radius = state.ballistics_studio_settings.preview_projectile.damage_radius;
    let mut damage_amt = state.ballistics_studio_settings.preview_projectile.damage_amount;
    let mut max_time = state.ballistics_studio_settings.max_simulation_time;
    let mut show_trajectories = state.ballistics_studio_settings.show_trajectories_viewport;
    let mut show_blast = state.ballistics_studio_settings.show_blast_spheres_viewport;
    let mut show_normals = state.ballistics_studio_settings.show_bounce_normals_viewport;

    // 3. Aiming & Combat Sightline Columns
    ui.columns(2, |cols| {
        // Left Column: Aiming & Ballistic Controls
        cols[0].group(|ui| {
            ui.heading("🕹 Aiming & Launch Controls");
            ui.label(
                RichText::new("Ballistic launch trajectory under UT99 gravity (-950 uu/s²).")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Pitch (Elevation):");
                ui.add(egui::Slider::new(&mut pitch, -89.0..=89.0).suffix("°"));
            });

            ui.horizontal(|ui| {
                ui.label("Yaw (Heading):");
                ui.add(egui::Slider::new(&mut yaw, 0.0..=360.0).suffix("°"));
            });

            ui.horizontal(|ui| {
                ui.label("Muzzle Speed:");
                ui.add(egui::Slider::new(&mut speed, 400.0..=30000.0).suffix(" uu/s").logarithmic(true));
            });

            ui.horizontal(|ui| {
                ui.label("Gravity Scale:");
                ui.add(egui::Slider::new(&mut gravity_scale, 0.0..=2.0).step_by(0.1));
            });

            ui.horizontal(|ui| {
                ui.label("Max Bounces:");
                ui.add(egui::Slider::new(&mut bounces, 0..=6));
            });

            ui.horizontal(|ui| {
                ui.label("Elasticity (Restitution):");
                ui.add(egui::Slider::new(&mut elasticity, 0.0..=1.0).step_by(0.05));
            });

            ui.horizontal(|ui| {
                ui.label("Splash Damage Radius:");
                ui.add(egui::Slider::new(&mut damage_radius, 0.0..=500.0).suffix(" uu"));
            });

            ui.horizontal(|ui| {
                ui.label("Direct Damage:");
                ui.add(egui::Slider::new(&mut damage_amt, 10.0..=300.0).suffix(" HP"));
            });

            ui.horizontal(|ui| {
                ui.label("Max Flight Time:");
                ui.add(egui::Slider::new(&mut max_time, 0.5..=6.0).suffix("s"));
            });
        });

        // Right Column: Sightline Clearance & Viewport Overlays
        cols[1].group(|ui| {
            ui.heading("🔬 Line-of-Sight & Sightline Audit");
            ui.label(
                RichText::new("Checks unoccluded weapon fire clearance from Builder Brush to camera.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            let muzzle = state.builder_brush.location + glam::Vec3::new(0.0, 0.0, 32.0);
            let target = state.camera.position;
            let audit = audit_combat_sightline(muzzle, target, &state.world_brushes);

            ui.horizontal(|ui| {
                ui.label("Sightline Status:");
                if audit.is_unobstructed {
                    ui.colored_label(Color32::from_rgb(80, 240, 120), RichText::new("✅ 100% Unobstructed Clear Shot").strong());
                } else {
                    ui.colored_label(
                        Color32::from_rgb(255, 90, 90),
                        RichText::new(format!("❌ Blocked by '{}'", audit.first_blocking_brush.as_deref().unwrap_or("World Brush"))).strong(),
                    );
                }
            });

            ui.horizontal(|ui| {
                ui.label("Hit Distance:");
                ui.label(format!("{:.1} uu / {:.1} uu ({:.0}% range)", audit.hit_distance, audit.total_distance, (audit.hit_distance / audit.total_distance.max(1.0)) * 100.0));
            });

            ui.add_space(6.0);
            ui.label(RichText::new("Viewport Visualizer Overlays:").strong());
            ui.checkbox(&mut show_trajectories, "Show 3D Trajectory Arcs in Viewport");
            ui.checkbox(&mut show_blast, "Show Explosion AOE Spheres in Viewport");
            ui.checkbox(&mut show_normals, "Show Ricochet Surface Normal Arrows");

            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("Current Weapon: {}", preset.display_name()))
                    .size(11.0)
                    .color(Color32::from_rgb(140, 210, 255)),
            );
        });
    });

    // Write back modified properties
    let pitch_changed = (state.ballistics_studio_settings.pitch_deg - pitch).abs() > 1e-3;
    let yaw_changed = (state.ballistics_studio_settings.yaw_deg - yaw).abs() > 1e-3;
    let speed_changed = (state.ballistics_studio_settings.preview_projectile.initial_speed - speed).abs() > 1e-3;

    state.ballistics_studio_settings.pitch_deg = pitch;
    state.ballistics_studio_settings.yaw_deg = yaw;
    state.ballistics_studio_settings.preview_projectile.initial_speed = speed;
    state.ballistics_studio_settings.preview_projectile.gravity_scale = gravity_scale;
    state.ballistics_studio_settings.preview_projectile.bounces_max = bounces;
    state.ballistics_studio_settings.preview_projectile.elasticity = elasticity;
    state.ballistics_studio_settings.preview_projectile.damage_radius = damage_radius;
    state.ballistics_studio_settings.preview_projectile.damage_amount = damage_amt;
    state.ballistics_studio_settings.max_simulation_time = max_time;
    state.ballistics_studio_settings.show_trajectories_viewport = show_trajectories;
    state.ballistics_studio_settings.show_blast_spheres_viewport = show_blast;
    state.ballistics_studio_settings.show_bounce_normals_viewport = show_normals;

    if pitch_changed || yaw_changed || speed_changed {
        state.calculate_active_weapon_trajectory();
    }

    ui.add_space(8.0);

    // 4. Placed Ballistic Probes Table
    ui.group(|ui| {
        ui.heading("📍 Placed Combat Test Probes");
        let probes = state.ballistics_studio_settings.probes.clone();

        if probes.is_empty() {
            ui.label(
                RichText::new("No ballistic probe markers placed yet. Click 'Place Ballistic Probe at Pivot' to record test locations.")
                    .italics()
                    .color(Color32::from_rgb(180, 180, 190)),
            );
        } else {
            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("ballistic_probes_table")
                        .striped(true)
                        .min_col_width(85.0)
                        .show(ui, |ui| {
                            ui.label(RichText::new("Name").strong());
                            ui.label(RichText::new("Weapon").strong());
                            ui.label(RichText::new("Location").strong());
                            ui.label(RichText::new("Pitch/Yaw").strong());
                            ui.label(RichText::new("Actions").strong());
                            ui.end_row();

                            let mut to_delete = None;
                            let mut to_fire = None;

                            for (idx, p) in probes.iter().enumerate() {
                                ui.label(&p.name);
                                ui.label(p.preset.display_name());
                                ui.label(format!("({:.0}, {:.0}, {:.0})", p.location.x, p.location.y, p.location.z));
                                ui.label(format!("{:.0}° / {:.0}°", p.pitch_deg, p.yaw_deg));

                                ui.horizontal(|ui| {
                                    if ui.button("🎯 Focus").clicked() {
                                        state.camera.position = p.location + glam::Vec3::new(0.0, -120.0, 40.0);
                                    }
                                    if ui.button("🚀 Fire").clicked() {
                                        to_fire = Some(idx);
                                    }
                                    if ui.button("🗑 Del").clicked() {
                                        to_delete = Some(idx);
                                    }
                                });
                                ui.end_row();
                            }

                            if let Some(f_idx) = to_fire {
                                let p = &state.ballistics_studio_settings.probes[f_idx];
                                state.builder_brush.location = p.location;
                                state.ballistics_studio_settings.pitch_deg = p.pitch_deg;
                                state.ballistics_studio_settings.yaw_deg = p.yaw_deg;
                                state.ballistics_studio_settings.selected_preset = p.preset;
                                state.ballistics_studio_settings.preview_projectile = p.preset.to_projectile();
                                state.calculate_active_weapon_trajectory();
                            }

                            if let Some(del_idx) = to_delete {
                                let removed = state.ballistics_studio_settings.probes.remove(del_idx);
                                state.history.push(crate::history::Action::DeleteBallisticProbe {
                                    probe_index: del_idx,
                                    name: removed.name,
                                    location: removed.location,
                                    pitch: removed.pitch_deg,
                                    yaw: removed.yaw_deg,
                                    preset: removed.preset,
                                });
                                state.calculate_active_weapon_trajectory();
                            }
                        });
                });
        }
    });
}
