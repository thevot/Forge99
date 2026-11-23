// crates/ut99-editor/src/gui/hazard_barrel_dialog.rs
//
// Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
// Explosive chaining radius, toxic leakage fields, damage accumulation thresholds,
// impulse momentum transfer, and procedural faceted barrel brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::BarrelHazardType;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_hazard_barrel_dialog {
        return;
    }

    let mut open = state.show_hazard_barrel_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Explosive Barrel, Toxic Drum & Hazard Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_hazard_barrel_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Barrel Presets:").strong());
        let archetypes = [
            BarrelHazardType::VolatileNapalmFuelDrum,
            BarrelHazardType::CorrosiveSludgeAcidCanister,
            BarrelHazardType::ReinforcedIndustrialKeg,
            BarrelHazardType::PyrotechnicGasCylinder,
            BarrelHazardType::DepletedUraniumWasteCask,
        ];
        for arch in archetypes {
            let is_sel = state.hazard_barrel_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_hazard_barrel_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.hazard_barrel_settings.show_barrel_mesh_viewport, "Barrel Mesh Wireframe");
        ui.checkbox(&mut state.hazard_barrel_settings.show_blast_radius_viewport, "Blast Radius Sphere");
        ui.checkbox(&mut state.hazard_barrel_settings.show_chain_links_viewport, "Chain-Detonation Links");
        ui.checkbox(&mut state.hazard_barrel_settings.show_toxic_spill_viewport, "Toxic Spill Footprint");
        ui.separator();
        if ui.button("🔄 Recalculate Blast Physics & Audit").clicked() {
            state.recalculate_hazard_barrel_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Pyrotechnic & Blast Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Barrel & Hazard Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Explosive & Fluid Spill Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Safety & Cascade Hazard Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.hazard_barrel_settings.cached_metrics;
    let cfg = &state.hazard_barrel_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(32, 22, 20));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(220, 110, 50)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("HAZARD DRUM: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(255, 175, 110),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Max Blast Damage: {:.0} HP", cfg.blast_damage),
        format!("Lethal Kill Radius: {:.0} uu", metrics.lethal_kill_radius_uu),
        format!("Blast Radius: {:.0} uu", cfg.blast_radius_uu),
        format!("Chain Reaction Barrels: {}", metrics.chain_reaction_count),
        format!("Impulse Ejection Velocity: {:.0} uu/s", metrics.impulse_ejection_velocity),
        format!("Detonation Health Threshold: {:.0} HP", cfg.health),
        format!("Toxic Hazard: {}", if cfg.is_toxic_spill { format!("{:.0} DPS ({:.1}s)", cfg.toxic_dps, cfg.toxic_duration_secs) } else { "None".to_string() }),
        format!("Total Barrels in Map: {}", metrics.total_barrels_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("Lethal") || stat.contains("Max Blast") {
            Color32::from_rgb(255, 140, 90)
        } else {
            Color32::from_rgb(240, 230, 220)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.hazard_barrel_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Barrel Placement Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.barrel_radius_uu).prefix("Radius: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.barrel_height_uu).prefix("Height: ").suffix(" uu")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Barrel Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.hazard_barrel_settings.active_config = cfg.clone();
        state.recalculate_hazard_barrel_preview();
        state.history.push(crate::history::Action::UpdateHazardBarrelConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Hazard Barrel Placement and Dimensions".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.hazard_barrel_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Blast: {:.0} HP in {:.0} uu", cfg.blast_damage, cfg.blast_radius_uu));
        ui.label(format!("Tag: '{}' | Event: '{}'", cfg.tag, cfg.event_on_explode));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Barrel Brush & Actor to Level").strong()).clicked() {
            state.commit_hazard_barrel_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.hazard_barrel_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("hazard_barrel_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Trigger Health (HP):");
            if ui.add(egui::Slider::new(&mut cfg.health, 5.0..=200.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Blast Damage (HP):");
            if ui.add(egui::Slider::new(&mut cfg.blast_damage, 20.0..=400.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Blast Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.blast_radius_uu, 100.0..=800.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Momentum Transfer:");
            if ui.add(egui::Slider::new(&mut cfg.momentum_transfer, 100.0..=2000.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Chain Detonation Delay:");
            if ui.add(egui::Slider::new(&mut cfg.chain_detonation_delay_secs, 0.05..=1.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Toxic Spill Field:");
            if ui.checkbox(&mut cfg.is_toxic_spill, "Enable Lingering Spill").changed() { changed = true; }
            ui.end_row();

            if cfg.is_toxic_spill {
                ui.label("Toxic DPS:");
                if ui.add(egui::Slider::new(&mut cfg.toxic_dps, 1.0..=50.0).suffix(" HP/s")).changed() { changed = true; }
                ui.end_row();

                ui.label("Spill Duration:");
                if ui.add(egui::Slider::new(&mut cfg.toxic_duration_secs, 2.0..=20.0).suffix(" s")).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Actor Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();

            ui.label("Explosion Event:");
            if ui.text_edit_singleline(&mut cfg.event_on_explode).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.hazard_barrel_settings.active_config = cfg.clone();
        state.recalculate_hazard_barrel_preview();
        state.history.push(crate::history::Action::UpdateHazardBarrelConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Hazard Barrel Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.hazard_barrel_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Barrel Placement Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Safe clearance from PlayerStarts, controlled blast radius, and no runaway cascades detected.");
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
