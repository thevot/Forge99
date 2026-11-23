// crates/ut99-editor/src/gui/bio_trap_dialog.rs
//
// Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
// Secondary fire charged gel glob traps, surface adhesion normal reflection, toxic slime coverage,
// detonation timers, environmental hazards, and procedural drainage grate brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use glam::Vec3;
use crate::editor_state::EditorState;
use ut99_bsp::{BioDispersionMode, BioTrapArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_bio_trap_dialog {
        return;
    }

    let mut open = state.show_bio_trap_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Bio-Rifle Slime Trap & Hazard Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_bio_trap_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Bio Trap Presets:").strong());
        let archetypes = [
            BioTrapArchetype::ChokepointDoorwayAmbushTrap,
            BioTrapArchetype::FlagPedestalSurroundMinefield,
            BioTrapArchetype::ElevatorShaftToxicWeb,
            BioTrapArchetype::CorridorAreaDenialVolley,
            BioTrapArchetype::ReactorVatSpillZone,
        ];
        for arch in archetypes {
            let is_sel = state.bio_trap_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_bio_trap_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.bio_trap_settings.show_grate_viewport, "Chemical Drain Grate");
        ui.checkbox(&mut state.bio_trap_settings.show_splash_radius_viewport, "Toxic Splash Footprint");
        ui.checkbox(&mut state.bio_trap_settings.show_adhesion_normal_viewport, "Adhesion Normal Ray");
        ui.separator();
        if ui.button("🔄 Recalculate Bio Telemetry & Audit").clicked() {
            state.recalculate_bio_trap_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Slime Hazard & Lethality Telemetry");
            render_bio_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Bio-Trap to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Dispersion & Detonation Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Toxic Balance & Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_bio_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.bio_trap_settings.cached_metrics;
    let cfg = &state.bio_trap_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 28, 16));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 120, 45)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("TRAP: {} | {}", cfg.archetype.display_name(), cfg.dispersion_mode.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(90, 240, 90),
    );

    // Ballistics Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Total Burst Damage: {:.0} HP", metrics.total_burst_damage_hp),
        format!("Continuous Corrosive DPS: {:.1} HP/s", cfg.continuous_dps),
        format!("Hazard Area: {:.1} k uu²", metrics.hazard_footprint_area_sq_uu / 1_000.0),
        format!("Player TTK (100 HP): {:.2}s", metrics.time_to_lethal_kill_secs),
        format!("Charged Cartridges: {}/10", cfg.glob_charge_count),
        format!("Detonation Timeout: {:.1}s", cfg.detonation_timeout_secs),
        format!("Surface Adhesion: ({:.1}, {:.1}, {:.1})", cfg.surface_normal.x, cfg.surface_normal.y, cfg.surface_normal.z),
        format!("Total Traps in Level: {}", metrics.total_traps_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(220, 250, 220));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.bio_trap_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Trap Placement & Host Surface:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            ui.label("Normal:");
            if ui.button("Floor (Up)").clicked() { cfg.surface_normal = Vec3::new(0.0, 0.0, 1.0); changed = true; }
            if ui.button("Ceiling (Down)").clicked() { cfg.surface_normal = Vec3::new(0.0, 0.0, -1.0); changed = true; }
            if ui.button("Wall +X").clicked() { cfg.surface_normal = Vec3::new(1.0, 0.0, 0.0); changed = true; }
            if ui.button("Wall +Y").clicked() { cfg.surface_normal = Vec3::new(0.0, 1.0, 0.0); changed = true; }
        });

        if ui.button("📍 Snap Trap to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.bio_trap_settings.active_config = cfg.clone();
        state.recalculate_bio_trap_preview();
        state.history.push(crate::history::Action::UpdateBioTrapConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Bio-Trap Coordinates and Normal".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.bio_trap_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Dispersion: {}", cfg.dispersion_mode.display_name()));
        ui.label(format!("Splash Radius: {:.0} uu", cfg.splash_radius_uu));
        ui.label(format!("Burst Damage: {:.0} HP", cfg.burst_damage_hp));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Drain Grate & BioGel Actor to World").strong()).clicked() {
            state.commit_bio_trap_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.bio_trap_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("bio_trap_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Dispersion Mode:");
            egui::ComboBox::from_id_salt("bio_dispersion_selector")
                .selected_text(cfg.dispersion_mode.display_name())
                .show_ui(ui, |ui| {
                    let modes = [
                        BioDispersionMode::ConcentratedGlob,
                        BioDispersionMode::DispersedMinefield,
                        BioDispersionMode::WallAdhesiveWeb,
                        BioDispersionMode::CorrosivePool,
                    ];
                    for m in modes {
                        if ui.selectable_value(&mut cfg.dispersion_mode, m, m.display_name()).clicked() {
                            cfg.splash_radius_uu = m.default_splash_radius();
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Glob Charge Count (1-10):");
            if ui.add(egui::Slider::new(&mut cfg.glob_charge_count, 1..=10)).changed() { changed = true; }
            ui.end_row();

            ui.label("Splash Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.splash_radius_uu, 60.0..=500.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Detonation Timeout (s):");
            if ui.add(egui::Slider::new(&mut cfg.detonation_timeout_secs, 2.0..=40.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Burst Damage (HP):");
            if ui.add(egui::Slider::new(&mut cfg.burst_damage_hp, 40.0..=300.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Continuous DPS (HP/s):");
            if ui.add(egui::Slider::new(&mut cfg.continuous_dps, 0.0..=60.0).suffix(" HP/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Drain Grate Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.grate_radius, 24.0..=128.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Drain Grate Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.grate_height, 8.0..=32.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Trap Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.bio_trap_settings.active_config = cfg.clone();
        state.recalculate_bio_trap_preview();
        state.history.push(crate::history::Action::UpdateBioTrapConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Bio-Trap Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.bio_trap_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Bio-Trap Balance & Sightlines Safe").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No spawn point overlap detected, fair dodge clearance, and sufficient detonation fuse.");
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
