// crates/ut99-editor/src/gui/ctf_studio_dialog.rs
//
// Interactive Capture The Flag (CTF) Flag Base & Transit Routes Studio
// Flag base placement, flag-to-flag distance metrics, sprint/translocator transit times,
// route symmetry auditing, defense radius visualization, and procedural stepped flag pedestal brush.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{CTFTeam, CtfBaseArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_ctf_base_dialog {
        return;
    }

    let mut open = state.show_ctf_base_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Capture The Flag Base & Transit Routes Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_ctf_base_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("CTF Presets:").strong());
        let archetypes = [
            CtfBaseArchetype::FaceMonolithBase,
            CtfBaseArchetype::NovemberSubmarineDeck,
            CtfBaseArchetype::CoretCryoChamber,
            CtfBaseArchetype::GauntletFortressDais,
            CtfBaseArchetype::DrearyIndustrialGutter,
        ];
        for arch in archetypes {
            let is_sel = state.ctf_base_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_ctf_base_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.ctf_base_settings.show_pedestal_viewport, "Stepped Dais Wireframe");
        ui.checkbox(&mut state.ctf_base_settings.show_defense_radius_viewport, "Defense Perimeter Ring");
        ui.checkbox(&mut state.ctf_base_settings.show_transit_route_viewport, "Transit Route Vector");
        ui.separator();
        if ui.button("🔄 Recalculate CTF Telemetry & Audit").clicked() {
            state.recalculate_ctf_base_preview();
        }
    });
    ui.separator();

    // 3. Responsive Two-Column Layout (collapses vertically below 640px)
    crate::gui::modal::responsive_two_columns(
        ui,
        state,
        640.0,
        0.48,
        |ui, state| {
            // Left Column: Telemetry & Coordinates
            ui.heading("CTF Route & Transit Telemetry");
            render_ctf_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Flag Base to World");
            render_commit_panel(ui, state);
        },
        |ui, state| {
            // Right Column: Parameters & Safety Audit
            ui.heading("Base Parameters & Team Color");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Competitive Balance & Route Audit");
            render_safety_audit_card(ui, state);
        },
    );
}

fn render_ctf_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.ctf_base_settings.cached_metrics;
    let cfg = &state.ctf_base_settings.active_config;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 20, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(70, 50, 95)), StrokeKind::Middle);

    // Title banner
    let team_col = match cfg.team {
        CTFTeam::Red => Color32::from_rgb(255, 90, 90),
        CTFTeam::Blue => Color32::from_rgb(90, 160, 255),
        CTFTeam::Green => Color32::from_rgb(90, 230, 110),
        CTFTeam::Gold => Color32::from_rgb(255, 210, 50),
    };

    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("CTF FLAG BASE: {} | {}", cfg.team.display_name(), cfg.archetype.display_name()),
        egui::FontId::proportional(13.0),
        team_col,
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let flag_dist_str = if metrics.flag_to_flag_distance_uu < 9000.0 {
        format!("{:.0} uu", metrics.flag_to_flag_distance_uu)
    } else {
        "No Opposing Flag".to_string()
    };

    let runner_str = if metrics.runner_transit_time_secs > 0.0 {
        format!("{:.1}s (400 uu/s)", metrics.runner_transit_time_secs)
    } else {
        "N/A".to_string()
    };

    let trans_str = if metrics.translocator_transit_time_secs > 0.0 {
        format!("{:.1}s (800 uu/s)", metrics.translocator_transit_time_secs)
    } else {
        "N/A".to_string()
    };

    let stats = [
        format!("Flag-to-Flag Dist: {}", flag_dist_str),
        format!("Runner Sprint Time: {}", runner_str),
        format!("Translocator Route Time: {}", trans_str),
        format!("Auto-Return Timeout: {:.1}s", cfg.flag_return_timeout_secs),
        format!("Defense Zone Area: {:.2} M uu²", metrics.defense_coverage_area_sq_uu / 1_000_000.0),
        format!("Total Flag Bases in Map: {}", metrics.total_flag_bases_in_level),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(235, 230, 245));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.ctf_base_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Flag Base Location:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Base to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.ctf_base_settings.active_config = cfg.clone();
        state.recalculate_ctf_base_preview();
        state.history.push(crate::history::Action::UpdateCtfBaseConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Flag Base Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.ctf_base_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Team: {}", cfg.team.display_name()));
        ui.label(format!("Actor Class: {}", cfg.team.actor_class_name()));
        ui.label(format!("Defense Radius: {:.0} uu", cfg.defense_radius_uu));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Flag Dais & Base Actor to World").strong()).clicked() {
            state.commit_ctf_base_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.ctf_base_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("ctf_base_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Team Assignment:");
            egui::ComboBox::from_id_salt("ctf_team_selector")
                .selected_text(cfg.team.display_name())
                .show_ui(ui, |ui| {
                    let teams = [CTFTeam::Red, CTFTeam::Blue, CTFTeam::Green, CTFTeam::Gold];
                    for t in teams {
                        if ui.selectable_value(&mut cfg.team, t, t.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Defense Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.defense_radius_uu, 300.0..=2500.0).suffix(" uu")).changed() { changed = true; }
            ui.end_row();

            ui.label("Flag Auto-Return Timeout:");
            if ui.add(egui::Slider::new(&mut cfg.flag_return_timeout_secs, 10.0..=60.0).suffix("s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_radius, 32.0..=160.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Pedestal Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.pedestal_height, 8.0..=64.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Base Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.ctf_base_settings.active_config = cfg.clone();
        state.recalculate_ctf_base_preview();
        state.history.push(crate::history::Action::UpdateCtfBaseConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Flag Base Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.ctf_base_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ CTF Match & Route Balance Optimal").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("Balanced transit times, clean line of sight, and complete opposing team presence.");
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
