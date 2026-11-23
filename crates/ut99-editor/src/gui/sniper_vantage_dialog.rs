// crates/ut99-editor/src/gui/sniper_vantage_dialog.rs
//
// Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
// Zoom magnification FOV, headshot hitbox height clearance, sweeping cone coverage,
// defensive parapet cover exposure ratings, and procedural crenelated balcony brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::SniperVantageArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_sniper_vantage_dialog {
        return;
    }

    let mut open = state.show_sniper_vantage_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Sniper Vantage Point & Headshot Nest Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_sniper_vantage_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Vantage Presets:").strong());
        let archetypes = [
            SniperVantageArchetype::HighAltitudeTowerSpire,
            SniperVantageArchetype::ElevatedCorridorCatwalk,
            SniperVantageArchetype::ShadowedRecessedAlcove,
            SniperVantageArchetype::CrossRiverBridgeOverlook,
            SniperVantageArchetype::BaseEntrancePillarPerch,
        ];
        for arch in archetypes {
            let is_sel = state.sniper_vantage_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_sniper_vantage_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.sniper_vantage_settings.show_parapet_mesh_viewport, "Parapet Balcony Wireframe");
        ui.checkbox(&mut state.sniper_vantage_settings.show_sightline_cone_viewport, "Sightline Cone & Firing Frustum");
        ui.checkbox(&mut state.sniper_vantage_settings.show_headshot_plane_viewport, "Headshot Clearance Plane");
        ui.separator();
        if ui.button("🔄 Recalculate Sightline & Audit").clicked() {
            state.recalculate_sniper_vantage_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Ballistic Sightline & Cover Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Balcony & Sniper Weapon to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Parapet Dimensions & Optic Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Fair-Play & Spawn-Camping Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.sniper_vantage_settings.cached_metrics;
    let cfg = &state.sniper_vantage_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 28, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 180, 200)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SNIPER VANTAGE: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(120, 230, 245),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Max Effective Sightline: {:.0} uu", cfg.max_effective_range_uu),
        format!("Headshot Clearance: {:.1} uu", metrics.headshot_clearance_uu),
        format!("Defensive Cover Exposure: {:.0}%", metrics.cover_exposure_pct),
        format!("Sweeping Field of View: {:.0}°", cfg.coverage_angle_deg),
        format!("Telescopic Zoom FOV: {:.0}°", cfg.zoom_fov_deg),
        format!("Estimated Time-To-Kill: {:.2}s", metrics.estimated_ttk_secs),
        format!("Parapet Height: {:.0} uu", cfg.parapet_height_uu),
        format!("Total Vantages in Level: {}", metrics.total_vantage_points_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("Defensive Cover Exposure: 100%") || stat.contains("Headshot Clearance: 0.0") {
            Color32::from_rgb(255, 140, 90)
        } else {
            Color32::from_rgb(220, 245, 250)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.sniper_vantage_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Nest Placement Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("👁 Look Direction Vector:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.look_direction.x).prefix("X: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.look_direction.y).prefix("Y: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.look_direction.z).prefix("Z: ").speed(0.05)).changed() { changed = true; }
        });

        if ui.button("📍 Snap Nest Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.sniper_vantage_settings.active_config = cfg.clone();
        state.recalculate_sniper_vantage_preview();
        state.history.push(crate::history::Action::UpdateSniperVantageConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Sniper Vantage Placement and Orientation".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.sniper_vantage_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Range: {:.0} uu | Zoom FOV: {:.0}°", cfg.max_effective_range_uu, cfg.zoom_fov_deg));
        ui.label(format!("Tag: '{}' (Actor: Botpack.SniperRifle)", cfg.tag));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Balcony Brush & Sniper Weapon to Level").strong()).clicked() {
            state.commit_sniper_vantage_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.sniper_vantage_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("sniper_vantage_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Parapet Battlement Height:");
            if ui.add(egui::Slider::new(&mut cfg.parapet_height_uu, 24.0..=64.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Balcony Width:");
            if ui.add(egui::Slider::new(&mut cfg.parapet_width_uu, 64.0..=320.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Balcony Depth:");
            if ui.add(egui::Slider::new(&mut cfg.parapet_depth_uu, 48.0..=240.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Max Sightline Range:");
            if ui.add(egui::Slider::new(&mut cfg.max_effective_range_uu, 800.0..=6000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Sweeping Coverage Cone:");
            if ui.add(egui::Slider::new(&mut cfg.coverage_angle_deg, 20.0..=180.0).suffix("°")).changed() { changed = true; }
            ui.end_row();

            ui.label("Telescopic Zoom FOV:");
            if ui.add(egui::Slider::new(&mut cfg.zoom_fov_deg, 10.0..=60.0).suffix("°")).changed() { changed = true; }
            ui.end_row();

            ui.label("Vantage Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.sniper_vantage_settings.active_config = cfg.clone();
        state.recalculate_sniper_vantage_preview();
        state.history.push(crate::history::Action::UpdateSniperVantageConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Sniper Vantage Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.sniper_vantage_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Sniper Vantage Configuration Balanced").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No unblocked sightlines to PlayerStarts, adequate cover protection, and unobstructed headshot clearance.");
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
