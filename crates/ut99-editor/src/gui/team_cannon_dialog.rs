// crates/ut99-editor/src/gui/team_cannon_dialog.rs
//
// Interactive Team Cannon & Stationary Automated Turret Defense Studio
// Automated base cannons, tracking speeds, firing rates, damage per shot, TTK analysis,
// spawn camping hazard auditing, and procedural reinforced bunker mount brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{CannonWeaponType, TeamCannonArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_team_cannon_dialog {
        return;
    }

    let mut open = state.show_team_cannon_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Team Cannon & Automated Turret Defense Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_team_cannon_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Cannon Presets:").strong());
        let archetypes = [
            TeamCannonArchetype::OverlordBeachBunkerCannon,
            TeamCannonArchetype::HiSpeedRoofMinigunCannon,
            TeamCannonArchetype::FaceSniperTowerGatling,
            TeamCannonArchetype::FrigateDeckMountedFlak,
            TeamCannonArchetype::SubterraneanCeilingTurret,
        ];
        for arch in archetypes {
            let is_sel = state.team_cannon_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_team_cannon_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.team_cannon_settings.show_bunker_viewport, "Bunker Mount Wireframe");
        ui.checkbox(&mut state.team_cannon_settings.show_range_sphere_viewport, "Targeting Range Dome");
        ui.checkbox(&mut state.team_cannon_settings.show_targeting_line_viewport, "Aim Vector & Pitch Ray");
        ui.separator();
        if ui.button("🔄 Recalculate Ballistics & Audit").clicked() {
            state.recalculate_team_cannon_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Cannon Ballistics & Lethality Telemetry");
            render_cannon_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Cannon to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Weapon Ballistics & Tracking Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Combat Balance & Anti-Camp Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_cannon_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.team_cannon_settings.cached_metrics;
    let cfg = &state.team_cannon_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(26, 22, 20));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(110, 60, 45)), StrokeKind::Middle);

    // Title banner
    let team_col = match cfg.team {
        0 => Color32::from_rgb(255, 90, 90),
        1 => Color32::from_rgb(90, 160, 255),
        2 => Color32::from_rgb(90, 230, 110),
        _ => Color32::from_rgb(255, 210, 50),
    };

    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("CANNON: {} | {}", cfg.archetype.display_name(), cfg.cannon_type.display_name()),
        egui::FontId::proportional(13.0),
        team_col,
    );

    // Ballistics Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Lethal DPS: {:.1} HP/s", metrics.dps),
        format!("Player TTK (100 HP): {:.2}s", metrics.time_to_kill_standard_player_secs),
        format!("Target Acquisition Slew: {:.2}s (180°)", metrics.target_acquisition_latency_secs),
        format!("Max Effective Range: {:.0} uu", cfg.max_range_uu),
        format!("Tracking Slew Speed: {:.0}°/s", cfg.tracking_speed_deg_per_sec),
        format!("Defensive Volume: {:.1} M uu³", metrics.effective_defense_volume_cu_uu / 1_000_000.0),
        format!("Mount Orientation: {}", if cfg.is_inverted { "Inverted Ceiling" } else { "Upright Ground" }),
        format!("Total Cannons in Level: {}", metrics.total_cannons_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(245, 230, 220));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.team_cannon_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Cannon Coordinates & Aim:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.yaw_degrees).prefix("Yaw: ").suffix("°")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.pitch_degrees).prefix("Pitch: ").suffix("°")).changed() { changed = true; }
            if ui.checkbox(&mut cfg.is_inverted, "Inverted Ceiling Mount").changed() { changed = true; }
        });

        if ui.button("📍 Snap Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.team_cannon_settings.active_config = cfg.clone();
        state.recalculate_team_cannon_preview();
        state.history.push(crate::history::Action::UpdateTeamCannonConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Cannon Coordinates and Angles".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.team_cannon_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Weapon Type: {}", cfg.cannon_type.display_name()));
        ui.label(format!("Team: {}", cfg.team));
        ui.label(format!("Effective Range: {:.0} uu", cfg.max_range_uu));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Bunker Mount & TeamCannon Actor to World").strong()).clicked() {
            state.commit_team_cannon_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.team_cannon_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("team_cannon_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Weapon System:");
            egui::ComboBox::from_id_salt("cannon_weapon_selector")
                .selected_text(cfg.cannon_type.display_name())
                .show_ui(ui, |ui| {
                    let wtypes = [
                        CannonWeaponType::MinigunGatling,
                        CannonWeaponType::PulseLaserTurret,
                        CannonWeaponType::RocketFlakPod,
                        CannonWeaponType::SniperRailBattery,
                    ];
                    for w in wtypes {
                        if ui.selectable_value(&mut cfg.cannon_type, w, w.display_name()).clicked() {
                            cfg.fire_rate_rpm = w.default_rpm();
                            cfg.damage_per_shot = w.default_damage();
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Team ID (0=Red, 1=Blue, 2=Green, 3=Gold):");
            if ui.add(egui::DragValue::new(&mut cfg.team).range(0..=3)).changed() { changed = true; }
            ui.end_row();

            ui.label("Fire Rate (RPM):");
            if ui.add(egui::Slider::new(&mut cfg.fire_rate_rpm, 20.0..=1200.0).suffix(" RPM")).changed() { changed = true; }
            ui.end_row();

            ui.label("Damage Per Shot:");
            if ui.add(egui::Slider::new(&mut cfg.damage_per_shot, 5.0..=150.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Tracking Speed (°/s):");
            if ui.add(egui::Slider::new(&mut cfg.tracking_speed_deg_per_sec, 20.0..=240.0).suffix(" °/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Max Range (uu):");
            if ui.add(egui::Slider::new(&mut cfg.max_range_uu, 800.0..=6000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Mount Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.bunker_mount_radius, 32.0..=128.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Mount Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.bunker_mount_height, 12.0..=64.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Cannon Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.team_cannon_settings.active_config = cfg.clone();
        state.recalculate_team_cannon_preview();
        state.history.push(crate::history::Action::UpdateTeamCannonConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Cannon Ballistics Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.team_cannon_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Cannon Balance & Sightlines Safe").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No spawn camping line-of-sight detected, fair TTK reaction time, and responsive tracking.");
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
