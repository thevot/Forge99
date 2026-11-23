// crates/ut99-editor/src/gui/force_field_dialog.rs
//
// Interactive Force Field, Energy Barrier & Power Core Overload Studio
// Mathematical modeling of electromagnetic deflection barriers, plasma containment gates,
// permeability filtering (Shock combo pass-through), generator linkage, and procedural CSG geometry.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ForceFieldArchetype, ForceFieldColorPalette, ForceFieldPermeability};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_force_field_dialog {
        return;
    }

    let mut open = state.show_force_field_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Force Field & Energy Barrier Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_force_field_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Barrier Presets:").strong());
        let archetypes = [
            ForceFieldArchetype::AssaultBulkheadForceField,
            ForceFieldArchetype::PhobosPlasmaContainmentGate,
            ForceFieldArchetype::MothershipEnergyShield,
            ForceFieldArchetype::CybrosisSecurityRepulsor,
            ForceFieldArchetype::ReactorCoreDeflectorGrid,
        ];
        for arch in archetypes {
            let is_sel = state.force_field_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_force_field_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.force_field_settings.show_field_slab_viewport, "Field Barrier Slab & Flux Lines");
        ui.checkbox(&mut state.force_field_settings.show_pylons_viewport, "Emitter Pylons Wireframe");
        ui.checkbox(&mut state.force_field_settings.show_generator_link_viewport, "Power Generator Vector Link");
        ui.separator();
        if ui.button("🔄 Recalculate Flux & Audit").clicked() {
            state.recalculate_force_field_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Electromagnetic Field Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Force Field & Pylons to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Permeability, Flux & Generator Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Lockout & Encroachment Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.force_field_settings.cached_metrics;
    let cfg = &state.force_field_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 28, 38));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 180, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("FORCE FIELD: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(180, 230, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Surface Area: {:.0} uu²", metrics.surface_area_uu2),
        format!("Power Consumption: {:.1} kW", metrics.power_consumption_kw),
        format!("Absorbed Energy: {:.1} kJ", metrics.absorbed_energy_kj),
        format!("Deflection Efficiency: {:.0}%", metrics.deflection_efficiency_pct),
        format!("Pylon Clearance: {:.1} uu", metrics.pylon_clearance_width_uu),
        format!("Permeability: {}", cfg.permeability.display_name()),
        format!("Generator: {}", if cfg.generator_linked { format!("{} ({:.0} HP)", cfg.generator_tag, cfg.generator_health) } else { "Direct Grid (Permanent)".to_string() }),
        format!("Total Fields in Level: {}", metrics.total_fields_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("Permanent") {
            Color32::from_rgb(255, 120, 100)
        } else if stat.contains("Clearance") && metrics.pylon_clearance_width_uu < 48.0 {
            Color32::from_rgb(255, 90, 90)
        } else {
            Color32::from_rgb(220, 245, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.force_field_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Barrier Center Position & Yaw:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });
        ui.horizontal(|ui| {
            ui.label("Rotation Yaw:");
            if ui.add(egui::Slider::new(&mut cfg.yaw_deg, 0.0..=360.0).suffix("°")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Barrier to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.force_field_settings.active_config = cfg.clone();
        state.recalculate_force_field_preview();
        state.history.push(crate::history::Action::UpdateForceFieldConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Force Field Position/Yaw".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.force_field_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Size: {:.0} x {:.0} uu | Voltage: {:.0} kV", cfg.width_uu, cfg.height_uu, cfg.voltage_kv));
        ui.label(format!("Permeability: {}", cfg.permeability.display_name()));
        ui.label(format!("Pylons: {}", if cfg.has_emitter_pylons { "Enabled (Semi-Solid)" } else { "Disabled" }));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Force Field & Pylons to Level").strong()).clicked() {
            state.commit_force_field_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.force_field_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("force_field_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Field Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.width_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Field Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.height_uu, 64.0..=768.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Field Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.thickness_uu, 2.0..=64.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Voltage Rating (kV):");
            if ui.add(egui::Slider::new(&mut cfg.voltage_kv, 10.0..=500.0).suffix(" kV")).changed() { changed = true; }
            ui.end_row();

            ui.label("Permeability Filtering:");
            egui::ComboBox::from_id_salt("permeability_combo")
                .selected_text(cfg.permeability.display_name())
                .show_ui(ui, |ui| {
                    for perm in &[
                        ForceFieldPermeability::BlocksAll,
                        ForceFieldPermeability::BlocksPlayersOnly,
                        ForceFieldPermeability::BlocksProjectilesOnly,
                        ForceFieldPermeability::ShockComboPassThrough,
                    ] {
                        if ui.selectable_value(&mut cfg.permeability, *perm, perm.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Color Palette:");
            egui::ComboBox::from_id_salt("color_palette_combo")
                .selected_text(cfg.color_palette.display_name())
                .show_ui(ui, |ui| {
                    for pal in &[
                        ForceFieldColorPalette::CyanAzure,
                        ForceFieldColorPalette::SkaarjEmerald,
                        ForceFieldColorPalette::CrimsonAlert,
                        ForceFieldColorPalette::VoidPurple,
                        ForceFieldColorPalette::GoldAmber,
                    ] {
                        if ui.selectable_value(&mut cfg.color_palette, *pal, pal.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Pulse Frequency (Hz):");
            if ui.add(egui::Slider::new(&mut cfg.pulse_frequency_hz, 0.5..=10.0).suffix(" Hz")).changed() { changed = true; }
            ui.end_row();

            ui.label("Emitter Pylons:");
            if ui.checkbox(&mut cfg.has_emitter_pylons, "Generate Pylon Brushes").changed() { changed = true; }
            ui.end_row();

            if cfg.has_emitter_pylons {
                ui.label("Pylon Radius (uu):");
                if ui.add(egui::Slider::new(&mut cfg.pylon_radius_uu, 8.0..=64.0).suffix(" uu")).changed() { changed = true; }
                ui.end_row();

                ui.label("Pylon Height (uu):");
                if ui.add(egui::Slider::new(&mut cfg.pylon_height_uu, 64.0..=800.0).suffix(" uu")).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Power Generator Linkage:");
            if ui.checkbox(&mut cfg.generator_linked, "Linked to Power Generator Actor").changed() { changed = true; }
            ui.end_row();

            if cfg.generator_linked {
                ui.label("Generator Tag:");
                if ui.text_edit_singleline(&mut cfg.generator_tag).changed() { changed = true; }
                ui.end_row();

                ui.label("Generator Health (HP):");
                if ui.add(egui::Slider::new(&mut cfg.generator_health, 50.0..=3000.0).suffix(" HP")).changed() { changed = true; }
                ui.end_row();

                ui.label("Collapse Delay (sec):");
                if ui.add(egui::Slider::new(&mut cfg.collapse_delay_sec, 0.0..=5.0).suffix(" s")).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Barrier Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();

            ui.label("Breach Event:");
            if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.force_field_settings.active_config = cfg.clone();
        state.recalculate_force_field_preview();
        state.history.push(crate::history::Action::UpdateForceFieldConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Force Field Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &mut EditorState) {
    let hazards = &state.force_field_settings.cached_hazards;

    ui.group(|ui| {
        if hazards.is_empty() {
            ui.colored_label(
                Color32::from_rgb(100, 220, 140),
                "✓ No Safety or Pacing Hazards Detected. Barrier is tournament ready.",
            );
        } else {
            for h in hazards {
                let col = if h.is_critical {
                    Color32::from_rgb(255, 80, 80)
                } else {
                    Color32::from_rgb(255, 200, 60)
                };
                ui.colored_label(col, format!("⚠ {}", h.title));
                ui.label(egui::RichText::new(&h.description).small());
                ui.add_space(2.0);
            }
        }
    });
}
