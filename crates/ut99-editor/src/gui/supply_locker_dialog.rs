// crates/ut99-editor/src/gui/supply_locker_dialog.rs
//
// Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
// Recessed armory locker geometry, weapon & ammo multi-pickup synchronization,
// replenishment throughput rates, supply starvation diagnostics, and procedural cabinet brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::SupplyLockerArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_supply_locker_dialog {
        return;
    }

    let mut open = state.show_supply_locker_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Liandri Supply Locker & Armory Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_supply_locker_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Locker Presets:").strong());
        let archetypes = [
            SupplyLockerArchetype::LiandriStandardHeavyArmory,
            SupplyLockerArchetype::ShockCoreEnergyDepot,
            SupplyLockerArchetype::BioHazardToxicCache,
            SupplyLockerArchetype::SniperOverwatchResupply,
            SupplyLockerArchetype::EmergencyFieldHospitalLocker,
        ];
        for arch in archetypes {
            let is_sel = state.supply_locker_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_supply_locker_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.supply_locker_settings.show_locker_mesh_viewport, "Cabinet Recessed Housing Wireframe");
        ui.checkbox(&mut state.supply_locker_settings.show_contained_items_viewport, "Contained Pickup Icons & Stacks");
        ui.checkbox(&mut state.supply_locker_settings.show_shelf_clearance_viewport, "Shelf Clearances & Front Facing Vector");
        ui.separator();
        if ui.button("🔄 Recalculate Armory & Audit").clicked() {
            state.recalculate_supply_locker_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Firepower & Replenishment Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Cabinet Brush & Weapon Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Cabinet Architecture & Inventory Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Supply Starvation & Clustering Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.supply_locker_settings.cached_metrics;
    let cfg = &state.supply_locker_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(26, 26, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(180, 160, 60)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SUPPLY LOCKER: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(240, 220, 140),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Firepower Score: {:.0} pts", metrics.total_contained_firepower_rating),
        format!("Usable Shelf Area: {:.0} uu²", metrics.shelf_usable_area_sq_uu),
        format!("Restock Rate: {:.1} items/min", metrics.replenishment_throughput_per_min),
        format!("Respawn Interval: {:.1}s", cfg.respawn_interval_secs),
        format!("Contained Weapon: '{}'", if cfg.primary_weapon_class.is_empty() { "NONE" } else { &cfg.primary_weapon_class }),
        format!("Contained Ammo: {}x '{}'", cfg.ammo_pack_count, cfg.primary_ammo_class),
        format!("Locker Tag: '{}'", cfg.tag),
        format!("Total Lockers/Weapons in Level: {}", metrics.total_supply_lockers_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("NONE") || stat.contains("0 pts") {
            Color32::from_rgb(255, 140, 100)
        } else {
            Color32::from_rgb(245, 245, 230)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.supply_locker_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Cabinet Base Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("➡ Cabinet Facing Direction:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.x).prefix("X: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.y).prefix("Y: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.z).prefix("Z: ").speed(0.05)).changed() { changed = true; }
        });

        if ui.button("📍 Snap Cabinet Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.supply_locker_settings.active_config = cfg.clone();
        state.recalculate_supply_locker_preview();
        state.history.push(crate::history::Action::UpdateSupplyLockerConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Supply Locker Position and Facing".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.supply_locker_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Size: {:.0}x{:.0}x{:.0} uu ({} Shelves)", cfg.locker_width_uu, cfg.locker_depth_uu, cfg.locker_height_uu, cfg.shelf_count));
        ui.label(format!("Weapon: '{}' | Ammo: {}x '{}'", cfg.primary_weapon_class, cfg.ammo_pack_count, cfg.primary_ammo_class));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Cabinet Brush & Weapon Actor to Level").strong()).clicked() {
            state.commit_supply_locker_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.supply_locker_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("supply_locker_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Cabinet Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.locker_width_uu, 48.0..=256.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Cabinet Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.locker_height_uu, 64.0..=256.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Cabinet Depth (uu):");
            if ui.add(egui::Slider::new(&mut cfg.locker_depth_uu, 16.0..=96.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Shelf Count:");
            if ui.add(egui::Slider::new(&mut cfg.shelf_count, 1..=6)).changed() { changed = true; }
            ui.end_row();

            ui.label("Primary Weapon Class:");
            if ui.text_edit_singleline(&mut cfg.primary_weapon_class).changed() { changed = true; }
            ui.end_row();

            ui.label("Primary Ammo Class:");
            if ui.text_edit_singleline(&mut cfg.primary_ammo_class).changed() { changed = true; }
            ui.end_row();

            ui.label("Ammo Pack Quantity:");
            if ui.add(egui::Slider::new(&mut cfg.ammo_pack_count, 0..=8)).changed() { changed = true; }
            ui.end_row();

            ui.label("Respawn Interval (s):");
            if ui.add(egui::Slider::new(&mut cfg.respawn_interval_secs, 5.0..=120.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Locker Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.supply_locker_settings.active_config = cfg.clone();
        state.recalculate_supply_locker_preview();
        state.history.push(crate::history::Action::UpdateSupplyLockerConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Supply Locker Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.supply_locker_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Supply Locker Balanced & Verified").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Valid weapon and ammo contents, adequate shelf depth clearance, and balanced weapon neighborhood dispersion.");
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
