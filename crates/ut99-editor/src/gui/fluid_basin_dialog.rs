// crates/ut99-editor/src/gui/fluid_basin_dialog.rs
//
// Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
// Excavated fluid basin geometry, submerged pain zone DPS calibration, buoyancy,
// escape ladder/ramp safety verification, and procedural CSG trough brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::FluidBasinArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_fluid_basin_dialog {
        return;
    }

    let mut open = state.show_fluid_basin_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Toxic Slime Acid Basin & Caustic Pool Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_fluid_basin_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Basin Presets:").strong());
        let archetypes = [
            FluidBasinArchetype::CorrosiveDeckSlimeVat,
            FluidBasinArchetype::MoltenMagmaChasm,
            FluidBasinArchetype::IndustrialCoolantReservoir,
            FluidBasinArchetype::NaliSacredHealingSpring,
            FluidBasinArchetype::CryogenicSubzeroBrine,
        ];
        for arch in archetypes {
            let is_sel = state.fluid_basin_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_fluid_basin_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.fluid_basin_settings.show_basin_mesh_viewport, "Excavated Basin Trough Wireframe");
        ui.checkbox(&mut state.fluid_basin_settings.show_fluid_surface_viewport, "Planar Fluid Surface Sheet");
        ui.checkbox(&mut state.fluid_basin_settings.show_escape_ramp_viewport, "Escape Ladder / Ramp Indicators");
        ui.separator();
        if ui.button("🔄 Recalculate Fluid & Audit").clicked() {
            state.recalculate_fluid_basin_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Chemical Dynamics & Lethality Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Basin Brush & Zone Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Basin Trough & PainZone Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Submerged Inescapable Trap Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.fluid_basin_settings.cached_metrics;
    let cfg = &state.fluid_basin_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 30, 18));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 200, 70)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("HAZARD BASIN: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(120, 245, 140),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Fluid Capacity: {:.1} k uu³", metrics.fluid_volume_k_cu_uu),
        format!("Damage Rate: {:.0} DPS ({})", cfg.dps, cfg.damage_type),
        format!("Survival Time (100 HP): {:.1}s", metrics.player_survival_time_secs),
        format!("Terminal Sink Speed: {:.0} uu/s", metrics.terminal_sink_speed_uu_s),
        format!("Fluid Viscous Friction: {:.1}", cfg.fluid_friction),
        format!("Buoyancy Multiplier: {:.2}", cfg.buoyancy_factor),
        format!("Escape Route: {}", if cfg.has_escape_ladder_or_ramp { "Present (Climbable)" } else { "MISSING (Inescapable Trap)" }),
        format!("Total Hazard Basins in Level: {}", metrics.total_hazard_basins_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("MISSING") || stat.contains("Survival Time (100 HP): 1.2s") {
            Color32::from_rgb(255, 100, 80)
        } else {
            Color32::from_rgb(220, 255, 230)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.fluid_basin_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Basin Rim Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Basin Center to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.fluid_basin_settings.active_config = cfg.clone();
        state.recalculate_fluid_basin_preview();
        state.history.push(crate::history::Action::UpdateFluidBasinConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Fluid Basin Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.fluid_basin_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Size: {:.0}x{:.0} uu (Depth: {:.0} uu) | DPS: {:.0}", cfg.width_uu, cfg.length_uu, cfg.depth_uu, cfg.dps));
        ui.label(format!("Tag: '{}' (Actor: {})", cfg.tag, if cfg.dps > 0.0 { "Botpack.PainZone" } else { "Botpack.WaterZone" }));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Basin Trough Brush & Zone Actor to Level").strong()).clicked() {
            state.commit_fluid_basin_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.fluid_basin_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("fluid_basin_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Basin Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.width_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Basin Length (uu):");
            if ui.add(egui::Slider::new(&mut cfg.length_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Basin Excavation Depth:");
            if ui.add(egui::Slider::new(&mut cfg.depth_uu, 16.0..=384.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Damage Per Second (DPS):");
            if ui.add(egui::Slider::new(&mut cfg.dps, -30.0..=200.0).suffix(" DPS")).changed() { changed = true; }
            ui.end_row();

            ui.label("Fluid Friction:");
            if ui.add(egui::Slider::new(&mut cfg.fluid_friction, 0.1..=6.0).step_by(0.1)).changed() { changed = true; }
            ui.end_row();

            ui.label("Buoyancy Factor:");
            if ui.add(egui::Slider::new(&mut cfg.buoyancy_factor, 0.2..=2.0).step_by(0.05)).changed() { changed = true; }
            ui.end_row();

            ui.label("Escape Ladder / Ramp:");
            if ui.checkbox(&mut cfg.has_escape_ladder_or_ramp, "Provide Exit Climbing Access").changed() { changed = true; }
            ui.end_row();

            ui.label("Damage Type Name:");
            if ui.text_edit_singleline(&mut cfg.damage_type).changed() { changed = true; }
            ui.end_row();

            ui.label("Basin Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.fluid_basin_settings.active_config = cfg.clone();
        state.recalculate_fluid_basin_preview();
        state.history.push(crate::history::Action::UpdateFluidBasinConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Fluid Basin Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.fluid_basin_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Fluid Hazard Basin Balanced").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Climbable exit route provided, balanced DPS progression, and stable fluid physics coefficients.");
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
