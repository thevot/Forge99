//! UT99 Skin & Character Authoring Studio Dialog
//!
//! Interactive graphical environment for creating, editing, previewing, and packaging
//! player and bot skins for Unreal Tournament 99.

use std::path::PathBuf;

use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke, StrokeKind, Ui, Vec2, Window};
use ut99_bsp::skin_studio::{
    audit_skin_project, export_skin_project, generate_complete_skin_project,
    generate_int_registration_manifest, generate_procedural_chest, generate_procedural_legs,
    generate_styled_talk_texture, generate_talk_texture_from_face, get_builtin_skin_models,
    randomize_skin_config, SkinArchetype, SkinAuditSeverity, SkinCamoPattern, SkinFaceEntry,
    SkinFaceStyle, SkinInsignia, SkinModelDefinition, SkinTeamColor, SkinTexture,
};

/// Shows the UT99 Skin & Character Studio window
pub fn show_skin_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_skin_studio_dialog {
        return;
    }

    let mut is_open = state.show_skin_studio_dialog;
    let models = get_builtin_skin_models();

    // Auto-rotate camera if enabled
    if state.skin_studio_settings.auto_rotate {
        let dt = ctx.input(|i| i.stable_dt);
        state.skin_studio_settings.preview_3d_yaw += dt * 45.0;
        if state.skin_studio_settings.preview_3d_yaw >= 360.0 {
            state.skin_studio_settings.preview_3d_yaw -= 360.0;
        }
        ctx.request_repaint();
    }

    let screen = ctx.screen_rect();
    let max_w = (screen.width() - 32.0).max(750.0);
    let max_h = (screen.height() - 48.0).max(520.0);
    let default_w = (screen.width() * 0.90).clamp(960.0, 1320.0).min(max_w);
    let default_h = (screen.height() * 0.88).clamp(640.0, 880.0).min(max_h);

    Window::new("🎨 UT99 Skin & Character Authoring Studio")
        .open(&mut is_open)
        .default_size([default_w, default_h])
        .min_width(750.0)
        .min_height(520.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.heading("UT99 Skin & Character Authoring Studio");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let report = audit_skin_project(&state.skin_studio_project);
                    let errors = report.iter().filter(|r| r.severity == SkinAuditSeverity::Error).count();
                    let warnings = report.iter().filter(|r| r.severity == SkinAuditSeverity::Warning).count();

                    if errors > 0 {
                        ui.colored_label(Color32::from_rgb(255, 90, 90), format!("❌ {errors} Errors"));
                    } else if warnings > 0 {
                        ui.colored_label(Color32::from_rgb(255, 200, 60), format!("⚠ {warnings} Warnings"));
                    } else {
                        ui.colored_label(Color32::from_rgb(80, 220, 100), "✔ Conforming to UT99");
                    }
                });
            });

            ui.separator();

            // Tab Bar
            ui.horizontal(|ui| {
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 0, "🪄 Skin Wizard");
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 1, "👤 Model & Identity");
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 2, "🛡 Textures & 4 Teams");
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 3, "🎭 Faces & TalkTextures");
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 4, "👁 3D Viewport");
                ui.selectable_value(&mut state.skin_studio_settings.active_tab, 5, "📦 Export & Manifest (.INT)");
            });

            ui.separator();

            // Active Tab Content
            match state.skin_studio_settings.active_tab {
                0 => render_skin_wizard_tab(ui, state, &models),
                1 => render_model_and_identity_tab(ui, state, &models),
                2 => render_textures_and_teams_tab(ui, state),
                3 => render_faces_and_talk_textures_tab(ui, state),
                4 => render_3d_viewport_tab(ui, state, &models),
                5 => render_export_and_manifest_tab(ui, state),
                _ => {}
            }
        });

    state.show_skin_studio_dialog = is_open;
}

// -------------------------------------------------------------------------------------------------
// Tab 0: Procedural Skin Wizard & Randomizer
// -------------------------------------------------------------------------------------------------

fn render_skin_wizard_tab(
    ui: &mut Ui,
    state: &mut EditorState,
    models: &[SkinModelDefinition],
) {
    let avail = ui.available_size();
    let left_w = (avail.x * 0.52).clamp(420.0, 680.0);
    let right_w = (avail.x - left_w - 12.0).max(340.0);

    ui.horizontal(|ui| {
        // Left Column: Controls, Sliders & Archetypes
        ui.allocate_ui_with_layout(
            egui::vec2(left_w, avail.y),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("wizard_controls_scroll")
                    .show(ui, |ui| {
                        // Banner
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.heading("🪄 Procedural Skin Wizard");
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new("1-Click Conforming Generator")
                                            .color(Color32::from_rgb(120, 220, 255))
                                            .strong(),
                                    );
                                });
                            });
                            ui.label(
                                "Synthesize complete, authentic UT99 skins with 4-team matrix, \
                                 quantized 256-color palettes, and HUD TalkTextures.",
                            );
                        });

                        ui.add_space(6.0);

                        // Master Generation Action Bar
                        ui.group(|ui| {
                            ui.horizontal_wrapped(|ui| {
                                if ui.button(RichText::new("🎲 Roll Random Skin").size(14.0).strong()).clicked() {
                                    state.skin_studio_settings.skin_texture_cache.0.clear();
                                    let rand_seed = (state.skin_studio_settings.wizard_config.seed.wrapping_mul(6364136223846793005)).wrapping_add(1442695040888963407);
                                    state.skin_studio_settings.wizard_config = randomize_skin_config(rand_seed, None);
                                    state.skin_studio_project = generate_complete_skin_project(
                                        &state.skin_studio_settings.wizard_config,
                                        state.skin_studio_project.selected_model_index,
                                    );
                                    state.skin_studio_settings.export_status_message = Some(format!(
                                        "Generated new randomized skin '{}' ({}) with seed #{rand_seed}!",
                                        state.skin_studio_project.skin_id,
                                        state.skin_studio_settings.wizard_config.archetype.display_name(),
                                    ));
                                }

                                if ui.button(RichText::new("⚡ Synthesize").size(13.0).strong()).clicked() {
                                    state.skin_studio_settings.skin_texture_cache.0.clear();
                                    state.skin_studio_project = generate_complete_skin_project(
                                        &state.skin_studio_settings.wizard_config,
                                        state.skin_studio_project.selected_model_index,
                                    );
                                    state.skin_studio_settings.export_status_message = Some(format!(
                                        "Synthesized skin '{}' with archetype '{}'!",
                                        state.skin_studio_project.skin_id,
                                        state.skin_studio_settings.wizard_config.archetype.display_name(),
                                    ));
                                }

                                if ui.button("🛡 Auto 4-Teams").clicked() {
                                    state.skin_studio_settings.skin_texture_cache.0.clear();
                                    let cfg = &state.skin_studio_settings.wizard_config;
                                    state.skin_studio_project.chest_teams[0] = Some(generate_procedural_chest(cfg, SkinTeamColor::Red));
                                    state.skin_studio_project.chest_teams[1] = Some(generate_procedural_chest(cfg, SkinTeamColor::Blue));
                                    state.skin_studio_project.chest_teams[2] = Some(generate_procedural_chest(cfg, SkinTeamColor::Green));
                                    state.skin_studio_project.chest_teams[3] = Some(generate_procedural_chest(cfg, SkinTeamColor::Gold));

                                    state.skin_studio_project.legs_teams[0] = Some(generate_procedural_legs(cfg, SkinTeamColor::Red));
                                    state.skin_studio_project.legs_teams[1] = Some(generate_procedural_legs(cfg, SkinTeamColor::Blue));
                                    state.skin_studio_project.legs_teams[2] = Some(generate_procedural_legs(cfg, SkinTeamColor::Green));
                                    state.skin_studio_project.legs_teams[3] = Some(generate_procedural_legs(cfg, SkinTeamColor::Gold));

                                    state.skin_studio_settings.export_status_message = Some(
                                        "Auto-generated 4-team color matrix (Red, Blue, Green, Gold) matching base theme!".to_string(),
                                    );
                                }

                                ui.separator();
                                ui.label("Seed:");
                                ui.add(egui::DragValue::new(&mut state.skin_studio_settings.wizard_config.seed).speed(1));
                                if ui.button("🎲").on_hover_text("Re-roll Seed").clicked() {
                                    state.skin_studio_settings.wizard_config.seed = state.skin_studio_settings.wizard_config.seed.wrapping_add(314159);
                                }
                            });
                        });

                        ui.add_space(6.0);

                        // Archetype Selector
                        ui.group(|ui| {
                            ui.label(RichText::new("1. Character Archetype Selection").strong());
                            ui.label("Choose a visual style, or let the randomizer pick:");

                            egui::Grid::new("archetype_selector_grid")
                                .num_columns(3)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    let all_archs = SkinArchetype::all();
                                    for (idx, arch) in all_archs.iter().enumerate() {
                                        let is_selected = state.skin_studio_settings.wizard_config.archetype == *arch;
                                        let text = RichText::new(arch.display_name()).strong();
                                        let text = if is_selected {
                                            text.color(Color32::from_rgb(100, 220, 255))
                                        } else {
                                            text
                                        };

                                        if ui.selectable_label(is_selected, text).on_hover_text(arch.description()).clicked() {
                                            state.skin_studio_settings.wizard_config.archetype = *arch;
                                            let temp_cfg = randomize_skin_config(state.skin_studio_settings.wizard_config.seed, Some(*arch));
                                            state.skin_studio_settings.wizard_config.camo_pattern = temp_cfg.camo_pattern;
                                            state.skin_studio_settings.wizard_config.insignia = temp_cfg.insignia;
                                            state.skin_studio_settings.wizard_config.face_style = temp_cfg.face_style;
                                            state.skin_studio_settings.wizard_config.skin_description = temp_cfg.skin_description;
                                            state.skin_studio_settings.wizard_config.skin_name = temp_cfg.skin_name;
                                        }

                                        if (idx + 1) % 3 == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });

                            ui.add_space(4.0);
                            ui.colored_label(
                                Color32::from_rgb(180, 200, 220),
                                format!("Active: {}", state.skin_studio_settings.wizard_config.archetype.description()),
                            );
                        });

                        ui.add_space(6.0);

                        // Fine-Tuning & Presets
                        ui.group(|ui| {
                            ui.label(RichText::new("2. Visual Elements & Tactical Presets").strong());

                            egui::Grid::new("wizard_presets_grid")
                                .num_columns(2)
                                .spacing([12.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Camouflage Pattern:");
                                    egui::ComboBox::from_id_salt("camo_combo")
                                        .selected_text(state.skin_studio_settings.wizard_config.camo_pattern.display_name())
                                        .show_ui(ui, |ui| {
                                            for pat in SkinCamoPattern::all() {
                                                ui.selectable_value(
                                                    &mut state.skin_studio_settings.wizard_config.camo_pattern,
                                                    *pat,
                                                    pat.display_name(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Chest Insignia / Decal:");
                                    egui::ComboBox::from_id_salt("insignia_combo")
                                        .selected_text(state.skin_studio_settings.wizard_config.insignia.display_name())
                                        .show_ui(ui, |ui| {
                                            for ins in SkinInsignia::all() {
                                                ui.selectable_value(
                                                    &mut state.skin_studio_settings.wizard_config.insignia,
                                                    *ins,
                                                    ins.display_name(),
                                                );
                                            }
                                        });
                                    ui.end_row();

                                    ui.label("Face & Head Styling:");
                                    egui::ComboBox::from_id_salt("face_style_combo")
                                        .selected_text(state.skin_studio_settings.wizard_config.face_style.display_name())
                                        .show_ui(ui, |ui| {
                                            for style in SkinFaceStyle::all() {
                                                ui.selectable_value(
                                                    &mut state.skin_studio_settings.wizard_config.face_style,
                                                    *style,
                                                    style.display_name(),
                                                );
                                            }
                                        });
                                    ui.end_row();
                                });

                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);

                            // Sliders
                            ui.horizontal(|ui| {
                                ui.label("Armor Plating:");
                                ui.add(
                                    egui::Slider::new(&mut state.skin_studio_settings.wizard_config.armor_coverage, 0.1..=0.95)
                                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Battle Weathering:");
                                ui.add(
                                    egui::Slider::new(&mut state.skin_studio_settings.wizard_config.weathering, 0.0..=0.95)
                                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                );
                            });

                            ui.horizontal(|ui| {
                                ui.label("Team Trim Accent:");
                                ui.add(
                                    egui::Slider::new(&mut state.skin_studio_settings.wizard_config.team_trim_intensity, 0.2..=1.0)
                                        .custom_formatter(|n, _| format!("{:.0}%", n * 100.0)),
                                );
                            });
                        });

                        ui.add_space(6.0);

                        // Identity & Naming
                        ui.group(|ui| {
                            ui.label(RichText::new("3. Package Identity & UT99 Registration").strong());

                            egui::Grid::new("wizard_identity_grid")
                                .num_columns(2)
                                .spacing([10.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label("Skin Prefix (3-6 chars):");
                                    ui.text_edit_singleline(&mut state.skin_studio_settings.wizard_config.skin_name);
                                    ui.end_row();

                                    ui.label("Skin Description:");
                                    ui.text_edit_singleline(&mut state.skin_studio_settings.wizard_config.skin_description);
                                    ui.end_row();

                                    ui.label("Author / Modder:");
                                    ui.text_edit_singleline(&mut state.skin_studio_settings.wizard_config.author);
                                    ui.end_row();
                                });
                        });
                    });
            },
        );

        ui.separator();

        // Right Column: Live Composite Character Viewport & GPU Swatch Inspector
        ui.allocate_ui_with_layout(
            egui::vec2(right_w, avail.y),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("wizard_preview_scroll")
                    .show(ui, |ui| {
                        render_wizard_live_preview_panel(ui, state, models);
                    });
            },
        );
    });
}

// -------------------------------------------------------------------------------------------------
// Tab 0: Model & Identity
// -------------------------------------------------------------------------------------------------

fn render_model_and_identity_tab(
    ui: &mut Ui,
    state: &mut EditorState,
    models: &[ut99_bsp::skin_studio::SkinModelDefinition],
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.group(|ui| {
            ui.label(RichText::new("Target Character Model Selection").strong());
            ui.horizontal(|ui| {
                ui.label("Base Mesh Model:");
                let cur_model_idx = state.skin_studio_project.selected_model_index.min(models.len().saturating_sub(1));
                let cur_name = models[cur_model_idx].display_name;

                egui::ComboBox::from_id_salt("skin_model_selector")
                    .selected_text(cur_name)
                    .show_ui(ui, |ui| {
                        for (idx, m) in models.iter().enumerate() {
                            if ui.selectable_value(&mut state.skin_studio_project.selected_model_index, idx, m.display_name).clicked() {
                                // Auto-update package prefix if default
                                let prefix = m.model_prefix;
                                state.skin_studio_project.package_name = format!("{prefix}Skins_{}", state.skin_studio_project.skin_id);
                            }
                        }
                    });
            });

            let cur_m = &models[state.skin_studio_project.selected_model_index.min(models.len().saturating_sub(1))];
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!("Mesh Actor: '{}'", cur_m.mesh_package));
                ui.separator();
                ui.label(format!("Prefix: '{}'", cur_m.model_prefix));
                ui.separator();
                ui.label(format!("Scale: [{:.2}, {:.2}, {:.2}]", cur_m.default_scale.x, cur_m.default_scale.y, cur_m.default_scale.z));
            });

            ui.add_space(4.0);
            ui.label("Supported Sections:");
            for sec in cur_m.sections {
                let team_badge = if sec.requires_team { " [4-Team Enabled]" } else { "" };
                let face_badge = if sec.requires_face { " [Face/Talk]" } else { "" };
                ui.label(format!("  • Slot {}: {} {}{}", sec.slot, sec.name, team_badge, face_badge));
            }
        });

        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(RichText::new("Package Identity & Naming Conventions").strong());
            ui.horizontal(|ui| {
                ui.label("Skin Identifier (Prefix):");
                if ui.text_edit_singleline(&mut state.skin_studio_project.skin_id).changed() {
                    let cur_m = &models[state.skin_studio_project.selected_model_index.min(models.len().saturating_sub(1))];
                    state.skin_studio_project.package_name = format!("{}Skins_{}", cur_m.model_prefix, state.skin_studio_project.skin_id);
                }
                ui.label("(e.g. 'army', 'sldr', 'bot')");
            });

            ui.horizontal(|ui| {
                ui.label("Package Name (.utx):");
                ui.text_edit_singleline(&mut state.skin_studio_project.package_name);
            });

            ui.horizontal(|ui| {
                ui.label("Friendly Skin Name:");
                ui.text_edit_singleline(&mut state.skin_studio_project.skin_description);
                ui.label("(Visible in Character Selection Menu)");
            });

            ui.horizontal(|ui| {
                ui.label("Author / Creator:");
                ui.text_edit_singleline(&mut state.skin_studio_project.author);
            });
        });
    });
}

// -------------------------------------------------------------------------------------------------
// Tab 1: Textures & 4 Teams
// -------------------------------------------------------------------------------------------------

fn render_textures_and_teams_tab(ui: &mut Ui, state: &mut EditorState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        // Team Selector
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Active Team Filter:").strong());
                for team in [
                    SkinTeamColor::None,
                    SkinTeamColor::Red,
                    SkinTeamColor::Blue,
                    SkinTeamColor::Green,
                    SkinTeamColor::Gold,
                ] {
                    let [r, g, b] = team.badge_rgb();
                    let text = RichText::new(team.display_name()).color(Color32::from_rgb(r, g, b));
                    ui.selectable_value(&mut state.skin_studio_project.active_team, team, text);
                }

                ui.separator();
                ui.checkbox(&mut state.skin_studio_settings.show_uv_wireframe_2d, "📐 Show UV Wireframe Seams Overlay");
            });
        });

        ui.add_space(8.0);

        let active_team = state.skin_studio_project.active_team;
        let show_wf = state.skin_studio_settings.show_uv_wireframe_2d;

        // Slot 1: Torso / Chest
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Slot 1: Torso / Chest Texture");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Fill Test Color").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let col = match active_team {
                            SkinTeamColor::None => [60, 80, 60, 255],
                            SkinTeamColor::Red => [200, 35, 35, 255],
                            SkinTeamColor::Blue => [35, 75, 210, 255],
                            SkinTeamColor::Green => [35, 180, 45, 255],
                            SkinTeamColor::Gold => [210, 180, 25, 255],
                        };
                        let name = format!("Chest_{}", active_team.display_name());
                        let tex = SkinTexture::create_solid(name, 256, 256, col);
                        if let Some(t_idx) = active_team.team_index() {
                            state.skin_studio_project.chest_teams[t_idx] = Some(tex);
                        } else {
                            state.skin_studio_project.chest_base = Some(tex);
                        }
                    }

                    if ui.button("🪄 Procedural Generate").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let tex = generate_procedural_chest(&state.skin_studio_settings.wizard_config, active_team);
                        if let Some(t_idx) = active_team.team_index() {
                            state.skin_studio_project.chest_teams[t_idx] = Some(tex);
                        } else {
                            state.skin_studio_project.chest_base = Some(tex);
                        }
                    }
                });
            });

            let tex_ref = match active_team.team_index() {
                Some(idx) => state.skin_studio_project.chest_teams[idx].as_ref(),
                None => state.skin_studio_project.chest_base.as_ref(),
            };

            ui.horizontal(|ui| {
                render_texture_swatch(ui, &mut state.skin_studio_settings.skin_texture_cache.0, tex_ref, Vec2::new(72.0, 72.0), show_wf);
                ui.vertical(|ui| {
                    if let Some(t) = tex_ref {
                        ui.label(format!("Resolution: {}x{}", t.width, t.height));
                        ui.label(format!("MipMaps: {} levels", t.mipmaps.len()));
                        ui.label("Palette: 256 Colors (Optimized)");
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 120, 120), "No texture assigned for this team.");
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Slot 2: Legs
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Slot 2: Legs / Lower Body Texture");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Fill Test Color").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let col = match active_team {
                            SkinTeamColor::None => [50, 50, 55, 255],
                            SkinTeamColor::Red => [150, 25, 25, 255],
                            SkinTeamColor::Blue => [25, 60, 160, 255],
                            SkinTeamColor::Green => [25, 130, 35, 255],
                            SkinTeamColor::Gold => [170, 140, 20, 255],
                        };
                        let name = format!("Legs_{}", active_team.display_name());
                        let tex = SkinTexture::create_solid(name, 256, 256, col);
                        if let Some(t_idx) = active_team.team_index() {
                            state.skin_studio_project.legs_teams[t_idx] = Some(tex);
                        } else {
                            state.skin_studio_project.legs_base = Some(tex);
                        }
                    }

                    if ui.button("🪄 Procedural Generate").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let tex = generate_procedural_legs(&state.skin_studio_settings.wizard_config, active_team);
                        if let Some(t_idx) = active_team.team_index() {
                            state.skin_studio_project.legs_teams[t_idx] = Some(tex);
                        } else {
                            state.skin_studio_project.legs_base = Some(tex);
                        }
                    }
                });
            });

            let tex_ref = match active_team.team_index() {
                Some(idx) => state.skin_studio_project.legs_teams[idx].as_ref(),
                None => state.skin_studio_project.legs_base.as_ref(),
            };

            ui.horizontal(|ui| {
                render_texture_swatch(ui, &mut state.skin_studio_settings.skin_texture_cache.0, tex_ref, Vec2::new(72.0, 72.0), show_wf);
                ui.vertical(|ui| {
                    if let Some(t) = tex_ref {
                        ui.label(format!("Resolution: {}x{}", t.width, t.height));
                        ui.label(format!("MipMaps: {} levels", t.mipmaps.len()));
                        ui.label("Palette: 256 Colors");
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 120, 120), "No texture assigned for this team.");
                    }
                });
            });
        });

        ui.add_space(8.0);

        // Slot 3: Arms (Invariant)
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Slot 3: Arms / Shoulders (Team Invariant)");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🪄 Procedural Generate").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let tex = ut99_bsp::skin_studio::generate_procedural_arms(&state.skin_studio_settings.wizard_config);
                        state.skin_studio_project.arms_base = Some(tex);
                    }
                });
            });
            let tex_ref = state.skin_studio_project.arms_base.as_ref();

            ui.horizontal(|ui| {
                render_texture_swatch(ui, &mut state.skin_studio_settings.skin_texture_cache.0, tex_ref, Vec2::new(72.0, 72.0), show_wf);
                ui.vertical(|ui| {
                    if let Some(t) = tex_ref {
                        ui.label(format!("Resolution: {}x{}", t.width, t.height));
                        ui.label(format!("MipMaps: {} levels", t.mipmaps.len()));
                    } else {
                        ui.colored_label(Color32::from_rgb(255, 120, 120), "No arms texture assigned.");
                    }
                });
            });
        });
    });
}

// -------------------------------------------------------------------------------------------------
// Tab 2: Faces & TalkTextures
// -------------------------------------------------------------------------------------------------

fn render_faces_and_talk_textures_tab(ui: &mut Ui, state: &mut EditorState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Character Faces & HUD TalkTextures (Slots 4 & 5)");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("+ Add New Face").clicked() {
                    state.skin_studio_settings.skin_texture_cache.0.clear();
                    let num = state.skin_studio_project.faces.len() + 1;
                    let face_tex = SkinTexture::create_solid(format!("Face_{num}"), 256, 256, [210, 160, 130, 255]);
                    state.skin_studio_project.faces.push(SkinFaceEntry::new(
                        format!("Face{num}"),
                        format!("Custom Face {num}"),
                        face_tex,
                    ));
                }
            });
        });

        ui.separator();

        let faces_len = state.skin_studio_project.faces.len();
        if faces_len == 0 {
            ui.colored_label(Color32::from_rgb(255, 100, 100), "No faces created. Click '+ Add New Face'.");
            return;
        }

        let cur_face_idx = state.skin_studio_project.active_face_index.min(faces_len - 1);

        // Face selector tabs
        ui.horizontal(|ui| {
            ui.label("Select Face:");
            for (idx, face) in state.skin_studio_project.faces.iter().enumerate() {
                ui.selectable_value(&mut state.skin_studio_project.active_face_index, idx, &face.display_name);
            }
        });

        ui.add_space(8.0);

        let active_face = &mut state.skin_studio_project.faces[cur_face_idx];

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Internal Face Name:");
                ui.text_edit_singleline(&mut active_face.face_name);
                ui.separator();
                ui.label("Display Name (INT Description):");
                ui.text_edit_singleline(&mut active_face.display_name);
            });
        });

        ui.add_space(8.0);

        let active_face_tex = active_face.face_texture.clone();
        let active_talk_tex = active_face.talk_texture.clone();

        ui.horizontal(|ui| {
            // Slot 4: Face
            ui.group(|ui| {
                ui.set_min_width(320.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Slot 4: Head & Face Texture (256x256)").strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🪄 Procedural Face").clicked() {
                            state.skin_studio_settings.skin_texture_cache.0.clear();
                            let face_tex = ut99_bsp::skin_studio::generate_procedural_face(&state.skin_studio_settings.wizard_config);
                            let talk_tex = generate_styled_talk_texture(
                                &face_tex,
                                &state.skin_studio_settings.wizard_config,
                                SkinTeamColor::None,
                            );
                            let cur_idx = state.skin_studio_project.active_face_index.min(state.skin_studio_project.faces.len() - 1);
                            state.skin_studio_project.faces[cur_idx].face_texture = face_tex;
                            state.skin_studio_project.faces[cur_idx].talk_texture = talk_tex;
                        }
                    });
                });
                render_texture_swatch(ui, &mut state.skin_studio_settings.skin_texture_cache.0, Some(&active_face_tex), Vec2::new(128.0, 128.0), state.skin_studio_settings.show_uv_wireframe_2d);
                ui.label(format!("Resolution: {}x{}", active_face_tex.width, active_face_tex.height));
            });

            // Slot 5: TalkTexture
            ui.group(|ui| {
                ui.set_min_width(320.0);
                ui.label(RichText::new("Slot 5: HUD TalkTexture (64x64 Icon)").strong());
                render_texture_swatch(ui, &mut state.skin_studio_settings.skin_texture_cache.0, Some(&active_talk_tex), Vec2::new(64.0, 64.0), false);
                ui.label(format!("Resolution: {}x{}", active_talk_tex.width, active_talk_tex.height));
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("⚡ CRT Tactical (64x64)").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let cur_idx = state.skin_studio_project.active_face_index.min(state.skin_studio_project.faces.len() - 1);
                        let talk = generate_styled_talk_texture(
                            &state.skin_studio_project.faces[cur_idx].face_texture,
                            &state.skin_studio_settings.wizard_config,
                            state.skin_studio_project.active_team,
                        );
                        state.skin_studio_project.faces[cur_idx].talk_texture = talk;
                    }
                    if ui.button("Crop Only").clicked() {
                        state.skin_studio_settings.skin_texture_cache.0.clear();
                        let cur_idx = state.skin_studio_project.active_face_index.min(state.skin_studio_project.faces.len() - 1);
                        let talk = generate_talk_texture_from_face(&state.skin_studio_project.faces[cur_idx].face_texture);
                        state.skin_studio_project.faces[cur_idx].talk_texture = talk;
                    }
                });
            });
        });
    });
}

// -------------------------------------------------------------------------------------------------
// Tab 3: 3D Viewport
// -------------------------------------------------------------------------------------------------

fn render_3d_viewport_tab(
    ui: &mut Ui,
    state: &mut EditorState,
    models: &[ut99_bsp::skin_studio::SkinModelDefinition],
) {
    let cur_m_idx = state.skin_studio_project.selected_model_index.min(models.len().saturating_sub(1));
    let cur_m = &models[cur_m_idx];

    // Viewport Controls Toolbar
    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Mesh: '{}'", cur_m.display_name)).strong());
        ui.separator();

        ui.label("Camera Preset:");
        for (p_idx, preset) in cur_m.camera_presets.iter().enumerate() {
            if ui.selectable_value(&mut state.skin_studio_project.active_camera_preset_index, p_idx, preset.name).clicked() {
                state.skin_studio_settings.preview_3d_yaw = preset.yaw_deg;
                state.skin_studio_settings.preview_3d_pitch = preset.pitch_deg;
            }
        }

        ui.separator();
        ui.checkbox(&mut state.skin_studio_settings.auto_rotate, "▶ Auto-Rotate");
        ui.checkbox(&mut state.skin_studio_settings.show_wireframe_guide, "Show Wireframe");
        if ui.button("⟲ Reset View").clicked() {
            state.skin_studio_settings.preview_3d_yaw = 25.0;
            state.skin_studio_settings.preview_3d_pitch = 0.0;
        }
    });

    ui.add_space(4.0);

    // Interactive Canvas Viewport
    let available_size = ui.available_size().max(Vec2::new(320.0, 320.0));
    render_character_composite_viewport(ui, state, models, available_size);
}

/// Right column live preview panel in Tab 0
fn render_wizard_live_preview_panel(
    ui: &mut Ui,
    state: &mut EditorState,
    models: &[ut99_bsp::skin_studio::SkinModelDefinition],
) {
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("👁 Real-Time Character Preview").strong().size(13.0));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⟲ Reset").on_hover_text("Reset Camera Rotation").clicked() {
                    state.skin_studio_settings.preview_3d_yaw = 20.0;
                    state.skin_studio_settings.preview_3d_pitch = 0.0;
                }
                ui.checkbox(&mut state.skin_studio_settings.auto_rotate, "▶ Auto-Rotate");
                ui.checkbox(&mut state.skin_studio_settings.show_wireframe_guide, "Wireframe");
            });
        });

        ui.add_space(4.0);

        // Team Selector Quick Switcher
        ui.horizontal_wrapped(|ui| {
            ui.label("Team Variant:");
            let teams = [
                (SkinTeamColor::None, "Base (DM)", Color32::from_rgb(180, 200, 220)),
                (SkinTeamColor::Red, "Red", Color32::from_rgb(255, 90, 90)),
                (SkinTeamColor::Blue, "Blue", Color32::from_rgb(90, 140, 255)),
                (SkinTeamColor::Green, "Green", Color32::from_rgb(90, 230, 120)),
                (SkinTeamColor::Gold, "Gold", Color32::from_rgb(255, 220, 70)),
            ];
            for (t_color, t_label, t_rgb) in teams {
                let is_sel = state.skin_studio_project.active_team == t_color;
                let text = RichText::new(t_label).strong();
                let text = if is_sel { text.color(t_rgb) } else { text };
                if ui.selectable_label(is_sel, text).clicked() {
                    state.skin_studio_project.active_team = t_color;
                }
            }
        });

        ui.add_space(6.0);

        // 3D Composite Character Viewport (GPU textured)
        let vp_w = ui.available_width().max(280.0);
        let vp_h = 280.0_f32;
        render_character_composite_viewport(ui, state, models, Vec2::new(vp_w, vp_h));
    });

    ui.add_space(6.0);

    // Live GPU Texture Stack
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Synthesized GPU Texture Stack (256x256)").strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut state.skin_studio_settings.show_uv_wireframe_2d, "📐 UV Seams");
            });
        });

        ui.add_space(4.0);

        let show_wf = state.skin_studio_settings.show_uv_wireframe_2d;
        let swatch_sz = Vec2::new(56.0, 56.0);

        ui.horizontal_wrapped(|ui| {
            let cache = &mut state.skin_studio_settings.skin_texture_cache.0;

            ui.vertical(|ui| {
                ui.label(RichText::new("Chest").size(10.5));
                let cur_chest = match state.skin_studio_project.active_team.team_index() {
                    Some(idx) => state.skin_studio_project.chest_teams[idx].as_ref().or(state.skin_studio_project.chest_base.as_ref()),
                    None => state.skin_studio_project.chest_base.as_ref(),
                };
                render_texture_swatch(ui, cache, cur_chest, swatch_sz, show_wf);
            });

            ui.vertical(|ui| {
                ui.label(RichText::new("Legs").size(10.5));
                let cur_legs = match state.skin_studio_project.active_team.team_index() {
                    Some(idx) => state.skin_studio_project.legs_teams[idx].as_ref().or(state.skin_studio_project.legs_base.as_ref()),
                    None => state.skin_studio_project.legs_base.as_ref(),
                };
                render_texture_swatch(ui, cache, cur_legs, swatch_sz, show_wf);
            });

            ui.vertical(|ui| {
                ui.label(RichText::new("Arms").size(10.5));
                render_texture_swatch(ui, cache, state.skin_studio_project.arms_base.as_ref(), swatch_sz, show_wf);
            });

            ui.vertical(|ui| {
                ui.label(RichText::new("Head/Face").size(10.5));
                let face_tex = state.skin_studio_project.faces.first().map(|f| &f.face_texture);
                render_texture_swatch(ui, cache, face_tex, swatch_sz, show_wf);
            });

            ui.vertical(|ui| {
                ui.label(RichText::new("HUD Talk").size(10.5));
                let talk_tex = state.skin_studio_project.faces.first().map(|f| &f.talk_texture);
                render_texture_swatch(ui, cache, talk_tex, swatch_sz, false);
            });
        });

        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let active_t = state.skin_studio_project.active_team;
            ui.label(format!("Active Team Variant: {}", active_t.display_name()));
            let col = active_t.badge_rgb();
            ui.colored_label(Color32::from_rgb(col[0], col[1], col[2]), "● Active");
        });
    });
}

/// Shared interactive composite character renderer mapping actual GPU textures
fn render_character_composite_viewport(
    ui: &mut Ui,
    state: &mut EditorState,
    models: &[ut99_bsp::skin_studio::SkinModelDefinition],
    size: Vec2,
) {
    let cur_m_idx = state.skin_studio_project.selected_model_index.min(models.len().saturating_sub(1));
    let cur_m = &models[cur_m_idx];

    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click_and_drag());
    if response.dragged() {
        let delta = response.drag_delta();
        state.skin_studio_settings.preview_3d_yaw += delta.x * 0.5;
        state.skin_studio_settings.preview_3d_pitch = (state.skin_studio_settings.preview_3d_pitch - delta.y * 0.5).clamp(-85.0, 85.0);
    }

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 6.0, Color32::from_rgb(20, 24, 32));

    // Arena background grid lines
    let grid_step = 32.0;
    let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgb(28, 33, 44));
    let mut gx = rect.left() + (rect.width() % grid_step) * 0.5;
    while gx <= rect.right() {
        painter.line_segment([egui::pos2(gx, rect.top()), egui::pos2(gx, rect.bottom())], grid_stroke);
        gx += grid_step;
    }
    let mut gy = rect.top() + (rect.height() % grid_step) * 0.5;
    while gy <= rect.bottom() {
        painter.line_segment([egui::pos2(rect.left(), gy), egui::pos2(rect.right(), gy)], grid_stroke);
        gy += grid_step;
    }

    // Active team ambient glow
    let team = state.skin_studio_project.active_team;
    let badge_rgb = team.badge_rgb();
    let glow_col = Color32::from_rgba_unmultiplied(badge_rgb[0] / 3, badge_rgb[1] / 3, badge_rgb[2] / 3, 50);
    painter.circle_filled(rect.center(), (rect.height() * 0.42).min(rect.width() * 0.42), glow_col);

    // Active textures
    let active_team_idx = team.team_index();
    let chest_tex = match active_team_idx {
        Some(idx) => state.skin_studio_project.chest_teams[idx].as_ref().or(state.skin_studio_project.chest_base.as_ref()),
        None => state.skin_studio_project.chest_base.as_ref(),
    };
    let legs_tex = match active_team_idx {
        Some(idx) => state.skin_studio_project.legs_teams[idx].as_ref().or(state.skin_studio_project.legs_base.as_ref()),
        None => state.skin_studio_project.legs_base.as_ref(),
    };
    let arms_tex = state.skin_studio_project.arms_base.as_ref();
    let face_tex = state.skin_studio_project.faces.first().map(|f| &f.face_texture);

    // 3D rotation projection factors
    let yaw_rad = state.skin_studio_settings.preview_3d_yaw.to_radians();
    let pitch_rad = state.skin_studio_settings.preview_3d_pitch.to_radians();
    let rot_x = yaw_rad.sin() * 12.0;
    let pitch_y = pitch_rad.sin() * 8.0;
    let center = rect.center() + Vec2::new(0.0, pitch_y);

    // Scale character to fit comfortably in viewport
    let scale = (rect.height() / 310.0).clamp(0.65, 1.35);

    // 1. Torso / Chest (GPU textured)
    let chest_w = 64.0 * scale;
    let chest_h = 84.0 * scale;
    let chest_rect = egui::Rect::from_center_size(center + Vec2::new(rot_x, -12.0 * scale), Vec2::new(chest_w, chest_h));

    if let Some(t) = chest_tex {
        if t.width > 0 && t.height > 0 && t.rgba.len() >= (t.width * t.height * 4) as usize {
            let handle = get_or_load_skin_texture(
                ui.ctx(),
                &mut state.skin_studio_settings.skin_texture_cache.0,
                "char_vp_chest",
                t,
            );
            painter.image(handle.id(), chest_rect, egui::Rect::from_min_max(egui::pos2(0.12, 0.08), egui::pos2(0.88, 0.92)), Color32::WHITE);
        } else {
            painter.rect_filled(chest_rect, 4.0, Color32::from_rgb(60, 65, 75));
        }
    } else {
        painter.rect_filled(chest_rect, 4.0, Color32::from_rgb(60, 65, 75));
    }
    painter.rect_stroke(chest_rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(100, 115, 135)), StrokeKind::Inside);

    // 2. Head / Face (GPU textured)
    let head_size = 44.0 * scale;
    let head_pos = center + Vec2::new(rot_x * 0.8, -70.0 * scale);
    let head_rect = egui::Rect::from_center_size(head_pos, Vec2::new(head_size, head_size));

    if let Some(t) = face_tex {
        if t.width > 0 && t.height > 0 && t.rgba.len() >= (t.width * t.height * 4) as usize {
            let handle = get_or_load_skin_texture(
                ui.ctx(),
                &mut state.skin_studio_settings.skin_texture_cache.0,
                "char_vp_face",
                t,
            );
            painter.image(handle.id(), head_rect, egui::Rect::from_min_max(egui::pos2(0.1, 0.1), egui::pos2(0.9, 0.9)), Color32::WHITE);
        } else {
            painter.circle_filled(head_pos, head_size * 0.5, Color32::from_rgb(210, 160, 130));
        }
    } else {
        painter.circle_filled(head_pos, head_size * 0.5, Color32::from_rgb(210, 160, 130));
    }
    painter.rect_stroke(head_rect, head_size * 0.4, Stroke::new(1.0_f32, Color32::from_rgb(90, 105, 125)), StrokeKind::Inside);

    // 3. Arms (Left & Right)
    let arm_w = 20.0 * scale;
    let arm_h = 76.0 * scale;
    let arm_left_rect = egui::Rect::from_center_size(center + Vec2::new((-46.0 * scale) + rot_x * 0.5, -8.0 * scale), Vec2::new(arm_w, arm_h));
    let arm_right_rect = egui::Rect::from_center_size(center + Vec2::new((46.0 * scale) + rot_x * 0.5, -8.0 * scale), Vec2::new(arm_w, arm_h));

    if let Some(t) = arms_tex {
        if t.width > 0 && t.height > 0 && t.rgba.len() >= (t.width * t.height * 4) as usize {
            let handle = get_or_load_skin_texture(
                ui.ctx(),
                &mut state.skin_studio_settings.skin_texture_cache.0,
                "char_vp_arms",
                t,
            );
            painter.image(handle.id(), arm_left_rect, egui::Rect::from_min_max(egui::pos2(0.1, 0.05), egui::pos2(0.48, 0.95)), Color32::WHITE);
            painter.image(handle.id(), arm_right_rect, egui::Rect::from_min_max(egui::pos2(0.52, 0.05), egui::pos2(0.9, 0.95)), Color32::WHITE);
        } else {
            painter.rect_filled(arm_left_rect, 3.0, Color32::from_rgb(55, 60, 65));
            painter.rect_filled(arm_right_rect, 3.0, Color32::from_rgb(55, 60, 65));
        }
    } else {
        painter.rect_filled(arm_left_rect, 3.0, Color32::from_rgb(55, 60, 65));
        painter.rect_filled(arm_right_rect, 3.0, Color32::from_rgb(55, 60, 65));
    }
    painter.rect_stroke(arm_left_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(90, 105, 120)), StrokeKind::Inside);
    painter.rect_stroke(arm_right_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(90, 105, 120)), StrokeKind::Inside);

    // 4. Legs (Left & Right)
    let leg_w = 24.0 * scale;
    let leg_h = 86.0 * scale;
    let leg_left_rect = egui::Rect::from_center_size(center + Vec2::new((-18.0 * scale) + rot_x * 0.6, 76.0 * scale), Vec2::new(leg_w, leg_h));
    let leg_right_rect = egui::Rect::from_center_size(center + Vec2::new((18.0 * scale) + rot_x * 0.6, 76.0 * scale), Vec2::new(leg_w, leg_h));

    if let Some(t) = legs_tex {
        if t.width > 0 && t.height > 0 && t.rgba.len() >= (t.width * t.height * 4) as usize {
            let handle = get_or_load_skin_texture(
                ui.ctx(),
                &mut state.skin_studio_settings.skin_texture_cache.0,
                "char_vp_legs",
                t,
            );
            painter.image(handle.id(), leg_left_rect, egui::Rect::from_min_max(egui::pos2(0.1, 0.05), egui::pos2(0.48, 0.95)), Color32::WHITE);
            painter.image(handle.id(), leg_right_rect, egui::Rect::from_min_max(egui::pos2(0.52, 0.05), egui::pos2(0.9, 0.95)), Color32::WHITE);
        } else {
            painter.rect_filled(leg_left_rect, 3.0, Color32::from_rgb(45, 50, 55));
            painter.rect_filled(leg_right_rect, 3.0, Color32::from_rgb(45, 50, 55));
        }
    } else {
        painter.rect_filled(leg_left_rect, 3.0, Color32::from_rgb(45, 50, 55));
        painter.rect_filled(leg_right_rect, 3.0, Color32::from_rgb(45, 50, 55));
    }
    painter.rect_stroke(leg_left_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(85, 95, 110)), StrokeKind::Inside);
    painter.rect_stroke(leg_right_rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(85, 95, 110)), StrokeKind::Inside);

    // Wireframe Guide overlay if enabled
    if state.skin_studio_settings.show_wireframe_guide {
        let wf_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 240, 200, 180));
        painter.line_segment([chest_rect.center_top(), chest_rect.center_bottom()], wf_stroke);
        painter.line_segment([chest_rect.left_center(), chest_rect.right_center()], wf_stroke);
        painter.line_segment([head_rect.center_top(), head_rect.center_bottom()], wf_stroke);
    }

    // Telemetry & Help
    painter.text(
        rect.left_top() + Vec2::new(10.0, 10.0),
        egui::Align2::LEFT_TOP,
        format!(
            "Model: {}\nTeam: {}\nYaw: {:.1}°  Pitch: {:.1}°\n(Drag mouse to rotate)",
            cur_m.display_name,
            team.display_name(),
            state.skin_studio_settings.preview_3d_yaw,
            state.skin_studio_settings.preview_3d_pitch
        ),
        egui::FontId::monospace(10.5),
        Color32::from_rgb(190, 205, 225),
    );
}

// -------------------------------------------------------------------------------------------------
// Tab 4: Export & Manifest
// -------------------------------------------------------------------------------------------------

fn render_export_and_manifest_tab(ui: &mut Ui, state: &mut EditorState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.heading("UT99 Package Compilation & Manifest (.INT) Generator");
        ui.separator();

        let manifest = generate_int_registration_manifest(&state.skin_studio_project);

        ui.label(RichText::new("Generated .INT Registration Manifest:").strong());
        egui::Frame::canvas(ui.style()).show(ui, |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut manifest.as_str())
                    .font(egui::TextStyle::Monospace)
                    .desired_rows(12)
                    .desired_width(f32::INFINITY)
            );
        });

        ui.add_space(8.0);

        // Audit Summary
        ui.group(|ui| {
            ui.label(RichText::new("Package Health & Engine Audit").strong());
            let report = audit_skin_project(&state.skin_studio_project);
            for item in report {
                let col = match item.severity {
                    SkinAuditSeverity::Info => Color32::from_rgb(80, 220, 100),
                    SkinAuditSeverity::Warning => Color32::from_rgb(255, 200, 60),
                    SkinAuditSeverity::Error => Color32::from_rgb(255, 90, 90),
                };
                ui.colored_label(col, format!("• {}: {}", item.title, item.detail));
            }
        });

        ui.add_space(8.0);

        // Export Destination & Options
        ui.group(|ui| {
            ui.label(RichText::new("Export Output Directory & Options").strong());
            ui.horizontal(|ui| {
                ui.label("Destination Path:");
                ui.add(
                    egui::TextEdit::singleline(&mut state.skin_studio_settings.export_output_dir)
                        .desired_width(340.0)
                        .hint_text("e.g. ExportOutput or path to UnrealTournament"),
                );
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        state.skin_studio_settings.export_output_dir = path.to_string_lossy().to_string();
                    }
                }
                if ui.button("Default").clicked() {
                    state.skin_studio_settings.export_output_dir = "ExportOutput".to_string();
                }
            });

            ui.checkbox(
                &mut state.skin_studio_settings.export_subfolders,
                "Structure into /Textures/ and /System/ subfolders (Drop-in ready for UT99 installation folder)",
            );
        });

        ui.add_space(8.0);

        // Export Actions
        ui.horizontal(|ui| {
            if ui.button(RichText::new("💾 Compile & Export .UTX and .INT").strong()).clicked() {
                let out_dir = PathBuf::from(state.skin_studio_settings.export_output_dir.trim());
                match export_skin_project(
                    &state.skin_studio_project,
                    &out_dir,
                    state.skin_studio_settings.export_subfolders,
                ) {
                    Ok(res) => {
                        state.skin_studio_settings.export_status_message = Some(format!(
                            "✔ Successfully compiled and exported!\n  • Package: '{}' ({:.1} KB)\n  • Manifest: '{}' ({} bytes)\n  • Textures Compiled: {}",
                            res.package_path.display(),
                            res.package_bytes as f64 / 1024.0,
                            res.manifest_path.display(),
                            res.manifest_bytes,
                            res.textures_compiled,
                        ));
                    }
                    Err(e) => {
                        state.skin_studio_settings.export_status_message = Some(format!("❌ Export compilation failed: {e}"));
                    }
                }
            }
        });

        if let Some(msg) = &state.skin_studio_settings.export_status_message {
            ui.add_space(4.0);
            ui.colored_label(Color32::from_rgb(100, 255, 140), msg);
        }
    });
}

// -------------------------------------------------------------------------------------------------
// Helper: Texture Caching & Swatch Thumbnail
// -------------------------------------------------------------------------------------------------

/// Retrieves a cached egui texture handle or allocates and uploads it if missing or updated.
pub fn get_or_load_skin_texture(
    ctx: &egui::Context,
    cache: &mut std::collections::HashMap<String, egui::TextureHandle>,
    prefix: &str,
    tex: &SkinTexture,
) -> egui::TextureHandle {
    // Evict old textures if cache gets too large to prevent unbounded VRAM growth
    if cache.len() > 64 {
        cache.clear();
    }

    let sample_step = (tex.rgba.len() / 64).max(1);
    let sample_sig = tex
        .rgba
        .iter()
        .step_by(sample_step)
        .fold(0u32, |acc, &b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let key = format!("{prefix}_{}_{}x{}_{:08x}", tex.name, tex.width, tex.height, sample_sig);

    if let Some(handle) = cache.get(&key) {
        return handle.clone();
    }

    let img = egui::ColorImage::from_rgba_unmultiplied(
        [tex.width as usize, tex.height as usize],
        &tex.rgba[..(tex.width * tex.height * 4) as usize],
    );
    let handle = ctx.load_texture(key.clone(), img, egui::TextureOptions::NEAREST);
    cache.insert(key, handle.clone());
    handle
}

fn render_texture_swatch(
    ui: &mut Ui,
    cache: &mut std::collections::HashMap<String, egui::TextureHandle>,
    tex: Option<&SkinTexture>,
    size: Vec2,
    show_wireframe: bool,
) {
    let (rect, _response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    if let Some(t) = tex {
        if t.width > 0 && t.height > 0 && t.rgba.len() >= (t.width * t.height * 4) as usize {
            // Draw subtle transparency checkerboard pattern
            let cell = 8.0_f32;
            let cols = (rect.width() / cell).ceil() as usize;
            let rows = (rect.height() / cell).ceil() as usize;
            for r in 0..rows {
                for c in 0..cols {
                    let col = if (r + c) % 2 == 0 {
                        Color32::from_rgb(26, 28, 34)
                    } else {
                        Color32::from_rgb(38, 42, 50)
                    };
                    let min = rect.min + egui::vec2(c as f32 * cell, r as f32 * cell);
                    let max = (min + egui::vec2(cell, cell)).min(rect.max);
                    painter.rect_filled(egui::Rect::from_min_max(min, max), 0.0, col);
                }
            }

            let tex_handle = get_or_load_skin_texture(
                ui.ctx(),
                cache,
                "swatch",
                t,
            );
            painter.image(
                tex_handle.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else {
            painter.rect_filled(rect, 0.0, Color32::from_rgb(35, 38, 45));
        }

        // Draw crisp border
        painter.rect_stroke(
            rect,
            2.0,
            Stroke::new(1.0_f32, Color32::from_rgb(90, 100, 120)),
            StrokeKind::Inside,
        );

        // UV Wireframe Seams Overlay if enabled
        if show_wireframe {
            let wf_col = Color32::from_rgba_unmultiplied(60, 240, 180, 190);
            let stroke = Stroke::new(1.0_f32, wf_col);
            // Center symmetry axis
            painter.line_segment([rect.center_top(), rect.center_bottom()], stroke);
            // Pectoral / Mid seam line
            let y_mid = rect.top() + rect.height() * 0.38;
            painter.line_segment(
                [egui::pos2(rect.left(), y_mid), egui::pos2(rect.right(), y_mid)],
                stroke,
            );
            // Belt / Lower seam line
            let y_belt = rect.top() + rect.height() * 0.82;
            painter.line_segment(
                [egui::pos2(rect.left(), y_belt), egui::pos2(rect.right(), y_belt)],
                stroke,
            );
            // Diagonal structural darts
            painter.line_segment(
                [egui::pos2(rect.left() + rect.width() * 0.2, rect.top() + rect.height() * 0.1),
                 egui::pos2(rect.left() + rect.width() * 0.35, y_mid)],
                stroke,
            );
            painter.line_segment(
                [egui::pos2(rect.right() - rect.width() * 0.2, rect.top() + rect.height() * 0.1),
                 egui::pos2(rect.right() - rect.width() * 0.35, y_mid)],
                stroke,
            );
        }
    } else {
        painter.rect_filled(rect, 3.0, Color32::from_rgb(26, 28, 34));
        painter.rect_stroke(rect, 3.0, Stroke::new(1.0_f32, Color32::from_rgb(90, 45, 45)), StrokeKind::Inside);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "None",
            egui::FontId::proportional(11.0),
            Color32::from_rgb(130, 130, 140),
        );
    }
}
