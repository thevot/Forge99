// crates/ut99-editor/src/gui/skylight_dialog.rs
//
// Interactive Glass Skylight, Observation Dome & Structural Truss Studio
// Glazed skylights, observation domes, industrial sawtooth roofs, structural mullions,
// solar lux transmittance ratings, and procedural skylight frame & glass sheet brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::SkylightArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_skylight_dialog {
        return;
    }

    let mut open = state.show_skylight_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Glass Skylight & Observation Dome Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_skylight_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Skylight Presets:").strong());
        let archetypes = [
            SkylightArchetype::CurvedObservationDome,
            SkylightArchetype::IndustrialSawtoothSkylight,
            SkylightArchetype::VaultedAtriumBarrelVault,
            SkylightArchetype::ReinforcedBlastPavementPane,
            SkylightArchetype::ShipBridgeObservationCanopy,
        ];
        for arch in archetypes {
            let is_sel = state.skylight_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_skylight_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.skylight_settings.show_skylight_mesh_viewport, "Glazed Perimeter Frame Wireframe");
        ui.checkbox(&mut state.skylight_settings.show_sunlight_shaft_viewport, "Sunlight Ray Shaft / Lux Cone");
        ui.checkbox(&mut state.skylight_settings.show_mullion_truss_viewport, "Structural Mullion Ribs & Ridge");
        ui.separator();
        if ui.button("🔄 Recalculate Glazing & Audit").clicked() {
            state.recalculate_skylight_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Glazing & Solar Transmittance Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Skylight Brush & Sunlight Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Architectural & Optical Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Structural Integrity & Shatter Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.skylight_settings.cached_metrics;
    let cfg = &state.skylight_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 28, 36));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(80, 180, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SKYLIGHT GLAZING: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(180, 230, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Glazed Surface Area: {:.0} uu²", metrics.total_glazed_surface_area_sq_uu),
        format!("Structural Aspect Ratio: {:.2}:1", metrics.structural_aspect_ratio),
        format!("Solar Lux Transmittance: {:.1} lux rating", metrics.lux_solar_transmittance_rating),
        format!("Glass Transmittance: {:.0}%", cfg.glass_transmittance_pct),
        format!("Structural Reinforcement: {}", if cfg.is_bulletproof { "BULLETPROOF / BLAST" } else { "STANDARD GLAZED" }),
        format!("Mullion Ribs: {} (Thickness: {:.0} uu)", cfg.rib_count, cfg.mullion_thickness_uu),
        format!("Skylight Tag: '{}'", cfg.tag),
        format!("Total Glazed Skylights in Level: {}", metrics.total_skylights_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("STANDARD GLAZED") {
            Color32::from_rgb(255, 200, 120)
        } else {
            Color32::from_rgb(230, 245, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.skylight_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Skylight Base Center Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Skylight Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.skylight_settings.active_config = cfg.clone();
        state.recalculate_skylight_preview();
        state.history.push(crate::history::Action::UpdateSkylightConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Skylight Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.skylight_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Dimensions: {:.0}x{:.0} uu (Rise: {:.0} uu, {} Ribs)", cfg.width_uu, cfg.length_uu, cfg.height_or_rise_uu, cfg.rib_count));
        ui.label(format!("Sunlight: Brightness {}, Hue {} | Tag: '{}'", cfg.sunlight_brightness, cfg.sunlight_hue, cfg.tag));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Skylight Brush & Sunlight Actor to Level").strong()).clicked() {
            state.commit_skylight_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.skylight_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("skylight_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Glazing Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.width_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Glazing Length (uu):");
            if ui.add(egui::Slider::new(&mut cfg.length_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Rise / Arch Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.height_or_rise_uu, 0.0..=256.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Mullion Rib Count:");
            if ui.add(egui::Slider::new(&mut cfg.rib_count, 1..=12)).changed() { changed = true; }
            ui.end_row();

            ui.label("Mullion Beam Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.mullion_thickness_uu, 4.0..=32.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Glass Transmittance (%):");
            if ui.add(egui::Slider::new(&mut cfg.glass_transmittance_pct, 0.0..=100.0).suffix(" %")).changed() { changed = true; }
            ui.end_row();

            ui.label("Reinforced / Bulletproof:");
            if ui.checkbox(&mut cfg.is_bulletproof, "Bulletproof Glass").changed() { changed = true; }
            ui.end_row();

            ui.label("Sunlight Emitter Brightness:");
            if ui.add(egui::Slider::new(&mut cfg.sunlight_brightness, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Sunlight Emitter Hue:");
            if ui.add(egui::Slider::new(&mut cfg.sunlight_hue, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Skylight Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.skylight_settings.active_config = cfg.clone();
        state.recalculate_skylight_preview();
        state.history.push(crate::history::Action::UpdateSkylightConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Skylight Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.skylight_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Glazed Skylight Structurally Verified").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Adequate mullion rib reinforcement, high solar transmittance rating, and shatter-resistant canopy.");
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
