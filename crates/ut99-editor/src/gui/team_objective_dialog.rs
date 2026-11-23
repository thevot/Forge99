// crates/ut99-editor/src/gui/team_objective_dialog.rs
//
// Interactive Team Base Objective, CTF Flag Run Route & Defense AI Studio
// Provides analytical pathfinding, flank route discovery, run-time asymmetry analysis,
// chokepoint detection, defense point line-of-sight verification, and Domination triangle metrics
// for tournament level design in Unreal Tournament 99.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{
    DefenseAuditSeverity, GameObjectiveType, TeamColor, TeamObjectivePreset,
};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_team_objective_dialog {
        return;
    }

    let mut open = state.show_team_objective_dialog;
    crate::gui::modal::studio_dialog_window(ctx, "Team Objective, CTF Route & Defense AI Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_content(ui, state);
            });
        });
    state.show_team_objective_dialog = open;
}

fn render_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Auto-refresh scan if objectives are empty
    if state.team_objective_settings.detected_objectives.is_empty() {
        state.scan_and_refresh_team_objectives();
    }

    ui.horizontal(|ui| {
        ui.heading("🏆 Tournament Objective & Defense AI Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Refresh Analysis").clicked() {
                state.scan_and_refresh_team_objectives();
            }
        });
    });
    ui.separator();

    // Mode Selector & Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Game Mode:").strong());
        let current_mode = state.team_objective_settings.active_game_mode;

        for mode in [
            GameObjectiveType::CaptureTheFlag,
            GameObjectiveType::Domination,
            GameObjectiveType::Assault,
            GameObjectiveType::TeamDeathmatch,
        ] {
            let is_sel = current_mode == mode;
            if ui.selectable_label(is_sel, mode.display_name()).clicked() {
                state.team_objective_settings.active_game_mode = mode;
                state.scan_and_refresh_team_objectives();
            }
        }
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Iconic Arenas:").strong());
        for preset in [
            TeamObjectivePreset::FaceTowerSniperAlley,
            TeamObjectivePreset::CoretIndustrialFlanks,
            TeamObjectivePreset::GauntletMoatChokepoints,
            TeamObjectivePreset::CrypticEquilateralDom,
            TeamObjectivePreset::OverlordBeachheadAssault,
        ] {
            let is_sel = state.team_objective_settings.selected_preset == Some(preset);
            if ui.selectable_label(is_sel, preset.display_name()).clicked() {
                state.apply_team_objective_preset(preset);
            }
        }
    });

    ui.separator();

    // Top Section: 2D Radar Canvas & Asymmetry Metrics
    ui.horizontal(|ui| {
        // Left Column: 2D Radar Canvas
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("🗺 Tactical Route & Objective Radar (XY)").strong());
            render_radar_canvas(ui, state);
        });

        ui.separator();

        // Right Column: Asymmetry & Timing Metrics
        ui.vertical(|ui| {
            ui.set_min_width(320.0);
            render_metrics_panel(ui, state);
        });
    });

    ui.separator();

    // Bottom Section: Objectives, Defense AI Table, and Audit Issues
    egui::ScrollArea::vertical()
        .max_height(240.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Objectives and Deployment Column
                ui.vertical(|ui| {
                    ui.set_width(360.0);
                    render_objectives_list(ui, state);
                });

                ui.separator();

                // Defense Points & Audit Column
                ui.vertical(|ui| {
                    render_audit_and_defense_table(ui, state);
                });
            });
        });

    ui.separator();

    // Viewport visualizer toggles
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("3D Viewport Visualizers:").strong());
        ui.checkbox(&mut state.team_objective_settings.show_route_ribbons_viewport, "Route Ribbons");
        ui.checkbox(&mut state.team_objective_settings.show_defense_cones_viewport, "Defense Vision Cones");
        ui.checkbox(&mut state.team_objective_settings.show_chokepoints_viewport, "Chokepoint Beacons");
        ui.checkbox(&mut state.team_objective_settings.show_flag_holograms_viewport, "Flag Holograms");
    });
}

fn render_radar_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let (response, painter) = ui.allocate_painter(Vec2::new(480.0, 260.0), egui::Sense::hover());
    let rect = response.rect;

    // Dark tactical radar background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 26));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 52, 75)), StrokeKind::Inside);

    // Grid lines
    let center = rect.center();
    let scale = 0.08; // 1 unit = 0.08 pixels (e.g. 4000 uu = 320 px)

    painter.line_segment(
        [Pos2::new(rect.min.x, center.y), Pos2::new(rect.max.x, center.y)],
        Stroke::new(0.5_f32, Color32::from_rgb(30, 40, 58)),
    );
    painter.line_segment(
        [Pos2::new(center.x, rect.min.y), Pos2::new(center.x, rect.max.y)],
        Stroke::new(0.5_f32, Color32::from_rgb(30, 40, 58)),
    );

    // Function to map world Vec3 to radar Pos2
    let world_to_screen = |loc: glam::Vec3| -> Pos2 {
        Pos2::new(
            center.x + loc.x * scale,
            center.y - loc.y * scale, // invert Y for screen
        )
    };

    // Draw Domination Triangle if active
    if let Some(ref tri) = state.team_objective_settings.dom_triangle {
        let cp_nodes: Vec<_> = state.team_objective_settings.detected_objectives
            .iter()
            .filter(|o| o.objective_type == GameObjectiveType::Domination)
            .collect();
        if cp_nodes.len() >= 3 {
            let p0 = world_to_screen(cp_nodes[0].location);
            let p1 = world_to_screen(cp_nodes[1].location);
            let p2 = world_to_screen(cp_nodes[2].location);

            painter.line_segment([p0, p1], Stroke::new(1.5_f32, Color32::from_rgb(180, 80, 220)));
            painter.line_segment([p1, p2], Stroke::new(1.5_f32, Color32::from_rgb(180, 80, 220)));
            painter.line_segment([p2, p0], Stroke::new(1.5_f32, Color32::from_rgb(180, 80, 220)));

            // Draw balance badge
            let tri_center = Pos2::new((p0.x + p1.x + p2.x) / 3.0, (p0.y + p1.y + p2.y) / 3.0);
            painter.text(
                tri_center,
                egui::Align2::CENTER_CENTER,
                format!("Triangle Bal: {:.0}%", tri.triangle_balance_factor * 100.0),
                egui::FontId::proportional(11.0),
                Color32::from_rgb(220, 160, 255),
            );
        }
    }

    // Draw Flag Routes
    for route in &state.team_objective_settings.routes {
        let (color, width) = if route.is_primary {
            (Color32::from_rgb(255, 200, 50), 2.0_f32)
        } else {
            (Color32::from_rgb(100, 180, 255), 1.5_f32)
        };

        if route.waypoints.len() >= 2 {
            for i in 0..route.waypoints.len() - 1 {
                let p1 = world_to_screen(route.waypoints[i]);
                let p2 = world_to_screen(route.waypoints[i + 1]);
                painter.line_segment([p1, p2], Stroke::new(width, color));
            }

            // Draw Route timing label midway
            let mid_idx = route.waypoints.len() / 2;
            let mid_pos = world_to_screen(route.waypoints[mid_idx]);
            painter.text(
                mid_pos + Vec2::new(0.0, -12.0),
                egui::Align2::CENTER_BOTTOM,
                format!("{}: {:.1}s", route.name, route.estimated_run_time_secs),
                egui::FontId::proportional(10.0),
                color,
            );
        }

        // Draw chokepoint markers
        for &cp in &route.chokepoint_locations {
            let cp_pos = world_to_screen(cp);
            painter.circle_stroke(cp_pos, 8.0, Stroke::new(1.5_f32, Color32::from_rgb(255, 120, 40)));
            painter.text(
                cp_pos,
                egui::Align2::CENTER_CENTER,
                "⚠",
                egui::FontId::proportional(10.0),
                Color32::from_rgb(255, 160, 60),
            );
        }
    }

    // Draw Defense Points with Vision Cones
    for dp in &state.team_objective_settings.detected_defense_points {
        let dp_pos = world_to_screen(dp.location);
        let dp_color = match dp.team {
            TeamColor::Red => Color32::from_rgb(255, 80, 80),
            TeamColor::Blue => Color32::from_rgb(80, 140, 255),
            TeamColor::Green => Color32::from_rgb(80, 220, 100),
            TeamColor::Gold => Color32::from_rgb(255, 210, 60),
        };

        painter.circle_filled(dp_pos, 4.0, dp_color);

        // Draw facing vision cone ray
        let yaw_rad = dp.rotation_yaw_deg.to_radians();
        let forward = Vec2::new(yaw_rad.cos(), -yaw_rad.sin()); // screen coords
        let ray_len = if dp.is_sniping { 35.0 } else { 20.0 };
        painter.line_segment(
            [dp_pos, dp_pos + forward * ray_len],
            Stroke::new(1.0_f32, dp_color),
        );
    }

    // Draw Objectives (FlagBases, ControlPoints)
    for obj in &state.team_objective_settings.detected_objectives {
        let obj_pos = world_to_screen(obj.location);
        let (obj_color, symbol) = match obj.team {
            Some(TeamColor::Red) => (Color32::from_rgb(255, 60, 60), "🚩 RED"),
            Some(TeamColor::Blue) => (Color32::from_rgb(60, 120, 255), "🚩 BLUE"),
            Some(TeamColor::Green) => (Color32::from_rgb(60, 220, 80), "🚩 GRN"),
            Some(TeamColor::Gold) => (Color32::from_rgb(255, 200, 40), "🚩 GLD"),
            None => (Color32::from_rgb(200, 100, 255), "⚡ DOM"),
        };

        // Outer pulsing ring
        painter.circle_stroke(obj_pos, 12.0, Stroke::new(1.5_f32, obj_color));
        painter.circle_filled(obj_pos, 6.0, obj_color);

        painter.text(
            obj_pos + Vec2::new(0.0, 14.0),
            egui::Align2::CENTER_TOP,
            symbol,
            egui::FontId::proportional(11.0),
            obj_color,
        );
    }
}

fn render_metrics_panel(ui: &mut egui::Ui, state: &EditorState) {
    ui.label(egui::RichText::new("⏱ Tournament Route Timing & Balance").strong());

    if let Some(ref asym) = state.team_objective_settings.asymmetry {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Red -> Blue Run Time:");
                ui.label(egui::RichText::new(format!("{:.1} s", asym.red_to_blue_time_secs)).strong().color(Color32::from_rgb(255, 100, 100)));
            });
            ui.horizontal(|ui| {
                ui.label("Blue -> Red Run Time:");
                ui.label(egui::RichText::new(format!("{:.1} s", asym.blue_to_red_time_secs)).strong().color(Color32::from_rgb(100, 160, 255)));
            });
            ui.horizontal(|ui| {
                ui.label("Run-Time Delta (Δt):");
                ui.label(egui::RichText::new(format!("{:.2} s ({:.1}%)", asym.delta_time_secs, asym.asymmetry_percentage)).strong());
            });

            if asym.is_competitively_fair {
                ui.label(
                    egui::RichText::new("⚖ 100% Competitive Fair (<=5% Delta)")
                        .color(Color32::from_rgb(80, 220, 100))
                        .strong(),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!("⚠ Asymmetric Hazard ({:.1}% Delta > 5%)", asym.asymmetry_percentage))
                        .color(Color32::from_rgb(255, 100, 60))
                        .strong(),
                );
            }
        });
    } else {
        ui.label(egui::RichText::new("Place both Red and Blue FlagBases to compute run times.").italics());
    }

    ui.add_space(8.0);
    ui.label(egui::RichText::new("Routes Discovered:").strong());
    for r in &state.team_objective_settings.routes {
        ui.horizontal(|ui| {
            let badge_color = if r.is_primary { Color32::from_rgb(255, 200, 50) } else { Color32::from_rgb(100, 180, 255) };
            ui.label(egui::RichText::new(&r.name).color(badge_color).strong());
            ui.label(format!("{:.1}s | {:.0} uu", r.estimated_run_time_secs, r.total_distance_uu));
            ui.label(format!("⚡ Translocator: {:.1}s", r.translocator_time_secs));
        });
    }
}

fn render_objectives_list(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.label(egui::RichText::new("🚩 Objectives in Level:").strong());

    let count = state.team_objective_settings.detected_objectives.len();
    if count == 0 {
        ui.label("No objectives detected.");
        return;
    }

    for (i, obj) in state.team_objective_settings.detected_objectives.iter().enumerate() {
        let is_selected = state.team_objective_settings.selected_objective_idx == i;
        let team_str = obj.team.map(|t| t.display_name()).unwrap_or("Neutral");
        let text = format!("{}: {} ({})", obj.name, obj.class_name, team_str);

        if ui.selectable_label(is_selected, text).clicked() {
            state.team_objective_settings.selected_objective_idx = i;
        }
    }

    ui.add_space(8.0);
    if ui.button(egui::RichText::new("🛡 Deploy 3 Coordinated DefensePoints").strong()).clicked() {
        state.generate_defense_points_for_active_objective();
    }
}

fn render_audit_and_defense_table(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.label(egui::RichText::new("📋 Defense AI & Route Health Audit:").strong());

    for issue in &state.team_objective_settings.audit_issues {
        let (icon, color) = match issue.severity {
            DefenseAuditSeverity::Critical => ("❌", Color32::from_rgb(255, 80, 80)),
            DefenseAuditSeverity::Warning => ("⚠", Color32::from_rgb(255, 180, 40)),
            DefenseAuditSeverity::Info => ("✅", Color32::from_rgb(80, 220, 100)),
        };

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(icon).color(color));
            ui.label(egui::RichText::new(&issue.title).strong().color(color));
            if let Some(loc) = issue.focus_location {
                if ui.small_button("🔍 Focus").clicked() {
                    state.camera.position = loc + glam::Vec3::new(0.0, -250.0, 150.0);
                }
            }
        });
        ui.label(egui::RichText::new(&issue.description).small());
    }

    ui.separator();
    ui.label(egui::RichText::new(format!("Active DefensePoints ({}):", state.team_objective_settings.detected_defense_points.len())).strong());

    for dp in &state.team_objective_settings.detected_defense_points {
        ui.horizontal(|ui| {
            let team_color = match dp.team {
                TeamColor::Red => Color32::from_rgb(255, 80, 80),
                TeamColor::Blue => Color32::from_rgb(80, 140, 255),
                _ => Color32::from_rgb(200, 200, 200),
            };
            ui.label(egui::RichText::new(&dp.actor_name).color(team_color));
            if dp.is_sniping {
                ui.label(egui::RichText::new("[SNIPER]").color(Color32::from_rgb(255, 210, 60)).small());
            }
            if dp.has_los_to_objective {
                ui.label(egui::RichText::new("LoS Clear").color(Color32::from_rgb(80, 220, 100)).small());
            } else {
                ui.label(egui::RichText::new("LoS Blocked").color(Color32::from_rgb(255, 80, 80)).small());
            }
            ui.label(format!("Elev: +{:.0} uu", dp.elevation_advantage_uu));
        });
    }
}
