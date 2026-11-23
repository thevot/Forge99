// crates/ut99-editor/src/gui/powerup_control_dialog.rs
//
// Interactive Tournament Powerup Control & Relic Altar Studio
// Multipliers, durations, respawn clock cadence, contention hotspot scoring,
// super-powerup stacking hazard detection, and procedural powerup altar generation.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{PowerupArchetype, TournamentPowerupType};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_powerup_control_dialog {
        return;
    }

    let mut open = state.show_powerup_control_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Tournament Powerup Control & Relic Altar Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_powerup_control_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Powerup Presets:").strong());
        let archetypes = [
            PowerupArchetype::Deck16HighLedgeUDamage,
            PowerupArchetype::LiandriCoreCrossfireUDamage,
            PowerupArchetype::PhobosGravityJumpBoots,
            PowerupArchetype::AgonySacredRelicAltar,
            PowerupArchetype::GothicDualPowerupClash,
        ];
        for arch in archetypes {
            let is_sel = state.powerup_control_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_powerup_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.powerup_control_settings.show_altar_viewport, "Altar Brush Wireframe");
        ui.checkbox(&mut state.powerup_control_settings.show_containment_cylinder_viewport, "Containment Cylinder & Vortex");
        ui.checkbox(&mut state.powerup_control_settings.show_contention_rays_viewport, "Contention Sightlines");
        ui.separator();
        if ui.button("🔄 Recalculate Contention Dynamics & Audit").clicked() {
            state.recalculate_powerup_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Contention Dashboard & Coordinates
        columns[0].vertical(|ui| {
            ui.heading("Contention Dynamics & Respawn Telemetry");
            render_contention_telemetry_canvas(ui, state);

            ui.add_space(8.0);
            render_positions_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Powerup Altar to World");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Powerup & Relic Configuration");
            render_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Super-Stacking & Contention Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_contention_telemetry_canvas(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = &state.powerup_control_settings.cached_metrics;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(22, 18, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(75, 45, 95)), StrokeKind::Middle);

    // Title banner
    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 12.0),
        egui::Align2::LEFT_TOP,
        format!("POWERUP CADENCE: {:.0}s DURATION | {:.0}s RESPAWN", metrics.active_duration_secs, metrics.respawn_delay_secs),
        egui::FontId::proportional(13.0),
        Color32::from_rgb(215, 100, 255),
    );

    // Telemetry Statistics
    let y_start = rect.min.y + 36.0;
    let stats = [
        format!("Match Uptime: {:.1}%", metrics.powerup_uptime_pct),
        format!("Contention Hotspot Rating: {:.0} / 100", metrics.contention_hotspot_rating),
        format!("Dist to Nearest Weapon: {:.0} uu", metrics.nearest_weapon_distance_uu),
        format!("Dist to Nearest Shield: {:.0} uu", metrics.nearest_shield_distance_uu),
        format!("Dist to Other Powerups: {:.0} uu", metrics.nearest_powerup_distance_uu),
        format!("Jump Skill Required: {}", if state.powerup_control_settings.active_config.b_require_jump_skills { "Yes (High Perch)" } else { "No (Ground Level)" }),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 20.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(220, 200, 235));
    }
}

fn render_positions_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.powerup_control_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    ui.group(|ui| {
        ui.label(egui::RichText::new("📍 Powerup Altar Coordinates:").strong());
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut cfg.position.x).prefix("X: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.y).prefix("Y: ")).changed() { changed = true; }
            if ui.add(egui::DragValue::new(&mut cfg.position.z).prefix("Z: ")).changed() { changed = true; }
        });

        if ui.button("📍 Snap Altar to Builder Brush").clicked() {
            cfg.position = state.builder_brush.location;
            changed = true;
        }
    });

    if changed {
        state.powerup_control_settings.active_config = cfg.clone();
        state.recalculate_powerup_preview();
        state.history.push(crate::history::Action::UpdatePowerupControlConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Updated Powerup Altar Coordinates".to_string(),
        });
    }
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.powerup_control_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Archetype: {}", cfg.archetype.display_name()));
        ui.label(format!("Powerup: {}", cfg.powerup_type.display_name()));
        ui.label(format!("Respawn Cadence: {:.0} seconds", cfg.respawn_delay_secs));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Powerup Altar & Actor to World").strong()).clicked() {
            state.commit_powerup_to_world();
        }
    });
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.powerup_control_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("powerup_control_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            ui.label("Powerup / Relic:");
            egui::ComboBox::from_id_salt("powerup_type_selector")
                .selected_text(cfg.powerup_type.display_name())
                .show_ui(ui, |ui| {
                    let powerups = [
                        TournamentPowerupType::UDamage,
                        TournamentPowerupType::Invisibility,
                        TournamentPowerupType::JumpBoots,
                        TournamentPowerupType::RelicStrength,
                        TournamentPowerupType::RelicRegen,
                        TournamentPowerupType::RelicDefense,
                        TournamentPowerupType::RelicSpeed,
                        TournamentPowerupType::RelicRedemption,
                        TournamentPowerupType::RelicDeath,
                    ];
                    for p in powerups {
                        if ui.selectable_value(&mut cfg.powerup_type, p, p.display_name()).clicked() {
                            cfg.respawn_delay_secs = p.default_respawn_delay_secs();
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            ui.label("Respawn Delay (s):");
            if ui.add(egui::Slider::new(&mut cfg.respawn_delay_secs, 10.0..=180.0).suffix(" s")).changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Radius (uu):");
            if ui.add(egui::Slider::new(&mut cfg.altar_radius, 24.0..=80.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Height (uu):");
            if ui.add(egui::Slider::new(&mut cfg.altar_height, 8.0..=36.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Containment Cylinder Height:");
            if ui.add(egui::Slider::new(&mut cfg.containment_beam_height, 50.0..=250.0)).changed() { changed = true; }
            ui.end_row();

            ui.label("Requires Jump Skills (Vantage):");
            if ui.checkbox(&mut cfg.b_require_jump_skills, "Elevated High Perch").changed() { changed = true; }
            ui.end_row();

            ui.label("Altar Identification Tag:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() { changed = true; }
            ui.end_row();
        });

    if changed {
        state.powerup_control_settings.active_config = cfg.clone();
        state.recalculate_powerup_preview();
        state.history.push(crate::history::Action::UpdatePowerupControlConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Powerup Control Parameters".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.powerup_control_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ Powerup Placement Balanced & Safe").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No super-stacking imbalance with ShieldBelt, excessive clustering, or fast respawn exploits detected.");
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
