// crates/ut99-editor/src/gui/domination_dialog.rs
//
// Interactive Domination Control Point & Point-Weight Triangle Studio
// Triangle geometry metrics, travel times, triangle asymmetry ratio, vertical elevation disparity,
// point capture radius, scoring cadence telemetry, and procedural stepped capture pedestal generation.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{DominationArchetype, DominationPointId};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_domination_dialog {
        return;
    }

    let mut open = state.show_domination_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Domination Control Point & Point-Weight Triangle Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_domination_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("DOM Presets:").strong());
        let archetypes = [
            DominationArchetype::CrypticTriadBalance,
            DominationArchetype::CinderVerticalTower,
            DominationArchetype::GearsIndustrialHub,
            DominationArchetype::LethalCorridorSprint,
            DominationArchetype::SesmarTombPyramid,
        ];
        for arch in archetypes {
            let is_sel = state.domination_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_domination_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.domination_settings.show_pedestal_viewport, "Pedestal Wireframe");
        ui.checkbox(&mut state.domination_settings.show_capture_radius_viewport, "Capture Cylinder Radius");
        ui.checkbox(&mut state.domination_settings.show_triangle_sightlines_viewport, "Triangle Sightlines & Sprint Vectors");
        ui.separator();
        if ui.button("🔄 Recalculate Domination Telemetry & Audit").clicked() {
            state.recalculate_domination_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Domination Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Domination Triangle Dashboard");
            render_domination_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit ControlPoint to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("ControlPoint Configuration");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Triangle Geometry & Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_domination_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.domination_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 26, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 75, 105)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("DOMINATON TRIANGLE: {} POINTS | ASYMMETRY: {:.1}%", metrics.total_points_in_level, metrics.triangle_asymmetry_ratio),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(255, 200, 80),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let dist_b_str = if metrics.dist_to_point_b < 9000.0 { format!("{:.0} uu ({:.1}s)", metrics.dist_to_point_b, metrics.sprint_time_to_point_b_secs) } else { "None".to_string() };
    let dist_c_str = if metrics.dist_to_point_c < 9000.0 { format!("{:.0} uu ({:.1}s)", metrics.dist_to_point_c, metrics.sprint_time_to_point_c_secs) } else { "None".to_string() };

    let stats = [
        format!("Distance to Secondary Point: {}", dist_b_str),
        format!("Distance to Tertiary Point: {}", dist_c_str),
        format!("Elevation Disparity (ΔZ): {:.0} uu", metrics.vertical_elevation_disparity_uu),
        format!("Scoring Rate: {:.0} pts/min", metrics.points_per_minute),
        format!("Capture Radius: {:.0} uu", state.domination_settings.active_config.capture_radius_uu),
        format!("Bot Defense Priority: {:.1}x", state.domination_settings.active_config.defense_priority),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(220, 230, 245));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.domination_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 ControlPoint Location:").strong());
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
        state.domination_settings.active_config = cfg.clone();
        state.recalculate_domination_preview();
        state.history.push(crate::history::Action::UpdateDominationConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated ControlPoint Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.domination_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Point ID: {}", cfg.active_point.display_name()));
        ui.label(format!("Scoring: 1 pt / {:.1}s", cfg.score_cadence_secs));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Pedestal & ControlPoint to World").strong()).clicked() {
            state.commit_domination_point_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.domination_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("domination_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Control Point Identity:");
            egui::ComboBox::from_id_salt("dom_point_id_selector")
                .selected_text(cfg.active_point.display_name())
                .show_ui(ui, |ui| {
                    let pts = [
                        DominationPointId::PointA,
                        DominationPointId::PointB,
                        DominationPointId::PointC,
                    ];
                    for p in pts {
                        if ui.selectable_value(&mut cfg.active_point, p, p.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Capture Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.capture_radius_uu, 40.0..=96.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Score Cadence (secs):");
            if ui.add(egui::Slider::new(&mut cfg.score_cadence_secs, 2.0..=10.0).suffix("s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Bot Defense Priority:");
            if ui.add(egui::Slider::new(&mut cfg.defense_priority, 0.5..=2.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_radius, 32.0..=80.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_height, 6.0..=24.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("ControlPoint Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.domination_settings.active_config = cfg.clone();
        state.recalculate_domination_preview();
        state.history.push(crate::history::Action::UpdateDominationConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Domination Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.domination_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Domination Triangle Balanced & Safe").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Equilateral spacing and standard 5.0s scoring cadence ensure competitive fairness without 2-point lock.");
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
