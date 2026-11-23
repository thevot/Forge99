// crates/ut99-editor/src/gui/assault_dialog.rs
//
// Interactive Assault Objective, Blast Door & Objective Chain Studio
// Sequential objective chains, blast door parameters, attacker forward spawn advance shifts,
// cumulative mission time telemetry, defense difficulty rating, and procedural reinforced blast door brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{AssaultArchetype, AssaultObjectiveType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_assault_dialog {
        return;
    }

    let mut open = state.show_assault_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Assault Objective, Blast Door & Objective Chain Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_assault_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("AS Presets:").strong());
        let archetypes = [
            AssaultArchetype::OverlordBeachLanding,
            AssaultArchetype::HiSpeedTrainInfiltration,
            AssaultArchetype::FrigateNavalAssault,
            AssaultArchetype::RookCastleSiege,
            AssaultArchetype::MazonPowerExtraction,
        ];
        for arch in archetypes {
            let is_sel = state.assault_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_assault_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.assault_settings.show_door_viewport, "Blast Door Frame Wireframe");
        ui.checkbox(&mut state.assault_settings.show_objective_beacon_viewport, "Objective Beacon Bounds");
        ui.checkbox(&mut state.assault_settings.show_chain_links_viewport, "Chain Progression & Forward Spawns");
        ui.separator();
        if ui.button("🔄 Recalculate Assault Chain Telemetry & Audit").clicked() {
            state.recalculate_assault_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Assault Chain Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Mission Progression Dashboard");
            render_assault_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Objective to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Objective & Blast Door Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Assault Chain & Progression Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_assault_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.assault_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(26, 22, 22));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(90, 45, 45)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("CHAIN PROGRESSION: {} OBJECTIVES | DEFENSE DIFFICULTY: {:.0}/100", metrics.total_objectives_in_chain, metrics.defense_difficulty_index),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(255, 120, 80),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let next_str = if metrics.distance_to_next_objective_uu < 9000.0 { format!("{:.0} uu", metrics.distance_to_next_objective_uu) } else { "End of Mission".to_string() };

    let stats = [
        format!("Distance to Next Objective: {}", next_str),
        format!("Attacker Forward Shift: {:.0} uu", state.assault_settings.active_config.forward_spawn_advance_dist_uu),
        format!("Sprint Advance Pacing: {:.1}s", metrics.attacker_sprint_advance_time_secs),
        format!("Total Chain Time Bonus: +{:.0}s", metrics.cumulative_time_bonus_secs),
        format!("Target Health: {:.0} HP", state.assault_settings.active_config.target_health),
        format!("Time Reward: +{:.0}s", state.assault_settings.active_config.time_limit_bonus_secs),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(240, 220, 220));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.assault_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Objective Location:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Objective to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.assault_settings.active_config = cfg.clone();
        state.recalculate_assault_preview();
        state.history.push(crate::history::Action::UpdateAssaultConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Assault Objective Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.assault_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Objective #{}: {}", cfg.objective_order, cfg.objective_name));
        ui.label(format!("Type: {}", cfg.objective_type.display_name()));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Blast Door & Objective Actor to World").strong()).clicked() {
            state.commit_assault_objective_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.assault_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("assault_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Objective Sequence Order:");
            if ui.add(egui::Slider::new(&mut cfg.objective_order, 1..=8).prefix("#")).changed() { changed = true; }
            ui.end_row();

            ui.label("Objective Name:");
            if ui.text_edit_singleline(&mut cfg.objective_name).changed() { changed = true; }
            ui.end_row();

            ui.label("Objective Type:");
            egui::ComboBox::from_id_salt("assault_obj_type_selector")
                .selected_text(cfg.objective_type.display_name())
                .show_ui(ui, |ui| {
                    let types = [
                        AssaultObjectiveType::DestroyableTarget,
                        AssaultObjectiveType::TriggerSwitch,
                        AssaultObjectiveType::TouchZone,
                    ];
                    for t in types {
                        if ui.selectable_value(&mut cfg.objective_type, t, t.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            if cfg.objective_type == AssaultObjectiveType::DestroyableTarget {
                ui.label("Target Health (HP):");
                if ui.add(egui::Slider::new(&mut cfg.target_health, 50.0..=2000.0).suffix(" HP")).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Time Limit Bonus (secs):");
            if ui.add(egui::Slider::new(&mut cfg.time_limit_bonus_secs, 30.0..=300.0).suffix("s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Forward Spawn Shift (uu):");
            if ui.add(egui::Slider::new(&mut cfg.forward_spawn_advance_dist_uu, 0.0..=1500.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Blast Door Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_width, 64.0..=384.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Blast Door Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_height, 64.0..=256.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Blast Door Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_thickness, 8.0..=48.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Objective Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();

            ui.label("Event On Complete:");
            if ui.text_edit_singleline(&mut cfg.event_on_complete).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.assault_settings.active_config = cfg.clone();
        state.recalculate_assault_preview();
        state.history.push(crate::history::Action::UpdateAssaultConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Assault Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.assault_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Assault Progression Valid & Unbroken").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Sequential order, positive target health, and completion events are correctly configured.");
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
