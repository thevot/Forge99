// crates/ut99-editor/src/gui/elevator_dialog.rs
//
// Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
// Multi-floor elevators, keyframe height offsets, ascent/descent velocity, stay time,
// bot LiftCenter & LiftExit pairing, crush damage encroachment, and procedural elevator platform brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ElevatorArchetype, EncroachBehavior};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_elevator_dialog {
        return;
    }

    let mut open = state.show_elevator_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Elevator, LiftCenter/LiftExit & Vertical Transit Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_elevator_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Elevator Presets:").strong());
        let archetypes = [
            ElevatorArchetype::StandardHydraulicLift,
            ElevatorArchetype::DoubleDeckerExpress,
            ElevatorArchetype::SubterraneanCargoDrop,
            ElevatorArchetype::HighSpeedTowerHoist,
            ElevatorArchetype::GravityPistonShaft,
        ];
        for arch in archetypes {
            let is_sel = state.elevator_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_elevator_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.elevator_settings.show_platform_viewport, "Platform Brush Wireframe");
        ui.checkbox(&mut state.elevator_settings.show_travel_axis_viewport, "Vertical Travel Axis & Keys");
        ui.checkbox(&mut state.elevator_settings.show_bot_waypoints_viewport, "Bot LiftCenter / LiftExits");
        ui.separator();
        if ui.button("🔄 Recalculate Lift Telemetry & Audit").clicked() {
            state.recalculate_elevator_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Telemetry & Placement
        columns[0].vertical(|ui| {
            ui.heading("Elevator Transit Telemetry");
            render_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_placement_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Elevator & Bot Nodes to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Kinematics & Encroachment Parameters");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Safety & Bot Pathing Diagnostics");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.elevator_settings.cached_metrics;
    let cfg = &state.elevator_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 26, 34));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(60, 130, 200)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("ELEVATOR: {} | Tag: '{}'", cfg.archetype.display_name(), cfg.tag),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(120, 210, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Ascent Velocity: {:.0} uu/s", metrics.ascent_velocity_uu_per_sec),
        format!("Round-Trip Period: {:.1}s", metrics.round_trip_period_secs),
        format!("Platform Area: {:.1} k uu²", metrics.platform_area_sq_uu / 1_000.0),
        format!("Travel Displacement: {:.0} uu", cfg.travel_height_uu),
        format!("LiftCenter Node: {}", if metrics.has_lift_center { "WIRED (OK)" } else { "MISSING" }),
        format!("LiftExit Landings: {} wired", metrics.total_lift_exits_wired),
        format!("Encroach Mode: {}", cfg.encroach_behavior.display_name()),
        format!("Total Lifts in Level: {}", metrics.total_elevators_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        let col_text = if stat.contains("MISSING") {
            Color32::from_rgb(255, 90, 90)
        } else {
            Color32::from_rgb(220, 240, 255)
        };
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), col_text);
    }
}

fn render_placement_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.elevator_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Platform Base Position (Key 0):").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.platform_width_uu).prefix("Width: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.platform_length_uu).prefix("Length: ").suffix(" uu")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.platform_thickness_uu).prefix("Thick: ").suffix(" uu")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Lift to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.elevator_settings.active_config = cfg.clone();
        state.recalculate_elevator_preview();
        state.history.push(crate::history::Action::UpdateElevatorConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Elevator Position and Dimensions".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.elevator_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Lift Tag: '{}'", cfg.tag));
        ui.label(format!("Displacement: {:.0} uu in {:.1}s", cfg.travel_height_uu, cfg.move_time_secs));
        ui.label(format!("Crush: {:.0} HP ({:?})", cfg.encroach_damage, cfg.encroach_behavior));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Platform Brush, LiftCenter & LiftExits to Level").strong()).clicked() {
            state.commit_elevator_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.elevator_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("elevator_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Travel Height Z:");
            if ui.add(egui::Slider::new(&mut cfg.travel_height_uu, 64.0..=2048.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Move Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.move_time_secs, 0.5..=15.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Stay Open Time (s):");
            if ui.add(egui::Slider::new(&mut cfg.stay_open_time_secs, 0.5..=10.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Encroach Behavior:");
            egui::ComboBox::from_id_salt("elevator_encroach_selector")
                .selected_text(cfg.encroach_behavior.display_name())
                .show_ui(ui, |ui| {
                    let modes = [
                        EncroachBehavior::StopWhenEncroached,
                        EncroachBehavior::ReturnWhenEncroached,
                        EncroachBehavior::CrushWhenEncroached,
                        EncroachBehavior::IgnoreEncroachment,
                    ];
                    for m in modes {
                        if ui.selectable_value(&mut cfg.encroach_behavior, m, m.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Encroach Damage:");
            if ui.add(egui::Slider::new(&mut cfg.encroach_damage, 0.0..=500.0).suffix(" HP")).changed() { changed = true; }
            ui.end_row();

            ui.label("Shared Lift Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.elevator_settings.active_config = cfg.clone();
        state.recalculate_elevator_preview();
        state.history.push(crate::history::Action::UpdateElevatorConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Elevator Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.elevator_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Elevator Transit & Bot Pathing Valid").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("LiftCenter and LiftExits wired, velocities safe, and stay window sufficient.");
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
