// crates/ut99-editor/src/gui/arena_rules_dialog.rs
//
// Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
// Native 64-bit mathematical modeling, mutator conflict resolution,
// weapon-to-player ratio telemetry, and tournament competitive balance auditing.

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ArenaRulesArchetype, GameTypeProfile, TournamentMutator};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_arena_rules_dialog {
        return;
    }

    let mut open = state.show_arena_rules_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Arena Match Rules & Tournament Mutator Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_arena_rules_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Arena Archetypes:").strong());
        let archetypes = [
            ArenaRulesArchetype::ClassicProTournament1v1,
            ArenaRulesArchetype::InstagibShockShowdown,
            ArenaRulesArchetype::RocketArenaHeavySiege,
            ArenaRulesArchetype::LowGravSniperExtravaganza,
            ArenaRulesArchetype::TeamCaptureTheFlagMasters,
        ];
        for arch in archetypes {
            let is_sel = state.arena_rules_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_arena_rules_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.arena_rules_settings.show_pickup_telemetry_viewport, "Pickup Telemetry");
        ui.checkbox(&mut state.arena_rules_settings.show_spawn_protection_viewport, "Spawn Protection");
        ui.separator();
        if ui.button("🔄 Recalculate Preview & World Weapons").clicked() {
            state.recalculate_arena_rules_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Visual Readiness Dashboard & Mutator Grid
        columns[0].vertical(|ui| {
            ui.heading("Competitive Balance Telemetry & Readiness");
            render_telemetry_dashboard(ui, state);

            ui.add_space(8.0);
            ui.heading("Tournament Mutator Matrix");
            render_mutator_matrix(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Rules & Mutators to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Match Rules & Physical Parameters, Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Match Rules & Physics Modifiers");
            render_rules_parameters_panel(ui, state);

            ui.add_space(8.0);
            ui.heading("Tournament Safety & Conflict Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_telemetry_dashboard(ui: &mut egui::Ui, state: &mut EditorState) {
    let metrics = state.arena_rules_settings.cached_metrics.clone();
    let weapons = state.arena_rules_settings.world_weapon_count;
    let ammo = state.arena_rules_settings.world_ammo_count;

    let avail_w = ui.available_width().max(200.0);
    let target_h = 160.0;

    let (response, painter) = ui.allocate_painter(Vec2::new(avail_w, target_h), egui::Sense::hover());
    let rect = response.rect;

    // Card background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(20, 24, 32));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 65, 85)), StrokeKind::Middle);

    // Readiness Gauge Bar
    let score = metrics.tournament_readiness_score;
    let gauge_col = if score >= 85.0 {
        Color32::from_rgb(50, 220, 110) // Emerald green
    } else if score >= 60.0 {
        Color32::from_rgb(230, 180, 50)  // Golden amber
    } else {
        Color32::from_rgb(230, 60, 60)   // Warning crimson
    };

    let bar_rect = Rect::from_min_size(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 36.0),
        Vec2::new(avail_w - 28.0, 16.0),
    );
    painter.rect_filled(bar_rect, 3.0, Color32::from_rgb(35, 42, 54));

    let fill_w = (bar_rect.width() * (score / 100.0)).clamp(0.0, bar_rect.width());
    let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_w, bar_rect.height()));
    painter.rect_filled(fill_rect, 3.0, gauge_col);

    painter.text(
        Pos2::new(rect.min.x + 14.0, rect.min.y + 14.0),
        egui::Align2::LEFT_TOP,
        format!("Tournament Readiness Score: {:.1}%", score),
        egui::FontId::proportional(14.0),
        Color32::from_rgb(220, 230, 245),
    );

    // Grid telemetry below bar
    let y_start = rect.min.y + 68.0;
    let stats = [
        format!("Weapons in World: {}", weapons),
        format!("Ammo Pickups: {}", ammo),
        format!("Weapons / Player: {:.2}", metrics.weapons_per_player),
        format!("Ammo / Weapon Ratio: {:.2}", metrics.ammo_to_weapon_ratio),
        format!("Est. Engagement TTK: {:.1}s", metrics.estimated_first_engagement_ttk_secs),
        format!("Active Mutators: {}", metrics.active_mutator_count),
    ];

    let col_w = (avail_w - 28.0) * 0.5;
    for (i, stat) in stats.iter().enumerate() {
        let col = i % 2;
        let row = i / 2;
        let p = Pos2::new(rect.min.x + 14.0 + (col as f32 * col_w), y_start + (row as f32 * 24.0));
        painter.text(p, egui::Align2::LEFT_TOP, stat, egui::FontId::proportional(12.0), Color32::from_rgb(180, 195, 215));
    }
}

fn render_mutator_matrix(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label("Search Mutators:");
        ui.text_edit_singleline(&mut state.arena_rules_settings.filter_mutator_query);
        if ui.button("Clear").clicked() {
            state.arena_rules_settings.filter_mutator_query.clear();
        }
    });

    let filter = state.arena_rules_settings.filter_mutator_query.to_lowercase();
    let all_mutators = [
        TournamentMutator::InstagibDM,
        TournamentMutator::RocketArena,
        TournamentMutator::FlakArena,
        TournamentMutator::ShockArena,
        TournamentMutator::SniperArena,
        TournamentMutator::LowGrav,
        TournamentMutator::NoPowerups,
        TournamentMutator::FatBoy,
        TournamentMutator::Chameleon,
        TournamentMutator::Relics,
    ];

    egui::Grid::new("mutators_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            for mutator in all_mutators {
                let name = mutator.display_name();
                if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                    continue;
                }

                let is_active = state.arena_rules_settings.active_config.active_mutators.contains(&mutator);
                let is_weapon_arena = mutator.is_weapon_arena();

                let mut checked = is_active;
                let text = if is_weapon_arena {
                    egui::RichText::new(format!("⚔ {}", name)).color(Color32::from_rgb(255, 185, 80))
                } else {
                    egui::RichText::new(name)
                };

                if ui.checkbox(&mut checked, text).changed() {
                    state.toggle_arena_mutator(mutator);
                }

                let tag = if is_weapon_arena { "Weapon Arena" } else { "Gameplay Modifier" };
                ui.label(egui::RichText::new(tag).size(11.0).color(Color32::GRAY));
                ui.end_row();
            }
        });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = state.arena_rules_settings.active_config.clone();

    ui.group(|ui| {
        ui.label(format!("Target GameType: {}", cfg.game_type.display_name()));
        ui.label(format!("FragLimit: {} | TimeLimit: {}m | MaxPlayers: {}", cfg.frag_limit, cfg.time_limit_mins, cfg.max_players));
        ui.label(format!("Active Mutator Actors to Spawn: {}", cfg.active_mutators.len()));

        ui.add_space(4.0);
        if ui.button(egui::RichText::new("💾 Commit Arena Rules & Mutators to Level").strong()).clicked() {
            state.commit_arena_rules_to_world();
        }
    });
}

fn render_rules_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;
    let old_cfg = state.arena_rules_settings.active_config.clone();
    let mut cfg = old_cfg.clone();

    egui::Grid::new("arena_rules_params_grid")
        .num_columns(2)
        .spacing([12.0, 6.0])
        .show(ui, |ui| {
            // GameType Profile Dropdown
            ui.label("Game Type Profile:");
            let game_types = [
                GameTypeProfile::DeathMatchPlus,
                GameTypeProfile::TeamGamePlus,
                GameTypeProfile::CTFGame,
                GameTypeProfile::Domination,
                GameTypeProfile::Assault,
            ];
            egui::ComboBox::from_id_salt("gametype_profile_select")
                .selected_text(cfg.game_type.display_name())
                .show_ui(ui, |ui| {
                    for gt in game_types {
                        if ui.selectable_value(&mut cfg.game_type, gt, gt.display_name()).clicked() {
                            changed = true;
                        }
                    }
                });
            ui.end_row();

            // Frag Limit / Capture Limit
            ui.label("Frag Limit / Goal Score:");
            if ui.add(egui::DragValue::new(&mut cfg.frag_limit).range(0..=200)).changed() {
                changed = true;
            }
            ui.end_row();

            // Time Limit (minutes)
            ui.label("Time Limit (Minutes):");
            if ui.add(egui::DragValue::new(&mut cfg.time_limit_mins).range(0..=60)).changed() {
                changed = true;
            }
            ui.end_row();

            // Max Players
            ui.label("Max Players:");
            if ui.add(egui::Slider::new(&mut cfg.max_players, 2..=32)).changed() {
                changed = true;
            }
            ui.end_row();

            // Spawn Protection (seconds)
            ui.label("Spawn Protection (Seconds):");
            if ui.add(egui::Slider::new(&mut cfg.spawn_protection_secs, 0.0..=5.0).suffix(" s")).changed() {
                changed = true;
            }
            ui.end_row();

            // Weapon Respawn Delay
            ui.label("Weapon Respawn Delay:");
            if ui.add(egui::Slider::new(&mut cfg.weapon_respawn_secs, 0.0..=60.0).suffix(" s")).changed() {
                changed = true;
            }
            ui.end_row();

            // Armor Respawn Delay
            ui.label("Armor Respawn Delay:");
            if ui.add(egui::Slider::new(&mut cfg.armor_respawn_secs, 0.0..=120.0).suffix(" s")).changed() {
                changed = true;
            }
            ui.end_row();

            // Powerup Respawn Delay
            ui.label("Powerup Respawn Delay:");
            if ui.add(egui::Slider::new(&mut cfg.powerup_respawn_secs, 0.0..=180.0).suffix(" s")).changed() {
                changed = true;
            }
            ui.end_row();

            // Air Control
            ui.label("Air Control Multiplier:");
            if ui.add(egui::Slider::new(&mut cfg.air_control, 0.05..=1.0).step_by(0.05)).changed() {
                changed = true;
            }
            ui.end_row();

            // World Gravity
            ui.label("World Gravity (uu/s²):");
            if ui.add(egui::DragValue::new(&mut cfg.gravity_uu_s2).range(0.0..=2000.0).speed(10.0)).changed() {
                changed = true;
            }
            ui.end_row();

            // Friendly Fire Damage Scale
            ui.label("Friendly Fire Damage Scale:");
            if ui.add(egui::Slider::new(&mut cfg.friendly_fire_scale, 0.0..=1.0).step_by(0.1)).changed() {
                changed = true;
            }
            ui.end_row();

            // Tournament Mode Toggle
            ui.label("Tournament Mode Enforcement:");
            if ui.checkbox(&mut cfg.tournament_mode, "Enforce Strict Tourney Flags").changed() {
                changed = true;
            }
            ui.end_row();

            // Tag
            ui.label("Rules Tag / Identification:");
            if ui.text_edit_singleline(&mut cfg.tag).changed() {
                changed = true;
            }
            ui.end_row();
        });

    if changed {
        state.arena_rules_settings.active_config = cfg.clone();
        state.recalculate_arena_rules_preview();
        state.history.push(crate::history::Action::UpdateArenaRulesConfig {
            old_config: old_cfg,
            new_config: cfg,
            description: "Modified Arena Match Rules".to_string(),
        });
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.arena_rules_settings.cached_hazards;

    if hazards.is_empty() {
        ui.group(|ui| {
            ui.label(egui::RichText::new("✅ 100% Competitive Tournament Ready").color(Color32::from_rgb(50, 220, 110)).strong());
            ui.label("No mutator conflicts, infinite match deadlocks, or weapon starvation detected.");
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
