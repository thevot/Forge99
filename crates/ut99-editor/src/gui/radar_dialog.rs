// crates/ut99-editor/src/gui/radar_dialog.rs
//
// Interactive Team Base Radar, Jammer & Holographic Threat Map Studio
// Team base radar stations, tactical radar jammers, sweep rates, detection latencies,
// threat tracking, friendly jammer interference auditing, and procedural parabolic dish antenna generation.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{RadarScanMode, RadarStudioArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_radar_dialog {
        return;
    }

    let mut open = state.show_radar_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Team Base Radar, Jammer & Threat Map Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_radar_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Radar Presets:").strong());
        let archetypes = [
            RadarStudioArchetype::OrbitalCommandHQ,
            RadarStudioArchetype::StealthInfiltrationOutpost,
            RadarStudioArchetype::FortressPerimeterSentinel,
            RadarStudioArchetype::SubterraneanBunkerGrid,
            RadarStudioArchetype::MobileAPCReconVehicle,
        ];
        for arch in archetypes {
            let is_sel = state.radar_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_radar_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.radar_settings.show_antenna_viewport, "Mast & Dish Wireframe");
        ui.checkbox(&mut state.radar_settings.show_detection_radius_viewport, "Detection Perimeter Circle");
        ui.checkbox(&mut state.radar_settings.show_sweep_cone_viewport, "Surveillance Sweep Frustum");
        ui.separator();
        if ui.button("🔄 Recalculate Radar Telemetry & Audit").clicked() {
            state.recalculate_radar_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Radar Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Surveillance Telemetry Dashboard");
            render_radar_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Radar to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Radar & Jammer Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Interference & Coverage Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_radar_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.radar_settings.cached_metrics;
    let cfg = &state.radar_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 24, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(40, 75, 95)), StrokeKind::Middle);

    // Title banner
    let mode_desc = if cfg.is_jammer { "TACTICAL SCRAMBLER / JAMMER" } else { "ACTIVE SURVEILLANCE RADAR" };
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("{}: {} | TEAM: {}", mode_desc, cfg.scan_mode.display_name(), cfg.team),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(60, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let peer_str = if metrics.distance_to_closest_peer_uu < 9000.0 { format!("{:.0} uu", metrics.distance_to_closest_peer_uu) } else { "Isolated Node".to_string() };

    let stats = [
        format!("Coverage Area: {:.1} M uu²", metrics.effective_coverage_area_sq_uu / 1_000_000.0),
        format!("Detection Latency: {:.2}s", metrics.threat_detection_latency_secs),
        format!("Jammer Counter: {:.0}%", metrics.jammer_counter_effectiveness_pct),
        format!("Closest Station: {}", peer_str),
        format!("Total Network Nodes: {}", metrics.total_nodes_in_level),
        format!("Sweep Rate: {:.1} RPM", cfg.sweep_rate_rpm),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(200, 230, 245));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.radar_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Station Location:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Station to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.radar_settings.active_config = cfg.clone();
        state.recalculate_radar_preview();
        state.history.push(crate::history::Action::UpdateRadarConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Radar Station Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.radar_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        let node_kind = if cfg.is_jammer { "Tactical Jammer" } else { "Surveillance Radar" };
        ui.label(format!("Station Type: {}", node_kind));
        ui.label(format!("Surveillance Radius: {:.0} uu", cfg.detection_radius_uu));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Mast Brush & Station Actor to World").strong()).clicked() {
            state.commit_radar_station_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.radar_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("radar_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Scan Mode:");
            egui::ComboBox::from_id_salt("radar_scan_mode_selector")
                .selected_text(cfg.scan_mode.display_name())
                .show_ui(ui, |ui| {
                    let modes = [
                        RadarScanMode::OmnidirectionalSweep,
                        RadarScanMode::DirectionalSectorCone,
                        RadarScanMode::FrequencyHoppingPulse,
                    ];
                    for m in modes {
                        if ui.selectable_value(&mut cfg.scan_mode, m, m.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Team Assignment:");
            egui::ComboBox::from_id_salt("radar_team_selector")
                .selected_text(match cfg.team {
                    0 => "Red Team (0)",
                    1 => "Blue Team (1)",
                    2 => "Green Team (2)",
                    _ => "Gold Team (3)",
                })
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut cfg.team, 0, "Red Team (0)").clicked() { changed = true; }
                    if ui.selectable_value(&mut cfg.team, 1, "Blue Team (1)").clicked() { changed = true; }
                    if ui.selectable_value(&mut cfg.team, 2, "Green Team (2)").clicked() { changed = true; }
                    if ui.selectable_value(&mut cfg.team, 3, "Gold Team (3)").clicked() { changed = true; }
                });
            ui.end_row();

            ui.label("Detection Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.detection_radius_uu, 500.0..=5000.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Sweep Rate (RPM):");
            if ui.add(egui::Slider::new(&mut cfg.sweep_rate_rpm, 0.0..=60.0).suffix(" RPM")).changed() { changed = true; }
            ui.end_row();

            if cfg.scan_mode == RadarScanMode::DirectionalSectorCone {
                ui.label("Sector Cone Angle:");
                if ui.add(egui::Slider::new(&mut cfg.cone_angle_degrees, 30.0..=180.0).suffix("°")).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Is Tactical Jammer:");
            if ui.checkbox(&mut cfg.is_jammer, "Jammer / Radar Cloak").changed() { changed = true; }
            ui.end_row();

            if cfg.is_jammer {
                ui.label("Jamming Strength:");
                if ui.add(egui::Slider::new(&mut cfg.jamming_strength, 0.1..=1.0)).changed() { changed = true; }
                ui.end_row();
            }

            ui.label("Dish Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.dish_radius, 20.0..=80.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Dish Depth (uu):");
            if ui.add(egui::Slider::new(&mut cfg.dish_depth, 4.0..=32.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Mast Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.mast_height, 32.0..=256.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Station Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.radar_settings.active_config = cfg.clone();
        state.recalculate_radar_preview();
        state.history.push(crate::history::Action::UpdateRadarConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Radar Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.radar_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Surveillance Network Optimal & Unscrambled").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No friendly jammer interference, responsive sweep rates, and clear perimeter coverage.");
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
