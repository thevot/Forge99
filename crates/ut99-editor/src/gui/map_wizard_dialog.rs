use crate::editor_state::{
    EditorState, LevelAuditFreshness, WizardMinimapDragState, WizardMinimapSelection, WizardMinimapTool,
    WizardResizeHandle,
};
use egui::{Color32, Pos2, Rect, RichText, Sense, Stroke, Vec2};
use ut99_bsp::{
    preview_wizard_topology, ut99_light_to_rgb, CtfDaisStyle, DominationDaisStyle, DominationNamingStyle, IssueSeverity,
    WizardArchetype, WizardArchetypeCategory, WizardGameMode, WizardItemDensity, WizardItemQuotaConfig,
    WizardLayoutStrategy, WizardMapScale, WizardPowerupPreset, WizardResolvedPalette, WizardRoomShape, WizardSymmetryMode,
    WizardThemePreset, WizardWeaponPreset, CLASSIC_UNREAL_WEAPONS, STOCK_POWERUPS_AND_PICKUPS,
    STOCK_TOURNAMENT_WEAPONS, UT99_MUSIC_TRACKS, WizardFogPreset, WizardRoomEnvironment,
    WizardArchitecturalDetail, WizardTeleportIssueSeverity,
};

fn show_wizard_audit_status(ui: &mut egui::Ui, state: &EditorState) {
    match state.level_audit_freshness() {
        LevelAuditFreshness::NotRun => { ui.colored_label(Color32::LIGHT_GRAY, "Level Doctor: audit not yet run"); }
        LevelAuditFreshness::Stale => { ui.colored_label(Color32::YELLOW, "Level Doctor: report is out of date; run audit again"); }
        LevelAuditFreshness::Current => {
            let errors = state.map_audit_issues.iter().filter(|i| i.severity == IssueSeverity::Error).count();
            let warnings = state.map_audit_issues.iter().filter(|i| i.severity == IssueSeverity::Warning).count();
            let color = if errors > 0 { Color32::LIGHT_RED } else { Color32::from_rgb(100, 220, 140) };
            ui.colored_label(color, format!("Level Doctor static report: {errors} error(s), {warnings} warning(s)"));
        }
    }
}

pub fn show_map_wizard_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_map_wizard_dialog {
        return;
    }

    let mut is_open = state.show_map_wizard_dialog;

    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 32.0).max(520.0);
    let max_h = (screen.height() - 48.0).max(380.0);
    let default_w = 980.0_f32.min(max_w);
    let default_h = 680.0_f32.min(max_h);
    let min_w = 640.0_f32.min(max_w);
    let min_h = 420.0_f32.min(max_h);

    egui::Window::new("★ New Valid UT99 Map Generator Wizard")
        .open(&mut is_open)
        .resizable(true)
        .max_size([max_w, max_h])
        .default_size([default_w, default_h])
        .min_width(min_w)
        .min_height(min_h)
        .show(ctx, |ui| {
            // Top Header & One-Click Tournament Presets
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("⚡ Quick Presets:")
                        .strong()
                        .color(Color32::from_rgb(255, 215, 90)),
                );
                if ui
                    .button(
                        RichText::new("★ 100% Playtest Ready")
                            .color(Color32::from_rgb(110, 255, 140))
                            .strong(),
                    )
                    .on_hover_text("Enable all lighting, skybox, weapons, bot AI paths, elevators, and hazards")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(0);
                }
                if ui
                    .button("■ Clean Blockout")
                    .on_hover_text("Pure CSG geometry, lighting, and basic spawns without clutter")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(1);
                }
                if ui
                    .button("⚔ DeathMatch (DM-)")
                    .on_hover_text("Standard 4-8 player tournament DeathMatch complex (DM-Deck16 style)")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(6);
                }
                if ui
                    .button("⚔ 1v1 Duel (DM-1on1)")
                    .on_hover_text("Compact multi-tier 1v1 tournament arena")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(2);
                }
                if ui
                    .button("⚑ Symmetrical CTF")
                    .on_hover_text("Twin bases with Red/Blue FlagBases, DefensePoints, and AlternatePaths")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(3);
                }
                if ui
                    .button("♛ Domination Triad")
                    .on_hover_text("3 ControlPoints (Alpha, Beta, Gamma) in an interconnected loop complex")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(4);
                }
                if ui
                    .button("▲ Assault Fortress")
                    .on_hover_text("3 sequential FortStandard objectives with Defender/Attacker spawns")
                    .clicked()
                {
                    state.apply_map_wizard_quick_preset(5);
                }
            });

            ui.add_space(2.0);
            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(24, 30, 42))
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            RichText::new("⚄ One-Click Randomizer:")
                                .strong()
                                .color(Color32::from_rgb(255, 210, 95)),
                        );
                        if ui
                            .button(
                                RichText::new("⚄ Randomize Everything (All Steps)")
                                    .strong()
                                    .color(Color32::from_rgb(255, 220, 100)),
                            )
                            .on_hover_text(
                                "Randomize all customizable parameters across Steps 1, 2, 3, and 4 in a single click while keeping the map 100% valid and playable",
                            )
                            .clicked()
                        {
                            state.randomize_all_wizard_settings();
                        }

                        if ui
                            .button(
                                RichText::new("⚡ 1-Click Mystery Map (Roll & Build 3D)")
                                    .strong()
                                    .color(Color32::from_rgb(105, 255, 155)),
                            )
                            .on_hover_text(
                                "Randomize all steps AND immediately generate the complete 3D map in the editor with Build All + Level Doctor verification",
                            )
                            .clicked()
                        {
                            state.randomize_all_wizard_settings();
                            state.generate_map_from_wizard();
                        }

                        ui.separator();
                        ui.checkbox(
                            &mut state.map_wizard_lock_game_mode,
                            RichText::new("Lock Game Mode")
                                .size(11.5)
                                .color(Color32::from_rgb(185, 215, 250)),
                        )
                        .on_hover_text(
                            "When checked, 'Randomize Everything' and '1-Click Mystery Map' keep the current Game Mode (e.g. CTF) and randomize all other settings",
                        );

                        ui.separator();
                        if ui
                            .button(RichText::new("⇪ Export JSON").size(11.0).color(Color32::from_rgb(180, 220, 255)))
                            .on_hover_text("Export this complete wizard configuration as a shareable JSON preset")
                            .clicked()
                        {
                            if let Ok(json) = state.export_wizard_preset_json() {
                                ui.ctx().copy_text(json.clone());
                                state.wizard_preset_json_modal = Some(json);
                                state.toast_info("Preset JSON copied to clipboard");
                            }
                        }
                        if ui
                            .button(RichText::new("⇩ Import JSON").size(11.0).color(Color32::from_rgb(255, 215, 120)))
                            .on_hover_text("Load and apply a saved JSON wizard preset")
                            .clicked()
                        {
                            state.wizard_preset_json_modal = Some(String::new());
                        }
                    });
                });

            ui.separator();

            // Step Navigation Bar (Steps 0..=4 + All-in-One mode at step 5)
            ui.horizontal(|ui| {
                let steps = [
                    (0, "1. Mode & Identity"),
                    (1, "2. Layout & Seed"),
                    (2, "3. Theme & Official DNA"),
                    (3, "4. Gameplay Systems"),
                    (4, "5. Build & Launch"),
                    (5, "☰ All-in-One View"),
                ];
                for (idx, label) in steps {
                    let selected = state.map_wizard_step == idx;
                    let text = if selected {
                        RichText::new(label)
                            .strong()
                            .color(Color32::from_rgb(120, 220, 255))
                    } else {
                        RichText::new(label)
                    };
                    if ui.selectable_label(selected, text).clicked() {
                        state.map_wizard_step = idx;
                    }
                }
            });

            ui.separator();

            // Main Two-Column Split: Left = Wizard Controls, Right = Real-Time 2D Schematic & Telemetry
            ui.columns(2, |columns| {
                // =========================================================
                // LEFT COLUMN: Configuration Steps
                // =========================================================
                columns[0].set_clip_rect(columns[0].max_rect());
                egui::ScrollArea::vertical()
                    .id_salt("map_wizard_left_scroll")
                    .auto_shrink([false, false])
                    .show(&mut columns[0], |ui| {
                        ui.set_max_width(ui.available_width());
                        let show_all = state.map_wizard_step == 5;

                        // --- STEP 1: Game Mode & Map Identity ---
                        if show_all || state.map_wizard_step == 0 {
                            egui::CollapsingHeader::new(
                                RichText::new("1. Game Mode & Level Identity")
                                    .strong()
                                    .color(Color32::from_rgb(130, 205, 255)),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .button(
                                            RichText::new("⚄ Randomize Step 1 (Mode & Identity)")
                                                .strong()
                                                .color(Color32::from_rgb(255, 215, 95)),
                                        )
                                        .on_hover_text(
                                            "Randomize Game Mode (unless Lock Game Mode is checked), Map Filename, Title, Enter Text, and Ideal Player Count",
                                        )
                                        .clicked()
                                    {
                                        let lock_mode = state.map_wizard_lock_game_mode;
                                        state
                                            .map_wizard_config
                                            .randomize_step1_identity(lock_mode);
                                    }
                                });
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.label("Game Mode:");
                                    egui::ComboBox::from_id_salt("wizard_game_mode_combo")
                                        .selected_text(state.map_wizard_config.game_mode.label())
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for &mode in WizardGameMode::all() {
                                                if ui
                                                    .selectable_value(
                                                        &mut state.map_wizard_config.game_mode,
                                                        mode,
                                                        mode.label(),
                                                    )
                                                    .clicked()
                                                {
                                                    state.map_wizard_config.sync_mode_conventions();
                                                }
                                            }
                                        });
                                });

                                ui.horizontal(|ui| {
                                    ui.label("DefaultGameType:");
                                    ui.label(
                                        RichText::new(state.map_wizard_config.game_mode.default_game_type_class())
                                            .monospace()
                                            .color(Color32::from_rgb(160, 230, 170)),
                                    );
                                });

                                egui::Grid::new("wizard_identity_grid")
                                    .num_columns(2)
                                    .spacing([10.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.label("Map Filename:");
                                        ui.horizontal(|ui| {
                                            if ui
                                                .text_edit_singleline(
                                                    &mut state.map_wizard_config.map_name,
                                                )
                                                .changed()
                                            {
                                                state.map_wizard_config.infer_mode_from_map_name_prefix();
                                            }
                                            if ui
                                                .button(
                                                    RichText::new("⚄ Random Name")
                                                        .color(Color32::from_rgb(255, 215, 90)),
                                                )
                                                .on_hover_text(
                                                    "Generate a new plausible UT99 map name (2 or 3 words from the internal lexicon) and sync Title & Enter Text",
                                                )
                                                .clicked()
                                            {
                                                state.map_wizard_config.randomize_map_identity();
                                            }
                                        });
                                        ui.end_row();

                                        ui.label("Level Title:");
                                        ui.text_edit_singleline(&mut state.map_wizard_config.title);
                                        ui.end_row();

                                        ui.label("Author:");
                                        ui.text_edit_singleline(&mut state.map_wizard_config.author);
                                        ui.end_row();

                                        ui.label("Ideal Player Count:");
                                        ui.text_edit_singleline(
                                            &mut state.map_wizard_config.ideal_player_count,
                                        );
                                        ui.end_row();

                                        ui.label("Enter Text:");
                                        ui.text_edit_singleline(&mut state.map_wizard_config.enter_text);
                                        ui.end_row();

                                        ui.label("Background Music:");
                                        let current_track_label = UT99_MUSIC_TRACKS
                                            .iter()
                                            .find(|(_, val)| *val == state.map_wizard_config.music_track)
                                            .map(|(lbl, _)| *lbl)
                                            .unwrap_or(if state.map_wizard_config.music_track.is_empty() {
                                                "Theme Default (Auto)"
                                            } else {
                                                &state.map_wizard_config.music_track
                                            });
                                        egui::ComboBox::from_id_salt("wizard_music_track_combo")
                                            .selected_text(current_track_label)
                                            .width(260.0)
                                            .show_ui(ui, |ui| {
                                                for (lbl, val) in UT99_MUSIC_TRACKS {
                                                    ui.selectable_value(
                                                        &mut state.map_wizard_config.music_track,
                                                        val.to_string(),
                                                        *lbl,
                                                    );
                                                }
                                            });
                                        ui.end_row();
                                    });
                            });
                            ui.add_space(6.0);
                        }

                        // --- STEP 2: Layout Strategy (Archetypes vs Procedural Seed) ---
                        if show_all || state.map_wizard_step == 1 {
                            egui::CollapsingHeader::new(
                                RichText::new("2. Architectural Layout & Procedural Seed")
                                    .strong()
                                    .color(Color32::from_rgb(130, 205, 255)),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .button(
                                            RichText::new(
                                                "⚄ Randomize Step 2 (Layout, Seed, Scale & Shape)",
                                            )
                                            .strong()
                                            .color(Color32::from_rgb(255, 215, 95)),
                                        )
                                        .on_hover_text(
                                            "Randomize Layout Strategy, Predefined Archetype, Procedural Seed & Graph Parameters, Symmetry Mode, Map Scale, and Room Footprint Shape",
                                        )
                                        .clicked()
                                    {
                                        state.map_wizard_config.randomize_step2_layout();
                                        state.map_wizard_archetype_filter = None;
                                    }
                                });
                                ui.add_space(2.0);
                                let is_archetype = state.map_wizard_config.layout_strategy
                                    == WizardLayoutStrategy::CanonicalArchetype;

                                ui.horizontal(|ui| {
                                    if ui
                                        .radio(is_archetype, "◆ Canonical UT99 Archetype")
                                        .clicked()
                                    {
                                        state.map_wizard_config.layout_strategy =
                                            WizardLayoutStrategy::CanonicalArchetype;
                                    }
                                    if ui
                                        .radio(!is_archetype, "⚄ Procedural Seed Generator")
                                        .clicked()
                                    {
                                        state.map_wizard_config.layout_strategy =
                                            WizardLayoutStrategy::ProceduralSeed;
                                    }
                                });

                                ui.add_space(4.0);

                                match state.map_wizard_config.layout_strategy {
                                    WizardLayoutStrategy::CanonicalArchetype => {
                                        ui.horizontal_wrapped(|ui| {
                                            let all_selected =
                                                state.map_wizard_archetype_filter.is_none();
                                            if ui
                                                .selectable_label(
                                                    all_selected,
                                                    RichText::new("All (18)").strong(),
                                                )
                                                .clicked()
                                            {
                                                state.map_wizard_archetype_filter = None;
                                            }
                                            for &cat in WizardArchetypeCategory::all() {
                                                let selected = state.map_wizard_archetype_filter
                                                    == Some(cat);
                                                if ui
                                                    .selectable_label(selected, cat.label())
                                                    .clicked()
                                                {
                                                    state.map_wizard_archetype_filter = Some(cat);
                                                }
                                            }
                                            if ui
                                                .button(
                                                    RichText::new("⚄ Random Layout")
                                                        .color(Color32::from_rgb(255, 215, 90)),
                                                )
                                                .on_hover_text(
                                                    "Select a random predefined architectural layout from the 32 UT99 archetypes",
                                                )
                                                .clicked()
                                            {
                                                state.map_wizard_config.randomize_archetype();
                                            }
                                        });
                                        ui.add_space(4.0);

                                        egui::ScrollArea::vertical()
                                            .id_salt("wizard_archetype_catalog_scroll")
                                            .max_height(260.0)
                                            .show(ui, |ui| {
                                                for &candidate in WizardArchetype::all() {
                                                    if let Some(filter_cat) =
                                                        state.map_wizard_archetype_filter
                                                    {
                                                        if candidate.category() != filter_cat {
                                                            continue;
                                                        }
                                                    }
                                                    ui.horizontal(|ui| {
                                                        ui.radio_value(
                                                            &mut state.map_wizard_config.archetype,
                                                            candidate,
                                                            RichText::new(candidate.label())
                                                                .strong(),
                                                        );
                                                        ui.label(
                                                            RichText::new(format!(
                                                                "[{}]",
                                                                candidate.category().badge()
                                                            ))
                                                            .size(10.0)
                                                            .monospace()
                                                            .color(Color32::from_rgb(
                                                                120, 195, 245,
                                                            )),
                                                        );
                                                    });
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "   {}",
                                                            candidate.description()
                                                        ))
                                                        .size(11.0)
                                                        .color(Color32::from_rgb(175, 180, 190)),
                                                    );
                                                    ui.add_space(2.0);
                                                }
                                            });
                                    }
                                    WizardLayoutStrategy::ProceduralSeed => {
                                        ui.horizontal(|ui| {
                                            ui.label("Procedural Seed:");
                                            ui.add(
                                                egui::DragValue::new(&mut state.map_wizard_config.seed)
                                                    .speed(1.0),
                                            );
                                            if ui.button("⚄ Randomize Seed").clicked() {
                                                state.map_wizard_config.seed = state
                                                    .map_wizard_config
                                                    .seed
                                                    .wrapping_mul(6364136223846793005)
                                                    .wrapping_add(1442695040888963407)
                                                    ^ 0x5DEECE66D;
                                            }
                                        });
                                        ui.add(
                                            egui::Slider::new(
                                                &mut state.map_wizard_config.procedural_room_count,
                                                3..=10,
                                            )
                                            .text("Target Room / Chamber Count"),
                                        );
                                        if state.map_wizard_config.game_mode == WizardGameMode::CaptureTheFlag
                                            || state.map_wizard_config.symmetry_mode != WizardSymmetryMode::None
                                        {
                                            let target = state.map_wizard_config.procedural_room_count.clamp(3, 12);
                                            let actual = 1 + 2 * ((target - 1) / 2);
                                            ui.label(format!("Symmetric layout: {actual} main rooms (hub and mirrored pairs)"));
                                        }
                                        ui.add(
                                            egui::Slider::new(
                                                &mut state.map_wizard_config.procedural_vertical_tiers,
                                                1..=3,
                                            )
                                            .text("Verticality Tiers"),
                                        );
                                        ui.add(
                                            egui::Slider::new(
                                                &mut state.map_wizard_config.procedural_loop_ratio,
                                                0.0..=1.0,
                                            )
                                            .text("Extra Loop Probability"),
                                        );
                                        ui.label("Values below 0.35 use 0.35; minimum connections may add more routes.");
                                    }
                                }

                                ui.horizontal(|ui| {
                                    ui.label("Symmetry Mode:");
                                    for &sym in WizardSymmetryMode::all() {
                                        ui.selectable_value(
                                            &mut state.map_wizard_config.symmetry_mode,
                                            sym,
                                            sym.label(),
                                        );
                                    }
                                });

                                ui.separator();
                                ui.horizontal(|ui| {
                                    ui.label("Arena Scale:");
                                    for &sc in WizardMapScale::all() {
                                        ui.selectable_value(
                                            &mut state.map_wizard_config.scale,
                                            sc,
                                            sc.label(),
                                        );
                                    }
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Room Profile:");
                                    egui::ComboBox::from_id_salt("wizard_room_shape_combo")
                                        .selected_text(state.map_wizard_config.room_shape.label())
                                        .width(280.0)
                                        .show_ui(ui, |ui| {
                                            for &sh in WizardRoomShape::all() {
                                                ui.selectable_value(
                                                    &mut state.map_wizard_config.room_shape,
                                                    sh,
                                                    sh.label(),
                                                );
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Architectural Detail:");
                                    egui::ComboBox::from_id_salt("wizard_arch_detail_combo")
                                        .selected_text(state.map_wizard_config.architectural_detail.label())
                                        .width(320.0)
                                        .show_ui(ui, |ui| {
                                            for &det in WizardArchitecturalDetail::all() {
                                                ui.selectable_value(
                                                    &mut state.map_wizard_config.architectural_detail,
                                                    det,
                                                    det.label(),
                                                );
                                            }
                                        });
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Construction rules:");
                                    let current = state.map_wizard_config.generator_version;
                                    let label = |v: u32| match v {
                                        ut99_bsp::GENERATOR_VERSION_LEGACY => "v1 Legacy (v0.23.5 output)",
                                        ut99_bsp::GENERATOR_VERSION_PORTALS => "v2 Portal-aligned corridors & anchored details",
                                        _ => "v3 + height library (procedural)",
                                    };
                                    egui::ComboBox::from_id_salt("wizard_generator_version_combo")
                                        .selected_text(label(current))
                                        .width(320.0)
                                        .show_ui(ui, |ui| {
                                            for v in [ut99_bsp::GENERATOR_VERSION_LEGACY, ut99_bsp::GENERATOR_VERSION_PORTALS, ut99_bsp::GENERATOR_VERSION_HEIGHTS] {
                                                ui.selectable_value(&mut state.map_wizard_config.generator_version, v, label(v));
                                            }
                                        })
                                        .response
                                        .on_hover_text("v2: wall-to-wall corridors, no floating frames/beams/fixtures. v3: also varied ceiling heights (procedural layouts). v1 reproduces v0.23.5 exactly.");
                                });
                            });
                            ui.add_space(6.0);
                        }

                        // --- STEP 3: Theme & Official UT99 Map DNA Cloner ---
                        if show_all || state.map_wizard_step == 2 {
                            egui::CollapsingHeader::new(
                                RichText::new("3. Visual Theme & Official UT99 Map DNA Cloner")
                                    .strong()
                                    .color(Color32::from_rgb(130, 205, 255)),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    ui.label("Curated UT99 Aesthetic Theme:");
                                    if ui
                                        .button(
                                            RichText::new("⚄ Random Theme Preset")
                                                .color(Color32::from_rgb(255, 215, 90)),
                                        )
                                        .on_hover_text(
                                            "Pick a random stock UT99 visual, lighting, and music theme preset",
                                        )
                                        .clicked()
                                    {
                                        state.map_wizard_config.randomize_step3_theme();
                                    }
                                });
                                let theme_combo_w = (ui.available_width() - 8.0).clamp(240.0, 360.0);
                                egui::ComboBox::from_id_salt("wizard_theme_preset_combo")
                                    .selected_text(state.map_wizard_config.theme.label())
                                    .width(theme_combo_w)
                                    .show_ui(ui, |ui| {
                                        for &th in WizardThemePreset::all() {
                                            if ui
                                                .selectable_value(
                                                    &mut state.map_wizard_config.theme,
                                                    th,
                                                    th.label(),
                                                )
                                                .clicked()
                                            {
                                                // Selecting a curated theme clears custom DNA override unless re-applied
                                                state.map_wizard_config.custom_dna_source_map = None;
                                                state.map_wizard_config.custom_palette_override = None;
                                            }
                                        }
                                    });

                                ui.add_space(6.0);
                                let active_theme_palette = state.map_wizard_config.active_palette();
                                render_theme_visual_preview(ui, ctx, state, &active_theme_palette);

                                ui.add_space(6.0);
                                ui.separator();

                                // Official UT99 Map DNA Cloner Card
                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgb(18, 24, 34))
                                    .inner_margin(egui::Margin::same(8))
                                    .show(ui, |ui| {
                                        ui.label(
                                            RichText::new("★ Clone DNA from Official UT99 Map (UnrealTournament/Maps)")
                                                .strong()
                                                .color(Color32::from_rgb(255, 205, 95)),
                                        );
                                        ui.label(
                                            RichText::new(
                                                "Forensically extracts real Wall/Floor/Ceiling/Trim/Sky textures, HSV lighting, music track, and weapon loadout from any installed .unr map.",
                                            )
                                            .size(11.0)
                                            .color(Color32::from_rgb(180, 185, 195)),
                                        );
                                        ui.add_space(4.0);

                                        // Mode filter pills for the Official Map list
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new("Mode Filter:")
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(165, 175, 190)),
                                            );
                                            for mode_filter in ["ALL", "DM", "CTF", "DOM", "AS"] {
                                                let active = state
                                                    .official_dna_mode_filter
                                                    .eq_ignore_ascii_case(mode_filter);
                                                if ui.selectable_label(active, mode_filter).clicked() {
                                                    state.official_dna_mode_filter = mode_filter.to_string();
                                                    if mode_filter != "ALL" {
                                                        let prefix = format!("{mode_filter}-");
                                                        if !state
                                                            .selected_official_dna_map
                                                            .to_ascii_uppercase()
                                                            .starts_with(&prefix)
                                                        {
                                                            if let Some(first_match) = state
                                                                .available_official_maps
                                                                .iter()
                                                                .find(|m| {
                                                                    m.to_ascii_uppercase()
                                                                        .starts_with(&prefix)
                                                                })
                                                                .cloned()
                                                            {
                                                                state.selected_official_dna_map =
                                                                    first_match;
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        });

                                        ui.horizontal(|ui| {
                                            if state.available_official_maps.is_empty() {
                                                ui.text_edit_singleline(&mut state.selected_official_dna_map);
                                            } else {
                                                let filter_upper =
                                                    state.official_dna_mode_filter.to_ascii_uppercase();
                                                let combo_w = (ui.available_width() - 115.0).clamp(160.0, 260.0);
                                                egui::ComboBox::from_id_salt("official_dna_map_combo")
                                                    .selected_text(&state.selected_official_dna_map)
                                                    .width(combo_w)
                                                    .show_ui(ui, |ui| {
                                                        for map_file in &state.available_official_maps {
                                                            if filter_upper != "ALL"
                                                                && !map_file
                                                                    .to_ascii_uppercase()
                                                                    .starts_with(&format!("{filter_upper}-"))
                                                            {
                                                                continue;
                                                            }
                                                            ui.selectable_value(
                                                                &mut state.selected_official_dna_map,
                                                                map_file.clone(),
                                                                map_file,
                                                            );
                                                        }
                                                    });
                                            }

                                            if ui
                                                .button(
                                                    RichText::new("★ Clone DNA")
                                                        .strong()
                                                        .color(Color32::from_rgb(110, 240, 255)),
                                                )
                                                .clicked()
                                            {
                                                let target_map = state.selected_official_dna_map.clone();
                                                if let Err(e) = state.clone_dna_from_official_map(&target_map) {
                                                    state.toast_error(e);
                                                }
                                            }
                                        });

                                        ui.add_space(3.0);

                                        ui.horizontal_wrapped(|ui| {
                                            if ui
                                                .button(
                                                    RichText::new("⚄ Random Official DNA")
                                                        .color(Color32::from_rgb(255, 215, 90)),
                                                )
                                                .on_hover_text(
                                                    "Pick a random official .unr map from UnrealTournament/Maps and clone its textures, lighting HSV, music, and props",
                                                )
                                                .clicked()
                                            {
                                                state.randomize_official_map_dna();
                                            }

                                            if ui
                                                .small_button("▦ Study Corpus")
                                                .on_hover_text(
                                                    "Forensically scan all official .unr maps in UnrealTournament/Maps and compute empirical per-mode patterns",
                                                )
                                                .clicked()
                                            {
                                                if let Err(e) = state.analyze_official_map_corpus() {
                                                    state.toast_error(e);
                                                }
                                            }

                                            if ui
                                                .small_button("↻ Rescan")
                                                .on_hover_text("Rescan UnrealTournament/Maps directory")
                                                .clicked()
                                            {
                                                state.scan_official_maps_for_wizard();
                                            }
                                        });
                                    });

                                if let Some(ref src_map) = state.map_wizard_config.custom_dna_source_map.clone() {
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(24, 34, 46))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "✔ Active DNA Override: {}",
                                                        src_map
                                                    ))
                                                    .strong()
                                                    .color(Color32::from_rgb(110, 255, 160)),
                                                );
                                                if ui.small_button("✕ Reset to Theme").clicked() {
                                                    state.map_wizard_config.custom_dna_source_map = None;
                                                    state.map_wizard_config.custom_palette_override = None;
                                                    state.map_wizard_config.apply_weapon_preset(WizardWeaponPreset::TournamentStandard);
                                                    state.map_wizard_config.apply_powerup_preset(WizardPowerupPreset::BalancedCompetitive);
                                                    state.map_wizard_config.custom_decoration_loadout.clear();
                                                    state.map_wizard_config.custom_ambient_sounds.clear();
                                                    state.last_cloned_dna = None;
                                                }
                                            });
                                            let pal = state.map_wizard_config.active_palette();
                                            ui.label(
                                                RichText::new(format!(
                                                    "Wall: {} | Floor: {} | Ceiling: {}",
                                                    pal.wall_texture,
                                                    pal.floor_texture,
                                                    pal.ceiling_texture
                                                ))
                                                .size(11.0)
                                                .monospace(),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "Sky: {} | Song: {} | Primary Light HSV: ({}, {}, {})",
                                                    pal.sky_texture,
                                                    pal.song,
                                                    pal.primary_light_hue,
                                                    pal.primary_light_sat,
                                                    pal.primary_light_bright
                                                ))
                                                .size(11.0)
                                                .monospace(),
                                            );

                                            ui.add_space(3.0);
                                            ui.horizontal_wrapped(|ui| {
                                                if ui
                                                    .small_button(
                                                        RichText::new("⚙ Auto-Match Mode, Archetype & Scale")
                                                            .color(Color32::from_rgb(255, 215, 110)),
                                                    )
                                                    .on_hover_text(
                                                        "Adapts Game Mode, Architectural Archetype, Map Scale, Room Count, Vertical Tiers, and Feature Toggles to match the cloned map's structural DNA",
                                                    )
                                                    .clicked()
                                                {
                                                    if !state.apply_cloned_dna_topology_and_mode() {
                                                        state.toast_error("No cloned Official Map DNA is currently active.");
                                                    }
                                                }

                                                if ui
                                                    .small_button("⎘ Copy Forensic DNA Report (MD)")
                                                    .on_hover_text(
                                                        "Copy a complete Markdown forensic report of the cloned official .unr map to clipboard",
                                                    )
                                                    .clicked()
                                                {
                                                    if let Some(md) = state.format_cloned_dna_markdown_report() {
                                                        ui.ctx().copy_text(md);
                                                        state.toast_info(
                                                            "Copied Forensic Official Map DNA Report (Markdown) to clipboard",
                                                        );
                                                    }
                                                }
                                            });

                                            if let Some(ref dna) = state.last_cloned_dna {
                                                if !dna.read_issues.is_empty() {
                                                    ui.colored_label(Color32::from_rgb(255, 180, 80), format!(
                                                        "Profile may be incomplete: {} decoding warning(s)", dna.read_issues.len()
                                                    ));
                                                    for issue in &dna.read_issues {
                                                        ui.label(format!("{}: {} (export {:?})", issue.actor_name.as_deref().unwrap_or("Unknown actor"), issue.detail, issue.export_index));
                                                    }
                                                }
                                                egui::CollapsingHeader::new(
                                                    RichText::new("◈ Forensic Official Map DNA Breakdown")
                                                        .size(11.0)
                                                        .color(Color32::from_rgb(165, 215, 255)),
                                                )
                                                .default_open(false)
                                                .show(ui, |ui| {
                                                    let dx = (dna.world_bounds_max[0] - dna.world_bounds_min[0]).max(0.0);
                                                    let dy = (dna.world_bounds_max[1] - dna.world_bounds_min[1]).max(0.0);
                                                    let dz = (dna.world_bounds_max[2] - dna.world_bounds_min[2]).max(0.0);
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Author: {} | IdealPlayers: {} | Extent: {:.0}×{:.0}×{:.0} UU",
                                                            if dna.author.is_empty() { "Unknown" } else { &dna.author },
                                                            if dna.ideal_player_count.is_empty() { "N/A" } else { &dna.ideal_player_count },
                                                            dx,
                                                            dy,
                                                            dz
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "CSG Brushes: {} (Sub: {}, Add: {}, Semi: {}) | Movers: {}",
                                                            dna.total_brushes,
                                                            dna.subtract_brushes,
                                                            dna.add_brushes,
                                                            dna.semisolid_brushes,
                                                            dna.mover_count
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "Spawns: {} | PathNodes: {} | LiftCenters/Exits: {}/{} | Teleporters: {} | Lights: {}",
                                                            dna.gameplay.player_start_count,
                                                            dna.gameplay.path_node_count,
                                                            dna.gameplay.lift_center_count,
                                                            dna.gameplay.lift_exit_count,
                                                            dna.gameplay.teleporter_count,
                                                            dna.lighting.light_count
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    if !dna.gameplay.weapon_classes.is_empty() {
                                                        let weapons_str = dna
                                                            .gameplay
                                                            .weapon_classes
                                                            .iter()
                                                            .map(|(w, c)| format!("{}×{}", w, c))
                                                            .collect::<Vec<_>>()
                                                            .join(", ");
                                                        ui.label(
                                                            RichText::new(format!("Weapons: {}", weapons_str))
                                                                .size(10.5)
                                                                .monospace()
                                                                .color(Color32::from_rgb(200, 230, 190)),
                                                        );
                                                    }
                                                    if !dna.gameplay.decoration_classes.is_empty() {
                                                        let deco_str = dna
                                                            .gameplay
                                                            .decoration_classes
                                                            .iter()
                                                            .take(6)
                                                            .map(|(d, c)| format!("{}×{}", d, c))
                                                            .collect::<Vec<_>>()
                                                            .join(", ");
                                                        ui.label(
                                                            RichText::new(format!("3D Props: {}", deco_str))
                                                                .size(10.5)
                                                                .monospace()
                                                                .color(Color32::from_rgb(235, 210, 160)),
                                                        );
                                                    }
                                                    if !dna.gameplay.trigger_and_special_classes.is_empty() {
                                                        let spec_str = dna
                                                            .gameplay
                                                            .trigger_and_special_classes
                                                            .iter()
                                                            .take(6)
                                                            .map(|(s, c)| format!("{}×{}", s, c))
                                                            .collect::<Vec<_>>()
                                                            .join(", ");
                                                        ui.label(
                                                            RichText::new(format!("Special Mechanics: {}", spec_str))
                                                                .size(10.5)
                                                                .monospace()
                                                                .color(Color32::from_rgb(185, 210, 245)),
                                                        );
                                                    }
                                                });
                                            }
                                        });
                                }

                                // Corpus Pattern Summary Table (shown when ▦ Study Corpus has been run)
                                if !state.official_corpus_stats.is_empty() {
                                    ui.add_space(4.0);
                                    egui::CollapsingHeader::new(
                                        RichText::new(format!(
                                            "▦ Official UT99 Map Corpus Study ({} Maps Analyzed)",
                                            state.official_corpus_profiles.len()
                                        ))
                                        .size(11.5)
                                        .strong()
                                        .color(Color32::from_rgb(255, 215, 115)),
                                    )
                                    .default_open(true)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(
                                                    "Empirical averages per game mode across UnrealTournament/Maps:",
                                                )
                                                .size(10.5)
                                                .color(Color32::from_rgb(185, 195, 210)),
                                            );
                                            if ui
                                                .small_button("⎘ Copy Corpus Study (MD)")
                                                .clicked()
                                            {
                                                if let Some(md) =
                                                    state.format_official_corpus_markdown_report()
                                                {
                                                    ui.ctx().copy_text(md);
                                                    state.toast_info(
                                                        "Copied Official UT99 Map Corpus Study (Markdown) to clipboard",
                                                    );
                                                }
                                            }
                                        });

                                        egui::Grid::new("official_corpus_stats_grid")
                                            .striped(true)
                                            .spacing([8.0, 3.0])
                                            .show(ui, |ui| {
                                                ui.label(RichText::new("Mode").strong().size(10.5));
                                                ui.label(RichText::new("Maps").strong().size(10.5));
                                                ui.label(RichText::new("Brushes").strong().size(10.5));
                                                ui.label(RichText::new("Sub%").strong().size(10.5));
                                                ui.label(RichText::new("Semi%").strong().size(10.5));
                                                ui.label(RichText::new("Movers").strong().size(10.5));
                                                ui.label(RichText::new("Lights").strong().size(10.5));
                                                ui.label(RichText::new("PathNodes").strong().size(10.5));
                                                ui.label(RichText::new("Top .utx / Props").strong().size(10.5));
                                                ui.end_row();

                                                for st in &state.official_corpus_stats {
                                                    ui.label(
                                                        RichText::new(&st.mode_prefix)
                                                            .strong()
                                                            .size(10.5)
                                                            .color(Color32::from_rgb(130, 215, 255)),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("{}", st.map_count))
                                                            .size(10.5)
                                                            .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("{:.0}", st.avg_brushes))
                                                            .size(10.5)
                                                            .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:.0}%",
                                                            st.avg_subtract_ratio_pct
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:.0}%",
                                                            st.avg_semisolid_ratio_pct
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("{:.1}", st.avg_movers))
                                                            .size(10.5)
                                                            .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!("{:.0}", st.avg_lights))
                                                            .size(10.5)
                                                            .monospace(),
                                                    );
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{:.0}",
                                                            st.avg_path_nodes
                                                        ))
                                                        .size(10.5)
                                                        .monospace(),
                                                    );
                                                    let top_pkg = st
                                                        .top_texture_packages
                                                        .first()
                                                        .map(|(p, _)| p.as_str())
                                                        .unwrap_or("-");
                                                    let top_deco = st
                                                        .top_decorations
                                                        .first()
                                                        .map(|(d, _)| d.as_str())
                                                        .unwrap_or("-");
                                                    ui.label(
                                                        RichText::new(format!(
                                                            "{}.utx / {}",
                                                            top_pkg, top_deco
                                                        ))
                                                        .size(10.0)
                                                        .monospace(),
                                                    );
                                                    ui.end_row();
                                                }
                                            });
                                    });
                                }
                            });
                            ui.add_space(6.0);
                        }

                        // --- STEP 4: Gameplay Systems, Armory & Custom Loadouts ---
                        if show_all || state.map_wizard_step == 3 {
                            egui::CollapsingHeader::new(
                                RichText::new("4. Gameplay Systems, Armory & Custom Loadouts")
                                    .strong()
                                    .color(Color32::from_rgb(130, 205, 255)),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .button(
                                            RichText::new(
                                                "⚄ Randomize Step 4 (Mechanics, Weapons & Powerups)",
                                            )
                                            .strong()
                                            .color(Color32::from_rgb(255, 215, 95)),
                                        )
                                        .on_hover_text(
                                            "Randomize dynamic gameplay mechanics, weapon loadout, powerup loadout, and item density while preserving lighting & bot navigation",
                                        )
                                        .clicked()
                                    {
                                        state.map_wizard_config.randomize_step4_gameplay_and_armory();
                                    }
                                    if ui
                                        .button(
                                            RichText::new("⚄ Random Mechanics Only")
                                                .color(Color32::from_rgb(225, 205, 130)),
                                        )
                                        .on_hover_text(
                                            "Randomize optional system toggles (Skybox, Elevators, Hazards, Decorations, Jump-Pads, Ramps, Teleporters)",
                                        )
                                        .clicked()
                                    {
                                        state.map_wizard_config.randomize_mechanics_toggles();
                                    }
                                });
                                ui.add_space(2.0);
                                let f = &mut state.map_wizard_config.toggles;
                                ui.checkbox(
                                    &mut f.include_lighting,
                                    "☀ Atmospheric HSV Lighting (Key Lights, Rim Lights & ZoneInfo)",
                                );
                                ui.checkbox(
                                    &mut f.include_skybox,
                                    "★ 3D Skybox Chamber (Isolated SkyZoneInfo + PF_FAKE_BACKDROP)",
                                );
                                ui.checkbox(
                                    &mut f.include_weapons_and_pickups,
                                    "⚔ Tournament Weapons, Ammo, Armor, MedBoxes & Powerups",
                                );
                                ui.checkbox(
                                    &mut f.include_bot_ai_paths,
                                    "⚙ Bot AI Navigation Network (PathNodes, ReachSpecs & Team AI)",
                                );
                                ui.checkbox(
                                    &mut f.include_elevators_and_movers,
                                    "↕ Vertical Elevators & Triggered Movers (LiftCenter + LiftExit)",
                                );
                                ui.checkbox(
                                    &mut f.include_environmental_hazards,
                                    "☣ Environmental Fluid Hazard Basins (Lava / Slime / WaterZone + Portal)",
                                );
                                ui.checkbox(
                                    &mut f.include_decorations_and_ambience,
                                    "◆ 3D Decorative Props & Ambient Soundscapes (TorchFlame, SteelBarrel, Urn, AmbientSound)",
                                );
                                ui.checkbox(
                                    &mut f.include_jump_pads,
                                    "▲ Kinetic Kicker Jump-Pads (Kicker Actors Launching to Upper Balconies)",
                                );
                                ui.checkbox(
                                    &mut f.include_ramps_and_bridges,
                                    "◢ Convex Ramps & Elevated Overwatch Decks (Walkable Multi-Tier Slopes)",
                                );
                                ui.checkbox(
                                    &mut f.include_teleporters,
                                    "◎ Paired Teleporter Portals (Bidirectional VisibleTeleporter0 ↔ VisibleTeleporter1 + R_SPECIAL)",
                                );
                                ui.checkbox(
                                    &mut f.ensure_vertical_reachability,
                                    "🪜 Guarantee Vertical Reachability (Stairs, Ramps & Connected Bot Waypoints)",
                                ).on_hover_text("Ensures rooms and corridors at different elevations (|ΔZ| > 24 UU) are physically interconnected by oriented walkable stairs/ramps (or kinetic jump-pads for high drops) with aligned navigation PathNodes so all zones are 100% accessible to bots and players.");

                                ui.horizontal(|ui| {
                                    ui.label("Atmospheric Distance Fog:");
                                    egui::ComboBox::from_id_salt("wizard_distance_fog_combo")
                                        .selected_text(state.map_wizard_config.distance_fog.label())
                                        .width(260.0)
                                        .show_ui(ui, |ui| {
                                            for &preset in ut99_bsp::WizardFogPreset::all() {
                                                ui.selectable_value(
                                                    &mut state.map_wizard_config.distance_fog,
                                                    preset,
                                                    preset.label(),
                                                );
                                            }
                                        });
                                });

                                if state.map_wizard_config.game_mode == WizardGameMode::Assault {
                                    ui.add_space(4.0);
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(34, 26, 20))
                                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 140, 50)))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("▲ Assault Mission Objectives & Gating")
                                                        .strong()
                                                        .color(Color32::from_rgb(255, 160, 60)),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "({} Objectives)",
                                                        state.map_wizard_config.assault_objective_count
                                                    ))
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(200, 190, 180)),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Objective Count (2 - 5):");
                                                let mut count_i32 =
                                                    state.map_wizard_config.assault_objective_count as i32;
                                                if ui
                                                    .add(egui::Slider::new(&mut count_i32, 2..=5).text("Objectives"))
                                                    .changed()
                                                {
                                                    state.map_wizard_config.assault_objective_count =
                                                        count_i32 as usize;
                                                }
                                                if ui.small_button("⚄ Randomize Objectives").clicked() {
                                                    let s = ut99_bsp::next_wizard_entropy_seed() as usize;
                                                    state.map_wizard_config.assault_objective_count = (s % 4) + 2;
                                                }
                                            });

                                            // Compute and display the live Assault Mission plan preview
                                            let preview_topo =
                                                ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
                                            let fort_markers: Vec<_> = preview_topo
                                                .markers
                                                .iter()
                                                .filter(|m| m.kind == "FortStandard")
                                                .collect();
                                            let gate_markers: Vec<_> = preview_topo
                                                .markers
                                                .iter()
                                                .filter(|m| m.label.starts_with("Gate Door"))
                                                .collect();

                                            ui.label(
                                                RichText::new(format!(
                                                    "Planned Chain: {} FortStandards, {} Gated Blast Doors",
                                                    fort_markers.len(),
                                                    gate_markers.len()
                                                ))
                                                .size(11.0)
                                                .monospace()
                                                .color(Color32::from_rgb(255, 220, 140)),
                                            );

                                            for (o_idx, m) in fort_markers.iter().enumerate() {
                                                let gate_info = if o_idx < gate_markers.len() {
                                                    format!(" ➔ [{}]", gate_markers[o_idx].label)
                                                } else if o_idx == fort_markers.len().saturating_sub(1) {
                                                    " ➔ [MISSION VICTORY]".to_string()
                                                } else {
                                                    " ➔ [Open Access]".to_string()
                                                };
                                                ui.label(
                                                    RichText::new(format!("  • {}{}", m.label, gate_info))
                                                        .size(11.0)
                                                        .monospace()
                                                        .color(if m.label.contains("[FINAL]") {
                                                            Color32::from_rgb(255, 100, 100)
                                                        } else {
                                                            Color32::from_rgb(140, 225, 255)
                                                        }),
                                                );
                                            }
                                        });
                                }

                                if state.map_wizard_config.game_mode == WizardGameMode::Domination {
                                    ui.add_space(4.0);
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(26, 32, 22))
                                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 215, 0)))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("⚑ Domination Mission Objectives & Triangle Topology")
                                                        .strong()
                                                        .color(Color32::from_rgb(255, 215, 0)),
                                                );
                                                ui.label(
                                                    RichText::new(format!(
                                                        "({} Control Points - {})",
                                                        state.map_wizard_config.domination_point_count,
                                                        if state.map_wizard_config.domination_point_count == 3 {
                                                            "Official Triad Standard"
                                                        } else if state.map_wizard_config.domination_point_count == 2 {
                                                            "Duel Pair"
                                                        } else {
                                                            "Quad Circuit"
                                                        }
                                                    ))
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(220, 220, 180)),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Control Points (2 - 4):");
                                                let mut count_i32 = state.map_wizard_config.domination_point_count as i32;
                                                if ui
                                                    .add(egui::Slider::new(&mut count_i32, 2..=4).text("Points"))
                                                    .changed()
                                                {
                                                    state.map_wizard_config.domination_point_count = count_i32 as usize;
                                                }
                                                if ui.small_button("⚄ Randomize Domination").clicked() {
                                                    let seed = ut99_bsp::next_wizard_entropy_seed();
                                                    let mut rng_val = seed;
                                                    state.map_wizard_config.domination_point_count = match rng_val % 10 {
                                                        0..=7 => 3,
                                                        8 => 2,
                                                        _ => 4,
                                                    };
                                                    let daises = DominationDaisStyle::all();
                                                    rng_val = rng_val.wrapping_mul(6364136223846793005).wrapping_add(1);
                                                    state.map_wizard_config.domination_dais_style = daises[(rng_val as usize) % daises.len()];
                                                    let namings = DominationNamingStyle::all();
                                                    rng_val = rng_val.wrapping_mul(6364136223846793005).wrapping_add(1);
                                                    state.map_wizard_config.domination_naming_style = namings[(rng_val as usize) % namings.len()];
                                                    state.map_wizard_config.domination_include_defense_points = (rng_val % 10) < 9;
                                                    state.map_wizard_config.domination_contested_center_powerup = (rng_val % 10) < 8;
                                                }
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Dais Architecture:");
                                                egui::ComboBox::from_id_salt("wizard_dom_dais_combo")
                                                    .selected_text(state.map_wizard_config.domination_dais_style.label())
                                                    .width(260.0)
                                                    .show_ui(ui, |ui| {
                                                        for &style in DominationDaisStyle::all() {
                                                            ui.selectable_value(
                                                                &mut state.map_wizard_config.domination_dais_style,
                                                                style,
                                                                style.label(),
                                                            );
                                                        }
                                                    });
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Naming Lexicon:");
                                                egui::ComboBox::from_id_salt("wizard_dom_naming_combo")
                                                    .selected_text(state.map_wizard_config.domination_naming_style.label())
                                                    .width(260.0)
                                                    .show_ui(ui, |ui| {
                                                        for &style in DominationNamingStyle::all() {
                                                            ui.selectable_value(
                                                                &mut state.map_wizard_config.domination_naming_style,
                                                                style,
                                                                style.label(),
                                                            );
                                                        }
                                                    });
                                            });

                                            ui.checkbox(
                                                &mut state.map_wizard_config.domination_include_defense_points,
                                                "Tactical Bot DefensePoints (DefensePoint Actors per ControlPoint)",
                                            );
                                            ui.checkbox(
                                                &mut state.map_wizard_config.domination_contested_center_powerup,
                                                "Contested Triangle Centroid Mega-Powerup (UT_ShieldBelt on Stepped Pedestal)",
                                            );

                                            // Live Domination topology summary
                                            let preview_topo = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
                                            let cp_markers: Vec<_> = preview_topo
                                                .markers
                                                .iter()
                                                .filter(|m| m.kind == "ControlPoint")
                                                .collect();

                                            ui.label(
                                                RichText::new(format!(
                                                    "Planned Triangle: {} ControlPoints with Dynamic Focal Lighting & Ambient Hum",
                                                    cp_markers.len()
                                                ))
                                                .size(11.0)
                                                .monospace()
                                                .color(Color32::from_rgb(255, 230, 140)),
                                            );

                                            for cp in &cp_markers {
                                                ui.label(
                                                    RichText::new(format!("  ⚑ {}", cp.label))
                                                        .size(11.0)
                                                        .monospace()
                                                        .color(Color32::from_rgb(255, 215, 0)),
                                                );
                                            }
                                        });
                                }

                                if state.map_wizard_config.game_mode == WizardGameMode::CaptureTheFlag {
                                    ui.add_space(4.0);
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(28, 22, 34))
                                        .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(255, 90, 90)))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new("🚩 Capture The Flag Tactical Architecture (CTF-)")
                                                        .strong()
                                                        .color(Color32::from_rgb(255, 105, 105)),
                                                );
                                                ui.label(
                                                    RichText::new("(Polar Bases & Multi-Lane AlternatePaths)")
                                                        .size(11.0)
                                                        .color(Color32::from_rgb(210, 200, 230)),
                                                );
                                            });

                                            ui.add_space(2.0);
                                            ui.label(
                                                RichText::new(
                                                    "Symmetrical base placement maximizing topological separation, with 3-tier bot defense hierarchy (Goal Guard, Chokepoint Sentries, High Overwatch), elevated sniper platforms, and balanced return corridors.",
                                                )
                                                .size(11.0)
                                                .color(Color32::from_rgb(180, 175, 195)),
                                            );

                                            ui.add_space(4.0);
                                            ui.horizontal(|ui| {
                                                ui.label("Flag Dais Architecture:");
                                                egui::ComboBox::from_id_salt("wizard_ctf_dais_style")
                                                    .selected_text(state.map_wizard_config.ctf_dais_style.label())
                                                    .width(280.0)
                                                    .show_ui(ui, |ui| {
                                                        for &style in CtfDaisStyle::all() {
                                                            ui.selectable_value(
                                                                &mut state.map_wizard_config.ctf_dais_style,
                                                                style,
                                                                style.label(),
                                                            );
                                                        }
                                                    });
                                            });

                                            ui.horizontal(|ui| {
                                                ui.checkbox(
                                                    &mut state.map_wizard_config.ctf_include_sniper_perches,
                                                    "Elevated Sniper Overwatch Decks (SniperRifle + RifleAmmo)",
                                                );
                                                if ui.small_button("⚄ Randomize CTF Tactical Setup").clicked() {
                                                    let daises = CtfDaisStyle::all();
                                                    let s = (state.map_wizard_config.seed ^ 0x0C7F_1999) as usize;
                                                    state.map_wizard_config.ctf_dais_style = daises[s % daises.len()];
                                                    state.map_wizard_config.ctf_include_sniper_perches = (s % 3) != 0;
                                                    state.map_wizard_config.ctf_contested_center_powerup = (s % 4) != 0;
                                                }
                                            });

                                            ui.checkbox(
                                                &mut state.map_wizard_config.ctf_contested_center_powerup,
                                                "Contested Midfield Mega-Powerup (UDamage / ShieldBelt at X=0)",
                                            );

                                            // Live CTF topology summary
                                            let preview_topo = ut99_bsp::preview_wizard_topology(&state.map_wizard_config);
                                            let red_flag_m = preview_topo.markers.iter().find(|m| m.kind == "FlagBase" && m.team == Some(0));
                                            let blue_flag_m = preview_topo.markers.iter().find(|m| m.kind == "FlagBase" && m.team == Some(1));
                                            let alt_paths_count = preview_topo.markers.iter().filter(|m| m.kind == "AlternatePath").count();

                                            if let (Some(r), Some(b)) = (red_flag_m, blue_flag_m) {
                                                let dist = r.location.distance(b.location);
                                                let scale_desc = if dist >= 4000.0 {
                                                    "Long-Range Battlefield (Epic Face/LavaGiant Scale)"
                                                } else if dist >= 2800.0 {
                                                    "Medium Combat Concourse (Coret/Gauntlet Scale)"
                                                } else {
                                                    "Close-Quarters Arena (Fast Flag Turnover)"
                                                };
                                                ui.label(
                                                    RichText::new(format!(
                                                        "Polar Separation: {:.0} UU Flag-to-Flag — {} | {} AlternatePath Waypoints",
                                                        dist, scale_desc, alt_paths_count
                                                    ))
                                                    .size(11.0)
                                                    .monospace()
                                                    .color(Color32::from_rgb(255, 200, 140)),
                                                );
                                            }
                                        });
                                }

                                if state.map_wizard_config.toggles.include_weapons_and_pickups {
                                    ui.add_space(4.0);
                                    ui.separator();

                                    // --- ARMORY: WEAPON LOADOUT ---
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(20, 26, 36))
                                        .show(ui, |ui| {
                                            let active_weapons = state.map_wizard_config.active_weapons();
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!("⚔ Armory Loadout ({} Selected)", active_weapons.len()))
                                                        .strong()
                                                        .color(Color32::from_rgb(255, 205, 95)),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Preset:");
                                                egui::ComboBox::from_id_salt("wizard_weapon_preset_combo")
                                                    .selected_text(state.map_wizard_config.weapon_preset.label())
                                                    .width(280.0)
                                                    .show_ui(ui, |ui| {
                                                        for &preset in WizardWeaponPreset::all() {
                                                            if ui.selectable_label(state.map_wizard_config.weapon_preset == preset, preset.label()).clicked() {
                                                                state.map_wizard_config.apply_weapon_preset(preset);
                                                            }
                                                        }
                                                    });
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new("📦 Munizioni di Base per Arma:").color(Color32::from_rgb(150, 220, 255)));
                                                ui.add(egui::Slider::new(&mut state.map_wizard_config.default_ammo_per_weapon, 0..=10).text("casse"));
                                                if ui.small_button("Reset").on_hover_text("Ripristina 1 cassa per arma e rimuove override personalizzati").clicked() {
                                                    state.map_wizard_config.default_ammo_per_weapon = 1;
                                                    state.map_wizard_config.custom_ammo_per_weapon.clear();
                                                }
                                            });

                                            ui.horizontal_wrapped(|ui| {
                                                if ui
                                                    .small_button(
                                                        RichText::new("⚄ Randomize Weapons")
                                                            .color(Color32::from_rgb(255, 215, 90)),
                                                    )
                                                    .on_hover_text(
                                                        "Generate a random balanced weapon loadout (guaranteed hitscan + splash coverage)",
                                                    )
                                                    .clicked()
                                                {
                                                    state.map_wizard_config.randomize_weapon_loadout();
                                                }
                                                if ui.small_button("✔ Select All Stock").clicked() {
                                                    state.map_wizard_config.custom_weapon_loadout = STOCK_TOURNAMENT_WEAPONS.iter().map(|w| w.class_name.to_string()).collect();
                                                    state.map_wizard_config.weapon_preset = WizardWeaponPreset::Custom;
                                                }
                                                if ui.small_button("✖ Clear All").clicked() {
                                                    state.map_wizard_config.custom_weapon_loadout.clear();
                                                    state.map_wizard_config.weapon_preset = WizardWeaponPreset::Custom;
                                                }
                                                if ui.small_button("+ Unreal 1 Pack").on_hover_text("Append Classic Unreal 1 weapons to the loadout").clicked() {
                                                    for w in CLASSIC_UNREAL_WEAPONS {
                                                        if !state.map_wizard_config.custom_weapon_loadout.iter().any(|cw| cw.eq_ignore_ascii_case(w.class_name)) {
                                                            state.map_wizard_config.custom_weapon_loadout.push(w.class_name.to_string());
                                                        }
                                                    }
                                                    state.map_wizard_config.weapon_preset = WizardWeaponPreset::Custom;
                                                }
                                            });

                                            ui.add_space(2.0);
                                            ui.label(RichText::new("Stock Tournament Weapons:").size(11.0).color(Color32::from_rgb(180, 200, 220)));

                                            // 2-Column Grid of Weapons
                                            egui::Grid::new("wizard_weapons_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 4.0])
                                                .show(ui, |ui| {
                                                    for (idx, w) in STOCK_TOURNAMENT_WEAPONS.iter().enumerate() {
                                                        let mut selected = state.map_wizard_config.custom_weapon_loadout.iter().any(|cw| cw.eq_ignore_ascii_case(w.class_name));
                                                        let mut ammo_count = state.map_wizard_config.ammo_count_for_weapon(w.class_name);
                                                        ui.horizontal(|ui| {
                                                            let label = format!("{} {}", w.icon, w.display_name);
                                                            if ui.checkbox(&mut selected, label).changed() {
                                                                state.map_wizard_config.weapon_preset = WizardWeaponPreset::Custom;
                                                                if selected {
                                                                    if !state.map_wizard_config.custom_weapon_loadout.iter().any(|cw| cw.eq_ignore_ascii_case(w.class_name)) {
                                                                        state.map_wizard_config.custom_weapon_loadout.push(w.class_name.to_string());
                                                                    }
                                                                } else {
                                                                    state.map_wizard_config.custom_weapon_loadout.retain(|cw| !cw.eq_ignore_ascii_case(w.class_name));
                                                                }
                                                            }
                                                            if let Some(ammo_name) = w.ammo_display {
                                                                ui.label(RichText::new(format!("📦 {}:", ammo_name)).size(10.0).color(Color32::from_rgb(180, 200, 230)));
                                                                if ui.add(egui::DragValue::new(&mut ammo_count).range(0..=20).speed(0.1)).changed() {
                                                                    state.map_wizard_config.custom_ammo_per_weapon.insert(w.class_name.to_string(), ammo_count);
                                                                }
                                                            }
                                                        });
                                                        if idx % 2 == 1 {
                                                            ui.end_row();
                                                        }
                                                    }
                                                });

                                            egui::CollapsingHeader::new(
                                                RichText::new("Classic Unreal 1 Arsenal (9 Functional Weapons)")
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(170, 210, 255)),
                                            )
                                            .default_open(false)
                                            .show(ui, |ui| {
                                                egui::Grid::new("wizard_classic_weapons_grid")
                                                    .num_columns(2)
                                                    .spacing([12.0, 4.0])
                                                    .show(ui, |ui| {
                                                        for (idx, w) in CLASSIC_UNREAL_WEAPONS.iter().enumerate() {
                                                            let mut selected = state.map_wizard_config.custom_weapon_loadout.iter().any(|cw| cw.eq_ignore_ascii_case(w.class_name));
                                                            let mut ammo_count = state.map_wizard_config.ammo_count_for_weapon(w.class_name);
                                                            ui.horizontal(|ui| {
                                                                let label = format!("{} {}", w.icon, w.display_name);
                                                                if ui.checkbox(&mut selected, label).changed() {
                                                                    state.map_wizard_config.weapon_preset = WizardWeaponPreset::Custom;
                                                                    if selected {
                                                                        if !state.map_wizard_config.custom_weapon_loadout.iter().any(|cw| cw.eq_ignore_ascii_case(w.class_name)) {
                                                                            state.map_wizard_config.custom_weapon_loadout.push(w.class_name.to_string());
                                                                        }
                                                                    } else {
                                                                        state.map_wizard_config.custom_weapon_loadout.retain(|cw| !cw.eq_ignore_ascii_case(w.class_name));
                                                                    }
                                                                }
                                                                if let Some(ammo_name) = w.ammo_display {
                                                                    ui.label(RichText::new(format!("📦 {}:", ammo_name)).size(10.0).color(Color32::from_rgb(180, 200, 230)));
                                                                    if ui.add(egui::DragValue::new(&mut ammo_count).range(0..=20).speed(0.1)).changed() {
                                                                        state.map_wizard_config.custom_ammo_per_weapon.insert(w.class_name.to_string(), ammo_count);
                                                                    }
                                                                }
                                                            });
                                                            if idx % 2 == 1 {
                                                                ui.end_row();
                                                            }
                                                        }
                                                    });
                                            });
                                        });

                                    ui.add_space(4.0);

                                    // --- POWERUPS, ARMOR & HEALTH PICKUPS ---
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(20, 26, 36))
                                        .show(ui, |ui| {
                                            let active_powerups = state.map_wizard_config.active_powerups();
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    RichText::new(format!("⚡ Powerups, Armor & Health ({} Selected)", active_powerups.len()))
                                                        .strong()
                                                        .color(Color32::from_rgb(110, 230, 180)),
                                                );
                                            });

                                            ui.horizontal(|ui| {
                                                ui.label("Preset:");
                                                egui::ComboBox::from_id_salt("wizard_powerup_preset_combo")
                                                    .selected_text(state.map_wizard_config.powerup_preset.label())
                                                    .width(280.0)
                                                    .show_ui(ui, |ui| {
                                                        for &preset in WizardPowerupPreset::all() {
                                                            if ui.selectable_label(state.map_wizard_config.powerup_preset == preset, preset.label()).clicked() {
                                                                state.map_wizard_config.apply_powerup_preset(preset);
                                                            }
                                                        }
                                                    });
                                            });

                                            ui.horizontal_wrapped(|ui| {
                                                if ui
                                                    .small_button(
                                                        RichText::new("⚄ Randomize Powerups")
                                                            .color(Color32::from_rgb(255, 215, 90)),
                                                    )
                                                    .on_hover_text(
                                                        "Generate a random pickup loadout (guaranteed baseline Health + Armor + random Relics)",
                                                    )
                                                    .clicked()
                                                {
                                                    state.map_wizard_config.randomize_powerup_loadout();
                                                }
                                                if ui.small_button("✔ Select All").clicked() {
                                                    state.map_wizard_config.custom_powerup_loadout = STOCK_POWERUPS_AND_PICKUPS.iter().map(|p| p.class_name.to_string()).collect();
                                                    state.map_wizard_config.powerup_preset = WizardPowerupPreset::Custom;
                                                }
                                                if ui.small_button("✖ Clear All").clicked() {
                                                    state.map_wizard_config.custom_powerup_loadout.clear();
                                                    state.map_wizard_config.powerup_preset = WizardPowerupPreset::Custom;
                                                }
                                            });

                                            ui.add_space(2.0);
                                            egui::Grid::new("wizard_powerups_grid")
                                                .num_columns(2)
                                                .spacing([12.0, 4.0])
                                                .show(ui, |ui| {
                                                    for (idx, p) in STOCK_POWERUPS_AND_PICKUPS.iter().enumerate() {
                                                        let mut selected = state.map_wizard_config.custom_powerup_loadout.iter().any(|cp| cp.eq_ignore_ascii_case(p.class_name));
                                                        let label = format!("{} {}", p.icon, p.display_name);
                                                        let mut resp = ui.checkbox(&mut selected, label);
                                                        if !p.description.is_empty() {
                                                            resp = resp.on_hover_text(p.description);
                                                        }
                                                        if resp.changed() {
                                                            state.map_wizard_config.powerup_preset = WizardPowerupPreset::Custom;
                                                            if selected {
                                                                if !state.map_wizard_config.custom_powerup_loadout.iter().any(|cp| cp.eq_ignore_ascii_case(p.class_name)) {
                                                                    state.map_wizard_config.custom_powerup_loadout.push(p.class_name.to_string());
                                                                }
                                                            } else {
                                                                state.map_wizard_config.custom_powerup_loadout.retain(|cp| !cp.eq_ignore_ascii_case(p.class_name));
                                                            }
                                                        }
                                                        if idx % 2 == 1 {
                                                            ui.end_row();
                                                        }
                                                    }
                                                });
                                        });

                                    ui.add_space(4.0);

                                    // --- ITEM DENSITY SELECTOR ---
                                    ui.horizontal(|ui| {
                                        ui.label("Armory Item Density:");
                                        for &density in WizardItemDensity::all() {
                                            let resp = ui.selectable_value(
                                                &mut state.map_wizard_config.item_density,
                                                density,
                                                match density {
                                                    WizardItemDensity::Sparse => "Sparse (1x Ammo)",
                                                    WizardItemDensity::Standard => "Standard",
                                                    WizardItemDensity::Abundant => "Abundant (2x Weapons & Ammo)",
                                                },
                                            );
                                            match density {
                                                WizardItemDensity::Sparse => {
                                                    resp.on_hover_text("Collocazione parsimoniosa di armi e munizioni singole per gameplay tattico");
                                                }
                                                WizardItemDensity::Standard => {
                                                    resp.on_hover_text("Cadenza autentica UT99: tutte le armi selezionate collocate bilanciatamente");
                                                }
                                                WizardItemDensity::Abundant => {
                                                    resp.on_hover_text("Raddoppia i punti di spawn delle armi (doppie copie), raddoppia le casse di munizioni (2x) e aumenta i restorativi");
                                                }
                                            }
                                        }
                                    });

                                    ui.add_space(6.0);
                                    egui::Frame::group(ui.style())
                                        .fill(Color32::from_rgb(20, 24, 32))
                                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(50, 110, 190)))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.checkbox(
                                                    &mut state.map_wizard_config.item_quota.enabled,
                                                    RichText::new("🎯 Exact Item Quota Mode (Absolute Quantities & Fair Map Distribution)")
                                                        .strong()
                                                        .color(Color32::from_rgb(110, 200, 255)),
                                                ).on_hover_text("Decide the absolute count of specific items (armors, redeemers, rifles, boots, medkits, etc.) distributed equitably across all map zones and rooms.");
                                            });

                                            if state.map_wizard_config.item_quota.enabled {
                                                ui.add_space(4.0);
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(RichText::new("Quota Presets:").italics());
                                                    if ui.small_button("⚖ Balanced / Tournament").on_hover_text("1x Redeemer, 1x of each major weapon, 1x Shield Belt, 2x Body Armor, 1x Keg, 1x UDamage, Jump Boots").clicked() {
                                                        state.map_wizard_config.item_quota = WizardItemQuotaConfig::competitive_tournament();
                                                    }
                                                    if ui.small_button("💥 Heavy Carnage").on_hover_text("High weapon counts (2x Redeemers, 4x Rockets, 3x Flak, etc.)").clicked() {
                                                        state.map_wizard_config.item_quota = WizardItemQuotaConfig::heavy_carnage();
                                                    }
                                                    if ui.small_button("🎯 Minimal 1v1 Duel").on_hover_text("Lean duel weapon set (1 Shock, 1 Rocket, 1 Flak, 1 Sniper, 1 Shield Belt, 1 Keg)").clicked() {
                                                        state.map_wizard_config.item_quota = WizardItemQuotaConfig::minimal_duel();
                                                    }
                                                    if ui.small_button("↺ Clear All (Zero)").on_hover_text("Set all item quota counts to zero").clicked() {
                                                        state.map_wizard_config.item_quota.clear_all();
                                                    }
                                                });

                                                ui.add_space(4.0);
                                                let q = &state.map_wizard_config.item_quota;
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(RichText::new(format!(
                                                        "Total: {} items | ⚔ Weapons: {} | 🛡 Armor: {} | 🧪 Health: {} | 💜 Powerups: {}",
                                                        q.total_items(),
                                                        q.total_weapons(),
                                                        q.total_armor(),
                                                        q.total_health(),
                                                        q.total_powerups(),
                                                    )).color(Color32::from_rgb(240, 230, 130)).strong());
                                                });

                                                ui.add_space(4.0);
                                                egui::Grid::new("wizard_item_quota_grid")
                                                    .num_columns(4)
                                                    .spacing([14.0, 4.0])
                                                    .show(ui, |ui| {
                                                        let q = &mut state.map_wizard_config.item_quota;

                                                        // Weapons
                                                        ui.label("☢ Redeemer:");
                                                        ui.add(egui::DragValue::new(&mut q.redeemer).range(0..=200));
                                                        ui.label("🚀 Rocket Launcher:");
                                                        ui.add(egui::DragValue::new(&mut q.rocket_launcher).range(0..=200));
                                                        ui.end_row();

                                                        ui.label("💥 Flak Cannon:");
                                                        ui.add(egui::DragValue::new(&mut q.flak_cannon).range(0..=200));
                                                        ui.label("⚡ Shock Rifle:");
                                                        ui.add(egui::DragValue::new(&mut q.shock_rifle).range(0..=200));
                                                        ui.end_row();

                                                        ui.label("🎯 Sniper Rifle:");
                                                        ui.add(egui::DragValue::new(&mut q.sniper_rifle).range(0..=200));
                                                        ui.label("🔥 Minigun:");
                                                        ui.add(egui::DragValue::new(&mut q.minigun).range(0..=200));
                                                        ui.end_row();

                                                        ui.label("🟢 Pulse Gun:");
                                                        ui.add(egui::DragValue::new(&mut q.pulse_gun).range(0..=200));
                                                        ui.label("💿 Ripper:");
                                                        ui.add(egui::DragValue::new(&mut q.ripper).range(0..=200));
                                                        ui.end_row();

                                                        ui.label("☣ Bio Rifle:");
                                                        ui.add(egui::DragValue::new(&mut q.bio_rifle).range(0..=200));
                                                        ui.label("🔫 Enforcer:");
                                                        ui.add(egui::DragValue::new(&mut q.enforcer).range(0..=200));
                                                        ui.end_row();

                                                        ui.label("🪚 Chainsaw:");
                                                        ui.add(egui::DragValue::new(&mut q.chainsaw).range(0..=200));
                                                        ui.label("");
                                                        ui.label("");
                                                        ui.end_row();

                                                        // Armor
                                                        ui.label(RichText::new("⚡ Shield Belt (+150):").color(Color32::from_rgb(255, 230, 100)));
                                                        ui.add(egui::DragValue::new(&mut q.shield_belt).range(0..=200));
                                                        ui.label(RichText::new("🛡 Body Armor (+100):").color(Color32::from_rgb(200, 220, 255)));
                                                        ui.add(egui::DragValue::new(&mut q.body_armor).range(0..=200));
                                                        ui.end_row();

                                                        ui.label(RichText::new("🦵 Thigh Pads (+50):").color(Color32::from_rgb(180, 200, 220)));
                                                        ui.add(egui::DragValue::new(&mut q.thigh_pads).range(0..=200));
                                                        ui.label("");
                                                        ui.label("");
                                                        ui.end_row();

                                                        // Health
                                                        ui.label(RichText::new("🧪 Super Keg (+100):").color(Color32::from_rgb(120, 255, 120)));
                                                        ui.add(egui::DragValue::new(&mut q.super_keg_health).range(0..=200));
                                                        ui.label(RichText::new("➕ MedBox (+20):").color(Color32::from_rgb(150, 255, 150)));
                                                        ui.add(egui::DragValue::new(&mut q.medbox).range(0..=200));
                                                        ui.end_row();

                                                        ui.label(RichText::new("💉 Health Vials (+5):").color(Color32::from_rgb(170, 255, 170)));
                                                        ui.add(egui::DragValue::new(&mut q.health_vials).range(0..=200));
                                                        ui.label("");
                                                        ui.label("");
                                                        ui.end_row();

                                                        // Powerups
                                                        ui.label(RichText::new("💜 UDamage (Amp x3):").color(Color32::from_rgb(240, 140, 255)));
                                                        ui.add(egui::DragValue::new(&mut q.udamage).range(0..=200));
                                                        ui.label(RichText::new("🥾 Jump Boots:").color(Color32::from_rgb(255, 180, 100)));
                                                        ui.add(egui::DragValue::new(&mut q.jump_boots).range(0..=200));
                                                        ui.end_row();

                                                        ui.label(RichText::new("👁 Invisibility:").color(Color32::from_rgb(160, 220, 240)));
                                                        ui.add(egui::DragValue::new(&mut q.invisibility).range(0..=200));
                                                        ui.label("");
                                                        ui.label("");
                                                        ui.end_row();
                                                    });
                                            }
                                        });

                                    ui.add_space(4.0);
                                    ui.horizontal(|ui| {
                                        ui.label("Atmospheric Distance Fog:");
                                        egui::ComboBox::from_id_salt("wizard_distance_fog_combo")
                                            .selected_text(match state.map_wizard_config.distance_fog {
                                                WizardFogPreset::None => "None (Clear UT99 View)",
                                                WizardFogPreset::CryptMurk => "Crypt Murk (Dark Green/Grey)",
                                                WizardFogPreset::SpaceHaze => "Space Haze (Deep Blue)",
                                                WizardFogPreset::IndustrialSmog => "Industrial Smog (Amber/Brown)",
                                                WizardFogPreset::LavaGlow => "Lava Glow (Fiery Orange/Red)",
                                            })
                                            .width(220.0)
                                            .show_ui(ui, |ui| {
                                                ui.selectable_value(&mut state.map_wizard_config.distance_fog, WizardFogPreset::None, "None (Clear UT99 View)");
                                                ui.selectable_value(&mut state.map_wizard_config.distance_fog, WizardFogPreset::CryptMurk, "Crypt Murk (Dark Green/Grey)");
                                                ui.selectable_value(&mut state.map_wizard_config.distance_fog, WizardFogPreset::SpaceHaze, "Space Haze (Deep Blue)");
                                                ui.selectable_value(&mut state.map_wizard_config.distance_fog, WizardFogPreset::IndustrialSmog, "Industrial Smog (Amber/Brown)");
                                                ui.selectable_value(&mut state.map_wizard_config.distance_fog, WizardFogPreset::LavaGlow, "Lava Glow (Fiery Orange/Red)");
                                            });
                                    });
                                }

                                ui.separator();
                                ui.checkbox(
                                    &mut state.map_wizard_config.auto_run_build_and_audit,
                                    RichText::new(
                                        "✔ Auto-Run Build All (BSP + Light + Paths) & Level Doctor Audit",
                                    )
                                    .strong(),
                                );
                            });
                            ui.add_space(6.0);
                        }

                        // --- STEP 5: Execution, Save & Live Playtest ---
                        if show_all || state.map_wizard_step == 4 {
                            egui::CollapsingHeader::new(
                                RichText::new("5. Generate, Validate & Playtest")
                                    .strong()
                                    .color(Color32::from_rgb(130, 205, 255)),
                            )
                            .default_open(true)
                            .show(ui, |ui| {
                                ui.label(
                                    "Generates a 100% valid UT99 map with atomic Undo/Redo support (Ctrl+Z):",
                                );
                                ui.add_space(4.0);

                                let out_dir = state.wizard_output_directory();
                                let clean_map_name = if state.map_wizard_config.map_name.to_ascii_lowercase().ends_with(".unr") {
                                    state.map_wizard_config.map_name.clone()
                                } else {
                                    format!("{}.unr", state.map_wizard_config.map_name)
                                };
                                let target_preview_path = out_dir.join(&clean_map_name);

                                egui::Frame::NONE
                                    .fill(Color32::from_rgba_unmultiplied(25, 30, 42, 220))
                                    .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(55, 75, 115)))
                                    .corner_radius(4.0)
                                    .inner_margin(6.0)
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("📁 Destination:").strong().color(Color32::from_rgb(180, 205, 235)));
                                            ui.monospace(
                                                RichText::new(target_preview_path.display().to_string())
                                                    .size(11.0)
                                                    .color(Color32::from_rgb(110, 225, 140)),
                                            );
                                            if ui.button("📁 Browse...").clicked() {
                                                if let Some(folder) = rfd::FileDialog::new()
                                                    .set_directory(&out_dir)
                                                    .pick_folder()
                                                {
                                                    state.wizard_output_dir = Some(folder);
                                                }
                                            }
                                            if ui.button("📂 Open Folder")
                                                .on_hover_text("Open maps export destination directory in Windows Explorer")
                                                .clicked()
                                            {
                                                let _ = std::fs::create_dir_all(&out_dir);
                                                state.reveal_in_file_explorer(&out_dir);
                                            }
                                        });
                                    });
                                ui.add_space(4.0);

                                ui.horizontal(|ui| {
                                    show_wizard_audit_status(ui, state);
                                });
                                ui.add_space(4.0);

                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .button(
                                            RichText::new("⚄ Re-Roll All Settings")
                                                .size(14.0)
                                                .strong()
                                                .color(Color32::from_rgb(255, 215, 95)),
                                        )
                                        .on_hover_text(
                                            "Randomize all wizard steps (1..4) in one click and preview the new layout before generating",
                                        )
                                        .clicked()
                                    {
                                        state.randomize_all_wizard_settings();
                                    }

                                    if ui
                                        .button(
                                            RichText::new("★ Generate in 3D Editor")
                                                .size(14.0)
                                                .strong()
                                                .color(Color32::from_rgb(100, 255, 140)),
                                        )
                                        .clicked()
                                    {
                                        state.generate_map_from_wizard();
                                    }

                                    if ui
                                        .button(
                                            RichText::new("▶ Generate & Save .unr")
                                                .size(14.0)
                                                .strong()
                                                .color(Color32::from_rgb(110, 210, 255)),
                                        )
                                        .on_hover_text("Generates the map, runs Level Doctor pre-export audit & auto-repairs, and saves to .unr")
                                        .clicked()
                                    {
                                        match state.generate_wizard_map_verified() {
                                            Ok(_) => {
                                                let fname = if state
                                                    .map_wizard_config
                                                    .map_name
                                                    .to_ascii_lowercase()
                                                    .ends_with(".unr")
                                                {
                                                    state.map_wizard_config.map_name.clone()
                                                } else {
                                                    format!("{}.unr", state.map_wizard_config.map_name)
                                                };
                                                let out_dir = state.wizard_output_directory();
                                                let target_path = out_dir.join(&fname);
                                                match state.save_wizard_unr_verified(&target_path) {
                                                    Ok(report) => {
                                                        if let Some(reason) = &report.native_reason {
                                                            state.toast_warning(format!("Package validated; saved with native writer: {reason}"));
                                                        } else if let Some(warning) = &report.engine_warning {
                                                            state.toast_warning(format!("Package validated; UT engine warning: {warning}"));
                                                        } else {
                                                            state.toast_success(format!("Package validated; saved with UT engine to {}", report.path.display()));
                                                        }
                                                    }
                                                    Err(e) => {
                                                        state.toast_error(format!("Save failed: {e}"));
                                                    }
                                                }
                                            }
                                            Err(errors) => {
                                                state.toast_error(format!(
                                                    "Export blocked: Level Doctor detected {} error(s).",
                                                    errors.len()
                                                ));
                                            }
                                        }
                                    }

                                    if ui
                                        .button("📥 Install in UT")
                                        .on_hover_text("Copies the saved map into the UT99 Maps folder for playtesting (never overwrites non-generated maps)")
                                        .clicked()
                                    {
                                        match state.map_path.clone().filter(|p| p.exists()) {
                                            Some(saved) => match state.install_wizard_map_in_ut(&saved) {
                                                Ok(dest) => state.toast_success(format!("Installed to {}", dest.display())),
                                                Err(e) => state.toast_error(format!("Install failed: {e}")),
                                            },
                                            None => state.toast_error("Save the map first (Generate & Save .unr)"),
                                        }
                                    }

                                    if ui
                                        .button(
                                            RichText::new("⚡ Generate, Save & Playtest (F8)")
                                                .size(14.0)
                                                .strong()
                                                .color(Color32::from_rgb(255, 195, 70)),
                                        )
                                        .on_hover_text("Generates the map, verifies 0 Level Doctor errors, saves .unr, and launches UT99")
                                        .clicked() && state.preflight_playtest().is_ok()
                                    {
                                        state.sync_audio_installation();
                                        match state.generate_wizard_map_verified() {
                                            Ok(_) => {
                                                if let Some(target) = state.map_path.clone() {
                                                    match state.save_wizard_unr_verified(&target) {
                                                        Ok(report) => {
                                                            if let Some(reason) = &report.native_reason {
                                                                state.toast_warning(format!("Package validated; native writer used: {reason}"));
                                                            } else if let Some(warning) = &report.engine_warning {
                                                                state.toast_warning(format!("Package validated; UT engine warning: {warning}"));
                                                            }
                                                            match state.install_wizard_map_in_ut(&report.path) {
                                                                Ok(installed) => {
                                                                    let _ = state.launch_saved_map_in_ut99(&installed);
                                                                }
                                                                Err(error) => state.toast_error(format!("Playtest install failed: {error}")),
                                                            }
                                                        }
                                                        Err(error) => state.toast_error(format!("Playtest save failed: {error}")),
                                                    }
                                                } else {
                                                    state.toast_error("Playtest save failed: no map destination");
                                                }
                                            }
                                            Err(errors) => {
                                                state.toast_error(format!(
                                                    "Playtest blocked: Level Doctor detected {} error(s).",
                                                    errors.len()
                                                ));
                                            }
                                        }
                                    }
                                });
                            });
                        }

                        // Step Back / Next Navigation Buttons when in step-by-step mode
                        if !show_all {
                            ui.add_space(10.0);
                            ui.separator();
                            ui.horizontal(|ui| {
                                if state.map_wizard_step > 0 {
                                    if ui.button("← Previous Step").clicked() {
                                        state.map_wizard_step -= 1;
                                    }
                                }
                                if state.map_wizard_step < 4 {
                                    if ui
                                        .button(
                                            RichText::new("Next Step →")
                                                .strong()
                                                .color(Color32::from_rgb(120, 220, 255)),
                                        )
                                        .clicked()
                                    {
                                        state.map_wizard_step += 1;
                                    }
                                } else if ui
                                    .button(
                                        RichText::new("★ Generate Valid Map Now")
                                            .strong()
                                            .color(Color32::from_rgb(100, 255, 140)),
                                    )
                                    .clicked()
                                {
                                    state.generate_map_from_wizard();
                                }
                            });
                        }
                    });

                // =========================================================
                // RIGHT COLUMN: Real-Time 2D Schematic Preview & Telemetry
                // =========================================================
                // =========================================================
                // RIGHT COLUMN: Real-Time 2D Schematic Preview & Telemetry
                // =========================================================
                let preview = preview_wizard_topology(&state.map_wizard_config);

                columns[1].set_clip_rect(columns[1].max_rect());
                egui::ScrollArea::vertical()
                    .id_salt("map_wizard_right_scroll")
                    .auto_shrink([false, false])
                    .show(&mut columns[1], |ui| {
                        ui.set_max_width(ui.available_width());

                        if !preview.warnings.is_empty() {
                            egui::CollapsingHeader::new(format!("⚠ Room connection checks ({})", preview.warnings.len()))
                                .default_open(true)
                                .show(ui, |ui| {
                                    for warning in &preview.warnings {
                                        ui.colored_label(Color32::YELLOW, warning);
                                    }
                                });
                        }

                        if let Some(graph) = &preview.procedural_graph {
                            egui::CollapsingHeader::new("Procedural room graph (before edits)")
                                .default_open(false)
                                .show(ui, |ui| {
                                    ui.label(format!("{} rooms, {} links, {} components, {} independent cycles; minimum degree {}.",
                                        graph.room_count, graph.edges.len(), graph.components, graph.independent_cycles, graph.minimum_degree));
                                    ui.label(format!("Links: {} initial, {} sampled, {} added for minimum degree. Effective local probability: {:.0}%.",
                                        graph.tree_edges, graph.random_edges, graph.minimum_degree_edges, graph.effective_extra_probability * 100.0));
                                    ui.weak("Logical graph before room edits and custom connections; it does not verify physical traversal.");
                                    for (from, to) in &graph.edges {
                                        ui.label(format!("{from} ↔ {to}"));
                                    }
                                });
                        }

                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("▦ Interactive 2D Schematic")
                                    .strong()
                                    .color(Color32::from_rgb(140, 220, 255)),
                            );

                            let custom_count = state.map_wizard_config.custom_rooms.len()
                                + state.map_wizard_config.custom_connections.len()
                                + state.map_wizard_config.room_overrides.len()
                                + state.map_wizard_config.custom_markers.len()
                                + state.map_wizard_config.suppressed_markers.len();
                            if custom_count > 0 {
                                if ui
                                    .button(
                                        RichText::new(format!("↺ Reset ({custom_count})"))
                                            .size(10.5)
                                            .color(Color32::from_rgb(255, 140, 140)),
                                    )
                                    .on_hover_text("Clear all user transformations, custom objects, and rooms")
                                    .clicked()
                                {
                                    state.clear_wizard_custom_overrides();
                                }
                            }
                            if ui
                                .selectable_label(state.wizard_minimap_tool == WizardMinimapTool::Delete, "🗑 Delete")
                                .on_hover_text("Delete Tool: Click a room, corridor, or object to delete/suppress it (or press Del key)")
                                .clicked()
                            {
                                state.wizard_minimap_tool = WizardMinimapTool::Delete;
                                state.wizard_minimap_connect_source = None;
                            }
                            if ui
                                .selectable_label(state.wizard_minimap_tool == WizardMinimapTool::ConnectRooms, "🔗 Connect")
                                .on_hover_text("Connect Tool: Click room A then room B to create a corridor")
                                .clicked()
                            {
                                state.wizard_minimap_tool = WizardMinimapTool::ConnectRooms;
                                state.wizard_minimap_connect_source = None;
                            }
                            if ui
                                .selectable_label(state.wizard_minimap_tool == WizardMinimapTool::AddRoom, "➕ Room")
                                .on_hover_text("Add Room Tool: Click on canvas to place a grid-aligned room")
                                .clicked()
                            {
                                state.wizard_minimap_tool = WizardMinimapTool::AddRoom;
                                state.wizard_minimap_connect_source = None;
                            }
                            if ui
                                .selectable_label(state.wizard_minimap_tool == WizardMinimapTool::AddObject, "● Object")
                                .on_hover_text("Add Object Tool: Click on canvas to place a gameplay pickup, start, or light")
                                .clicked()
                            {
                                state.wizard_minimap_tool = WizardMinimapTool::AddObject;
                                state.wizard_minimap_connect_source = None;
                            }
                            if state.wizard_minimap_tool == WizardMinimapTool::AddObject {
                                egui::ComboBox::from_id_salt("wizard_add_object_kind_combo")
                                    .selected_text(&state.wizard_minimap_add_object_kind)
                                    .width(110.0)
                                    .show_ui(ui, |ui| {
                                        for kind in &[
                                            "ShockRifle", "RocketLauncher", "FlakCannon", "SniperRifle",
                                            "Minigun", "PulseGun", "BioRifle", "PlayerStart", "Health",
                                            "HealthVial", "Armor", "ShieldBelt", "UDamage", "Light", "Teleporter"
                                        ] {
                                            ui.selectable_value(&mut state.wizard_minimap_add_object_kind, kind.to_string(), *kind);
                                        }
                                    });
                            }
                            if ui
                                .selectable_label(state.wizard_minimap_tool == WizardMinimapTool::Inspect, "🔍 Inspect")
                                .on_hover_text("Inspect & Transform Tool: Click to select, drag to move, drag handles to resize")
                                .clicked()
                            {
                                state.wizard_minimap_tool = WizardMinimapTool::Inspect;
                                state.wizard_minimap_connect_source = None;
                            }

                            ui.separator();
                            let zoom_pct = (state.wizard_minimap_zoom * 100.0).round();
                            if ui
                                .small_button(RichText::new(format!("⌕ {zoom_pct:.0}%")).color(Color32::from_rgb(180, 230, 255)))
                                .on_hover_text("Scroll wheel over canvas to Zoom • Right/Middle Click Drag to Pan • Click here to Reset View")
                                .clicked()
                            {
                                state.wizard_minimap_zoom = 1.0;
                                state.wizard_minimap_pan = egui::Vec2::ZERO;
                            }

                            ui.label("Snap:");
                            let snap_val = state.wizard_minimap_grid_snap;
                            let snap_lbl = if snap_val <= 0.0 { "Free".to_string() } else { format!("{snap_val:.0} UU") };
                            egui::ComboBox::from_id_salt("wizard_grid_snap_combo")
                                .selected_text(snap_lbl)
                                .width(74.0)
                                .show_ui(ui, |ui| {
                                    for s in [0.0, 16.0, 32.0, 64.0, 128.0, 256.0] {
                                        let l = if s <= 0.0 { "Free".to_string() } else { format!("{s:.0} UU") };
                                        ui.selectable_value(&mut state.wizard_minimap_grid_snap, s, l);
                                    }
                                });

                            if ui
                                .selectable_label(state.wizard_minimap_show_bot_paths, "☊ Paths")
                                .on_hover_text("Toggle real-time Bot AI Path reachability lines (<= 850 UU)")
                                .clicked()
                            {
                                state.wizard_minimap_show_bot_paths = !state.wizard_minimap_show_bot_paths;
                            }
                        });

                        // Keyboard shortcuts: Delete/Backspace to remove, Ctrl+D to duplicate
                        if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace)) {
                            state.delete_wizard_selection();
                        }
                        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::D)) {
                            state.duplicate_wizard_selection();
                        }

                        // Teleporter Connectivity & Safety Alert Banner
                        if preview.teleport_audit.teleporter_count > 0 {
                            let audit = &preview.teleport_audit;
                            if !audit.is_healthy {
                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgb(45, 24, 20))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(255, 140, 60)))
                                    .show(ui, |ui| {
                                        ui.horizontal_wrapped(|ui| {
                                            ui.label(
                                                RichText::new("⚠ Teleporter Routing Warning:")
                                                    .strong()
                                                    .color(Color32::from_rgb(255, 180, 80)),
                                            );
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} teleporter(s) placed, {} issue(s) detected.",
                                                    audit.teleporter_count,
                                                    audit.issues.len()
                                                ))
                                                .size(11.5)
                                                .color(Color32::from_rgb(240, 220, 210)),
                                            );
                                            if ui
                                                .button(
                                                    RichText::new("🛠 Auto-Fix Teleporters")
                                                        .strong()
                                                        .color(Color32::from_rgb(110, 255, 140)),
                                                )
                                                .on_hover_text("Automatically resolve lone teleporters, connect dangling URLs, and prevent loop telefrags")
                                                .clicked()
                                            {
                                                state.auto_fix_wizard_teleporters();
                                            }
                                        });
                                        for issue in audit.issues.iter().take(3) {
                                            let col = match issue.severity {
                                                WizardTeleportIssueSeverity::Error => Color32::from_rgb(255, 100, 100),
                                                _ => Color32::from_rgb(255, 200, 90),
                                            };
                                            ui.label(
                                                RichText::new(format!(" • [{}] {}", issue.teleporter_name, issue.message))
                                                    .size(11.0)
                                                    .color(col),
                                            );
                                        }
                                        if audit.issues.len() > 3 {
                                            ui.label(
                                                RichText::new(format!(" ... and {} more issue(s)", audit.issues.len() - 3))
                                                    .size(10.5)
                                                    .italics()
                                                    .color(Color32::from_rgb(200, 180, 170)),
                                            );
                                        }
                                    });
                                ui.add_space(3.0);
                            } else if audit.teleporter_count >= 2 {
                                egui::Frame::group(ui.style())
                                    .fill(Color32::from_rgb(20, 36, 26))
                                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(70, 190, 100)))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                RichText::new(format!(
                                                    "✔ Teleporters Verified: {} nodes, {} active routes",
                                                    audit.teleporter_count,
                                                    audit.valid_links.len()
                                                ))
                                                .size(11.5)
                                                .color(Color32::from_rgb(110, 255, 140)),
                                            );
                                        });
                                    });
                                ui.add_space(3.0);
                            }
                        }

                        // 2D Schematic Canvas
                        let canvas_size = Vec2::new(ui.available_width().max(320.0), 340.0);
                        let (rect, response) = ui.allocate_exact_size(canvas_size, Sense::click_and_drag());
                        let painter = ui.painter_at(rect);

                        // Interactive Pan (Right-click or Middle-click drag)
                        if response.dragged_by(egui::PointerButton::Secondary) || response.dragged_by(egui::PointerButton::Middle) {
                            state.wizard_minimap_pan += response.drag_delta();
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                        }

                        // Interactive Zoom (Mouse wheel scroll over canvas)
                        if response.hovered() {
                            let scroll_y = ui.input(|i| i.raw_scroll_delta.y);
                            if scroll_y != 0.0 {
                                let zoom_mul = if scroll_y > 0.0 { 1.12 } else { 0.89 };
                                state.wizard_minimap_zoom = (state.wizard_minimap_zoom * zoom_mul).clamp(0.25, 4.0);
                            }
                        }

                        // Background
                        painter.rect_filled(rect, 6.0, Color32::from_rgb(16, 20, 28));
                        painter.rect_stroke(
                            rect,
                            6.0,
                            Stroke::new(1.0_f32, Color32::from_rgb(55, 70, 95)),
                            egui::StrokeKind::Inside,
                        );

                        // Compute world bounds for auto-fitting
                        let mut min_x = -1024.0_f32;
                        let mut max_x = 1024.0_f32;
                        let mut min_y = -1024.0_f32;
                        let mut max_y = 1024.0_f32;
                        for r in &preview.rooms {
                            min_x = min_x.min(r.center.x - r.size.x * 0.55);
                            max_x = max_x.max(r.center.x + r.size.x * 0.55);
                            min_y = min_y.min(r.center.y - r.size.y * 0.55);
                            max_y = max_y.max(r.center.y + r.size.y * 0.55);
                        }
                        for m in &preview.markers {
                            min_x = min_x.min(m.location.x - 128.0);
                            max_x = max_x.max(m.location.x + 128.0);
                            min_y = min_y.min(m.location.y - 128.0);
                            max_y = max_y.max(m.location.y + 128.0);
                        }
                        let span_x = (max_x - min_x).max(512.0);
                        let span_y = (max_y - min_y).max(512.0);
                        let pad = 24.0;
                        let usable_w = (rect.width() - pad * 2.0).max(64.0);
                        let usable_h = (rect.height() - pad * 2.0).max(64.0);
                        let base_scale = (usable_w / span_x).min(usable_h / span_y);
                        let scale = base_scale * state.wizard_minimap_zoom;
                        let center_world_x = (min_x + max_x) * 0.5 - state.wizard_minimap_pan.x / scale;
                        let center_world_y = (min_y + max_y) * 0.5 - state.wizard_minimap_pan.y / scale;

                        let to_screen = |wx: f32, wy: f32| -> Pos2 {
                            let sx = rect.center().x + (wx - center_world_x) * scale;
                            let sy = rect.center().y + (wy - center_world_y) * scale;
                            Pos2::new(sx, sy)
                        };

                        let to_world = |pos: Pos2| -> Vec2 {
                            let wx = center_world_x + (pos.x - rect.center().x) / scale;
                            let wy = center_world_y + (pos.y - rect.center().y) / scale;
                            Vec2::new(wx, wy)
                        };

                        // Grid lines
                        let grid_step = 512.0_f32;
                        let mut gx = (min_x / grid_step).floor() * grid_step;
                        while gx <= max_x {
                            let p0 = to_screen(gx, min_y);
                            let p1 = to_screen(gx, max_y);
                            painter.line_segment(
                                [
                                    Pos2::new(p0.x.clamp(rect.left(), rect.right()), rect.top() + 2.0),
                                    Pos2::new(p1.x.clamp(rect.left(), rect.right()), rect.bottom() - 2.0),
                                ],
                                Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(60, 75, 100, 70)),
                            );
                            gx += grid_step;
                        }
                        let mut gy = (min_y / grid_step).floor() * grid_step;
                        while gy <= max_y {
                            let p0 = to_screen(min_x, gy);
                            let p1 = to_screen(max_x, gy);
                            painter.line_segment(
                                [
                                    Pos2::new(rect.left() + 2.0, p0.y.clamp(rect.top(), rect.bottom())),
                                    Pos2::new(rect.right() - 2.0, p1.y.clamp(rect.top(), rect.bottom())),
                                ],
                                Stroke::new(0.5_f32, Color32::from_rgba_unmultiplied(60, 75, 100, 70)),
                            );
                            gy += grid_step;
                        }

                        // Symmetry axis indicator
                        let origin_s = to_screen(0.0, 0.0);
                        if rect.contains(origin_s) {
                            painter.line_segment(
                                [
                                    Pos2::new(origin_s.x, rect.top() + 4.0),
                                    Pos2::new(origin_s.x, rect.bottom() - 4.0),
                                ],
                                Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 210, 80, 90)),
                            );
                        }

                        // Draw CTF Flag-to-Flag Tactical Vector & Route Networks
                        let red_flag = preview.markers.iter().find(|m| m.kind == "FlagBase" && m.team == Some(0));
                        let blue_flag = preview.markers.iter().find(|m| m.kind == "FlagBase" && m.team == Some(1));
                        if let (Some(rf), Some(bf)) = (red_flag, blue_flag) {
                            let r_pos = to_screen(rf.location.x, rf.location.y);
                            let b_pos = to_screen(bf.location.x, bf.location.y);

                            painter.line_segment(
                                [r_pos, b_pos],
                                Stroke::new(2.2_f32, Color32::from_rgba_unmultiplied(255, 190, 70, 140)),
                            );

                            painter.circle_stroke(
                                r_pos,
                                11.0,
                                Stroke::new(2.0_f32, Color32::from_rgba_unmultiplied(255, 50, 50, 220)),
                            );
                            painter.circle_stroke(
                                b_pos,
                                11.0,
                                Stroke::new(2.0_f32, Color32::from_rgba_unmultiplied(50, 130, 255, 220)),
                            );

                            let mid_pos = Pos2::new((r_pos.x + b_pos.x) * 0.5, (r_pos.y + b_pos.y) * 0.5);
                            let dist_uu = rf.location.distance(bf.location);
                            painter.text(
                                mid_pos,
                                egui::Align2::CENTER_BOTTOM,
                                format!("{:.0} UU", dist_uu),
                                egui::FontId::monospace(10.0),
                                Color32::from_rgb(255, 225, 120),
                            );

                            for ap in preview.markers.iter().filter(|m| m.kind == "AlternatePath") {
                                let ap_pos = to_screen(ap.location.x, ap.location.y);
                                let base_pos = if ap.team == Some(0) { r_pos } else { b_pos };
                                let route_col = if ap.team == Some(0) {
                                    Color32::from_rgba_unmultiplied(255, 100, 100, 70)
                                } else {
                                    Color32::from_rgba_unmultiplied(100, 150, 255, 70)
                                };
                                painter.line_segment([base_pos, ap_pos], Stroke::new(1.0_f32, route_col));
                            }
                        }

                        // Draw Domination Circuit / Triangle Lines
                        let cp_markers_all: Vec<_> = preview.markers.iter().filter(|m| m.kind == "ControlPoint").collect();
                        if cp_markers_all.len() >= 2 {
                            let cp_screen_pos: Vec<Pos2> = cp_markers_all.iter().map(|m| to_screen(m.location.x, m.location.y)).collect();
                            for i in 0..cp_screen_pos.len() {
                                let next_i = (i + 1) % cp_screen_pos.len();
                                if cp_screen_pos.len() == 2 && i == 1 {
                                    break;
                                }
                                painter.line_segment(
                                    [cp_screen_pos[i], cp_screen_pos[next_i]],
                                    Stroke::new(1.8_f32, Color32::from_rgba_unmultiplied(255, 215, 0, 160)),
                                );
                            }
                            if let Some(cent_m) = preview.markers.iter().find(|m| m.label.contains("Contested Center")) {
                                let cent_pos = to_screen(cent_m.location.x, cent_m.location.y);
                                for p in &cp_screen_pos {
                                    painter.line_segment(
                                        [cent_pos, *p],
                                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 215, 100, 75)),
                                    );
                                }
                            }
                        }

                        // Draw Teleporter Transit Links (Magenta/Violet directional vector & distance)
                        for link in &preview.teleport_audit.valid_links {
                            let p_from = to_screen(link.source_loc.x, link.source_loc.y);
                            let p_to = to_screen(link.dest_loc.x, link.dest_loc.y);
                            let beam_color = Color32::from_rgba_unmultiplied(225, 90, 255, 175);
                            painter.line_segment([p_from, p_to], Stroke::new(1.8_f32, beam_color));

                            // Portal halo rings at endpoints
                            painter.circle_stroke(
                                p_from,
                                7.5,
                                Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(230, 110, 255, 200)),
                            );
                            painter.circle_stroke(
                                p_to,
                                7.5,
                                Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(230, 110, 255, 200)),
                            );

                            // Midpoint distance & directional label
                            let p_mid = Pos2::new((p_from.x + p_to.x) * 0.5, (p_from.y + p_to.y) * 0.5);
                            painter.circle_filled(p_mid, 3.5, Color32::from_rgb(240, 140, 255));
                            painter.text(
                                p_mid + Vec2::new(0.0, -6.0),
                                egui::Align2::CENTER_BOTTOM,
                                format!("⮀ {:.0} UU", link.distance),
                                egui::FontId::monospace(9.0),
                                Color32::from_rgb(235, 180, 255),
                            );
                        }

                        // Draw Bot AI Path Reachability Network (<= 850 UU)
                        if state.wizard_minimap_show_bot_paths {
                            let path_markers: Vec<_> = preview
                                .markers
                                .iter()
                                .filter(|m| {
                                    matches!(
                                        m.kind.as_ref(),
                                        "PathNode"
                                            | "PlayerStart"
                                            | "FlagBase"
                                            | "ControlPoint"
                                            | "FortStandard"
                                            | "Weapon"
                                            | "Powerup"
                                            | "Teleporter"
                                    )
                                })
                                .collect();

                            for i in 0..path_markers.len() {
                                for j in (i + 1)..path_markers.len() {
                                    let m1 = path_markers[i];
                                    let m2 = path_markers[j];
                                    let dist = m1.location.distance(m2.location);
                                    if dist <= 850.0 {
                                        let p1 = to_screen(m1.location.x, m1.location.y);
                                        let p2 = to_screen(m2.location.x, m2.location.y);
                                        let alpha_factor = (1.0 - dist / 850.0).clamp(0.2, 0.85);
                                        let col = Color32::from_rgba_unmultiplied(
                                            60,
                                            240,
                                            120,
                                            (alpha_factor * 160.0) as u8,
                                        );
                                        painter.line_segment([p1, p2], Stroke::new(1.2_f32, col));
                                    }
                                }
                            }
                        }

                        // Draw rooms & corridors
                        let selected_room_name = match &state.wizard_minimap_selected {
                            Some(WizardMinimapSelection::Room(name)) => Some(name.clone()),
                            _ => None,
                        };

                        for room in &preview.rooms {
                            let p_min = to_screen(
                                room.center.x - room.size.x * 0.5,
                                room.center.y - room.size.y * 0.5,
                            );
                            let p_max = to_screen(
                                room.center.x + room.size.x * 0.5,
                                room.center.y + room.size.y * 0.5,
                            );
                            let r_rect = Rect::from_two_pos(p_min, p_max).intersect(rect);

                            let is_custom = state.map_wizard_config.custom_rooms.iter().any(|cr| cr.name == room.label);
                            let is_selected = selected_room_name.as_deref() == Some(&room.label);
                            let is_connect_src = state.wizard_minimap_connect_source.as_deref() == Some(&room.label);

                            let (fill, border) = if is_connect_src {
                                (
                                    Color32::from_rgba_unmultiplied(40, 180, 120, 140),
                                    Color32::from_rgb(80, 255, 160),
                                )
                            } else if is_selected {
                                (
                                    Color32::from_rgba_unmultiplied(200, 170, 40, 140),
                                    Color32::from_rgb(255, 220, 50),
                                )
                            } else if is_custom {
                                (
                                    Color32::from_rgba_unmultiplied(120, 60, 180, 120),
                                    Color32::from_rgb(190, 120, 255),
                                )
                            } else if room.is_hazard || room.environment == WizardRoomEnvironment::Lava {
                                (
                                    Color32::from_rgba_unmultiplied(220, 75, 30, 95),
                                    Color32::from_rgb(255, 120, 50),
                                )
                            } else if room.environment == WizardRoomEnvironment::Water {
                                (
                                    Color32::from_rgba_unmultiplied(35, 95, 205, 130),
                                    Color32::from_rgb(60, 160, 255),
                                )
                            } else if room.environment == WizardRoomEnvironment::Slime {
                                (
                                    Color32::from_rgba_unmultiplied(40, 180, 50, 130),
                                    Color32::from_rgb(80, 240, 90),
                                )
                            } else if room.environment == WizardRoomEnvironment::LowGravity {
                                (
                                    Color32::from_rgba_unmultiplied(140, 45, 190, 130),
                                    Color32::from_rgb(200, 100, 255),
                                )
                            } else if room.is_corridor {
                                (
                                    Color32::from_rgba_unmultiplied(55, 85, 125, 110),
                                    Color32::from_rgb(100, 150, 210),
                                )
                            } else {
                                match room.team_tint {
                                    Some(0) => (
                                        Color32::from_rgba_unmultiplied(155, 45, 45, 120),
                                        Color32::from_rgb(255, 95, 95),
                                    ),
                                    Some(1) => (
                                        Color32::from_rgba_unmultiplied(45, 95, 175, 120),
                                        Color32::from_rgb(95, 185, 255),
                                    ),
                                    _ => (
                                        Color32::from_rgba_unmultiplied(65, 135, 175, 130),
                                        Color32::from_rgb(130, 220, 255),
                                    ),
                                }
                            };

                            let corner_round = if room.is_octagonal { 8.0 } else { 2.0 };
                            painter.rect_filled(r_rect, corner_round, fill);
                            let stroke_w = if is_selected || is_connect_src { 2.2_f32 } else { 1.2_f32 };
                            painter.rect_stroke(
                                r_rect,
                                corner_round,
                                Stroke::new(stroke_w, border),
                                egui::StrokeKind::Inside,
                            );

                            if !room.is_corridor && r_rect.width() > 36.0 && r_rect.height() > 16.0 {
                                let env_suffix = match room.environment {
                                    WizardRoomEnvironment::Water => " [Water]",
                                    WizardRoomEnvironment::Slime => " [Slime]",
                                    WizardRoomEnvironment::Lava => " [Lava]",
                                    WizardRoomEnvironment::LowGravity => " [Low-G]",
                                    WizardRoomEnvironment::Normal => "",
                                };
                                let label_text = format!("{}{}", room.label, env_suffix);
                                painter.text(
                                    r_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    label_text,
                                    egui::FontId::proportional(10.0),
                                    Color32::from_rgb(225, 235, 245),
                                );
                            }
                        }

                        // Selected Room Gizmo Transformation Handles
                        let sel_room_info = selected_room_name
                            .as_ref()
                            .and_then(|name| preview.rooms.iter().find(|r| &r.label == name));

                        let mut handles: Vec<(WizardResizeHandle, Pos2, egui::CursorIcon)> = Vec::new();
                        if let Some(r) = sel_room_info {
                            let p_min = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                            let p_max = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                            let sel_box = Rect::from_two_pos(p_min, p_max);

                            // Draw selection marquee outline
                            painter.rect_stroke(
                                sel_box,
                                2.0,
                                Stroke::new(2.0_f32, Color32::from_rgb(255, 220, 50)),
                                egui::StrokeKind::Outside,
                            );

                            handles = vec![
                                (WizardResizeHandle::North, Pos2::new(sel_box.center().x, sel_box.top()), egui::CursorIcon::ResizeVertical),
                                (WizardResizeHandle::South, Pos2::new(sel_box.center().x, sel_box.bottom()), egui::CursorIcon::ResizeVertical),
                                (WizardResizeHandle::East, Pos2::new(sel_box.right(), sel_box.center().y), egui::CursorIcon::ResizeHorizontal),
                                (WizardResizeHandle::West, Pos2::new(sel_box.left(), sel_box.center().y), egui::CursorIcon::ResizeHorizontal),
                                (WizardResizeHandle::NorthWest, sel_box.left_top(), egui::CursorIcon::ResizeNwSe),
                                (WizardResizeHandle::NorthEast, sel_box.right_top(), egui::CursorIcon::ResizeNeSw),
                                (WizardResizeHandle::SouthWest, sel_box.left_bottom(), egui::CursorIcon::ResizeNeSw),
                                (WizardResizeHandle::SouthEast, sel_box.right_bottom(), egui::CursorIcon::ResizeNwSe),
                            ];

                            // Draw each handle square
                            for (_, h_pos, _) in &handles {
                                let h_rect = Rect::from_center_size(*h_pos, Vec2::splat(8.0));
                                let is_h_hovered = response.hover_pos().map_or(false, |hp| h_rect.contains(hp));
                                let h_fill = if is_h_hovered { Color32::WHITE } else { Color32::from_rgb(255, 230, 90) };
                                painter.rect_filled(h_rect, 1.0, h_fill);
                                painter.rect_stroke(h_rect, 1.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(20, 20, 20)), egui::StrokeKind::Outside);
                            }
                        }

                        // Draw Markers (PlayerStarts, Flags, ControlPoints, Weapons, Pickups, Lights, Teleporters)
                        for marker in &preview.markers {
                            let pos = to_screen(marker.location.x, marker.location.y);
                            if !rect.contains(pos) {
                                continue;
                            }
                            let is_objective = matches!(
                                marker.kind.as_ref(),
                                "FlagBase" | "ControlPoint" | "FortStandard"
                            );
                            let is_selected = match &state.wizard_minimap_selected {
                                Some(WizardMinimapSelection::Marker(label)) => label == &marker.label,
                                _ => false,
                            };

                            let col = match (marker.kind.as_ref(), marker.team) {
                                ("ControlPoint", _) => Color32::from_rgb(255, 215, 0),
                                ("FortStandard", _) => {
                                    if marker.label.contains("[FINAL]") {
                                        Color32::from_rgb(255, 60, 60)
                                    } else {
                                        Color32::from_rgb(255, 120, 80)
                                    }
                                }
                                ("Mover", Some(1)) => Color32::from_rgb(255, 180, 40),
                                ("Mover", _) => Color32::from_rgb(180, 110, 255),
                                (_, Some(0)) => Color32::from_rgb(255, 85, 85),
                                (_, Some(1)) => Color32::from_rgb(85, 170, 255),
                                ("PlayerStart", _) => Color32::from_rgb(90, 230, 120),
                                ("Weapon", _) => Color32::from_rgb(80, 210, 255),
                                ("Powerup", _) => Color32::from_rgb(255, 215, 70),
                                ("Health" | "HealthVial", _) => Color32::from_rgb(100, 255, 180),
                                ("Armor" | "ShieldBelt", _) => Color32::from_rgb(255, 200, 60),
                                ("Light", _) => Color32::from_rgb(255, 255, 180),
                                ("Lift", _) => Color32::from_rgb(200, 140, 255),
                                ("Ramp", _) => Color32::from_rgb(160, 220, 140),
                                ("Teleporter", _) => Color32::from_rgb(235, 110, 255),
                                _ => Color32::from_rgb(255, 180, 90),
                            };
                            let radius = if is_objective { 5.5 } else { 4.0 };
                            if is_selected {
                                painter.circle_stroke(pos, radius + 4.0, Stroke::new(2.0_f32, Color32::from_rgb(255, 220, 50)));
                            }
                            if marker.kind == "Teleporter" {
                                if let Some(issue) = preview.teleport_audit.issues.iter().find(|i| i.teleporter_name == marker.label) {
                                    let halo_col = match issue.severity {
                                        WizardTeleportIssueSeverity::Error => Color32::from_rgb(255, 70, 70),
                                        _ => Color32::from_rgb(255, 175, 40),
                                    };
                                    painter.circle_stroke(pos, radius + 4.5, Stroke::new(1.8_f32, halo_col));
                                    painter.text(
                                        pos + Vec2::new(radius + 3.0, -radius - 3.0),
                                        egui::Align2::LEFT_BOTTOM,
                                        "⚠",
                                        egui::FontId::monospace(10.5),
                                        halo_col,
                                    );
                                } else {
                                    painter.circle_stroke(
                                        pos,
                                        radius + 3.5,
                                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(225, 110, 255, 180)),
                                    );
                                }
                            }
                            painter.circle_filled(pos, radius, col);
                            painter.circle_stroke(pos, radius + 1.0, Stroke::new(1.0_f32, Color32::BLACK));
                        }

                        // Hover Handling & Interactive Feedback
                        if let Some(hover_pos) = response.hover_pos() {
                            if rect.contains(hover_pos) {
                                let world_pt = to_world(hover_pos);
                                let snapped_x = (world_pt.x / 128.0).round() * 128.0;
                                let snapped_y = (world_pt.y / 128.0).round() * 128.0;

                                // Update cursor icon based on hovered handle, marker, or room
                                let mut handled_cursor = false;
                                for (_, h_pos, cursor_icon) in &handles {
                                    let h_rect = Rect::from_center_size(*h_pos, Vec2::splat(9.0));
                                    if h_rect.contains(hover_pos) {
                                        ui.ctx().set_cursor_icon(*cursor_icon);
                                        handled_cursor = true;
                                        break;
                                    }
                                }
                                if !handled_cursor {
                                    let hit_marker = preview.markers.iter().rev().find(|m| {
                                        to_screen(m.location.x, m.location.y).distance(hover_pos) <= 9.0
                                    });
                                    if hit_marker.is_some() {
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                    } else if let Some(r) = sel_room_info {
                                        let p0 = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                                        let p1 = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                                        if Rect::from_two_pos(p0, p1).contains(hover_pos) {
                                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                                        }
                                    }
                                }

                                match state.wizard_minimap_tool {
                                    WizardMinimapTool::AddObject => {
                                        let obj_x = (world_pt.x / 64.0).round() * 64.0;
                                        let obj_y = (world_pt.y / 64.0).round() * 64.0;
                                        let ghost_pos = to_screen(obj_x, obj_y);
                                        painter.circle_filled(ghost_pos, 5.0, Color32::from_rgba_unmultiplied(100, 220, 255, 140));
                                        painter.circle_stroke(ghost_pos, 5.0, Stroke::new(1.0_f32, Color32::WHITE));
                                        painter.text(
                                            ghost_pos + Vec2::new(0.0, -10.0),
                                            egui::Align2::CENTER_BOTTOM,
                                            format!("+ Place {}", state.wizard_minimap_add_object_kind),
                                            egui::FontId::proportional(10.5),
                                            Color32::from_rgb(180, 240, 255),
                                        );
                                    }
                                    WizardMinimapTool::AddRoom => {
                                        let ghost_pmin = to_screen(snapped_x - 320.0, snapped_y - 320.0);
                                        let ghost_pmax = to_screen(snapped_x + 320.0, snapped_y + 320.0);
                                        let g_rect = Rect::from_two_pos(ghost_pmin, ghost_pmax);
                                        painter.rect_filled(g_rect, 4.0, Color32::from_rgba_unmultiplied(80, 220, 120, 50));
                                        painter.rect_stroke(g_rect, 4.0, Stroke::new(1.5_f32, Color32::from_rgb(100, 255, 160)), egui::StrokeKind::Inside);
                                        painter.text(
                                            to_screen(snapped_x, snapped_y),
                                            egui::Align2::CENTER_CENTER,
                                            format!("Place Room ({:.0}, {:.0})", snapped_x, snapped_y),
                                            egui::FontId::proportional(10.0),
                                            Color32::from_rgb(180, 255, 200),
                                        );
                                    }
                                    WizardMinimapTool::ConnectRooms => {
                                        if let Some(ref src_name) = state.wizard_minimap_connect_source {
                                            if let Some(src_r) = preview.rooms.iter().find(|r| &r.label == src_name) {
                                                let src_pos = to_screen(src_r.center.x, src_r.center.y);
                                                painter.line_segment([src_pos, hover_pos], Stroke::new(2.0_f32, Color32::from_rgb(80, 220, 255)));
                                            }
                                        }
                                    }
                                    WizardMinimapTool::Inspect => {
                                        let hit_marker = preview.markers.iter().rev().find(|m| {
                                            let sp = to_screen(m.location.x, m.location.y);
                                            sp.distance(hover_pos) <= 9.0
                                        });
                                        if let Some(m) = hit_marker {
                                            response.clone().on_hover_ui_at_pointer(|ui| {
                                                ui.label(RichText::new(&m.label).strong().color(Color32::from_rgb(255, 215, 90)));
                                                ui.label(RichText::new(format!("Type: {}", m.kind)).size(10.5));
                                                ui.label(RichText::new(format!("Location: ({:.0}, {:.0}, {:.0})", m.location.x, m.location.y, m.location.z)).monospace().size(10.0));
                                                ui.label(RichText::new("Drag to move object • Click Delete to remove").size(10.0).italics().color(Color32::LIGHT_GRAY));
                                            });
                                        } else {
                                            let hit_room = preview.rooms.iter().rev().find(|r| {
                                                let p0 = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                                                let p1 = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                                                Rect::from_two_pos(p0, p1).contains(hover_pos)
                                            });
                                            if let Some(r) = hit_room {
                                                response.clone().on_hover_ui_at_pointer(|ui| {
                                                    ui.label(RichText::new(&r.label).strong().color(Color32::from_rgb(140, 220, 255)));
                                                    ui.label(RichText::new(format!("Size: {:.0}x{:.0}x{:.0}", r.size.x, r.size.y, r.size.z)).size(10.5));
                                                    ui.label(RichText::new(format!("Center: ({:.0}, {:.0}, {:.0})", r.center.x, r.center.y, r.center.z)).monospace().size(10.0));
                                                    ui.label(RichText::new("Drag handles to resize • Drag room body to reposition").size(10.0).italics().color(Color32::LIGHT_GRAY));
                                                });
                                            }
                                        }
                                    }
                                    WizardMinimapTool::Delete => {}
                                }
                            }
                        }

                        // Drag Interaction Handling (Resizing Room, Moving Room, Moving Marker)
                        if response.drag_started() {
                            if let Some(ptr_pos) = response.interact_pointer_pos() {
                                if rect.contains(ptr_pos) {
                                    let world_pt = to_world(ptr_pos);
                                    let world_glam = glam::Vec2::new(world_pt.x, world_pt.y);
                                    let mut handle_dragged = false;

                                    if let Some(r) = sel_room_info {
                                        for (h_type, h_pos, _) in &handles {
                                            let h_rect = Rect::from_center_size(*h_pos, Vec2::splat(10.0));
                                            if h_rect.contains(ptr_pos) {
                                                state.wizard_minimap_drag_state = WizardMinimapDragState::ResizingRoom {
                                                    room_name: r.label.clone(),
                                                    handle: *h_type,
                                                    initial_click_world: world_glam,
                                                    initial_center: r.center,
                                                    initial_size: r.size,
                                                };
                                                handle_dragged = true;
                                                break;
                                            }
                                        }
                                    }

                                    if !handle_dragged {
                                        let hit_marker = preview.markers.iter().rev().find(|m| {
                                            to_screen(m.location.x, m.location.y).distance(ptr_pos) <= 10.0
                                        });
                                        if let Some(m) = hit_marker {
                                            let is_alt = ui.input(|i| i.modifiers.alt);
                                            let target_label = if is_alt {
                                                let new_label = state.add_wizard_custom_marker(&m.kind, m.location.x, m.location.y);
                                                state.toast_info(format!("Duplicated object '{}'", m.label));
                                                new_label
                                            } else {
                                                m.label.clone()
                                            };
                                            state.wizard_minimap_selected = Some(WizardMinimapSelection::Marker(target_label.clone()));
                                            state.wizard_minimap_drag_state = WizardMinimapDragState::MovingMarker {
                                                marker_label: target_label,
                                                initial_click_world: world_glam,
                                                initial_loc: m.location,
                                            };
                                        } else {
                                            let hit_room = preview.rooms.iter().rev().find(|r| {
                                                let p0 = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                                                let p1 = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                                                Rect::from_two_pos(p0, p1).contains(ptr_pos)
                                            });
                                            if let Some(r) = hit_room {
                                                state.wizard_minimap_selected = Some(WizardMinimapSelection::Room(r.label.clone()));
                                                state.wizard_minimap_drag_state = WizardMinimapDragState::MovingRoom {
                                                    room_name: r.label.clone(),
                                                    initial_click_world: world_glam,
                                                    initial_center: r.center,
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        if response.dragged() {
                            if let Some(ptr_pos) = response.interact_pointer_pos() {
                                let cur_w = to_world(ptr_pos);
                                let snap = if state.wizard_minimap_grid_snap > 0.0 { state.wizard_minimap_grid_snap } else { 1.0 };
                                match &state.wizard_minimap_drag_state.clone() {
                                    WizardMinimapDragState::MovingMarker { marker_label, initial_click_world, initial_loc } => {
                                        let raw_dx = cur_w.x - initial_click_world.x;
                                        let raw_dy = cur_w.y - initial_click_world.y;
                                        let snap_dx = (raw_dx / snap).round() * snap;
                                        let snap_dy = (raw_dy / snap).round() * snap;
                                        let new_loc = *initial_loc + glam::Vec3::new(snap_dx, snap_dy, 0.0);
                                        state.move_wizard_marker(marker_label, new_loc);
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    }
                                    WizardMinimapDragState::MovingRoom { room_name, initial_click_world, initial_center } => {
                                        let raw_dx = cur_w.x - initial_click_world.x;
                                        let raw_dy = cur_w.y - initial_click_world.y;
                                        let snap_dx = (raw_dx / snap).round() * snap;
                                        let snap_dy = (raw_dy / snap).round() * snap;
                                        let new_center = *initial_center + glam::Vec3::new(snap_dx, snap_dy, 0.0);
                                        let cur_size = preview.rooms.iter().find(|r| &r.label == room_name).map(|r| r.size).unwrap_or(glam::Vec3::splat(512.0));
                                        state.update_wizard_room_transform(room_name, new_center, cur_size);
                                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    }
                                    WizardMinimapDragState::ResizingRoom { room_name, handle, initial_click_world, initial_center, initial_size } => {
                                        let raw_dx = cur_w.x - initial_click_world.x;
                                        let raw_dy = cur_w.y - initial_click_world.y;
                                        let snap_dx = (raw_dx / snap).round() * snap;
                                        let snap_dy = (raw_dy / snap).round() * snap;

                                        let mut min_x = initial_center.x - initial_size.x * 0.5;
                                        let mut max_x = initial_center.x + initial_size.x * 0.5;
                                        let mut min_y = initial_center.y - initial_size.y * 0.5;
                                        let mut max_y = initial_center.y + initial_size.y * 0.5;

                                        match handle {
                                            WizardResizeHandle::East => max_x += snap_dx,
                                            WizardResizeHandle::West => min_x += snap_dx,
                                            WizardResizeHandle::South => max_y += snap_dy,
                                            WizardResizeHandle::North => min_y += snap_dy,
                                            WizardResizeHandle::NorthEast => { max_x += snap_dx; min_y += snap_dy; }
                                            WizardResizeHandle::NorthWest => { min_x += snap_dx; min_y += snap_dy; }
                                            WizardResizeHandle::SouthEast => { max_x += snap_dx; max_y += snap_dy; }
                                            WizardResizeHandle::SouthWest => { min_x += snap_dx; max_y += snap_dy; }
                                        }

                                        if max_x - min_x < 256.0 {
                                            match handle {
                                                WizardResizeHandle::West | WizardResizeHandle::NorthWest | WizardResizeHandle::SouthWest => min_x = max_x - 256.0,
                                                _ => max_x = min_x + 256.0,
                                            }
                                        }
                                        if max_y - min_y < 256.0 {
                                            match handle {
                                                WizardResizeHandle::North | WizardResizeHandle::NorthWest | WizardResizeHandle::NorthEast => min_y = max_y - 256.0,
                                                _ => max_y = min_y + 256.0,
                                            }
                                        }

                                        let new_center = glam::Vec3::new((min_x + max_x) * 0.5, (min_y + max_y) * 0.5, initial_center.z);
                                        let new_size = glam::Vec3::new(max_x - min_x, max_y - min_y, initial_size.z);
                                        state.update_wizard_room_transform(room_name, new_center, new_size);
                                    }
                                    WizardMinimapDragState::None => {}
                                }
                            }
                        }

                        if response.drag_stopped() {
                            state.wizard_minimap_drag_state = WizardMinimapDragState::None;
                        }

                        // Click handling
                        if response.clicked() {
                            if let Some(click_pos) = response.interact_pointer_pos() {
                                let world_pt = to_world(click_pos);
                                let snapped_x = (world_pt.x / 128.0).round() * 128.0;
                                let snapped_y = (world_pt.y / 128.0).round() * 128.0;

                                let hit_marker = preview.markers.iter().rev().find(|m| {
                                    let sp = to_screen(m.location.x, m.location.y);
                                    sp.distance(click_pos) <= 9.0
                                });
                                let hit_room = preview.rooms.iter().rev().find(|r| !r.is_corridor && {
                                    let p0 = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                                    let p1 = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                                    Rect::from_two_pos(p0, p1).contains(click_pos)
                                }).or_else(|| {
                                    preview.rooms.iter().rev().find(|r| {
                                        let p0 = to_screen(r.center.x - r.size.x * 0.5, r.center.y - r.size.y * 0.5);
                                        let p1 = to_screen(r.center.x + r.size.x * 0.5, r.center.y + r.size.y * 0.5);
                                        Rect::from_two_pos(p0, p1).contains(click_pos)
                                    })
                                });

                                match state.wizard_minimap_tool {
                                    WizardMinimapTool::Inspect => {
                                        if let Some(m) = hit_marker {
                                            state.wizard_minimap_selected = Some(WizardMinimapSelection::Marker(m.label.clone()));
                                        } else if let Some(r) = hit_room {
                                            if let Some(conn) = state.map_wizard_config.custom_connections.iter().find(|c| r.label.contains(&c.id)) {
                                                state.wizard_minimap_selected = Some(WizardMinimapSelection::CustomConnection(conn.id.clone()));
                                            } else {
                                                state.wizard_minimap_selected = Some(WizardMinimapSelection::Room(r.label.clone()));
                                            }
                                        } else {
                                            state.wizard_minimap_selected = None;
                                        }
                                    }
                                    WizardMinimapTool::AddObject => {
                                        let obj_x = (world_pt.x / 64.0).round() * 64.0;
                                        let obj_y = (world_pt.y / 64.0).round() * 64.0;
                                        let kind = if state.wizard_minimap_add_object_kind.is_empty() {
                                            "ShockRifle".to_string()
                                        } else {
                                            state.wizard_minimap_add_object_kind.clone()
                                        };
                                        state.add_wizard_custom_marker(&kind, obj_x, obj_y);
                                    }
                                    WizardMinimapTool::AddRoom => {
                                        state.add_wizard_custom_room(snapped_x, snapped_y);
                                    }
                                    WizardMinimapTool::ConnectRooms => {
                                        if let Some(r) = hit_room {
                                            if !r.is_corridor {
                                                if let Some(src) = state.wizard_minimap_connect_source.take() {
                                                    state.add_wizard_custom_connection(&src, &r.label);
                                                } else {
                                                    state.wizard_minimap_connect_source = Some(r.label.clone());
                                                    state.toast_info(format!("Selected '{}'. Now click target room to connect.", r.label));
                                                }
                                            }
                                        }
                                    }
                                    WizardMinimapTool::Delete => {
                                        if let Some(m) = hit_marker {
                                            state.wizard_minimap_selected = Some(WizardMinimapSelection::Marker(m.label.clone()));
                                            state.delete_wizard_selection();
                                        } else if let Some(r) = hit_room {
                                            if let Some(conn) = state.map_wizard_config.custom_connections.iter().find(|c| r.label.contains(&c.id)) {
                                                state.wizard_minimap_selected = Some(WizardMinimapSelection::CustomConnection(conn.id.clone()));
                                            } else {
                                                state.wizard_minimap_selected = Some(WizardMinimapSelection::Room(r.label.clone()));
                                            }
                                            state.delete_wizard_selection();
                                        }
                                    }
                                }
                            }
                        }

                        ui.add_space(6.0);

                        // Legend
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("● Spawn").size(11.0).color(Color32::from_rgb(90, 230, 120)));
                            ui.label(RichText::new("🚩 Red/Blue Flag").size(11.0).color(Color32::from_rgb(255, 95, 95)));
                            ui.label(RichText::new("⚑ ControlPoint").size(11.0).color(Color32::from_rgb(255, 215, 0)));
                            ui.label(RichText::new("🎯 Sniper Perch").size(11.0).color(Color32::from_rgb(255, 170, 50)));
                            ui.label(RichText::new("● Red/Blue Obj").size(11.0).color(Color32::from_rgb(255, 95, 95)));
                            ui.label(RichText::new("● Switch").size(11.0).color(Color32::from_rgb(255, 180, 40)));
                            ui.label(RichText::new("● Gate/Lift").size(11.0).color(Color32::from_rgb(180, 110, 255)));
                            ui.label(RichText::new("● Weapon").size(11.0).color(Color32::from_rgb(80, 210, 255)));
                            ui.label(RichText::new("● Armor/Powerup").size(11.0).color(Color32::from_rgb(255, 215, 70)));
                            ui.label(RichText::new("● Teleporter").size(11.0).color(Color32::from_rgb(235, 110, 255)));
                            ui.label(RichText::new("■ Hazard Basin").size(11.0).color(Color32::from_rgb(255, 120, 50)));
                        });

                        // Interactive Minimap Inspector Card
                        if let Some(ref sel) = state.wizard_minimap_selected.clone() {
                            ui.add_space(4.0);
                            egui::Frame::group(ui.style())
                                .fill(Color32::from_rgb(22, 28, 40))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(100, 160, 220)))
                                .show(ui, |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        match sel {
                                            WizardMinimapSelection::Room(name) => {
                                                ui.label(RichText::new(format!("🔍 Room: {name}")).strong().color(Color32::from_rgb(255, 215, 90)));
                                                let is_custom = state.map_wizard_config.custom_rooms.iter().any(|cr| &cr.name == name);
                                                if is_custom {
                                                    ui.colored_label(Color32::from_rgb(100, 255, 160), "[Custom Room]");
                                                } else {
                                                    ui.colored_label(Color32::from_rgb(140, 200, 255), "[Archetype Room]");
                                                }
                                            }
                                            WizardMinimapSelection::Marker(label) => {
                                                ui.label(RichText::new(format!("🎯 Object: {label}")).strong().color(Color32::from_rgb(255, 215, 90)));
                                            }
                                            WizardMinimapSelection::CustomConnection(id) => {
                                                ui.label(RichText::new(format!("🔗 Custom Corridor: {id}")).strong().color(Color32::from_rgb(255, 215, 90)));
                                            }
                                        }
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            if ui.small_button("✖ Close").clicked() {
                                                state.wizard_minimap_selected = None;
                                            }
                                            if let WizardMinimapSelection::Room(name) = sel {
                                                if state.map_wizard_config.custom_rooms.iter().any(|cr| &cr.name == name) {
                                                    if ui.button(RichText::new("🗑 Delete Room").size(10.5).color(Color32::from_rgb(255, 120, 120))).clicked() {
                                                        state.delete_wizard_selection();
                                                    }
                                                }
                                            } else if let WizardMinimapSelection::Marker(_) = sel {
                                                if ui.button(RichText::new("🗑 Delete Object").size(10.5).color(Color32::from_rgb(255, 120, 120))).clicked() {
                                                    state.delete_wizard_selection();
                                                }
                                            } else if let WizardMinimapSelection::CustomConnection(_) = sel {
                                                if ui.button(RichText::new("🗑 Delete Corridor").size(10.5).color(Color32::from_rgb(255, 120, 120))).clicked() {
                                                    state.delete_wizard_selection();
                                                }
                                            }
                                        });
                                    });

                                    ui.separator();

                                    match sel {
                                        WizardMinimapSelection::Room(name) => {
                                            if let Some(pos) = state.map_wizard_config.custom_rooms.iter().position(|cr| &cr.name == name) {
                                                let mut cr = state.map_wizard_config.custom_rooms[pos].clone();
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Center X:");
                                                    ui.add(egui::DragValue::new(&mut cr.center.x).speed(16.0));
                                                    ui.label("Center Y:");
                                                    ui.add(egui::DragValue::new(&mut cr.center.y).speed(16.0));
                                                    ui.label("Center Z:");
                                                    ui.add(egui::DragValue::new(&mut cr.center.z).speed(16.0));
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Size X:");
                                                    ui.add(egui::DragValue::new(&mut cr.size.x).speed(16.0).range(256.0..=2048.0));
                                                    ui.label("Size Y:");
                                                    ui.add(egui::DragValue::new(&mut cr.size.y).speed(16.0).range(256.0..=2048.0));
                                                    ui.label("Height Z:");
                                                    ui.add(egui::DragValue::new(&mut cr.size.z).speed(16.0).range(256.0..=1024.0));
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.checkbox(&mut cr.is_octagonal, "Octagonal");
                                                    ui.checkbox(&mut cr.has_hazard_pit, "Hazard Basin");
                                                    ui.checkbox(&mut cr.sky_ceiling, "Sky Ceiling");
                                                });
                                                let cur_env = cr.environment;
                                                let mut sel_env = cur_env;
                                                ui.horizontal(|ui| {
                                                    ui.label("Medium / Zone:");
                                                    egui::ComboBox::from_id_salt(format!("cr_env_{name}"))
                                                        .selected_text(match sel_env {
                                                            WizardRoomEnvironment::Normal => "Normal (Air)",
                                                            WizardRoomEnvironment::Water => "Water Zone (Swimmable)",
                                                            WizardRoomEnvironment::Slime => "Slime Zone (Acidic Toxic)",
                                                            WizardRoomEnvironment::Lava => "Lava Zone (Lethal Heat)",
                                                            WizardRoomEnvironment::LowGravity => "Low Gravity (Space / Moon)",
                                                        })
                                                        .width(180.0)
                                                        .show_ui(ui, |ui| {
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Normal, "Normal (Air)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Water, "Water Zone (Swimmable)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Slime, "Slime Zone (Acidic Toxic)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Lava, "Lava Zone (Lethal Heat)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::LowGravity, "Low Gravity (Space / Moon)");
                                                        });
                                                });
                                                if sel_env != cur_env {
                                                    cr.environment = sel_env;
                                                }
                                                state.map_wizard_config.custom_rooms[pos] = cr;
                                            } else if let Some(r) = preview.rooms.iter().find(|r| &r.label == name) {
                                                let mut cur_center = r.center;
                                                let mut cur_size = r.size;
                                                let mut changed = false;

                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Center X:");
                                                    if ui.add(egui::DragValue::new(&mut cur_center.x).speed(16.0)).changed() { changed = true; }
                                                    ui.label("Center Y:");
                                                    if ui.add(egui::DragValue::new(&mut cur_center.y).speed(16.0)).changed() { changed = true; }
                                                    ui.label("Center Z:");
                                                    if ui.add(egui::DragValue::new(&mut cur_center.z).speed(16.0)).changed() { changed = true; }
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Size X:");
                                                    if ui.add(egui::DragValue::new(&mut cur_size.x).speed(16.0).range(256.0..=4096.0)).changed() { changed = true; }
                                                    ui.label("Size Y:");
                                                    if ui.add(egui::DragValue::new(&mut cur_size.y).speed(16.0).range(256.0..=4096.0)).changed() { changed = true; }
                                                    ui.label("Height Z:");
                                                    if ui.add(egui::DragValue::new(&mut cur_size.z).speed(16.0).range(256.0..=2048.0)).changed() { changed = true; }
                                                });

                                                if changed {
                                                    state.update_wizard_room_transform(name, cur_center, cur_size);
                                                }

                                                let cur_env = state.map_wizard_config.get_room_environment(name);
                                                let mut sel_env = cur_env;
                                                ui.horizontal(|ui| {
                                                    ui.label("Medium / Zone:");
                                                    egui::ComboBox::from_id_salt(format!("arch_env_{name}"))
                                                        .selected_text(match sel_env {
                                                            WizardRoomEnvironment::Normal => "Normal (Air)",
                                                            WizardRoomEnvironment::Water => "Water Zone (Swimmable)",
                                                            WizardRoomEnvironment::Slime => "Slime Zone (Acidic Toxic)",
                                                            WizardRoomEnvironment::Lava => "Lava Zone (Lethal Heat)",
                                                            WizardRoomEnvironment::LowGravity => "Low Gravity (Space / Moon)",
                                                        })
                                                        .width(180.0)
                                                        .show_ui(ui, |ui| {
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Normal, "Normal (Air)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Water, "Water Zone (Swimmable)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Slime, "Slime Zone (Acidic Toxic)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::Lava, "Lava Zone (Lethal Heat)");
                                                            ui.selectable_value(&mut sel_env, WizardRoomEnvironment::LowGravity, "Low Gravity (Space / Moon)");
                                                        });
                                                });
                                                if sel_env != cur_env {
                                                    state.set_wizard_room_environment(name, sel_env);
                                                }

                                                ui.horizontal_wrapped(|ui| {
                                                    let is_overridden = state.map_wizard_config.room_overrides.iter().any(|ro| &ro.room_name == name);
                                                    if is_overridden {
                                                        if ui.button(RichText::new("↺ Reset Room Transform").color(Color32::from_rgb(255, 180, 100))).clicked() {
                                                            state.map_wizard_config.room_overrides.retain(|ro| &ro.room_name != name);
                                                        }
                                                    }
                                                    let team_str = match r.team_tint {
                                                        Some(0) => "Red Team",
                                                        Some(1) => "Blue Team",
                                                        _ => "Neutral",
                                                    };
                                                    ui.label(format!("Owner: {}", team_str));
                                                });
                                            }
                                        }
                                        WizardMinimapSelection::Marker(label) => {
                                            if let Some(m) = preview.markers.iter().find(|m| &m.label == label) {
                                                let mut loc = m.location;
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(format!("Kind: {}", m.kind));
                                                    if let Some(t) = m.team {
                                                        ui.separator();
                                                        ui.label(format!("Team: {}", if t == 0 { "Red" } else { "Blue" }));
                                                    }
                                                });
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label("Pos X:");
                                                    if ui.add(egui::DragValue::new(&mut loc.x).speed(16.0)).changed() {
                                                        state.move_wizard_marker(label, loc);
                                                    }
                                                    ui.label("Pos Y:");
                                                    if ui.add(egui::DragValue::new(&mut loc.y).speed(16.0)).changed() {
                                                        state.move_wizard_marker(label, loc);
                                                    }
                                                    ui.label("Pos Z:");
                                                    if ui.add(egui::DragValue::new(&mut loc.z).speed(8.0)).changed() {
                                                        state.move_wizard_marker(label, loc);
                                                    }
                                                });
                                                if m.kind == "Teleporter" {
                                                    ui.separator();
                                                    let (mut cur_tag, mut cur_url) = if let Some(cm) = state.map_wizard_config.custom_markers.iter().find(|cm| &cm.label == label || &cm.id == label) {
                                                        (cm.teleporter_tag.clone(), cm.teleporter_url.clone())
                                                    } else {
                                                        match label.as_str() {
                                                            "WizardTeleA" => ("WizardTeleA".to_string(), "WizardTeleB".to_string()),
                                                            "WizardTeleB" => ("WizardTeleB".to_string(), "WizardTeleA".to_string()),
                                                            _ => (label.clone(), String::new()),
                                                        }
                                                    };
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.label(RichText::new("🌀 Teleporter Routing:").strong().color(Color32::from_rgb(230, 130, 255)));
                                                        if let Some(issue) = preview.teleport_audit.issues.iter().find(|i| &i.teleporter_name == label) {
                                                            ui.colored_label(Color32::from_rgb(255, 110, 100), format!("⚠ {}", issue.message));
                                                            if ui.button("🛠 Auto-Fix").clicked() {
                                                                state.auto_fix_wizard_teleporters();
                                                            }
                                                        } else {
                                                            let dest = if cur_url.is_empty() { "None" } else { &cur_url };
                                                            ui.colored_label(Color32::from_rgb(110, 255, 140), format!("✔ Linked → Tag '{dest}'"));
                                                        }
                                                    });
                                                    ui.horizontal(|ui| {
                                                        ui.label("Tag:");
                                                        let resp_tag = ui.text_edit_singleline(&mut cur_tag);
                                                        ui.label("Target URL:");
                                                        let resp_url = ui.text_edit_singleline(&mut cur_url);
                                                        if resp_tag.changed() || resp_url.changed() {
                                                            state.update_wizard_teleporter_properties(label, &cur_tag, &cur_url);
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                        WizardMinimapSelection::CustomConnection(id) => {
                                            if let Some(pos) = state.map_wizard_config.custom_connections.iter().position(|c| &c.id == id) {
                                                let mut conn = state.map_wizard_config.custom_connections[pos].clone();
                                                ui.horizontal_wrapped(|ui| {
                                                    ui.label(format!("Route: {} ⟷ {}", conn.from_room, conn.to_room));
                                                    ui.separator();
                                                    ui.label("Width:");
                                                    ui.add(egui::DragValue::new(&mut conn.width).speed(16.0).range(128.0..=512.0));
                                                    ui.label("Height:");
                                                    ui.add(egui::DragValue::new(&mut conn.height).speed(16.0).range(192.0..=512.0));
                                                    ui.checkbox(&mut conn.has_blast_door, "Sliding Blast Door Mover");
                                                });
                                                state.map_wizard_config.custom_connections[pos] = conn;
                                            }
                                        }
                                    }
                                });
                        }

                        ui.separator();

                        // Live Architectural & Gameplay Telemetry Card
                        egui::Frame::group(ui.style())
                            .fill(Color32::from_rgb(20, 25, 34))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("▦ Live Generation Telemetry & Audit Readiness")
                                        .strong()
                                        .color(Color32::from_rgb(255, 215, 90)),
                                );
                                egui::Grid::new("wizard_telemetry_grid")
                                    .num_columns(4)
                                    .spacing([14.0, 4.0])
                                    .show(ui, |ui| {
                                        ui.label("CSG Brushes:");
                                        ui.label(RichText::new(format!("{}", preview.estimated_brushes)).strong());
                                        ui.label("Lights & Movers:");
                                        ui.label(
                                            RichText::new(format!(
                                                "{} / {}",
                                                preview.estimated_lights, preview.estimated_movers
                                            ))
                                            .strong(),
                                        );
                                        ui.end_row();

                                        ui.label("Spawns & Pickups:");
                                        ui.label(
                                            RichText::new(format!(
                                                "{} / {}",
                                                preview.estimated_spawns,
                                                preview.estimated_weapons_pickups
                                            ))
                                            .strong(),
                                        );
                                        ui.label("PathNodes:");
                                        ui.label(RichText::new(format!("{}", preview.estimated_path_nodes)).strong());
                                        ui.end_row();

                                        ui.label("Teleporters:");
                                        if preview.teleport_audit.teleporter_count == 0 {
                                            ui.label(RichText::new("None").color(Color32::GRAY));
                                        } else if preview.teleport_audit.is_healthy {
                                            ui.label(
                                                RichText::new(format!(
                                                    "{} linked ({} routes)",
                                                    preview.teleport_audit.teleporter_count,
                                                    preview.teleport_audit.valid_links.len()
                                                ))
                                                .color(Color32::from_rgb(110, 255, 140))
                                                .strong(),
                                            );
                                        } else {
                                            ui.label(
                                                RichText::new(format!(
                                                    "⚠ {} nodes ({} issue(s))",
                                                    preview.teleport_audit.teleporter_count,
                                                    preview.teleport_audit.issues.len()
                                                ))
                                                .color(Color32::from_rgb(255, 140, 60))
                                                .strong(),
                                            );
                                        }
                                        ui.label("Transit Status:");
                                        if preview.teleport_audit.teleporter_count == 0 {
                                            ui.label(RichText::new("Unused").color(Color32::GRAY));
                                        } else if preview.teleport_audit.is_healthy {
                                            ui.label(RichText::new("✔ 100% Routed").color(Color32::from_rgb(110, 255, 140)).strong());
                                        } else {
                                            ui.label(RichText::new("⚠ Action Required").color(Color32::from_rgb(255, 140, 60)).strong());
                                        }
                                        ui.end_row();
                                    });

                                ui.add_space(4.0);
                                show_wizard_audit_status(ui, state);
                            });
                    });
            });
        });

    state.show_map_wizard_dialog = is_open;

    // Preset JSON Import / Export Modal Dialog
    if let Some(mut json_text) = state.wizard_preset_json_modal.clone() {
        let mut modal_open = true;
        let mut should_close = false;
        egui::Window::new("★ Map Wizard Preset JSON")
            .open(&mut modal_open)
            .resizable(true)
            .default_size([560.0, 420.0])
            .show(ctx, |ui| {
                ui.label(RichText::new("Copy, save, or paste a complete Map Wizard JSON preset below:").size(11.5));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("📋 Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(json_text.clone());
                        state.toast_info("Preset JSON copied to clipboard");
                    }
                    if ui
                        .button(
                            RichText::new("✔ Apply & Import Preset")
                                .strong()
                                .color(Color32::from_rgb(100, 255, 140)),
                        )
                        .clicked()
                    {
                        match state.import_wizard_preset_json(&json_text) {
                            Ok(()) => {
                                state.toast_info("Wizard preset successfully applied!");
                                should_close = true;
                            }
                            Err(e) => {
                                state.toast_error(format!("Failed to import preset: {e}"));
                            }
                        }
                    }
                    if ui.button("✖ Cancel").clicked() {
                        should_close = true;
                    }
                });
                ui.add_space(6.0);
                egui::ScrollArea::vertical()
                    .max_height(320.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut json_text)
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY)
                                .desired_rows(18),
                        );
                    });
            });
        if modal_open && !should_close {
            state.wizard_preset_json_modal = Some(json_text);
        } else {
            state.wizard_preset_json_modal = None;
        }
    }
}

#[allow(dead_code)]
fn render_mini_texture_swatch(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut EditorState,
    texture_ref: &str,
    size: f32,
) {
    let clean_ref = texture_ref.trim().trim_matches('"').trim_matches('\'');
    let handle_opt = state.texture_manager.get_or_create_egui_texture_by_ref(ctx, clean_ref);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    if let Some(ref handle) = handle_opt {
        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
        ui.painter().image(handle.id(), rect, uv, Color32::WHITE);
        ui.painter().rect_stroke(
            rect,
            1.0,
            Stroke::new(1.0_f32, Color32::from_rgb(90, 105, 125)),
            egui::StrokeKind::Outside,
        );
    } else {
        ui.painter().rect_filled(rect, 1.0, Color32::from_rgb(35, 40, 48));
        ui.painter().rect_stroke(
            rect,
            1.0,
            Stroke::new(1.0_f32, Color32::from_rgb(60, 70, 85)),
            egui::StrokeKind::Inside,
        );
    }
    resp.on_hover_ui(|ui| {
        ui.label(RichText::new(clean_ref).monospace().size(11.0));
    });
}

fn render_texture_palette_row(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut EditorState,
    role_title: &str,
    role_subtitle: &str,
    texture_ref: &str,
    accent_color: Color32,
    is_hero: bool,
) {
    let clean_ref = texture_ref.trim().trim_matches('"').trim_matches('\'');
    let handle_opt = state.texture_manager.get_or_create_egui_texture_by_ref(ctx, clean_ref);
    let dims_opt = state.texture_manager.get_texture_dimensions_by_ref(clean_ref);

    let card_fill = if is_hero {
        Color32::from_rgb(26, 34, 48)
    } else {
        Color32::from_rgb(20, 26, 36)
    };
    let border_stroke = if is_hero {
        Stroke::new(1.4_f32, accent_color)
    } else {
        Stroke::new(1.0_f32, Color32::from_rgb(50, 62, 78))
    };

    egui::Frame::group(ui.style())
        .fill(card_fill)
        .stroke(border_stroke)
        .corner_radius(6.0_f32)
        .inner_margin(egui::Margin::symmetric(6, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                // 1. Thumbnail on the left
                let thumb_size = Vec2::new(44.0, 44.0);
                let (rect, resp_thumb) = ui.allocate_exact_size(thumb_size, Sense::click());

                let border_col = if resp_thumb.hovered() {
                    Color32::WHITE
                } else {
                    accent_color
                };

                if let Some(ref handle) = handle_opt {
                    let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                    ui.painter().image(handle.id(), rect, uv, Color32::WHITE);
                } else {
                    ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(32, 38, 48));
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "UTX",
                        egui::FontId::monospace(10.0),
                        Color32::from_rgb(130, 145, 165),
                    );
                }
                ui.painter().rect_stroke(
                    rect,
                    2.0,
                    Stroke::new(if resp_thumb.hovered() || is_hero { 1.6_f32 } else { 1.0_f32 }, border_col),
                    egui::StrokeKind::Outside,
                );

                if resp_thumb.clicked() {
                    state.active_texture = Some(clean_ref.to_string());
                    state.toast_info(format!("Active texture set to: {}", clean_ref));
                }

                resp_thumb.on_hover_ui(|ui| {
                    ui.set_max_width(280.0);
                    ui.vertical(|ui| {
                        ui.label(RichText::new(role_title).strong().color(accent_color));
                        ui.label(RichText::new(role_subtitle).size(10.0).color(Color32::from_rgb(170, 185, 205)));
                        ui.separator();
                        ui.label(RichText::new(clean_ref).monospace().size(11.0).color(Color32::from_rgb(140, 210, 255)));
                        if let Some((w, h)) = dims_opt {
                            ui.label(RichText::new(format!("Dimensions: {}x{} px", w, h)).size(10.5).color(Color32::LIGHT_GRAY));
                        }
                        if let Some((pkg, _)) = clean_ref.split_once('.') {
                            ui.label(RichText::new(format!("Package: {}.utx", pkg)).size(10.0).color(Color32::GRAY));
                        }
                        ui.separator();
                        ui.label(RichText::new("Click thumbnail to set as Active Texture in Editor").size(10.0).italics().color(Color32::from_rgb(120, 230, 160)));
                    });
                });

                ui.add_space(8.0);

                // 2. Center Content Column (Role, Ref, Metadata)
                ui.vertical(|ui| {
                    // Line 1: Role title + subtitle badge
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(role_title)
                                .strong()
                                .size(11.0)
                                .color(if is_hero { Color32::from_rgb(255, 220, 110) } else { accent_color }),
                        );
                        ui.label(
                            RichText::new(format!("• {}", role_subtitle))
                                .size(9.5)
                                .color(Color32::from_rgb(145, 160, 180)),
                        );
                    });

                    // Line 2: Monospace full texture reference
                    let display_ref = if clean_ref.is_empty() { "None" } else { clean_ref };
                    ui.label(
                        RichText::new(display_ref)
                            .monospace()
                            .size(10.5)
                            .strong()
                            .color(Color32::from_rgb(220, 230, 245)),
                    );

                    // Line 3: Dimensions & Package metadata
                    ui.horizontal(|ui| {
                        if let Some((w, h)) = dims_opt {
                            ui.label(
                                RichText::new(format!("{}x{} px", w, h))
                                    .size(9.5)
                                    .color(Color32::from_rgb(155, 175, 195)),
                            );
                        }
                        if let Some((pkg, _)) = clean_ref.split_once('.') {
                            ui.label(
                                RichText::new(format!("in {}.utx", pkg))
                                    .size(9.5)
                                    .color(Color32::from_rgb(130, 150, 170)),
                            );
                        }
                    });
                });

                // 3. Action button on the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let is_active = state.active_texture.as_deref() == Some(clean_ref);
                    if is_active {
                        ui.label(
                            RichText::new("✔ Active")
                                .size(10.5)
                                .strong()
                                .color(Color32::from_rgb(100, 255, 160)),
                        );
                    } else if ui
                        .small_button(RichText::new("Set Active").size(10.0))
                        .on_hover_text("Select this texture as the active brush texture in the 3D editor")
                        .clicked()
                    {
                        state.active_texture = Some(clean_ref.to_string());
                        state.toast_info(format!("Active texture set to: {}", clean_ref));
                    }
                });
            });
        });
}

fn render_theme_visual_preview(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    state: &mut EditorState,
    pal: &WizardResolvedPalette,
) {
    egui::Frame::group(ui.style())
        .fill(Color32::from_rgb(18, 24, 34))
        .inner_margin(egui::Margin::same(8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("◈ Visual Theme & Texture Palette Preview")
                        .strong()
                        .color(Color32::from_rgb(255, 215, 95)),
                );
                if let Some(ref src_map) = state.map_wizard_config.custom_dna_source_map {
                    ui.colored_label(Color32::from_rgb(110, 255, 160), format!("✔ Cloned Official DNA: {}", src_map));
                } else {
                    ui.colored_label(Color32::from_rgb(130, 205, 255), format!("★ Theme: {}", state.map_wizard_config.theme.label()));
                }
            });

            ui.add_space(2.0);
            ui.label(
                RichText::new("Hierarchical architectural surfaces for room brushes. Click any texture or 'Set Active' to edit.")
                    .size(10.0)
                    .color(Color32::from_rgb(150, 165, 185)),
            );

            ui.add_space(6.0);

            let sec_wall = if pal.secondary_wall_texture.is_empty() {
                &pal.wall_texture
            } else {
                &pal.secondary_wall_texture
            };

            // 1. Primary Wall (Hero, Gold)
            render_texture_palette_row(
                ui,
                ctx,
                state,
                "★ Primary Wall",
                "Main Surfaces & Dominant Walls",
                &pal.wall_texture,
                Color32::from_rgb(255, 215, 90),
                true,
            );
            ui.add_space(3.0);

            // 2. Secondary Wall (Directly underneath Primary Wall! Cyan)
            render_texture_palette_row(
                ui,
                ctx,
                state,
                "Secondary Wall",
                "Accent Panels, Pillars & Inset Alcoves",
                sec_wall,
                Color32::from_rgb(110, 210, 255),
                false,
            );
            ui.add_space(3.0);

            // 3. Floor (Emerald Green)
            render_texture_palette_row(
                ui,
                ctx,
                state,
                "Floor",
                "Ground Walkways & Arena Combat Floors",
                &pal.floor_texture,
                Color32::from_rgb(90, 230, 140),
                false,
            );
            ui.add_space(3.0);

            // 4. Ceiling (Purple)
            render_texture_palette_row(
                ui,
                ctx,
                state,
                "Ceiling",
                "Overhead Vaults & Roof Structures",
                &pal.ceiling_texture,
                Color32::from_rgb(195, 140, 255),
                false,
            );
            ui.add_space(3.0);

            // 5. Trim (Warm Amber)
            render_texture_palette_row(
                ui,
                ctx,
                state,
                "Trim",
                "Architectural Steps, Support Beams & Edges",
                &pal.trim_texture,
                Color32::from_rgb(255, 175, 75),
                false,
            );
            ui.add_space(3.0);

            // 6. Skybox or Liquid / Hazard
            if !pal.sky_texture.is_empty() {
                render_texture_palette_row(
                    ui,
                    ctx,
                    state,
                    "Skybox",
                    "3D Atmosphere Chamber & Outer Voids",
                    &pal.sky_texture,
                    Color32::from_rgb(120, 215, 255),
                    false,
                );
            } else {
                render_texture_palette_row(
                    ui,
                    ctx,
                    state,
                    "Liquid / Hazard",
                    "Fluid Hazard Basin (Lava / Slime / Water)",
                    &pal.liquid_texture,
                    Color32::from_rgb(255, 110, 80),
                    false,
                );
            }

            ui.add_space(6.0);
            ui.separator();

            // Atmospheric Lighting & Mood Sub-card
            let light_rgb = ut99_light_to_rgb(pal.primary_light_hue, pal.primary_light_sat, pal.primary_light_bright);
            let primary_col = Color32::from_rgb(
                (light_rgb.x.clamp(0.0, 1.0) * 255.0) as u8,
                (light_rgb.y.clamp(0.0, 1.0) * 255.0) as u8,
                (light_rgb.z.clamp(0.0, 1.0) * 255.0) as u8,
            );
            let amb_intensity = ((pal.ambient_brightness as f32 / 32.0).clamp(0.15, 1.0) * 255.0) as u8;
            let amb_rgb = ut99_light_to_rgb(pal.ambient_hue, pal.ambient_saturation, amb_intensity);
            let ambient_col = Color32::from_rgb(
                (amb_rgb.x.clamp(0.0, 1.0) * 255.0) as u8,
                (amb_rgb.y.clamp(0.0, 1.0) * 255.0) as u8,
                (amb_rgb.z.clamp(0.0, 1.0) * 255.0) as u8,
            );

            egui::Frame::group(ui.style())
                .fill(Color32::from_rgb(15, 20, 28))
                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)))
                .corner_radius(6.0_f32)
                .inner_margin(egui::Margin::symmetric(8, 6))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new("Mood & Atmosphere:").size(11.0).strong().color(Color32::from_rgb(190, 210, 235)));

                            // Primary Light Swatch
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, primary_col);
                            ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
                            resp.on_hover_text(format!(
                                "Primary Light HSV: (Hue={}, Sat={}, Bright={})",
                                pal.primary_light_hue, pal.primary_light_sat, pal.primary_light_bright
                            ));
                            ui.label(RichText::new(format!("Primary (H:{})", pal.primary_light_hue)).size(10.5));

                            ui.separator();

                            // Ambient Tint Swatch
                            let (rect, resp) = ui.allocate_exact_size(Vec2::new(14.0, 14.0), Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, ambient_col);
                            ui.painter().rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Outside);
                            resp.on_hover_text(format!(
                                "Ambient Tone HSV: (Hue={}, Sat={}, Bright={})",
                                pal.ambient_hue, pal.ambient_saturation, pal.ambient_brightness
                            ));
                            ui.label(RichText::new(format!("Ambient (H:{})", pal.ambient_hue)).size(10.5));
                        });

                        ui.add_space(2.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(RichText::new(format!("♫ Song: {}", pal.song)).size(10.5).color(Color32::from_rgb(195, 225, 175)));

                            if pal.hazard_damage_per_sec > 0 {
                                ui.separator();
                                ui.label(RichText::new(format!("☣ Hazard Dmg: {} DPS", pal.hazard_damage_per_sec)).size(10.5).color(Color32::from_rgb(255, 140, 120)));
                            }
                        });
                    });
                });
        });
}
