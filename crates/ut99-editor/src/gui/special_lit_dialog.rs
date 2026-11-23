// crates/ut99-editor/src/gui/special_lit_dialog.rs
//
// Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
// bSpecialLit light actor isolation, PF_SpecialLit surface matching, dual-pass contrast ratio,
// hue/saturation/brightness filters, and procedural recessed spotlight fixture brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::SpecialLitArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_special_lit_dialog {
        return;
    }

    let mut open = state.show_special_lit_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "SpecialLit & Zone Lighting Mask Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_special_lit_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Lighting Presets:").strong());
        let archetypes = [
            SpecialLitArchetype::ToxicSlimeGlowIsolator,
            SpecialLitArchetype::CelestialBackdropIlluminator,
            SpecialLitArchetype::TeamBaseAccentSpot,
            SpecialLitArchetype::CathedralStainedGlassBeam,
            SpecialLitArchetype::SubterraneanLavaRimLight,
        ];
        for arch in archetypes {
            let is_sel = state.special_lit_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_special_lit_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.special_lit_settings.show_fixture_brush_viewport, "Fixture Brush Wireframe");
        ui.checkbox(&mut state.special_lit_settings.show_special_radius_viewport, "SpecialLit Radius Sphere");
        ui.checkbox(&mut state.special_lit_settings.show_surface_rays_viewport, "Surface Illumination Rays");
        ui.separator();
        if ui.button("🔄 Recalculate Illumination & Audit").clicked() {
            state.recalculate_special_lit_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("SpecialLit Illumination Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Fixture & Light Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Color, Hue & Attenuation Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("SpecialLit Isolation & Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.special_lit_settings.cached_metrics;
    let cfg = &state.special_lit_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 26, 36));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(70, 180, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SPECIALLIT PRESET: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(150, 220, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Effective Flux: {:.1} lm/uu²", metrics.effective_illumination_flux),
        format!("Matched PF_SpecialLit Surfaces: {}", metrics.matched_special_surfaces_count),
        format!("SpecialLit Coverage: {:.0} uu²", metrics.special_lit_coverage_sq_uu),
        format!("Isolation Efficiency: {:.0}%", metrics.light_isolation_efficiency_pct),
        format!("Light Radius: {:.0} uu", cfg.light_radius_uu),
        format!("Brightness: {} / 255", cfg.light_brightness),
        format!("Hue / Saturation: {} / {}", cfg.light_hue, cfg.light_saturation),
        format!("Total Special Lights: {}", metrics.total_special_lights_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("Matched PF_SpecialLit Surfaces: 0") {
            Color32::from_rgb(255, 120, 90)
        } else {
            Color32::from_rgb(220, 240, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.special_lit_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Light Placement Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.fixture_radius_uu).prefix("Fixture Radius: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.fixture_depth_uu).prefix("Fixture Depth: ").suffix(" uu")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Fixture to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.special_lit_settings.active_config = cfg.clone();
        state.recalculate_special_lit_preview();
        state.history.push(crate::history::Action::UpdateSpecialLitConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated SpecialLit Placement and Fixture Dimensions".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.special_lit_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Radius: {:.0} uu | Brightness: {}", cfg.light_radius_uu, cfg.light_brightness));
        ui.label(format!("Tag: '{}' (bSpecialLit=true)", cfg.tag));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Fixture Brush & Special Light to Level").strong()).clicked() {
            state.commit_special_lit_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.special_lit_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("special_lit_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Light Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.light_radius_uu, 64.0..=2000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Brightness:");
            if ui.add(egui::Slider::new(&mut cfg.light_brightness, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Hue (Color):");
            if ui.add(egui::Slider::new(&mut cfg.light_hue, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Saturation:");
            if ui.add(egui::Slider::new(&mut cfg.light_saturation, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Type:");
            if ui.text_edit_singleline(&mut cfg.light_type).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Effect:");
            if ui.text_edit_singleline(&mut cfg.light_effect).changed() { changed = true; }
            ui.end_row();

            ui.label("Actor Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.special_lit_settings.active_config = cfg.clone();
        state.recalculate_special_lit_preview();
        state.history.push(crate::history::Action::UpdateSpecialLitConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified SpecialLit Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.special_lit_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ SpecialLit Configuration Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Target surfaces match PF_SpecialLit flag, brightness is within bounds, and no orphan lights detected.");
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
