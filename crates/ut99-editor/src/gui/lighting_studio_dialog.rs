use egui::{Color32, RichText, Ui};
use ut99_bsp::LightingBakeQuality;
use crate::editor_state::{EditorState, TournamentLightPreset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LightingTab {
    Baking,
    Presets,
    Luxmeter,
}

pub fn render_lighting_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_lighting_studio_dialog {
        return;
    }

    let mut open = state.show_lighting_studio_dialog;

    let tab_id = egui::Id::new("lighting_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<LightingTab>(tab_id)
            .unwrap_or(LightingTab::Baking)
    });

    crate::gui::modal::standard_dialog_window(
        ctx,
        "Radiosity & Direct Light Baking Studio",
        &mut open,
        [780.0, 560.0],
        [580.0, 380.0],
    )
    .show(ctx, |ui| {
            // Header stats
            if let Some(ref stats) = state.last_lighting_stats {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Last Bake:").strong());
                    ui.colored_label(Color32::from_rgb(100, 220, 140), format!("{} ms", stats.elapsed_ms));
                    ui.separator();
                    ui.label(format!("{} Polygons", stats.total_polygons));
                    ui.separator();
                    ui.label(format!("{} Luxels", stats.total_luxels));
                    ui.separator();
                    ui.label(format!("{} Shadow Rays", stats.shadow_rays_cast));
                });
            } else {
                ui.colored_label(Color32::LIGHT_GRAY, "Lighting not yet compiled for current session.");
            }

            ui.add_space(4.0);

            // Tabs
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == LightingTab::Baking, "⚡ Baking & Profiles").clicked() {
                    current_tab = LightingTab::Baking;
                }
                if ui.selectable_label(current_tab == LightingTab::Presets, "◫ Tournament Palette").clicked() {
                    current_tab = LightingTab::Presets;
                }
                if ui.selectable_label(current_tab == LightingTab::Luxmeter, "▥ Luxmeter & Audit").clicked() {
                    current_tab = LightingTab::Luxmeter;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            match current_tab {
                LightingTab::Baking => render_baking_tab(ui, state),
                LightingTab::Presets => render_presets_tab(ui, state),
                LightingTab::Luxmeter => render_luxmeter_tab(ui, state),
            }
        });

    state.show_lighting_studio_dialog = open;
}

fn render_baking_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Multithreaded Rayon Light Compiler & Radiosity Engine").strong().size(14.0));
    ui.add_space(4.0);

    ui.group(|ui| {
        ui.label(RichText::new("Quality Profile").strong());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.lighting_config.quality, LightingBakeQuality::DraftFast, "⚡ Draft Fast");
            ui.selectable_value(&mut state.lighting_config.quality, LightingBakeQuality::StandardTournament, "◈ Standard Tournament");
            ui.selectable_value(&mut state.lighting_config.quality, LightingBakeQuality::ProductionRadiosity, "☼ Production Radiosity");
        });
        ui.label(RichText::new(state.lighting_config.quality.label()).size(11.0).color(Color32::GRAY));
    });

    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Compiler Parameters").strong());
        ui.checkbox(&mut state.lighting_config.enable_shadows, "Enable Raytraced Shadow Occlusion");
        ui.checkbox(&mut state.lighting_config.enable_overbright, "Enable Overbright Lighting (>1.0 HDR)");

        ui.horizontal(|ui| {
            ui.label("Lightmap Grid Resolution:");
            ui.add(egui::DragValue::new(&mut state.lighting_config.lightmap_resolution).speed(1.0).range(8.0..=128.0).suffix(" uu/luxel"));
        });

        ui.horizontal(|ui| {
            ui.label("Scene Ambient Color (RGB):");
            ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.x).speed(0.01).range(0.0..=1.0).prefix("R:"));
            ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.y).speed(0.01).range(0.0..=1.0).prefix("G:"));
            ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.z).speed(0.01).range(0.0..=1.0).prefix("B:"));
        });

        if state.lighting_config.quality == LightingBakeQuality::ProductionRadiosity {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Radiosity Diffuse Bounces:");
                ui.add(egui::DragValue::new(&mut state.lighting_config.radiosity_bounces).range(1..=3));
                ui.label("Diffuse Albedo:");
                ui.add(egui::DragValue::new(&mut state.lighting_config.radiosity_albedo).speed(0.05).range(0.1..=0.8));
            });
        }
    });

    ui.add_space(10.0);

    ui.horizontal(|ui| {
        if ui.button(RichText::new("⚡ Bake Full Scene Lighting").strong().size(14.0)).clicked() {
            state.build_lighting();
            if let Some(ref stats) = state.last_lighting_stats {
                state.toast_success(format!("Scene lighting bake complete: {} luxels in {} ms.", stats.total_luxels, stats.elapsed_ms));
            } else {
                state.toast_info("Full scene lighting bake completed.");
            }
        }

        if ui.button(RichText::new("🎯 Bake Selected Lights Only").strong().size(14.0))
            .on_hover_text("Fast differential bake: recomputes lightmaps and vertex colors ONLY for brushes within the influence of selected lights")
            .clicked()
        {
            match state.bake_selected_lights_only() {
                Ok(stats) => {
                    state.toast_success(format!("Differential bake complete: {} luxels in {} ms.", stats.total_luxels, stats.elapsed_ms));
                }
                Err(e) => {
                    state.toast_warning(format!("Differential bake: {}", e));
                }
            }
        }

        if state.history.can_undo() && ui.button("↺ Undo Bake").clicked() {
            state.undo();
            state.toast_info("Reverted lighting bake.");
        }
    });
}

fn render_presets_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Tournament Visual Identity & Light Presets").strong().size(14.0));
    ui.label("Apply iconic, competition-calibrated lighting setups to selected lights or all lights in the map.");

    ui.add_space(8.0);

    let presets = [
        TournamentLightPreset::FacingWorldsMidnight,
        TournamentLightPreset::Deck16ToxicSlime,
        TournamentLightPreset::PhobosCyberRed,
        TournamentLightPreset::LavaCoreAmber,
        TournamentLightPreset::MorpheusHighNoon,
        TournamentLightPreset::LiandriPureWhite,
    ];

    egui::ScrollArea::vertical().show(ui, |ui| {
        for preset in presets {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let rgb = preset.preview_rgb();
                    let preview_col = Color32::from_rgb(
                        (rgb[0] * 255.0).clamp(0.0, 255.0) as u8,
                        (rgb[1] * 255.0).clamp(0.0, 255.0) as u8,
                        (rgb[2] * 255.0).clamp(0.0, 255.0) as u8,
                    );

                    let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, preview_col);
                    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, Color32::WHITE), egui::StrokeKind::Inside);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{} {}", preset.icon(), preset.label())).strong());
                        let (h, s, b, r) = preset.values();
                        ui.label(format!("Hue: {} | Sat: {} | Brightness: {} | Radius: {} ({:.0} uu)", h, s, b, r, r as f32 * 25.0));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✓ Apply to Selected Lights").clicked() {
                            let count = state.apply_light_preset_to_selected(preset);
                            if count > 0 {
                                state.toast_success(format!("Applied {} preset to {} light(s).", preset.label(), count));
                            } else {
                                state.toast_warning("No light actors selected to apply preset.");
                            }
                        }
                    });
                });
            });
            ui.add_space(2.0);
        }
    });
}

fn render_luxmeter_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Luxmeter Photometric Heatmap & Coverage Audit").strong().size(14.0));
    ui.add_space(4.0);

    ui.horizontal(|ui| {
        ui.checkbox(&mut state.render_lux_heatmap, RichText::new("Render Luxmeter False-Color Overlay (Alt+6)").strong());
    });

    ui.add_space(8.0);

    let stats = state.compute_luxmeter_stats();

    ui.group(|ui| {
        ui.label(RichText::new("Overall Photometric Summary").strong());
        ui.horizontal(|ui| {
            ui.label(format!("Total Surfaces: {}", stats.total_surfaces));
            ui.separator();
            ui.label(format!("Lit Surfaces: {}", stats.lit_surfaces));
            ui.separator();
            ui.label(format!("Average Map Illuminance: {:.1} Lux", stats.average_lux));
        });
    });

    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Competitive Tournament Illuminance Distribution").strong());
        let total_samples = (stats.under_lit_count + stats.shadow_count + stats.tournament_ideal_count + stats.bright_count + stats.overbright_count).max(1) as f32;

        let pct_dark = (stats.under_lit_count as f32 / total_samples) * 100.0;
        let pct_shadow = (stats.shadow_count as f32 / total_samples) * 100.0;
        let pct_ideal = (stats.tournament_ideal_count as f32 / total_samples) * 100.0;
        let pct_bright = (stats.bright_count as f32 / total_samples) * 100.0;
        let pct_over = (stats.overbright_count as f32 / total_samples) * 100.0;

        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(60, 60, 220), format!("■ Dark (<15 Lux): {:.1}% ({})", pct_dark, stats.under_lit_count));
        });
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(0, 200, 220), format!("■ Tactical Shadow (15-50 Lux): {:.1}% ({})", pct_shadow, stats.shadow_count));
        });
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(40, 220, 60), format!("■ Tournament Optimal (50-100 Lux): {:.1}% ({})", pct_ideal, stats.tournament_ideal_count));
        });
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(220, 180, 40), format!("■ Bright (100-180 Lux): {:.1}% ({})", pct_bright, stats.bright_count));
        });
        ui.horizontal(|ui| {
            ui.colored_label(Color32::from_rgb(255, 60, 60), format!("■ Overbright / Glare (>180 Lux): {:.1}% ({})", pct_over, stats.overbright_count));
        });

        ui.separator();
        if pct_ideal >= 35.0 {
            ui.colored_label(Color32::from_rgb(40, 220, 60), RichText::new("✅ High Tournament Visibility Rating: Optimal balance for competitive duel/TDM matches.").strong());
        } else if pct_dark > 30.0 {
            ui.colored_label(Color32::from_rgb(255, 170, 50), RichText::new("⚠ Low Visibility Warning: Over 30% of surfaces in deep darkness; consider adding fill lights.").strong());
        } else {
            ui.colored_label(Color32::from_rgb(180, 210, 255), "Balanced illumination across standard engagement areas.");
        }
    });
}
