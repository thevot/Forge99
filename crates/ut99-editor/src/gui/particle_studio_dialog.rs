use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use ut99_bsp::ParticlePresetTemplate;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParticleTab {
    Parameters,
    Presets,
    EmittersList,
}

/// Dynamic Particle FX & Volumetric Emitter Studio
pub fn render_particle_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_particle_studio_dialog {
        return;
    }

    if state.particle_settings.is_simulating {
        let dt = ctx.input(|i| i.stable_dt);
        state.update_particles(dt);
        ctx.request_repaint();
    }

    let mut is_open = state.show_particle_studio_dialog;
    let tab_id = egui::Id::new("particle_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<ParticleTab>(tab_id)
            .unwrap_or(ParticleTab::Parameters)
    });

    Window::new("Dynamic Particle FX & Volumetric Emitter Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(540.0)
        .min_width(620.0)
        .min_height(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header bar
            ui.horizontal(|ui| {
                ui.heading("✨ Particle FX Studio");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📍 Place New Emitter at Pivot").clicked() {
                        state.place_particle_emitter_at_pivot(None);
                    }
                    let sel_idx = state.particle_settings.selected_emitter_index;
                    if ui.button("🧹 Clear Particles").clicked() {
                        state.clear_emitter_particles(sel_idx);
                    }
                });
            });

            ui.separator();

            // Transport & Simulation Bar
            let total_particles: usize = state.particle_emitters.iter().map(|e| e.particles.len()).sum();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if state.particle_settings.is_simulating {
                        if ui.button("⏸ Pause Sim").clicked() {
                            state.particle_settings.is_simulating = false;
                        }
                    } else {
                        if ui.button("▶ Resume Sim").clicked() {
                            state.particle_settings.is_simulating = true;
                        }
                    }

                    ui.separator();
                    ui.label("Sim Speed:");
                    ui.selectable_value(&mut state.particle_settings.sim_speed, 0.25, "0.25x");
                    ui.selectable_value(&mut state.particle_settings.sim_speed, 0.5, "0.5x");
                    ui.selectable_value(&mut state.particle_settings.sim_speed, 1.0, "1.0x");
                    ui.selectable_value(&mut state.particle_settings.sim_speed, 2.0, "2.0x");

                    ui.separator();
                    ui.checkbox(&mut state.particle_settings.show_emitter_bounds, "Show Bounds");
                    ui.checkbox(&mut state.particle_settings.show_collision_plane, "Show Floor Plane");
                    ui.checkbox(&mut state.particle_settings.auto_detect_floor, "Auto-Floor Raycast");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(
                            Color32::from_rgb(100, 220, 255),
                            format!("Active Particles: {} (in {} emitters)", total_particles, state.particle_emitters.len()),
                        );
                    });
                });
            });

            ui.add_space(4.0);

            // Tab Navigation
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == ParticleTab::Parameters, "⚙ Emitter Parameters & Physics").clicked() {
                    current_tab = ParticleTab::Parameters;
                }
                if ui.selectable_label(current_tab == ParticleTab::Presets, "🎨 Preset FX Templates").clicked() {
                    current_tab = ParticleTab::Presets;
                }
                if ui.selectable_label(current_tab == ParticleTab::EmittersList, "📋 Active Emitters Directory").clicked() {
                    current_tab = ParticleTab::EmittersList;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            let emit_count = state.particle_emitters.len();
            if emit_count == 0 {
                ui.vertical_centered(|ui| {
                    ui.add_space(30.0);
                    ui.label(RichText::new("No Particle Emitters placed in level.").color(Color32::GRAY));
                    ui.add_space(10.0);
                    if ui.button("📍 Place Default Torch Fire Emitter").clicked() {
                        state.place_particle_emitter_at_pivot(Some(ParticlePresetTemplate::TorchFireFlames));
                    }
                });
                return;
            }

            match current_tab {
                ParticleTab::Parameters => render_parameters_tab(ui, state),
                ParticleTab::Presets => render_presets_tab(ui, state),
                ParticleTab::EmittersList => render_emitters_list_tab(ui, state),
            }
        });

    state.show_particle_studio_dialog = is_open;
}

fn render_parameters_tab(ui: &mut Ui, state: &mut EditorState) {
    let sel_idx = state.particle_settings.selected_emitter_index.min(state.particle_emitters.len() - 1);
    state.particle_settings.selected_emitter_index = sel_idx;

    let names: Vec<String> = state.particle_emitters.iter().enumerate().map(|(i, e)| {
        format!("#{i}: {} ({:?})", e.name, e.emitter_type)
    }).collect();

    // Emitter Selector Combo
    ui.horizontal(|ui| {
        ui.label(RichText::new("Selected Emitter:").strong());
        let current_label = names.get(sel_idx).cloned().unwrap_or_default();
        egui::ComboBox::from_id_salt("emitter_select_combo")
            .selected_text(&current_label)
            .show_ui(ui, |ui| {
                for (i, name) in names.iter().enumerate() {
                    if ui.selectable_label(i == sel_idx, name).clicked() {
                        state.particle_settings.selected_emitter_index = i;
                    }
                }
            });

        if ui.button("🎯 Raycast Floor Down").clicked() {
            if let Some(floor_z) = state.detect_nearest_floor_for_emitter(sel_idx) {
                state.particle_emitters[sel_idx].floor_z = Some(floor_z);
                state.log(format!("Raycasted floor plane to Z = {:.1}", floor_z));
            }
        }
    });

    ui.separator();

    let emitter = &mut state.particle_emitters[sel_idx];

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("emitter_params_grid")
            .striped(true)
            .min_col_width(140.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                // Name & Type
                ui.label(RichText::new("Emitter Name").strong());
                ui.text_edit_singleline(&mut emitter.name);
                ui.end_row();

                ui.label(RichText::new("Type").strong());
                ui.label(format!("{:?}", emitter.emitter_type));
                ui.end_row();

                // Emission rate & cone
                ui.label(RichText::new("Emission Rate").strong());
                ui.add(egui::Slider::new(&mut emitter.emission_rate, 1.0..=120.0).suffix(" particles/s"));
                ui.end_row();

                ui.label(RichText::new("Spread Cone").strong());
                ui.add(egui::Slider::new(&mut emitter.spread_cone_degrees, 0.0..=90.0).suffix("°"));
                ui.end_row();

                ui.label(RichText::new("Speed Range").strong());
                ui.horizontal(|ui| {
                    ui.label("Min:");
                    ui.add(egui::DragValue::new(&mut emitter.initial_speed_min).speed(1.0));
                    ui.label("Max:");
                    ui.add(egui::DragValue::new(&mut emitter.initial_speed_max).speed(1.0));
                });
                ui.end_row();

                ui.label(RichText::new("Air Drag Damping").strong());
                ui.add(egui::Slider::new(&mut emitter.drag, 0.0..=0.4).step_by(0.005));
                ui.end_row();

                // Gravity & Wind
                ui.label(RichText::new("Gravity Vector (Z)").strong());
                ui.add(egui::DragValue::new(&mut emitter.gravity.z).speed(5.0).suffix(" uu/s²"));
                ui.end_row();

                ui.label(RichText::new("Wind Drift (X, Y)").strong());
                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut emitter.wind_velocity.x).speed(1.0));
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut emitter.wind_velocity.y).speed(1.0));
                });
                ui.end_row();

                // Sizes
                ui.label(RichText::new("Size Curve").strong());
                ui.horizontal(|ui| {
                    ui.label("Start:");
                    ui.add(egui::DragValue::new(&mut emitter.size_start).speed(0.5).range(0.5..=64.0));
                    ui.label("End:");
                    ui.add(egui::DragValue::new(&mut emitter.size_end).speed(0.5).range(0.5..=64.0));
                });
                ui.end_row();

                // Colors
                ui.label(RichText::new("Start Color (RGBA)").strong());
                ui.horizontal(|ui| {
                    let mut c = Color32::from_rgba_unmultiplied(
                        (emitter.color_start[0] * 255.0) as u8,
                        (emitter.color_start[1] * 255.0) as u8,
                        (emitter.color_start[2] * 255.0) as u8,
                        (emitter.color_start[3] * 255.0) as u8,
                    );
                    if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                        let rgba = c.to_array();
                        emitter.color_start = [rgba[0] as f32 / 255.0, rgba[1] as f32 / 255.0, rgba[2] as f32 / 255.0, emitter.color_start[3]];
                    }
                    ui.label(format!("Alpha: {:.2}", emitter.color_start[3]));
                    ui.add(egui::Slider::new(&mut emitter.color_start[3], 0.0..=1.0));
                });
                ui.end_row();

                ui.label(RichText::new("End Color (RGBA)").strong());
                ui.horizontal(|ui| {
                    let mut c = Color32::from_rgba_unmultiplied(
                        (emitter.color_end[0] * 255.0) as u8,
                        (emitter.color_end[1] * 255.0) as u8,
                        (emitter.color_end[2] * 255.0) as u8,
                        (emitter.color_end[3] * 255.0) as u8,
                    );
                    if egui::color_picker::color_edit_button_srgba(ui, &mut c, egui::color_picker::Alpha::Opaque).changed() {
                        let rgba = c.to_array();
                        emitter.color_end = [rgba[0] as f32 / 255.0, rgba[1] as f32 / 255.0, rgba[2] as f32 / 255.0, emitter.color_end[3]];
                    }
                    ui.label(format!("Alpha: {:.2}", emitter.color_end[3]));
                    ui.add(egui::Slider::new(&mut emitter.color_end[3], 0.0..=1.0));
                });
                ui.end_row();

                // Floor Collision & Bounce Damping
                ui.label(RichText::new("Floor Collision Z").strong());
                ui.horizontal(|ui| {
                    let mut has_floor = emitter.floor_z.is_some();
                    if ui.checkbox(&mut has_floor, "Enable").changed() {
                        emitter.floor_z = if has_floor { Some(emitter.location.z - 100.0) } else { None };
                    }
                    if let Some(ref mut fz) = emitter.floor_z {
                        ui.add(egui::DragValue::new(fz).speed(2.0).suffix(" uu"));
                    }
                });
                ui.end_row();

                ui.label(RichText::new("Bounce Damping").strong());
                ui.add(egui::Slider::new(&mut emitter.bounce_damping, 0.0..=0.9).suffix(" (elasticity)"));
                ui.end_row();
            });
    });
}

fn render_presets_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Canonical Unreal Tournament 99 Particle Presets").strong().size(13.0));
    ui.label(RichText::new("Apply iconic FX configurations with pre-calibrated physics, buoyancy, and collision parameters.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let sel_idx = state.particle_settings.selected_emitter_index.min(state.particle_emitters.len().saturating_sub(1));

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        for &preset in ParticlePresetTemplate::all() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(preset.label()).strong().size(13.0));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Apply to Selected").clicked() {
                            state.apply_particle_preset(sel_idx, preset);
                        }
                        if ui.button("Spawn at Pivot").clicked() {
                            state.place_particle_emitter_at_pivot(Some(preset));
                        }
                    });
                });
                ui.add_space(2.0);
                ui.label(RichText::new(preset.description()).size(11.0).color(Color32::from_rgb(180, 200, 220)));
            });
            ui.add_space(4.0);
        }
    });
}

fn render_emitters_list_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Level Particle Emitters Directory").strong().size(13.0));
    ui.label(RichText::new("Overview of all active volumetric emitters placed across the map.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let sel_idx = state.particle_settings.selected_emitter_index;

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("emitters_dir_grid")
            .striped(true)
            .min_col_width(70.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("#").strong());
                ui.label(RichText::new("Name").strong());
                ui.label(RichText::new("Type").strong());
                ui.label(RichText::new("Location (X, Y, Z)").strong());
                ui.label(RichText::new("Particles").strong());
                ui.label(RichText::new("Floor Z").strong());
                ui.label(RichText::new("Action").strong());
                ui.end_row();

                let mut to_select = None;
                for (idx, emitter) in state.particle_emitters.iter().enumerate() {
                    let is_sel = idx == sel_idx;
                    let name_text = if is_sel {
                        RichText::new(&emitter.name).strong().color(Color32::from_rgb(255, 210, 40))
                    } else {
                        RichText::new(&emitter.name)
                    };

                    ui.label(format!("#{idx}"));
                    ui.label(name_text);
                    ui.label(format!("{:?}", emitter.emitter_type));
                    ui.label(format!("({:.0}, {:.0}, {:.0})", emitter.location.x, emitter.location.y, emitter.location.z));
                    ui.colored_label(Color32::from_rgb(100, 220, 255), format!("{}", emitter.particles.len()));
                    ui.label(emitter.floor_z.map_or("-".to_string(), |z| format!("{:.0}", z)));

                    if ui.button(if is_sel { "Selected" } else { "Select" }).clicked() {
                        to_select = Some(idx);
                    }
                    ui.end_row();
                }

                if let Some(s) = to_select {
                    state.particle_settings.selected_emitter_index = s;
                }
            });
    });
}
