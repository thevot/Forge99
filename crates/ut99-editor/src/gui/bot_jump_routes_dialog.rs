// crates/ut99-editor/src/gui/bot_jump_routes_dialog.rs
//
// Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
// Ballistic parabolic trajectory calculation, Impact Hammer jump boost,
// bot translocator beacon toss simulation, and JumpSpot generation.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{BotJumpArchetype, BotJumpType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_bot_jump_routes_dialog {
        return;
    }

    let mut open = state.show_bot_jump_routes_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Bot AI Tactical JumpSpot & Translocator Route Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_bot_jump_routes_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Route Archetypes:").strong());
        let archetypes = [
            BotJumpArchetype::Deck16SlimeBridgeJump,
            BotJumpArchetype::FaceSniperTowerClimb,
            BotJumpArchetype::PhobosGravityLeap,
            BotJumpArchetype::CoretFlagRunAscent,
            BotJumpArchetype::TurbineVentVault,
        ];
        for arch in archetypes {
            let is_sel = state.bot_jump_routes_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_jump_route_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.bot_jump_routes_settings.show_trajectory_viewport, "Trajectory Arc");
        ui.checkbox(&mut state.bot_jump_routes_settings.show_landing_zone_viewport, "Landing Zone Ring");
        ui.checkbox(&mut state.bot_jump_routes_settings.show_jump_plate_viewport, "Jump Plate Brush");
        ui.separator();
        if ui.button("🔄 Recalculate Trajectory & Audit").clicked() {
            state.recalculate_jump_route_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Ballistic Solver & Positions
        columns[0].vertical(|ui| {
            ui.heading("Ballistic Flight Telemetry & Solver");
            render_trajectory_solver_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Route to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Jump Mechanics & Actor Configuration");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Navigation Reachability & Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_trajectory_solver_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let sol = &state.bot_jump_routes_settings.cached_solution;
    let metrics = &state.bot_jump_routes_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Card background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 24, 30));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 65, 85)), StrokeKind::Middle);

    // Reachability Status Banner
    let (status_text, status_col) = if sol.is_reachable {
        ("ROUTE REACHABLE & BALANCED", Color32::from_rgb(60, 235, 120))
    } else {
        ("PHYSICALLY UNREACHABLE (BOTS WILL FALL)", Color32::from_rgb(255, 60, 60))
    };

    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("Status: {}", status_text),
        egui::FontId::proportional(13.0),
        status_col,
    );

    // 2D Profile Arc Drawing in miniature
    let mini_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 36.0),
        Vec2::new(avail_w - 28.0, 36.0),
    );
    painter.rect_filled(mini_rect, 2.0, Color32::from_rgb(28, 36, 46));

    if sol.trajectory_points.len() >= 2 {
        let pts = &sol.trajectory_points;
        let arc_col = Color32::from_rgb(80, 200, 255);
        for i in 0..pts.len() - 1 {
            let u0 = i as f32 / pts.len() as f32;
            let u1 = (i + 1) as f32 / pts.len() as f32;
            let x0 = mini_rect.min.x + u0 * mini_rect.width();
            let x1 = mini_rect.min.x + u1 * mini_rect.width();
            let h_norm0 = ((pts[i].z - state.bot_jump_routes_settings.active_config.start_pos.z) / sol.apex_height_uu.max(1.0)).clamp(0.0, 1.0);
            let h_norm1 = ((pts[i+1].z - state.bot_jump_routes_settings.active_config.start_pos.z) / sol.apex_height_uu.max(1.0)).clamp(0.0, 1.0);
            let y0 = mini_rect.max.y - h_norm0 * (mini_rect.height() - 4.0) - 2.0;
            let y1 = mini_rect.max.y - h_norm1 * (mini_rect.height() - 4.0) - 2.0;
            painter.line_segment([Pos2::new(x0, y0), Pos2::new(x1, y1)], Stroke::new(1.5_f32, arc_col));
        }
    }

    // Telemetry Statistics
    let y_start = rect.min.y + 80.0;
    let stats = [
        format!("Apex Clearance: {:.1} uu", sol.apex_height_uu),
        format!("Total Flight Time: {:.2} s", sol.total_flight_time_secs),
        format!("Launch Speed: {:.0} uu/s", sol.launch_velocity.length()),
        format!("Vertical Vel (Vz): {:.0} uu/s", sol.launch_velocity.z),
        format!("Landing Dispersion: {:.1} uu", sol.landing_error_uu),
        format!("Self-Damage Penalty: {:.0} HP", metrics.self_damage_penalty),
        format!("Bot Skill Required: Level {}/7", metrics.min_bot_skill_required),
        format!("Traversal Cost: {}", metrics.bot_traversal_cost_rating),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 18.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(11.5), Color32::from_rgb(180, 200, 220));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.bot_jump_routes_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Launch / Start Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.start_pos.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.start_pos.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.start_pos.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.add_space(4.0);
        ui.label(egui::RichText::new("🎯 Target / Perch Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.target_pos.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.target_pos.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.target_pos.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Launch to Builder Brush").clicked() {
            cfg.start_pos = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.bot_jump_routes_settings.active_config = cfg.clone();
        state.recalculate_jump_route_preview();
        state.history.push(crate::history::Action::UpdateBotJumpRoutesConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Jump Route Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.bot_jump_routes_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Jump Type: {}", cfg.jump_type.display_name()));
        ui.label(format!("Actors Created: PathNode (Launch) -> JumpSpot (Target) {}",
            if cfg.b_always_translocate { "+ TranslocTarget" } else { "" }));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Bot Jump Route & Actors to World").strong()).clicked() {
            state.commit_jump_route_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.bot_jump_routes_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("bot_jump_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Tactical Jump Type:");
            egui::ComboBox::from_id_salt("jump_type_selector")
                .selected_text(cfg.jump_type.display_name())
                .show_ui(ui, |ui| {
                    let types = [
                        BotJumpType::StandardJump,
                        BotJumpType::DodgeLeap,
                        BotJumpType::ImpactHammerJump,
                        BotJumpType::JumpBootsJump,
                        BotJumpType::TranslocatorToss,
                    ];
                    for jt in types {
                        if ui.selectable_value(&mut cfg.jump_type, jt, jt.display_name()).clicked() {
                            changed = true;
                            if jt == BotJumpType::TranslocatorToss {
                                cfg.b_always_translocate = true;
                                cfg.b_impact_jump = false;
                            } else if jt == BotJumpType::ImpactHammerJump {
                                cfg.b_impact_jump = true;
                                cfg.b_always_translocate = false;
                            }
                        }
                    }
                });
            ui.end_row();

            ui.label("World Gravity (uu/s²):");
            if ui.add(egui::Slider::new(&mut cfg.gravity_uu_s2, 100.0..=1500.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Landing Tolerance (uu):");
            if ui.add(egui::Slider::new(&mut cfg.landing_tolerance_radius, 16.0..=128.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Minimum Bot Skill:");
            if ui.add(egui::Slider::new(&mut cfg.bot_skill_level, 1..=7).prefix("Skill ")).changed() { changed = true; }
            ui.end_row();

            ui.label("JumpSpot bAlwaysTranslocate:");
            if ui.checkbox(&mut cfg.b_always_translocate, "Always Translocate").changed() { changed = true; }
            ui.end_row();

            ui.label("JumpSpot bImpactJump:");
            if ui.checkbox(&mut cfg.b_impact_jump, "Requires Hammer Boost").changed() { changed = true; }
            ui.end_row();

            ui.label("Launch Plate Radius:");
            if ui.add(egui::Slider::new(&mut cfg.plate_radius, 24.0..=96.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Route Identification Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.bot_jump_routes_settings.active_config = cfg.clone();
        state.recalculate_jump_route_preview();
        state.history.push(crate::history::Action::UpdateBotJumpRoutesConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Bot Jump Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.bot_jump_routes_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Jump Route 100% Viable & Safe").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Bot trajectory arrives within target platform tolerance without obstruction.");
        });
        return;
    }

    egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
        for h in hazards {
            let (icon, header_col) = if h.is_critical {
                ("🚨", Color32::from_rgb(255, 80, 80))
            } else {
                ("⚠", Color32::from_rgb(255, 190, 50))
            };

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(icon);
                    ui.label(egui::RichText::new(&h.title).color(header_col).strong());
                });
                ui.label(&h.description);
            });
            ui.add_space(4.0);
        }
    });
}
