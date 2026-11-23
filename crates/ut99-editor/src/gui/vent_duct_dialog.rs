// crates/ut99-editor/src/gui/vent_duct_dialog.rs
//
// Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
// Hollow ventilation conduits, crouch crawlspace clearances, aerodynamic air currents,
// breakable infiltration grates, and procedural duct brush geometry.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::VentDuctArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_vent_duct_dialog {
        return;
    }

    let mut open = state.show_vent_duct_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Ventilation Duct & Crawlspace Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_vent_duct_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Vent Presets:").strong());
        let archetypes = [
            VentDuctArchetype::AssaultOverlordAirVentilationBypass,
            VentDuctArchetype::SubterraneanMaintenanceDrainCrawlway,
            VentDuctArchetype::HighTechTurbineCoolingChute,
            VentDuctArchetype::CastleDungeonOublietteEscapeShaft,
            VentDuctArchetype::IndustrialCleanroomExhaustTrunk,
        ];
        for arch in archetypes {
            let is_sel = state.vent_duct_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_vent_duct_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.vent_duct_settings.show_duct_mesh_viewport, "Hollow Conduit Casing Wireframe");
        ui.checkbox(&mut state.vent_duct_settings.show_air_current_viewport, "Aerodynamic Air Current Streamlines");
        ui.checkbox(&mut state.vent_duct_settings.show_crouch_clearance_viewport, "Player Crouch Clearance & Grates");
        ui.separator();
        if ui.button("🔄 Recalculate Vent & Audit").clicked() {
            state.recalculate_vent_duct_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Clearance & Aerodynamic Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Duct Brush & Grate Mover to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Conduit Geometry & Grate Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Crouch Entrapment & Airflow Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.vent_duct_settings.cached_metrics;
    let cfg = &state.vent_duct_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 28, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 200, 140)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("VENTILATION DUCT: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(180, 245, 210),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Cross-Section: {:.0} uu²", metrics.internal_clearance_cross_section_sq_uu),
        format!("Crouch Transit Time: {:.2}s", metrics.player_crouch_transit_time_secs),
        format!("Convection Drag: {:.0} uu/s²", metrics.aerodynamic_convection_force),
        format!("Air Current Velocity: {:.0} uu/s", cfg.air_draft_velocity),
        format!("Intake Grate: {} | Exhaust: {}", if cfg.has_intake_grate { "YES" } else { "NONE" }, if cfg.has_exhaust_grate { "YES" } else { "NONE" }),
        format!("Grate Breakable: {}", if cfg.grate_breakable { "YES (MOVER)" } else { "LOCKED (TRIGGER)" }),
        format!("Tag: '{}' | Event: '{}'", cfg.tag, if cfg.event.is_empty() { "NONE" } else { &cfg.event }),
        format!("Total Vent Ducts in Level: {}", metrics.total_vent_ducts_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("NONE") || stat.contains("LOCKED") {
            Color32::from_rgb(255, 180, 100)
        } else {
            Color32::from_rgb(235, 255, 245)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.vent_duct_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Duct Intake Origin Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("➡ Conduit Axis Direction:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.x).prefix("X: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.y).prefix("Y: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.z).prefix("Z: ").speed(0.05)).changed() { changed = true; }
        });

        if ui.button("📍 Snap Duct Origin to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.vent_duct_settings.active_config = cfg.clone();
        state.recalculate_vent_duct_preview();
        state.history.push(crate::history::Action::UpdateVentDuctConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Vent Duct Position and Axis".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.vent_duct_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Size: {:.0}x{:.0} uu, Length: {:.0} uu", cfg.duct_width_uu, cfg.duct_height_uu, cfg.duct_length_uu));
        ui.label(format!("Air Current: {:.0} uu/s | Grate Breakable: {}", cfg.air_draft_velocity, cfg.grate_breakable));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Vent Duct Brush & Grate Mover to Level").strong()).clicked() {
            state.commit_vent_duct_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.vent_duct_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("vent_duct_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Duct Length (uu):");
            if ui.add(egui::Slider::new(&mut cfg.duct_length_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Outer Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.duct_width_uu, 48.0..=192.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Outer Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.duct_height_uu, 48.0..=192.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Wall Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.wall_thickness_uu, 4.0..=24.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Air Draft Velocity (uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.air_draft_velocity, 0.0..=800.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Intake Grate Mesh:");
            if ui.checkbox(&mut cfg.has_intake_grate, "Intake Grate").changed() { changed = true; }
            ui.end_row();

            ui.label("Exhaust Grate Mesh:");
            if ui.checkbox(&mut cfg.has_exhaust_grate, "Exhaust Grate").changed() { changed = true; }
            ui.end_row();

            ui.label("Breakable / Kickable:");
            if ui.checkbox(&mut cfg.grate_breakable, "Shootable Mover").changed() { changed = true; }
            ui.end_row();

            ui.label("Unlock / Breach Event:");
            if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            ui.end_row();

            ui.label("Duct Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.vent_duct_settings.active_config = cfg.clone();
        state.recalculate_vent_duct_preview();
        state.history.push(crate::history::Action::UpdateVentDuctConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Vent Duct Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.vent_duct_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Ventilation Conduit Validated").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Sufficient player crouch clearance, uninhibited transit passage, and balanced aerodynamic drift.");
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
