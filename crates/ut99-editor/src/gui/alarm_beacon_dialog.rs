// crates/ut99-editor/src/gui/alarm_beacon_dialog.rs
//
// Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
// Warning strobe beacons, rotating reflector rotors, emergency red alerts,
// acoustic siren decibels, event trigger wiring, and procedural beacon mount brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::AlarmBeaconArchetype;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_alarm_beacon_dialog {
        return;
    }

    let mut open = state.show_alarm_beacon_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Alarm Siren Beacon & Emergency Strobe Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_alarm_beacon_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Alarm Presets:").strong());
        let archetypes = [
            AlarmBeaconArchetype::NuclearReactorBreachStrobe,
            AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren,
            AlarmBeaconArchetype::HazardDecompressionPulse,
            AlarmBeaconArchetype::ContainmentFailureKlaxon,
            AlarmBeaconArchetype::StealthInfiltrationSilentBeacon,
        ];
        for arch in archetypes {
            let is_sel = state.alarm_beacon_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_alarm_beacon_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.alarm_beacon_settings.show_beacon_mesh_viewport, "Beacon Mount Wireframe");
        ui.checkbox(&mut state.alarm_beacon_settings.show_strobe_pulse_viewport, "Strobe Pulse / Rotor Beam");
        ui.checkbox(&mut state.alarm_beacon_settings.show_audible_radius_viewport, "Acoustic Siren Decibel Range");
        ui.separator();
        if ui.button("🔄 Recalculate Telemetry & Audit").clicked() {
            state.recalculate_alarm_beacon_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Photometric & Acoustic Siren Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Beacon Brush & Light Actor to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Optical & Stroboscopic Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Photosensitivity & Trigger Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.alarm_beacon_settings.cached_metrics;
    let cfg = &state.alarm_beacon_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(32, 16, 16));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(240, 50, 50)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("ALARM SYSTEM: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(255, 160, 160),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Photometric Reach: {:.0} uu", metrics.photometric_reach_uu),
        format!("Audible Siren Range: {:.0} uu ({:.0} dB)", metrics.acoustic_audible_radius_uu, cfg.sound_volume_db),
        format!("Flash/Rotor Cycle: {:.2}s", metrics.flash_cycle_duration_secs),
        format!("Strobe Frequency: {:.1} Hz", cfg.strobe_frequency_hz),
        format!("Rotor Angular Velocity: {:.0} RPM", cfg.rotor_rpm),
        format!("Audio Klaxon: '{}'", if cfg.alarm_sound.is_empty() { "SILENT / MUTED" } else { &cfg.alarm_sound }),
        format!("Tag: '{}' | Event: '{}'", cfg.tag, if cfg.event.is_empty() { "NONE" } else { &cfg.event }),
        format!("Total Alarms in Level: {}", metrics.total_alarms_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("SILENT") || stat.contains("NONE") {
            Color32::from_rgb(255, 180, 100)
        } else {
            Color32::from_rgb(255, 235, 235)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.alarm_beacon_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Beacon Mount Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Beacon Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.alarm_beacon_settings.active_config = cfg.clone();
        state.recalculate_alarm_beacon_preview();
        state.history.push(crate::history::Action::UpdateAlarmBeaconConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Alarm Beacon Position".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.alarm_beacon_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Radius: {:.0} uu | Strobe: {:.1} Hz | Rotor: {:.0} RPM", cfg.light_radius_uu, cfg.strobe_frequency_hz, cfg.rotor_rpm));
        ui.label(format!("Tag: '{}' (Event: '{}')", cfg.tag, cfg.event));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Beacon Brush & Light Actor to Level").strong()).clicked() {
            state.commit_alarm_beacon_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.alarm_beacon_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("alarm_beacon_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Illumination Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.light_radius_uu, 64.0..=1024.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Brightness:");
            if ui.add(egui::Slider::new(&mut cfg.light_brightness, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Light Hue:");
            if ui.add(egui::Slider::new(&mut cfg.light_hue, 0..=255)).changed() { changed = true; }
            ui.end_row();

            ui.label("Strobe Frequency (Hz):");
            if ui.add(egui::Slider::new(&mut cfg.strobe_frequency_hz, 0.0..=20.0).suffix(" Hz")).changed() { changed = true; }
            ui.end_row();

            ui.label("Rotor Angular Velocity:");
            if ui.add(egui::Slider::new(&mut cfg.rotor_rpm, 0.0..=300.0).suffix(" RPM")).changed() { changed = true; }
            ui.end_row();

            ui.label("Sound Volume (dB):");
            if ui.add(egui::Slider::new(&mut cfg.sound_volume_db, 0.0..=120.0).suffix(" dB")).changed() { changed = true; }
            ui.end_row();

            ui.label("Alarm Sound Asset:");
            if ui.text_edit_singleline(&mut cfg.alarm_sound).changed() { changed = true; }
            ui.end_row();

            ui.label("Trigger Event Name:");
            if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            ui.end_row();

            ui.label("Beacon Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.alarm_beacon_settings.active_config = cfg.clone();
        state.recalculate_alarm_beacon_preview();
        state.history.push(crate::history::Action::UpdateAlarmBeaconConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Alarm Beacon Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.alarm_beacon_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Alarm Siren Beacon Validated").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Stroboscopic flash rate within safe accessibility bounds, audio siren connected, and trigger event wired.");
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
