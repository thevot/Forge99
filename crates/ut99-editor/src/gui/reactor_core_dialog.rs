// crates/ut99-editor/src/gui/reactor_core_dialog.rs
//
// Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
// Mathematical modeling of centrifugal kinetic rings, rotating gyroscopes, pulsating reactor cores,
// radiation hazard envelopes, mover synchronization, and procedural CSG geometry.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ReactorCoreArchetype, ReactorHazardType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_reactor_core_dialog {
        return;
    }

    let mut open = state.show_reactor_core_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Reactor Core & Kinetic Ring Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_reactor_core_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Reactor Presets:").strong());
        let archetypes = [
            ReactorCoreArchetype::LiandriCentrifugalCore,
            ReactorCoreArchetype::TurbineVentilationRotor,
            ReactorCoreArchetype::PhobosQuantumGyroscope,
            ReactorCoreArchetype::NivenNuclearContainmentVessel,
            ReactorCoreArchetype::SkaarjMothershipGravitonNexus,
        ];
        for arch in archetypes {
            let is_sel = state.reactor_core_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_reactor_core_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.reactor_core_settings.show_core_brush_viewport, "Plasma Core Wireframe");
        ui.checkbox(&mut state.reactor_core_settings.show_rings_viewport, "Concentric Kinetic Rings Wireframe");
        ui.checkbox(&mut state.reactor_core_settings.show_hazard_radius_viewport, "Radiation Hazard Sphere Boundary");
        ui.separator();
        if ui.button("🔄 Recalculate Kinematics & Audit").clicked() {
            state.recalculate_reactor_core_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Kinetic & Radiation Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Reactor Core & Rings to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Kinetic Rotation & Hazard Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Radiation & Velocity Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.reactor_core_settings.cached_metrics;
    let cfg = &state.reactor_core_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(28, 20, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(220, 100, 240)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("REACTOR CORE: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(250, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Core Volume: {:.0} uu³", metrics.core_volume_uu3),
        format!("Rim Tangential Speed: {:.1} uu/s", metrics.outer_rim_tangential_speed_uus),
        format!("Centrifugal Force: {:.2} g", metrics.centrifugal_g_force),
        format!("Stored Kinetic Energy: {:.2} MJ", metrics.stored_kinetic_energy_mj),
        format!("Hazard Volume: {:.0} uu³", metrics.hazard_volume_uu3),
        format!("Time to Lethal (TTL): {:.1}s ({:.0} HP/s)", metrics.time_to_lethal_sec, cfg.damage_per_sec),
        format!("Hazard Type: {}", cfg.hazard_type.display_name()),
        format!("Total Reactors in Level: {}", metrics.total_reactors_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("Rim Tangential Speed") && metrics.outer_rim_tangential_speed_uus > 2000.0 {
            Color32::from_rgb(255, 90, 90)
        } else if stat.contains("TTL") && metrics.time_to_lethal_sec < 3.0 {
            Color32::from_rgb(255, 140, 90)
        } else {
            Color32::from_rgb(245, 230, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.reactor_core_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Reactor Center Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Core to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.reactor_core_settings.active_config = cfg.clone();
        state.recalculate_reactor_core_preview();
        state.history.push(crate::history::Action::UpdateReactorCoreConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Reactor Core Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.reactor_core_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Core R:{:.0} uu, H:{:.0} uu | Outer Ring R:{:.0} uu", cfg.core_radius_uu, cfg.core_height_uu, cfg.outer_ring_radius_uu));
        ui.label(format!("Kinematics: Outer {:.1} RPM | Mid {:.1} RPM", cfg.outer_ring_rpm, cfg.mid_ring_rpm));
        ui.label(format!("Hazard: {} ({:.0} uu, {:.0} HP/s)", cfg.hazard_type.display_name(), cfg.hazard_radius_uu, cfg.damage_per_sec));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Reactor Core & Rings to Level").strong()).clicked() {
            state.commit_reactor_core_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.reactor_core_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("reactor_core_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Plasma Core Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.core_radius_uu, 16.0..=256.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Plasma Core Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.core_height_uu, 32.0..=512.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Outer Ring Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.outer_ring_radius_uu, 64.0..=600.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Mid Ring Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.mid_ring_radius_uu, 48.0..=500.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Ring Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.ring_thickness_uu, 6.0..=64.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Ring Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.ring_height_uu, 8.0..=128.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Outer Ring RPM:");
            if ui.add(egui::Slider::new(&mut cfg.outer_ring_rpm, -120.0..=120.0).suffix(" RPM")).changed() { changed = true; }
            ui.end_row();

            ui.label("Mid Ring RPM:");
            if ui.add(egui::Slider::new(&mut cfg.mid_ring_rpm, -120.0..=120.0).suffix(" RPM")).changed() { changed = true; }
            ui.end_row();

            ui.label("Hazard Classification:");
            egui::ComboBox::from_id_salt("hazard_type_combo")
                .selected_text(cfg.hazard_type.display_name())
                .show_ui(ui, |ui| {
                    for ht in &[
                        ReactorHazardType::IonizingRadiation,
                        ReactorHazardType::ExtremeThermal,
                        ReactorHazardType::PlasmaArcDischarge,
                        ReactorHazardType::GravitationalCrush,
                        ReactorHazardType::ToxicCoolantMist,
                    ] {
                        if ui.selectable_value(&mut cfg.hazard_type, *ht, ht.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Hazard Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.hazard_radius_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Hazard Damage Per Sec:");
            if ui.add(egui::Slider::new(&mut cfg.damage_per_sec, 0.0..=200.0).suffix(" HP/s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Containment Health (HP):");
            if ui.add(egui::Slider::new(&mut cfg.containment_health, 100.0..=5000.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Critical Meltdown Countdown:");
            if ui.add(egui::Slider::new(&mut cfg.critical_meltdown_sec, 5.0..=120.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Reactor Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();

            ui.label("Meltdown Event:");
            if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.reactor_core_settings.active_config = cfg.clone();
        state.recalculate_reactor_core_preview();
        state.history.push(crate::history::Action::UpdateReactorCoreConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Reactor Core Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &mut EditorState) {
    let hazards = &state.reactor_core_settings.cached_hazards;

    ui.group(|ui| {
        if hazards.is_empty() {
            ui.colored_label(
                Color32::from_rgb(100, 220, 140),
                "✓ No Safety or Physics Hazards Detected. Reactor core is tournament ready.",
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
