// crates/ut99-editor/src/gui/piston_dialog.rs
//
// Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
// Kinematic stroke analysis, crushing force, player squish lethality, encroachment handling,
// warning siren latency, and procedural compactor brush geometry.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{PistonArchetype, PistonEncroachAction, PistonOrientation};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_piston_dialog {
        return;
    }

    let mut open = state.show_piston_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Industrial Steam Piston & Compactor Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_piston_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Piston Presets:").strong());
        let archetypes = [
            PistonArchetype::PressureChamberVerticalCrusher,
            PistonArchetype::HorizontalBulkheadRam,
            PistonArchetype::ReciprocatingSteamPiston,
            PistonArchetype::SubterraneanCompactorJaw,
            PistonArchetype::HighSpeedPneumaticStamper,
        ];
        for arch in archetypes {
            let is_sel = state.piston_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_piston_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.piston_settings.show_piston_mesh_viewport, "Piston Head & Shaft Wireframe");
        ui.checkbox(&mut state.piston_settings.show_stroke_trajectory_viewport, "Stroke Travel Axis & Ghost Head");
        ui.checkbox(&mut state.piston_settings.show_squish_hazard_viewport, "Crush Hazard Danger Perimeter");
        ui.separator();
        if ui.button("🔄 Recalculate Piston & Audit").clicked() {
            state.recalculate_piston_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Kinematic & Crushing Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Piston Brush & Mover to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Mechanical Stroke & Hazard Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Crushing Lockout & Entrapment Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.piston_settings.cached_metrics;
    let cfg = &state.piston_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(32, 26, 22));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(220, 140, 60)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("INDUSTRIAL PISTON: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(255, 200, 150),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Stroke Travel: {:.0} uu", metrics.stroke_length_uu),
        format!("One-Way Travel Time: {:.2}s", metrics.one_way_travel_time_s),
        format!("Full Cycle Period: {:.2}s", metrics.full_cycle_period_s),
        format!("Compression Force: {:.0} kN", metrics.compression_impulse_force),
        format!("Squish Damage: {} HP ({})", cfg.squish_damage, if metrics.is_instant_kill { "INSTANT GIB" } else { "NON-LETHAL" }),
        format!("Effective DPS: {:.1} HP/s", metrics.effective_dps),
        format!("Compactor Surface: {:.0} uu²", metrics.compactor_surface_area_uu2),
        format!("Total Pistons in Level: {}", metrics.total_pistons_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("INSTANT GIB") {
            Color32::from_rgb(255, 80, 80)
        } else if stat.contains("NON-LETHAL") {
            Color32::from_rgb(100, 220, 140)
        } else {
            Color32::from_rgb(245, 235, 220)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.piston_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Piston Origin (Retracted Head Center):").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("📐 Piston Stroke Direction:").strong());
        ui.horizontal(|ui| {
            let orientations = [
                (PistonOrientation::VerticalDown, "Vertical Down (-Z)"),
                (PistonOrientation::VerticalUp, "Vertical Up (+Z)"),
                (PistonOrientation::HorizontalX, "Horizontal (+X)"),
                (PistonOrientation::HorizontalY, "Horizontal (+Y)"),
            ];
            for (o, label) in orientations {
                if ui.selectable_label(cfg.orientation == o, label).clicked() {
                    cfg.orientation = o;
                    changed = true;
                }
            }
        });

        if ui.button("📍 Snap Piston Origin to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.piston_settings.active_config = cfg.clone();
        state.recalculate_piston_preview();
        state.history.push(crate::history::Action::UpdatePistonConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Piston Position and Orientation".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.piston_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Head Size: {:.0}x{:.0}x{:.0} uu | Stroke: {:.0} uu", cfg.head_width_uu, cfg.head_depth_uu, cfg.head_thickness_uu, cfg.stroke_length_uu));
        ui.label(format!("Velocity: {:.0} uu/s | Squish: {} HP ({})", cfg.stroke_velocity_uu_s, cfg.squish_damage, cfg.encroachment.display_name()));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Piston Brush & Mover to Level").strong()).clicked() {
            state.commit_piston_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.piston_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("piston_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Stroke Travel Length (uu):");
            if ui.add(egui::Slider::new(&mut cfg.stroke_length_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Compactor Head Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.head_width_uu, 32.0..=512.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Compactor Head Depth (uu):");
            if ui.add(egui::Slider::new(&mut cfg.head_depth_uu, 32.0..=512.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Compactor Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.head_thickness_uu, 16.0..=128.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Shaft Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.shaft_radius_uu, 8.0..=64.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Stroke Velocity (uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.stroke_velocity_uu_s, 50.0..=1200.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Apex Dwell / Wait Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.wait_time_s, 0.1..=5.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Squish Damage (HP):");
            if ui.add(egui::Slider::new(&mut cfg.squish_damage, 0..=1000).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Warning Klaxon Lead Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.warning_lead_time_s, 0.0..=3.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Encroachment Action:");
            egui::ComboBox::from_id_salt("piston_encroach_combo")
                .selected_text(cfg.encroachment.display_name())
                .show_ui(ui, |ui| {
                    let actions = [
                        PistonEncroachAction::CrushWhenEncroach,
                        PistonEncroachAction::StopWhenEncroach,
                        PistonEncroachAction::ReturnWhenEncroach,
                    ];
                    for act in actions {
                        if ui.selectable_label(cfg.encroachment == act, act.display_name()).clicked() {
                            cfg.encroachment = act;
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Safety & Sound Options:");
            ui.horizontal(|ui| {
                if ui.checkbox(&mut cfg.has_warning_siren, "Warning Siren").changed() { changed = true; }
                if ui.checkbox(&mut cfg.has_steam_emitter, "Steam Vent FX").changed() { changed = true; }
                if ui.checkbox(&mut cfg.has_abort_button, "Abort Console").changed() { changed = true; }
            });
            ui.end_row();

            ui.label("Actor Tag / Event:");
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
                if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            });
            ui.end_row();
        });

    if changed {
        state.piston_settings.active_config = cfg.clone();
        state.recalculate_piston_preview();
        state.history.push(crate::history::Action::UpdatePistonConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Industrial Piston Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.piston_settings.cached_hazards;

    ui.group(|ui| {
        if hazards.is_empty() {
            ui.colored_label(Color32::from_rgb(80, 220, 120), "✅ All Safety Audits Passed: No Compactor Lockouts or Unfair Hazards Detected.");
        } else {
            ui.colored_label(Color32::from_rgb(255, 120, 80), format!("⚠ Found {} Hazard Warnings:", hazards.len()));
            for h in hazards {
                ui.horizontal(|ui| {
                    let col = if h.is_critical {
                        Color32::from_rgb(255, 60, 60)
                    } else {
                        Color32::from_rgb(255, 180, 60)
                    };
                    ui.colored_label(col, if h.is_critical { "[CRITICAL]" } else { "[WARNING]" });
                    ui.label(egui::RichText::new(&h.title).strong());
                });
                ui.label(egui::RichText::new(&h.description).small());
                ui.separator();
            }
        }
    });
}
