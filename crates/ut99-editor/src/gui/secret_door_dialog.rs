// crates/ut99-editor/src/gui/secret_door_dialog.rs
//
// Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
// Concealed trigger mechanisms, mover displacement keyframes, passage dimensions,
// anti-entrapment return triggers, and procedural secret door slab brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{SecretConcealmentType, SecretDoorArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_secret_door_dialog {
        return;
    }

    let mut open = state.show_secret_door_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Secret Door, Concealed Passage & Mover Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_secret_door_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Secret Door Presets:").strong());
        let archetypes = [
            SecretDoorArchetype::FlushWallSlidingPanel,
            SecretDoorArchetype::RotatingPivotingBookshelf,
            SecretDoorArchetype::SunkenFloorTrapdoor,
            SecretDoorArchetype::DisplacedStoneSlab,
            SecretDoorArchetype::HighTechCamouflagedLocker,
        ];
        for arch in archetypes {
            let is_sel = state.secret_door_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_secret_door_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.secret_door_settings.show_door_mesh_viewport, "Door Slab Wireframe (Key 0)");
        ui.checkbox(&mut state.secret_door_settings.show_displacement_path_viewport, "Displacement Trajectory & Open Ghost (Key 1)");
        ui.checkbox(&mut state.secret_door_settings.show_trigger_radius_viewport, "Trigger Activation Radius");
        ui.separator();
        if ui.button("🔄 Recalculate Secrecy & Audit").clicked() {
            state.recalculate_secret_door_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Concealment & Transit Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Door Mover & Trigger to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Mover Kinematics & Timing Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Entrapment & Safety Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.secret_door_settings.cached_metrics;
    let cfg = &state.secret_door_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(28, 20, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(180, 80, 220)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("SECRET PASSAGE: {}", cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(230, 160, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Concealment Stealth: {:.0}%", metrics.stealth_rating_pct),
        format!("Trigger Mechanism: {}", cfg.concealment.display_name()),
        format!("Passage Cross Section: {:.0} uu²", metrics.passage_cross_section_sq_uu),
        format!("Total Transit Cycle: {:.2}s", metrics.transit_duration_secs),
        format!("Stay Open Time: {:.1}s", cfg.stay_open_time_secs),
        format!("Open Travel Time: {:.1}s", cfg.open_time_secs),
        format!("Internal Return Trigger: {}", if cfg.has_internal_exit_trigger { "Present (Safe)" } else { "MISSING (Hazard)" }),
        format!("Total Secret Doors in Level: {}", metrics.total_secret_doors_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("MISSING") {
            Color32::from_rgb(255, 90, 90)
        } else {
            Color32::from_rgb(245, 230, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.secret_door_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Door Placement Position:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.label(egui::RichText::new("🚀 Mover Travel Displacement (Key 0 -> Key 1):").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.travel_displacement.x).prefix("ΔX: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.travel_displacement.y).prefix("ΔY: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.travel_displacement.z).prefix("ΔZ: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Door to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.secret_door_settings.active_config = cfg.clone();
        state.recalculate_secret_door_preview();
        state.history.push(crate::history::Action::UpdateSecretDoorConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Secret Door Placement and Displacement".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.secret_door_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Size: {:.0}x{:.0} uu | Stay: {:.1}s", cfg.door_width_uu, cfg.door_height_uu, cfg.stay_open_time_secs));
        ui.label(format!("Tag: '{}' | Event: '{}'", cfg.tag, cfg.event));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Secret Door & Trigger to Level").strong()).clicked() {
            state.commit_secret_door_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.secret_door_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("secret_door_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Concealment Mechanism:");
            egui::ComboBox::from_id_salt("secret_conceal_combo")
                .selected_text(cfg.concealment.display_name())
                .show_ui(ui, |ui| {
                    let items = [
                        SecretConcealmentType::BumpPressureSensitive,
                        SecretConcealmentType::ShootImpactTrigger,
                        SecretConcealmentType::ProximityHiddenRadius,
                        SecretConcealmentType::InteractiveUseKeypad,
                    ];
                    for item in items {
                        if ui.selectable_value(&mut cfg.concealment, item, item.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Door Width (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_width_uu, 32.0..=256.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Door Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_height_uu, 64.0..=300.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Door Thickness (uu):");
            if ui.add(egui::Slider::new(&mut cfg.door_thickness_uu, 8.0..=64.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Stay Open Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.stay_open_time_secs, 0.5..=15.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Opening Travel Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.open_time_secs, 0.2..=5.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Internal Exit Trigger:");
            if ui.checkbox(&mut cfg.has_internal_exit_trigger, "Enable Safe Anti-Entrapment Trigger").changed() { changed = true; }
            ui.end_row();

            ui.label("Encroach Behavior:");
            if ui.text_edit_singleline(&mut cfg.encroach_behavior).changed() { changed = true; }
            ui.end_row();

            ui.label("Mover Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();

            ui.label("Activation Event:");
            if ui.text_edit_singleline(&mut cfg.event).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.secret_door_settings.active_config = cfg.clone();
        state.recalculate_secret_door_preview();
        state.history.push(crate::history::Action::UpdateSecretDoorConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Secret Door Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.secret_door_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Secret Door Configuration Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Internal exit trigger present, safe encroachment behavior, and adequate StayOpen window.");
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
