// crates/ut99-editor/src/gui/teleporter_arch_dialog.rs
//
// Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
// Physical portal frame geometry, vortex planar sheets, exit boost velocity,
// destination node link integrity, and procedural CSG arch brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::TeleporterArchArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_teleporter_arch_dialog {
        return;
    }

    let mut open = state.show_teleporter_arch_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Teleporter Portal Arch & Dimensional Gate Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_teleporter_arch_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Portal Presets:").strong());
        let archetypes = [
            TeleporterArchArchetype::GothicMonolithGate,
            TeleporterArchArchetype::IndustrialSludgeDrainTeleport,
            TeleporterArchArchetype::CosmicVoidStargate,
            TeleporterArchArchetype::HydraulicBulkheadPortal,
            TeleporterArchArchetype::SubterraneanCavernRift,
        ];
        for arch in archetypes {
            let is_sel = state.teleporter_arch_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_teleporter_arch_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.teleporter_arch_settings.show_arch_mesh_viewport, "Portal Arch Wireframe");
        ui.checkbox(&mut state.teleporter_arch_settings.show_vortex_sheet_viewport, "Planar Vortex Sheet");
        ui.checkbox(&mut state.teleporter_arch_settings.show_exit_trajectory_viewport, "Exit Launch Trajectory & Arrow");
        ui.separator();
        if ui.button("🔄 Recalculate Links & Audit").clicked() {
            state.recalculate_teleporter_arch_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Dimensional Vortex & Transit Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Arch & Teleporter Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Aperture Geometry & Redirection Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Routing & Telefrag Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.teleporter_arch_settings.cached_metrics;
    let cfg = &state.teleporter_arch_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 26, 40));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(30, 140, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("PORTAL GATE: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(140, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Aperture Cross Section: {:.0} uu²", metrics.portal_aperture_area_sq_uu),
        format!("Exit Launch Speed: {:.0} uu/s", metrics.exit_launch_speed),
        format!("Destination Clearance: {:.0} uu", metrics.destination_clearance_uu),
        format!("Velocity Redirection: {}", if cfg.b_changes_velocity { "Active (Boosted)" } else { "Preserve Kinetic" }),
        format!("Yaw Redirection: {}", if cfg.b_changes_yaw { "Enabled" } else { "Preserve Facing" }),
        format!("URL Link: '{}'", if cfg.url.is_empty() { "UNLINKED (Dud)" } else { &cfg.url }),
        format!("Portal Tag: '{}'", cfg.tag),
        format!("Total Portal Gates in Level: {}", metrics.total_portal_gates_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("UNLINKED") || stat.contains("0 uu") {
            Color32::from_rgb(255, 120, 100)
        } else {
            Color32::from_rgb(220, 240, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.teleporter_arch_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Arch Placement Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("➡ Facing Direction Vector:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.x).prefix("X: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.y).prefix("Y: ").speed(0.05)).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.facing_direction.z).prefix("Z: ").speed(0.05)).changed() { changed = true; }
        });

        if ui.button("📍 Snap Arch Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.teleporter_arch_settings.active_config = cfg.clone();
        state.recalculate_teleporter_arch_preview();
        state.history.push(crate::history::Action::UpdateTeleporterArchConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Teleporter Arch Placement and Orientation".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.teleporter_arch_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Aperture: {:.0}x{:.0} uu | Exit Boost: {:.0} uu/s", cfg.arch_width_uu, cfg.arch_height_uu, cfg.exit_boost_velocity));
        ui.label(format!("Tag: '{}' -> URL: '{}'", cfg.tag, cfg.url));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Portal Arch & Teleporter Actor to Level").strong()).clicked() {
            state.commit_teleporter_arch_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.teleporter_arch_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("teleporter_arch_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Arch Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.arch_width_uu, 64.0..=320.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Arch Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.arch_height_uu, 96.0..=384.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Pillar Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pillar_thickness_uu, 12.0..=48.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Exit Boost Velocity:");
            if ui.add(egui::Slider::new(&mut cfg.exit_boost_velocity, 0.0..=1200.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Changes Velocity:");
            if ui.checkbox(&mut cfg.b_changes_velocity, "Override Arrival Momentum").changed() { changed = true; }
            ui.end_row();

            ui.label("Changes Yaw:");
            if ui.checkbox(&mut cfg.b_changes_yaw, "Align Player View to Exit Vector").changed() { changed = true; }
            ui.end_row();

            ui.label("Destination URL (Exit Tag):");
            if ui.text_edit_singleline(&mut cfg.url).changed() { changed = true; }
            ui.end_row();

            ui.label("Teleporter Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.teleporter_arch_settings.active_config = cfg.clone();
        state.recalculate_teleporter_arch_preview();
        state.history.push(crate::history::Action::UpdateTeleporterArchConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Teleporter Arch Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.teleporter_arch_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Teleporter Portal Routing Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Destination URL resolved, adequate portal aperture clearance, and safe telefrag margins.");
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
