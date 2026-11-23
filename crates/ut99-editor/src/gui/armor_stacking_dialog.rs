// crates/ut99-editor/src/gui/armor_stacking_dialog.rs
//
// Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
// Absorption hierarchy simulation, Effective Health (EHP) calculation,
// pickup clustering hazard detection, and procedural pedestal generation.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::ArmorArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_armor_stacking_dialog {
        return;
    }

    let mut open = state.show_armor_stacking_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Armor, Health & ShieldBelt Stacking Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_armor_stacking_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Armor Archetypes:").strong());
        let archetypes = [
            ArmorArchetype::ClassicDeck16Stack,
            ArmorArchetype::HardcoreDuelStandoff,
            ArmorArchetype::TeamFortressHeavyDefense,
            ArmorArchetype::InstagibCleanSweep,
            ArmorArchetype::RelicRegenPowerSurge,
        ];
        for arch in archetypes {
            let is_sel = state.armor_stacking_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_armor_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.armor_stacking_settings.show_pedestals_viewport, "Pedestals");
        ui.checkbox(&mut state.armor_stacking_settings.show_absorption_bubbles_viewport, "Absorption Bubbles");
        ui.checkbox(&mut state.armor_stacking_settings.show_cluster_hazards_viewport, "Clustering Hazards (<400uu)");
        ui.separator();
        if ui.button("🔄 Recalculate Preview & World Pickups").clicked() {
            state.recalculate_armor_stacking_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Damage Simulator & EHP Meter
        columns[0].vertical(|ui| {
            ui.heading("Absorption Hierarchy & Damage Simulator");
            render_damage_simulator_canvas(ui, state);

            ui.add_space(8.0);
            render_probe_controls(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Armor Station to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Station Configuration & Pickup Quantities");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Armor Stacking & Clustering Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_damage_simulator_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let res = &state.armor_stacking_settings.cached_damage_result;
    let metrics = &state.armor_stacking_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Card background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 30));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 65, 80)), StrokeKind::Middle);

    // Title & Status
    let status_text = if res.player_survived {
        egui::RichText::new("PLAYER SURVIVED").color(Color32::from_rgb(60, 230, 110))
    } else {
        egui::RichText::new("FATAL OVERKILL (LETHAL)").color(Color32::from_rgb(255, 60, 60))
    };

    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("Incoming Damage: {:.0} | Status: {}", res.initial_damage, status_text.text()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(230, 240, 255),
    );

    // Visual Segmented Bar
    let bar_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 36.0),
        Vec2::new(avail_w - 28.0, 20.0),
    );
    painter.rect_filled(bar_rect, 3.0, Color32::from_rgb(32, 38, 48));

    let total_max = (state.armor_stacking_settings.simulated_damage_probe_hp
        + state.armor_stacking_settings.simulated_damage_probe_shield
        + state.armor_stacking_settings.simulated_damage_probe_body
        + state.armor_stacking_settings.simulated_damage_probe_thigh).max(1.0);

    let mut current_x = bar_rect.min.x;
    let segments = [
        (res.final_shield_belt, Color32::from_rgb(255, 210, 40), "ShieldBelt"),
        (res.final_body_armor, Color32::from_rgb(50, 120, 240), "BodyArmor"),
        (res.final_thigh_pads, Color32::from_rgb(40, 210, 100), "ThighPads"),
        (res.final_health, Color32::from_rgb(240, 60, 60), "Health"),
    ];

    for (val, color, _) in segments {
        if val > 0.0 {
            let seg_w = (bar_rect.width() * (val / total_max)).clamp(0.0, bar_rect.max.x - current_x);
            let seg_rect = Rect::from_min_size(Pos2::new(current_x, bar_rect.min.y), Vec2::new(seg_w, bar_rect.height()));
            painter.rect_filled(seg_rect, 0.0, color);
            current_x += seg_w;
        }
    }

    // Absorption Breakdown Text
    let y_start = rect.min.y + 68.0;
    let stats = [
        format!("ShieldBelt Left: {:.0}", res.final_shield_belt),
        format!("Body Armor Left: {:.0}", res.final_body_armor),
        format!("ThighPads Left: {:.0}", res.final_thigh_pads),
        format!("Health Left: {:.0}", res.final_health),
        format!("Armor Absorbed: {:.0} ({:.0}%)", res.total_armor_absorbed,
            if res.initial_damage > 0.0 { (res.total_armor_absorbed / res.initial_damage) * 100.0 } else { 0.0 }),
        format!("Health Bleed-Through: {:.0}", res.health_damage_taken),
        format!("Total Level Armor Pool: {}", metrics.total_armor_pool),
        format!("Max EHP Potential: {:.0}", metrics.max_effective_hp),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(11.5), Color32::from_rgb(180, 195, 215));
    }
}

fn render_probe_controls(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.group(|ui| {
        ui.label(egui::RichText::new("🧪 Test Weapon / Damage Preset:").strong());
        ui.horizontal(|ui| {
            let presets = [
                ("Enforcer (20)", 20.0),
                ("Shock Beam (40)", 40.0),
                ("Sniper Body (75)", 75.0),
                ("Rocket / Headshot (100)", 100.0),
                ("Flak Slug (150)", 150.0),
                ("Shock Combo (200)", 200.0),
            ];
            for (label, dmg) in presets {
                if ui.button(label).clicked() {
                    state.simulate_armor_test_damage(dmg);
                }
            }
        });

        ui.add_space(4.0);
        let mut dmg = state.armor_stacking_settings.test_incoming_damage;
        if ui.add(egui::Slider::new(&mut dmg, 0.0..=300.0).text("Custom Damage Input")).changed() {
            state.simulate_armor_test_damage(dmg);
        }

        ui.separator();
        ui.label(egui::RichText::new("Player Starting Stack:").strong());
        let mut hp = state.armor_stacking_settings.simulated_damage_probe_hp;
        let mut sb = state.armor_stacking_settings.simulated_damage_probe_shield;
        let mut ba = state.armor_stacking_settings.simulated_damage_probe_body;
        let mut tp = state.armor_stacking_settings.simulated_damage_probe_thigh;

        let mut changed = false;
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut hp).range(1.0..=199.0).prefix("HP: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut sb).range(0.0..=150.0).prefix("Shield: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut ba).range(0.0..=100.0).prefix("Body: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut tp).range(0.0..=50.0).prefix("Thigh: ")).changed() { changed = true; }
        });

        if changed {
            state.armor_stacking_settings.simulated_damage_probe_hp = hp;
            state.armor_stacking_settings.simulated_damage_probe_shield = sb;
            state.armor_stacking_settings.simulated_damage_probe_body = ba;
            state.armor_stacking_settings.simulated_damage_probe_thigh = tp;
            state.simulate_armor_test_damage(dmg);
        }
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.armor_stacking_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Station Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Pedestal Geometry: {} sides, {:.0} uu radius, {:.0} uu height",
            cfg.pedestal_sides, cfg.pedestal_radius, cfg.pedestal_height));
        ui.label(format!("ShieldBelt: {} | BodyArmor: {} | ThighPads: {} | Keg: {}",
            cfg.shield_belt_count, cfg.body_armor_count, cfg.thigh_pads_count, cfg.keg_of_health_count));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Armor Pedestal & Pickups to Level").strong()).clicked() {
            state.commit_armor_stacking_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.armor_stacking_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("armor_stacking_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("ShieldBelt Pickups:");
            if ui.add(egui::Slider::new(&mut cfg.shield_belt_count, 0..=4)).changed() { changed = true; }
            ui.end_row();

            ui.label("Body Armor Pickups:");
            if ui.add(egui::Slider::new(&mut cfg.body_armor_count, 0..=6)).changed() { changed = true; }
            ui.end_row();

            ui.label("ThighPads Pickups:");
            if ui.add(egui::Slider::new(&mut cfg.thigh_pads_count, 0..=6)).changed() { changed = true; }
            ui.end_row();

            ui.label("Keg of Health (+100 HP):");
            if ui.add(egui::Slider::new(&mut cfg.keg_of_health_count, 0..=4)).changed() { changed = true; }
            ui.end_row();

            ui.label("MedBox Pickups (+20 HP):");
            if ui.add(egui::Slider::new(&mut cfg.med_box_count, 0..=16)).changed() { changed = true; }
            ui.end_row();

            ui.label("Health Vials (+5 HP Overstack):");
            if ui.add(egui::Slider::new(&mut cfg.health_vial_count, 0..=32)).changed() { changed = true; }
            ui.end_row();

            ui.label("Respawn Time Multiplier:");
            if ui.add(egui::Slider::new(&mut cfg.respawn_multiplier, 0.2..=2.0).suffix("x")).changed() { changed = true; }
            ui.end_row();

            ui.label("Passive HP Regeneration:");
            if ui.add(egui::Slider::new(&mut cfg.regeneration_rate_hp_per_sec, 0.0..=10.0).suffix(" HP/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_radius, 32.0..=128.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_height, 8.0..=64.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Sides (Polygon):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_sides, 6..=16)).changed() { changed = true; }
            ui.end_row();

            ui.label("Tag / Identification:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.armor_stacking_settings.active_config = cfg.clone();
        state.recalculate_armor_stacking_preview();
        state.history.push(crate::history::Action::UpdateArmorStackingConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Armor Stacking Setup".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.armor_stacking_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Armor & Health Distribution 100% Balanced").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No pickup clustering, excessive ShieldBelts, or starvation detected.");
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
