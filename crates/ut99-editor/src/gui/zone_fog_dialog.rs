use egui::{Color32, RichText, Ui};
use crate::editor_state::{EditorState, FogPresetId, FogShellTargetMode};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FogTab {
    Profiler,
    Presets,
    VisibilityAudit,
}

pub fn render_zone_fog_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_zone_fog_studio_dialog {
        return;
    }

    let mut open = state.show_zone_fog_studio_dialog;

    let tab_id = egui::Id::new("zone_fog_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<FogTab>(tab_id)
            .unwrap_or(FogTab::Profiler)
    });

    crate::gui::modal::standard_dialog_window(
        ctx,
        "Real-time Zone Fog & Volumetric Attenuation Studio",
        &mut open,
        [780.0, 560.0],
        [620.0, 420.0],
    )
    .show(ctx, |ui| {
            // Header bar with status
            ui.horizontal(|ui| {
                let status_text = if state.distance_fog_enabled {
                    RichText::new("● FOG ENABLED").color(Color32::from_rgb(80, 220, 140)).strong()
                } else {
                    RichText::new("○ FOG DISABLED").color(Color32::GRAY).strong()
                };
                ui.label(status_text);

                if ui.button(if state.distance_fog_enabled { "Disable Viewport Fog" } else { "Enable Viewport Fog" }).clicked() {
                    state.distance_fog_enabled = !state.distance_fog_enabled;
                    state.log(format!("Viewport distance fog {}", if state.distance_fog_enabled { "ENABLED" } else { "DISABLED" }));
                }

                ui.separator();
                ui.label(format!("Cutoff: {:.0} uu", state.fog_end));
                ui.separator();
                ui.label(format!("Start: {:.0} uu", state.fog_start));
                ui.separator();
                ui.label(format!("Density: {:.1}x", state.fog_density));
            });

            ui.add_space(4.0);

            // Tab Navigation
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == FogTab::Profiler, "🗺 Zone Fog Inspector").clicked() {
                    current_tab = FogTab::Profiler;
                }
                if ui.selectable_label(current_tab == FogTab::Presets, "🎨 Environmental Presets").clicked() {
                    current_tab = FogTab::Presets;
                }
                if ui.selectable_label(current_tab == FogTab::VisibilityAudit, "🎯 Line-of-Sight & Weapon Audit").clicked() {
                    current_tab = FogTab::VisibilityAudit;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            match current_tab {
                FogTab::Profiler => render_profiler_tab(ui, state),
                FogTab::Presets => render_presets_tab(ui, state),
                FogTab::VisibilityAudit => render_visibility_audit_tab(ui, state),
            }
        });

    state.show_zone_fog_studio_dialog = open;
}

fn render_profiler_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Zone Actors & Environmental Volumetric Profiles").strong().size(14.0));
    ui.label(RichText::new("Inspect and synchronize fog parameters for all ZoneInfo, WaterZone, LavaZone, and SlimeZone actors.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let profiles = state.collect_level_zone_fog_profiles();

    ui.horizontal(|ui| {
        ui.label(format!("Detected Zones: {}", profiles.len()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📍 Place New Fog Zone at Pivot").clicked() {
                let new_idx = state.place_fog_zone_at_pivot(state.zone_fog_settings.active_preset.unwrap_or(FogPresetId::MistyMorningSwamp));
                state.zone_fog_settings.selected_zone_index = Some(new_idx);
            }
        });
    });

    ui.add_space(4.0);

    egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
        if profiles.is_empty() {
            ui.label(RichText::new("No ZoneInfo or fluid zone actors found in current level.").italics().color(Color32::GRAY));
            ui.label("Click 'Place New Fog Zone at Pivot' above to insert a ZoneInfo with calibrated fog.");
        } else {
            egui::Grid::new("zone_profiles_table")
                .striped(true)
                .min_col_width(80.0)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Class").strong());
                    ui.label(RichText::new("Name").strong());
                    ui.label(RichText::new("Fog").strong());
                    ui.label(RichText::new("Color").strong());
                    ui.label(RichText::new("Cutoff (uu)").strong());
                    ui.label(RichText::new("Density").strong());
                    ui.label(RichText::new("Actions").strong());
                    ui.end_row();

                    for p in &profiles {
                        let is_selected = state.zone_fog_settings.selected_zone_index == Some(p.actor_index)
                            || state.selected_actor_index == Some(p.actor_index);

                        let class_color = if p.class_name.contains("Water") {
                            Color32::from_rgb(80, 160, 240)
                        } else if p.class_name.contains("Slime") {
                            Color32::from_rgb(100, 220, 60)
                        } else if p.class_name.contains("Lava") {
                            Color32::from_rgb(240, 120, 40)
                        } else {
                            Color32::from_rgb(180, 140, 240)
                        };

                        ui.colored_label(class_color, &p.class_name);
                        
                        let label_text = if is_selected {
                            RichText::new(&p.zone_name).strong().color(Color32::GOLD)
                        } else {
                            RichText::new(&p.zone_name)
                        };
                        ui.label(label_text);

                        if p.b_fog_zone {
                            ui.colored_label(Color32::GREEN, "ACTIVE");
                        } else {
                            ui.colored_label(Color32::DARK_GRAY, "OFF");
                        }

                        // Color swatch
                        let r = (p.fog_color[0] * 255.0) as u8;
                        let g = (p.fog_color[1] * 255.0) as u8;
                        let b = (p.fog_color[2] * 255.0) as u8;
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(28.0, 14.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 2.0, Color32::from_rgb(r, g, b));

                        ui.label(format!("{:.0}", p.fog_distance));
                        ui.label(format!("{:.1}x", p.fog_density));

                        ui.horizontal(|ui| {
                            if ui.small_button("Select").clicked() {
                                state.selected_actor_index = Some(p.actor_index);
                                state.zone_fog_settings.selected_zone_index = Some(p.actor_index);
                            }
                            if ui.small_button("🔄 Sync Viewport").clicked() {
                                state.distance_fog_enabled = p.b_fog_zone;
                                state.fog_start = p.fog_start;
                                state.fog_end = p.fog_distance;
                                state.fog_density = p.fog_density;
                                state.fog_color = p.fog_color;
                                state.log(format!("Synchronized viewport fog from Zone '{}'", p.zone_name));
                            }
                        });
                        ui.end_row();
                    }
                });
        }
    });

    ui.add_space(8.0);

    // Selected zone inspector & live tuner
    let sel_idx_opt = state.zone_fog_settings.selected_zone_index.or(state.selected_actor_index);
    if let Some(sel_idx) = sel_idx_opt {
        let is_zone = state.loaded_unr.as_ref()
            .and_then(|l| l.actors.get(sel_idx))
            .map(|a| a.class_name.contains("Zone"))
            .unwrap_or(false);

        if is_zone {
            let act_name = state.loaded_unr.as_ref()
                .and_then(|l| l.actors.get(sel_idx))
                .map(|a| a.name.clone())
                .unwrap_or_else(|| format!("Actor #{sel_idx}"));
            ui.group(|ui| {
                ui.label(RichText::new(format!("Selected Zone: {act_name}")).strong());
                
                ui.horizontal(|ui| {
                    ui.label("Fog Color:");
                    let mut col3 = [state.fog_color[0], state.fog_color[1], state.fog_color[2]];
                    if ui.color_edit_button_rgb(&mut col3).changed() {
                        state.fog_color = [col3[0], col3[1], col3[2], 1.0];
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Cutoff Distance (uu):");
                    ui.add(egui::Slider::new(&mut state.fog_end, 100.0..=15000.0).logarithmic(true));
                });

                ui.horizontal(|ui| {
                    ui.label("Start Distance (uu): ");
                    ui.add(egui::Slider::new(&mut state.fog_start, 0.0..=5000.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Density Factor:       ");
                    ui.add(egui::Slider::new(&mut state.fog_density, 0.1..=4.0));
                });

                ui.add_space(4.0);
                if ui.button("💾 Apply Current Settings to Selected Zone (Undoable)").clicked() {
                    state.update_zone_fog_properties(
                        sel_idx,
                        state.distance_fog_enabled,
                        state.fog_color,
                        state.fog_end,
                        state.fog_start,
                        state.fog_density,
                        format!("Updated fog properties on {act_name}"),
                    );
                }
            });
        }
    }
}

fn render_presets_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Canonical Environmental Atmospheric Presets").strong().size(14.0));
    ui.label(RichText::new("One-click tuned profiles for iconic Unreal Engine 1 and UT99 environments.").size(11.0).color(Color32::GRAY));
    ui.add_space(8.0);

    let sel_zone_idx = state.zone_fog_settings.selected_zone_index.or(state.selected_actor_index);

    egui::ScrollArea::vertical().show(ui, |ui| {
        for &preset in FogPresetId::all() {
            let col = preset.color();
            let r = (col[0] * 255.0) as u8;
            let g = (col[1] * 255.0) as u8;
            let b = (col[2] * 255.0) as u8;

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, Color32::from_rgb(r, g, b));
                    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, Color32::from_white_alpha(100)), egui::StrokeKind::Inside);

                    ui.vertical(|ui| {
                        ui.label(RichText::new(preset.label()).strong().size(13.0));
                        ui.label(RichText::new(preset.description()).size(11.0).color(Color32::GRAY));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Apply to Selected Zone").clicked() {
                            if let Some(idx) = sel_zone_idx {
                                state.apply_fog_preset_to_zone(idx, preset);
                            } else {
                                // Apply to viewport directly
                                state.distance_fog_enabled = true;
                                state.fog_start = preset.start();
                                state.fog_end = preset.distance();
                                state.fog_density = preset.density();
                                state.fog_color = preset.color();
                                state.log(format!("Applied preset '{}' to viewport (no zone actor selected)", preset.label()));
                            }
                        }

                        if ui.button("👁 Preview").clicked() {
                            state.distance_fog_enabled = true;
                            state.fog_start = preset.start();
                            state.fog_end = preset.distance();
                            state.fog_density = preset.density();
                            state.fog_color = preset.color();
                            state.zone_fog_settings.active_preset = Some(preset);
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Cutoff: {:.0} uu", preset.distance())).size(11.0).color(Color32::LIGHT_GRAY));
                    ui.separator();
                    ui.label(RichText::new(format!("Start: {:.0} uu", preset.start())).size(11.0).color(Color32::LIGHT_GRAY));
                    ui.separator();
                    ui.label(RichText::new(format!("Density: {:.1}x", preset.density())).size(11.0).color(Color32::LIGHT_GRAY));
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn render_visibility_audit_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Line-of-Sight & Weapon Engagement Profiler").strong().size(14.0));
    ui.label(RichText::new("Analytical measurement of visual clarity and weapon effectiveness across combat sightlines.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    // 3D Distance Shells Controls
    ui.group(|ui| {
        ui.label(RichText::new("3D Viewport Volumetric Shells").strong());
        ui.checkbox(&mut state.zone_fog_settings.show_distance_shells, "Render Concentric Distance Shells (25%, 50%, 75%, 100%) in 3D Viewport");
        
        ui.horizontal(|ui| {
            ui.label("Shell Center:");
            ui.radio_value(&mut state.zone_fog_settings.shell_target_mode, FogShellTargetMode::ActiveCamera, "Active Camera Position");
            ui.radio_value(&mut state.zone_fog_settings.shell_target_mode, FogShellTargetMode::SelectedZone, "Selected Zone Anchor Location");
        });
    });

    ui.add_space(8.0);

    // Weapon visibility matrix
    let ratings = EditorState::compute_weapon_visibility_ratings(state.fog_start, state.fog_end, state.fog_density);

    ui.group(|ui| {
        ui.label(RichText::new("Weapon Engagement Visibility Matrix").strong());
        ui.add_space(4.0);

        for (weapon_name, distance, visibility) in ratings {
            let vis_pct = (visibility * 100.0).round();
            let (bar_color, status_text) = if visibility >= 0.75 {
                (Color32::from_rgb(60, 220, 100), "Crystal Clear (Sniper Lethal)")
            } else if visibility >= 0.40 {
                (Color32::from_rgb(220, 190, 40), "Moderate Haze (Silhouette Visible)")
            } else if visibility >= 0.15 {
                (Color32::from_rgb(240, 120, 40), "Heavy Obscurity (Blind Fire)")
            } else {
                (Color32::from_rgb(140, 140, 150), "Complete Blackout (Total Concealment)")
            };

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("{weapon_name} ({:.0} uu):", distance)).strong().color(Color32::WHITE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(bar_color, format!("{vis_pct}% - {status_text}"));
                });
            });

            // Progress bar
            let progress = visibility.clamp(0.0, 1.0);
            let bar = egui::ProgressBar::new(progress)
                .fill(bar_color)
                .show_percentage();
            ui.add(bar);
            ui.add_space(4.0);
        }
    });

    ui.add_space(8.0);
    ui.label(RichText::new("💡 Pro Tip: Competitive tournament maps maintain >= 40% visibility at 1536 uu (Shock Rifle combo range) to ensure balanced combat without frustrating sightline blackout.").size(11.0).italics().color(Color32::GRAY));
}
