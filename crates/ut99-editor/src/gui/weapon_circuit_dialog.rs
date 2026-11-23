// crates/ut99-editor/src/gui/weapon_circuit_dialog.rs
//
// Interactive Tournament Weapon Placement & Ammo Circuit Studio
// Weapon DPS, burst damage and range modeling, weapon circuit sprint time matrix,
// ammo box pairing proximity scoring, and procedural weapon altar/pedestal generation.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{TournamentWeaponType, WeaponCircuitArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_weapon_circuit_dialog {
        return;
    }

    let mut open = state.show_weapon_circuit_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Tournament Weapon Placement & Ammo Circuit Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_weapon_circuit_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Competitive Circuits:").strong());
        let archetypes = [
            WeaponCircuitArchetype::Deck16DuelMasterCircuit,
            WeaponCircuitArchetype::CampgroundsHeavyTDM,
            WeaponCircuitArchetype::FaceLongRangeAssault,
            WeaponCircuitArchetype::GothicCloseQuartersCarnage,
            WeaponCircuitArchetype::MinimalistDuel1v1,
        ];
        for arch in archetypes {
            let is_sel = state.weapon_circuit_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_weapon_circuit_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.weapon_circuit_settings.show_circuit_links_viewport, "Circuit Sprint Links");
        ui.checkbox(&mut state.weapon_circuit_settings.show_altar_viewport, "Altar Brush Wireframe");
        ui.checkbox(&mut state.weapon_circuit_settings.show_ammo_tethers_viewport, "Ammo Tethers");
        ui.separator();
        if ui.button("🔄 Recalculate Circuit Telemetry & Audit").clicked() {
            state.recalculate_weapon_circuit_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Circuit Sprint Dashboard & Positions
        columns[0].vertical(|ui| {
            ui.heading("Weapon Circuit Sprint & Economy Telemetry");
            render_circuit_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Station to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Weapon & Ammunition Configuration");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Circuit Balance & Starvation Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_circuit_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.weapon_circuit_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(55, 65, 75)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("CIRCUIT FLOW: {} WEAPONS | {} AMMO BOXES", metrics.total_weapons_in_level, metrics.total_ammo_boxes_in_level),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(80, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Avg Sprint Between Weapons: {:.1} s", metrics.avg_sprint_time_between_weapons_secs),
        format!("Ammo Pairing Coverage: {:.0}%", metrics.ammo_pairing_coverage_pct),
        format!("Hitscan / Proj Ratio: {:.2}", metrics.hitscan_vs_projectile_ratio),
        format!("Map Burst Damage: {:.0} HP", metrics.total_max_burst_damage_potential),
        format!("Total Circuit Links: {}", metrics.circuit_links.len()),
        format!("Weapon Stay Mode: {}", if state.weapon_circuit_settings.active_config.b_weapon_stay { "Enabled" } else { "Disabled" }),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(190, 210, 230));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.weapon_circuit_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Weapon Altar Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Altar to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.weapon_circuit_settings.active_config = cfg.clone();
        state.recalculate_weapon_circuit_preview();
        state.history.push(crate::history::Action::UpdateWeaponCircuitConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Weapon Altar Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.weapon_circuit_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Primary Weapon: {}", cfg.primary_weapon.display_name()));
        ui.label(format!("Flanking Ammo: {} boxes", cfg.flanking_ammo_count));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Weapon Station & Ammo to World").strong()).clicked() {
            state.commit_weapon_station_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.weapon_circuit_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("weapon_circuit_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Primary Weapon:");
            egui::ComboBox::from_id_salt("weapon_type_selector")
                .selected_text(cfg.primary_weapon.display_name())
                .show_ui(ui, |ui| {
                    let weapons = [
                        TournamentWeaponType::ShockRifle,
                        TournamentWeaponType::FlakCannon,
                        TournamentWeaponType::RocketLauncher,
                        TournamentWeaponType::SniperRifle,
                        TournamentWeaponType::Minigun,
                        TournamentWeaponType::BioRifle,
                        TournamentWeaponType::PulseGun,
                        TournamentWeaponType::Ripper,
                        TournamentWeaponType::Enforcer,
                        TournamentWeaponType::DualEnforcers,
                        TournamentWeaponType::Redeemer,
                    ];
                    for w in weapons {
                        if ui.selectable_value(&mut cfg.primary_weapon, w, w.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Flanking Ammo Boxes:");
            if ui.add(egui::Slider::new(&mut cfg.flanking_ammo_count, 0..=3)).changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.altar_width, 40.0..=120.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Depth (uu):");
            if ui.add(egui::Slider::new(&mut cfg.altar_depth, 30.0..=90.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.altar_height, 8.0..=32.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Respawn Delay (s):");
            if ui.add(egui::Slider::new(&mut cfg.respawn_delay_secs, 10.0..=60.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Weapon Stay (bWeaponStay):");
            if ui.checkbox(&mut cfg.b_weapon_stay, "Enabled").changed() { changed = true; }
            ui.end_row();

            ui.label("Station Identification Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.weapon_circuit_settings.active_config = cfg.clone();
        state.recalculate_weapon_circuit_preview();
        state.history.push(crate::history::Action::UpdateWeaponCircuitConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Weapon Circuit Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.weapon_circuit_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Weapon Circuit Balanced & Viable").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No weapon proximity clustering, starvation, or excessive burst hazards detected.");
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
