// crates/ut99-editor/src/gui/zone_velocity_dialog.rs
//
// Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
// Fluid medium, velocity force vectors, buoyancy calculation, low-G vacuum drift,
// damage over time pain zones, and procedural zone portal sheet brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ZoneMediumType, ZoneVelocityArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_zone_velocity_dialog {
        return;
    }

    let mut open = state.show_zone_velocity_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Zone Velocity, Convection & Low-G Vacuum Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_zone_velocity_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Zone Presets:").strong());
        let archetypes = [
            ZoneVelocityArchetype::HydroelectricRiverCurrent,
            ZoneVelocityArchetype::IndustrialUpdraftWindTunnel,
            ZoneVelocityArchetype::OuterSpaceVacuumDrift,
            ZoneVelocityArchetype::ToxicSlimeGutterFlume,
            ZoneVelocityArchetype::HighGravCentrifugeVortex,
        ];
        for arch in archetypes {
            let is_sel = state.zone_velocity_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_zone_velocity_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.zone_velocity_settings.show_portal_sheet_viewport, "Portal Sheet Wireframe");
        ui.checkbox(&mut state.zone_velocity_settings.show_velocity_vectors_viewport, "Velocity Vectors");
        ui.checkbox(&mut state.zone_velocity_settings.show_drift_trajectory_viewport, "Drift Trajectory Line");
        ui.separator();
        if ui.button("🔄 Recalculate Fluid Dynamics & Audit").clicked() {
            state.recalculate_zone_velocity_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Fluid Convection & Velocity Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Zone Portal & ZoneInfo to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Medium & Fluid Vector Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Safety & Fluid Hazard Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.zone_velocity_settings.cached_metrics;
    let cfg = &state.zone_velocity_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 28, 38));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 160, 220)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("ZONE FLUID: {} | Medium: {}", cfg.archetype.display_name(), cfg.medium.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(130, 220, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Effective Velocity: {:.0} uu/s", metrics.effective_velocity_magnitude),
        format!("Drift Transit Time: {:.1}s", metrics.drift_transit_time_secs),
        format!("Buoyancy Rating: {:+.0}%", metrics.buoyancy_rating_pct),
        format!("Terminal Fall Speed: {:.0} uu/s", metrics.terminal_speed_uu_per_sec),
        format!("Fluid Friction (µ): {:.2}", cfg.fluid_friction),
        format!("Zone Gravity: {}", if cfg.is_zero_gravity { "Zero-G (Free Drift)" } else { "Normal (950 uu/s²)" }),
        format!("Hazard Level: {}", if cfg.is_pain_zone { format!("PAIN ZONE ({:.0} DPS)", cfg.damage_per_sec) } else { "Safe Fluid".to_string() }),
        format!("Total Level Zones: {}", metrics.total_velocity_zones_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("PAIN ZONE") {
            Color32::from_rgb(255, 120, 90)
        } else {
            Color32::from_rgb(220, 240, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.zone_velocity_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Portal Sheet Base Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.portal_width_uu).prefix("Portal Width: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.portal_height_uu).prefix("Portal Height: ").suffix(" uu")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Portal Sheet to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.zone_velocity_settings.active_config = cfg.clone();
        state.recalculate_zone_velocity_preview();
        state.history.push(crate::history::Action::UpdateZoneVelocityConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Zone Velocity Placement and Dimensions".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.zone_velocity_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Medium: {}", cfg.medium.display_name()));
        ui.label(format!("Velocity: ({:.0}, {:.0}, {:.0}) uu/s", cfg.zone_velocity.x, cfg.zone_velocity.y, cfg.zone_velocity.z));
        ui.label(format!("Tag: '{}'", cfg.zone_tag));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Portal Sheet & ZoneInfo to Level").strong()).clicked() {
            state.commit_zone_velocity_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.zone_velocity_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("zone_velocity_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Medium Type:");
            egui::ComboBox::from_id_salt("zone_medium_selector")
                .selected_text(cfg.medium.display_name())
                .show_ui(ui, |ui| {
                    let mediums = [
                        ZoneMediumType::AtmosphericAir,
                        ZoneMediumType::SubmergedWater,
                        ZoneMediumType::CorrosiveAcidSlime,
                        ZoneMediumType::MoltenLava,
                        ZoneMediumType::DeepSpaceVacuum,
                    ];
                    for m in mediums {
                        if ui.selectable_value(&mut cfg.medium, m, m.display_name()).clicked() {
                            cfg.fluid_friction = m.default_friction();
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Zone Velocity X (uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.zone_velocity.x, -1500.0..=1500.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Zone Velocity Y (uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.zone_velocity.y, -1500.0..=1500.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Zone Velocity Z (uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.zone_velocity.z, -1500.0..=1500.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Zone Length (uu):");
            if ui.add(egui::Slider::new(&mut cfg.zone_length_uu, 200.0..=4000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Fluid Friction (µ):");
            if ui.add(egui::Slider::new(&mut cfg.fluid_friction, 0.05..=10.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Terminal Velocity:");
            if ui.add(egui::Slider::new(&mut cfg.terminal_velocity, 500.0..=5000.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Zero Gravity:");
            if ui.checkbox(&mut cfg.is_zero_gravity, "bZeroGravity").changed() { changed = true; }
            ui.end_row();

            ui.label("Pain Zone:");
            if ui.checkbox(&mut cfg.is_pain_zone, "bPainZone").changed() { changed = true; }
            ui.end_row();

            if cfg.is_pain_zone {
                ui.label("Damage Per Sec:");
                if ui.add(egui::Slider::new(&mut cfg.damage_per_sec, 1.0..=100.0).suffix(" HP/s")).changed() { changed = true; }
                ui.end_row();

                ui.label("Damage Type:");
                if ui.text_edit_singleline(&mut cfg.damage_type).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Zone Tag:");
            if ui.text_edit_singleline(&mut cfg.zone_tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.zone_velocity_settings.active_config = cfg.clone();
        state.recalculate_zone_velocity_preview();
        state.history.push(crate::history::Action::UpdateZoneVelocityConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Zone Velocity Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.zone_velocity_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Zone Fluid Dynamics Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Velocity magnitude within safe limits, friction provides traction, and no lethal pain traps detected.");
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
