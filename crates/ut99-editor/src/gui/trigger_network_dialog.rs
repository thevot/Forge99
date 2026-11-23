// crates/ut99-editor/src/gui/trigger_network_dialog.rs
//
// Interactive Pressure Plate & Proximity Trigger Network Studio
// Pressure plates, proximity trigger cylinders, tripwires, event-tag network graph resolution,
// orphan event diagnostics, trigger loop detection, and procedural pressure plate frame brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{TriggerActivationType, TriggerNetworkArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_trigger_network_dialog {
        return;
    }

    let mut open = state.show_trigger_network_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Pressure Plate & Proximity Trigger Network Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_trigger_network_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Trigger Network Presets:").strong());
        let archetypes = [
            TriggerNetworkArchetype::PressureChamberDecompressorSwitch,
            TriggerNetworkArchetype::TrapdoorHiddenPitTrigger,
            TriggerNetworkArchetype::ReinforcedSecurityGateTerminal,
            TriggerNetworkArchetype::IntruderAlarmTripwire,
            TriggerNetworkArchetype::GravityGeneratorOverrideConsole,
        ];
        for arch in archetypes {
            let is_sel = state.trigger_network_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_trigger_network_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.trigger_network_settings.show_plate_viewport, "Pressure Plate Frame");
        ui.checkbox(&mut state.trigger_network_settings.show_activation_volume_viewport, "Activation Volume Cylinder");
        ui.checkbox(&mut state.trigger_network_settings.show_event_wiring_lines_viewport, "Event-to-Tag Link Lines");
        ui.separator();
        if ui.button("🔄 Recalculate Network Wiring & Audit").clicked() {
            state.recalculate_trigger_network_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Trigger & Event Network Telemetry");
            render_network_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Trigger to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Logic & Wiring Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Safety & Network Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_network_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.trigger_network_settings.cached_metrics;
    let cfg = &state.trigger_network_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 110, 180)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("TRIGGER: {} | Event: '{}'", cfg.archetype.display_name(), cfg.event_name),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(110, 200, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Wired Recipient Actors: {}", metrics.total_target_actors_wired),
        format!("Closest Recipient Link: {:.1} uu", metrics.network_link_distance_uu),
        format!("Activation Volume: {:.1} k uu³", metrics.trigger_volume_cu_uu / 1_000.0),
        format!("Retrigger Delay: {:.1}s", cfg.retrigger_delay_secs),
        format!("Single Use Only: {}", if cfg.is_single_use { "YES" } else { "NO" }),
        format!("Team Filter: {}", match cfg.team_filter { 1 => "Red", 2 => "Blue", 3 => "Green", 4 => "Gold", _ => "Any" }),
        format!("Trigger Mode: {}", cfg.trigger_type.display_name()),
        format!("Total Triggers in Map: {}", metrics.total_triggers_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(220, 240, 255));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.trigger_network_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Trigger Position & Bounding Extents:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::Slider::new(&mut cfg.activation_radius_uu, 16.0..=500.0).prefix("Radius: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::Slider::new(&mut cfg.activation_height_uu, 8.0..=256.0).prefix("Height: ").suffix(" uu")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Trigger to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.trigger_network_settings.active_config = cfg.clone();
        state.recalculate_trigger_network_preview();
        state.history.push(crate::history::Action::UpdateTriggerNetworkConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Trigger Position and Bounds".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.trigger_network_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Mechanism: {}", cfg.trigger_type.display_name()));
        ui.label(format!("Event Broadcast: '{}'", cfg.event_name));
        ui.label(format!("Retrigger Delay: {:.1}s", cfg.retrigger_delay_secs));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Plate Frame & Trigger Actor to World").strong()).clicked() {
            state.commit_trigger_network_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.trigger_network_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("trigger_network_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Trigger Type:");
            egui::ComboBox::from_id_salt("trigger_type_selector")
                .selected_text(cfg.trigger_type.display_name())
                .show_ui(ui, |ui| {
                    let types = [
                        TriggerActivationType::PlayerProximityRadius,
                        TriggerActivationType::DepressibleFloorPad,
                        TriggerActivationType::BeamBreakInfraredTripwire,
                        TriggerActivationType::UseKeyInteractiveConsole,
                        TriggerActivationType::ShootDamageTrigger,
                    ];
                    for t in types {
                        if ui.selectable_value(&mut cfg.trigger_type, t, t.display_name()).clicked() {
                            let (r, h) = t.default_radius_and_height();
                            cfg.activation_radius_uu = r;
                            cfg.activation_height_uu = h;
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Event Name (Wired Target):");
            if ui.text_edit_singleline(&mut cfg.event_name).changed() { changed = true; }
            ui.end_row();

            ui.label("Retrigger Delay (s):");
            if ui.add(egui::Slider::new(&mut cfg.retrigger_delay_secs, 0.1..=30.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Single Use Only:");
            if ui.checkbox(&mut cfg.is_single_use, "Fire once and destroy").changed() { changed = true; }
            ui.end_row();

            ui.label("Team Filter:");
            egui::ComboBox::from_id_salt("trigger_team_filter_selector")
                .selected_text(match cfg.team_filter { 1 => "Red Team Only", 2 => "Blue Team Only", 3 => "Green Team Only", 4 => "Gold Team Only", _ => "Any Team" })
                .show_ui(ui, |ui| {
                    let teams = [(0, "Any Team"), (1, "Red Team Only"), (2, "Blue Team Only"), (3, "Green Team Only"), (4, "Gold Team Only")];
                    for (t_id, t_name) in teams {
                        if ui.selectable_value(&mut cfg.team_filter, t_id, t_name).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Instigator Message:");
            if ui.text_edit_singleline(&mut cfg.message).changed() { changed = true; }
            ui.end_row();

            ui.label("Mount Plate Dimensions:");
            ui.horizontal(|ui| {
                if ui.add(egui::DragValue::new(&mut cfg.plate_width_uu).prefix("W: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.plate_length_uu).prefix("L: ")).changed() { changed = true; }
                if ui.add(egui::DragValue::new(&mut cfg.plate_thickness_uu).prefix("H: ")).changed() { changed = true; }
            });
            ui.end_row();
        });

    if changed {
        state.trigger_network_settings.active_config = cfg.clone();
        state.recalculate_trigger_network_preview();
        state.history.push(crate::history::Action::UpdateTriggerNetworkConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Trigger Network Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.trigger_network_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Trigger Network Wiring Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Recipient actors found, no infinite loops detected, and retrigger delay is fair.");
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
