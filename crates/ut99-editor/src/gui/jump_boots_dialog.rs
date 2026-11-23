// crates/ut99-editor/src/gui/jump_boots_dialog.rs
//
// Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
// Ballistic jump physics, parabolic apex calculation, target platform reachability,
// ceiling clearance concussion hazard auditing, and procedural boot cradle brush geometry.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::JumpBootsArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_jump_boots_dialog {
        return;
    }

    let mut open = state.show_jump_boots_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Jump Boots Cache & High-Vault Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_jump_boots_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Vault Presets:").strong());
        let archetypes = [
            JumpBootsArchetype::PhobosLowGravityVault,
            JumpBootsArchetype::FacingWorldsTowerSpireAscent,
            JumpBootsArchetype::MorpheusRooftopLeap,
            JumpBootsArchetype::CoretLavaPitTraverse,
            JumpBootsArchetype::GothicCathedralRaftersStash,
        ];
        for arch in archetypes {
            let is_sel = state.jump_boots_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_jump_boots_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.jump_boots_settings.show_cradle_mesh_viewport, "Pedestal & Induction Core Wireframe");
        ui.checkbox(&mut state.jump_boots_settings.show_ballistic_arc_viewport, "Parabolic Vault Arc & Apex Ring");
        ui.checkbox(&mut state.jump_boots_settings.show_target_platform_viewport, "Target Landing Pad Clearance Box");
        ui.separator();
        if ui.button("🔄 Recalculate Ballistics & Audit").clicked() {
            state.recalculate_jump_boots_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Ballistic Vault Flight Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Cradle Brush & Jump Boots to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Ballistic Impulse & Target Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Ceiling Concussion & Stranding Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.jump_boots_settings.cached_metrics;
    let cfg = &state.jump_boots_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 26, 36));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(80, 160, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("JUMP BOOTS VAULT: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(180, 220, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Apex Altitude: +{:.0} uu", metrics.peak_apex_height_uu),
        format!("Time to Apex: {:.2}s", metrics.time_to_apex_s),
        format!("Total Hang Time: {:.2}s", metrics.total_hang_time_s),
        format!("Max Vault Distance: {:.0} uu", metrics.maximum_vault_distance_uu),
        format!("Target Reachable: {}", if metrics.is_target_reachable { "REACHABLE (OK)" } else { "UNREACHABLE (OVERSHOOT)" }),
        format!("Landing Dispersion: ±{:.0} uu", metrics.landing_dispersion_radius_uu),
        format!("Total Cache Capacity: {} Jumps (+{:.0} uu)", cfg.charge_count, metrics.total_cache_potential_altitude_uu),
        format!("Total Boot Caches in Level: {}", metrics.total_boot_caches_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("UNREACHABLE") {
            Color32::from_rgb(255, 90, 90)
        } else if stat.contains("REACHABLE (OK)") {
            Color32::from_rgb(90, 230, 130)
        } else {
            Color32::from_rgb(225, 240, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.jump_boots_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Boot Cradle Pad Center Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Cradle Center to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.jump_boots_settings.active_config = cfg.clone();
        state.recalculate_jump_boots_preview();
        state.history.push(crate::history::Action::UpdateJumpBootsConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Jump Boots Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.jump_boots_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Boost: +{:.0} uu/s | Gravity: {:.0} uu/s²", cfg.launch_boost_vz, cfg.world_gravity_z));
        ui.label(format!("Target Offset: ({:.0}, {:.0}, +{:.0}) uu", cfg.target_platform_offset.x, cfg.target_platform_offset.y, cfg.target_platform_offset.z));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Cradle Brush & Jump Boots to Level").strong()).clicked() {
            state.commit_jump_boots_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.jump_boots_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("jump_boots_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Vertical Boost Impulse (vz uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.launch_boost_vz, 400.0..=2000.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Forward Sprint Velocity (vx uu/s):");
            if ui.add(egui::Slider::new(&mut cfg.forward_velocity_vx, 100.0..=800.0).suffix(" uu/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("World Gravity (Z uu/s²):");
            if ui.add(egui::Slider::new(&mut cfg.world_gravity_z, -1500.0..=-100.0).suffix(" uu/s²")).changed() { changed = true; }
            ui.end_row();

            ui.label("Boot Charges Count:");
            if ui.add(egui::Slider::new(&mut cfg.charge_count, 1..=10)).changed() { changed = true; }
            ui.end_row();

            ui.label("Cradle Radius & Height (uu):");
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut cfg.cradle_radius_uu).prefix("R: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.cradle_height_uu).prefix("H: ")).changed() { changed = true; }
            });
            ui.end_row();

            ui.label("Target Platform Offset (uu):");
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut cfg.target_platform_offset.x).prefix("X: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.target_platform_offset.y).prefix("Y: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.target_platform_offset.z).prefix("Z: ")).changed() { changed = true; }
            });
            ui.end_row();

            ui.label("Target Platform Size (W x D):");
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut cfg.target_platform_size.x).prefix("W: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.target_platform_size.y).prefix("D: ")).changed() { changed = true; }
            });
            ui.end_row();

            ui.label("Overhead Ceiling Clearance (uu):");
            if ui.add(egui::Slider::new(&mut cfg.overhead_ceiling_clearance_uu, 100.0..=3000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Return Route Available:");
            if ui.checkbox(&mut cfg.has_return_route, "Stairs/Lift Present (Avoids Stranding)").changed() { changed = true; }
            ui.end_row();

            ui.label("Actor Tag / Event:");
            ui.horizontal(|ui| {
                if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
                if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            });
            ui.end_row();
        });

    if changed {
        state.jump_boots_settings.active_config = cfg.clone();
        state.recalculate_jump_boots_preview();
        state.history.push(crate::history::Action::UpdateJumpBootsConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Jump Boots Ballistics Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.jump_boots_settings.cached_hazards;

    ui.group(|ui| {
        if hazards.is_empty() {
            ui.colored_label(Color32::from_rgb(80, 220, 120), "✅ All Safety Audits Passed: Target Platform Reachable and No Ceiling Concussion Hazards.");
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
