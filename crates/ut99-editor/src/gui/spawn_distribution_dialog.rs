// crates/ut99-editor/src/gui/spawn_distribution_dialog.rs
//
// Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
// Team distribution, collision hull clearances, telefrag prevention, spawn camping exposure
// analytics, unarmed sprint time calculations, and procedural directional spawn pad generation.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{SpawnDistributionArchetype, TournamentTeamId};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_spawn_distribution_dialog {
        return;
    }

    let mut open = state.show_spawn_distribution_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Tournament Spawn Point Balancing & Telefrag Prevention Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_spawn_distribution_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Spawn Presets:").strong());
        let archetypes = [
            SpawnDistributionArchetype::Deck16DuelSpawnRing,
            SpawnDistributionArchetype::FaceSymmetricCTFSpawns,
            SpawnDistributionArchetype::CampgroundsTDMGrid,
            SpawnDistributionArchetype::PressureTrapPerimeter,
            SpawnDistributionArchetype::NoviceLinearSpawnHub,
        ];
        for arch in archetypes {
            let is_sel = state.spawn_distribution_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_spawn_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.spawn_distribution_settings.show_pad_viewport, "Spawn Pad Wireframe");
        ui.checkbox(&mut state.spawn_distribution_settings.show_player_hull_viewport, "Player Collision Hull (R=17, H=78)");
        ui.checkbox(&mut state.spawn_distribution_settings.show_direction_vector_viewport, "Spawn Direction Vector");
        ui.separator();
        if ui.button("🔄 Recalculate Spawn Telemetry & Audit").clicked() {
            state.recalculate_spawn_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Spawn Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Spawn Equity & Telefrag Dashboard");
            render_spawn_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Spawn Pad to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("PlayerStart Configuration");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Telefrag & Camping Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_spawn_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.spawn_distribution_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 65, 95)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SPAWN TELEMETRY: {} TOTAL STARTS | NEAREST WEAPON: {:.0} uu", metrics.total_player_starts_in_level, metrics.nearest_weapon_distance_uu),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(100, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Same-Team Spawns: {}", metrics.same_team_spawns),
        format!("Opposing Team Spawns: {}", metrics.opposing_team_spawns),
        format!("Nearest Spawn Distance: {:.0} uu", metrics.nearest_spawn_distance_uu),
        format!("Unarmed Sprint Delay: {:.1}s", metrics.unarmed_sprint_time_secs),
        format!("Spawn Camping Exposure: {:.0} / 100", metrics.spawn_camping_exposure_index),
        format!("Spawn Facing Yaw: {:.0}°", state.spawn_distribution_settings.active_config.yaw_degrees),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(200, 220, 245));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.spawn_distribution_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 PlayerStart Location:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Pad to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.spawn_distribution_settings.active_config = cfg.clone();
        state.recalculate_spawn_preview();
        state.history.push(crate::history::Action::UpdateSpawnDistributionConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated PlayerStart Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.spawn_distribution_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Team: {}", cfg.team_number.display_name()));
        ui.label(format!("Facing: {:.0}°", cfg.yaw_degrees));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Spawn Pad & PlayerStart to World").strong()).clicked() {
            state.commit_spawn_pad_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.spawn_distribution_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("spawn_distribution_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Team Assignment:");
            egui::ComboBox::from_id_salt("team_number_selector")
                .selected_text(cfg.team_number.display_name())
                .show_ui(ui, |ui| {
                    let teams = [
                        TournamentTeamId::FFA,
                        TournamentTeamId::Red,
                        TournamentTeamId::Blue,
                        TournamentTeamId::Green,
                        TournamentTeamId::Gold,
                    ];
                    for t in teams {
                        if ui.selectable_value(&mut cfg.team_number, t, t.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Facing Yaw (°):");
            if ui.add(egui::Slider::new(&mut cfg.yaw_degrees, 0.0..=360.0).suffix("°")).changed() { changed = true; }
            ui.end_row();

            ui.label("Pad Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pad_radius, 24.0..=64.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pad Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pad_height, 4.0..=16.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Tournament Flags:");
            ui.horizontal(|ui| {
                if ui.checkbox(&mut cfg.b_single_player_start, "SinglePlayer").changed() { changed = true; }
                if ui.checkbox(&mut cfg.b_coop_start, "Coop").changed() { changed = true; }
                if ui.checkbox(&mut cfg.b_enabled, "Enabled").changed() { changed = true; }
            });
            ui.end_row();

            ui.label("PlayerStart Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.spawn_distribution_settings.active_config = cfg.clone();
        state.recalculate_spawn_preview();
        state.history.push(crate::history::Action::UpdateSpawnDistributionConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Spawn Distribution Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.spawn_distribution_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Spawn Point Safe & Balanced").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Clearance > 120 uu avoids telefrags, proximity to weapons ensures fair defense against campers.");
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
