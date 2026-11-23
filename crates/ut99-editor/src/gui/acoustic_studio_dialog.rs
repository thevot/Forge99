//! 3D Acoustic & Ambient Sound Studio

use egui::{Color32, RichText, Ui};
use crate::editor_state::{AcousticListenerSource, EditorState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AcousticStudioTab {
    Emitters,
    Profiler,
    Settings,
}

pub fn render_acoustic_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_acoustics_dialog {
        return;
    }

    let mut open = state.show_acoustics_dialog;
    let mut current_tab = AcousticStudioTab::Emitters;

    let tab_id = egui::Id::new("acoustic_studio_tab_id");
    ctx.data_mut(|d| {
        if let Some(t) = d.get_temp::<u8>(tab_id) {
            current_tab = match t {
                1 => AcousticStudioTab::Profiler,
                2 => AcousticStudioTab::Settings,
                _ => AcousticStudioTab::Emitters,
            };
        }
    });

    egui::Window::new("3D Acoustic & Ambient Sound Studio")
        .open(&mut open)
        .default_width(680.0)
        .default_height(520.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Tabs
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == AcousticStudioTab::Emitters, "♪ Sound Emitters").clicked() {
                    current_tab = AcousticStudioTab::Emitters;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 0u8));
                }
                if ui.selectable_label(current_tab == AcousticStudioTab::Profiler, "◎ Attenuation & Occlusion").clicked() {
                    current_tab = AcousticStudioTab::Profiler;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 1u8));
                }
                if ui.selectable_label(current_tab == AcousticStudioTab::Settings, "⚙ Acoustics & Presets").clicked() {
                    current_tab = AcousticStudioTab::Settings;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 2u8));
                }
            });

            ui.separator();

            match current_tab {
                AcousticStudioTab::Emitters => render_emitters_tab(ui, state),
                AcousticStudioTab::Profiler => render_profiler_tab(ui, state),
                AcousticStudioTab::Settings => render_settings_tab(ui, state),
            }
        });

    state.show_acoustics_dialog = open;
}

fn render_emitters_tab(ui: &mut Ui, state: &mut EditorState) {
    let emitters = state.collect_sound_emitters();

    ui.vertical(|ui| {
        // Quick Action Bar
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("Emitters in Level: {}", emitters.len())).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔊 Place AmbientSound at Pivot").clicked() {
                    let snd = state.selected_sound_name.clone().unwrap_or_else(|| "AmbAncient.Loop.Hum".to_string());
                    let _ = state.place_ambient_sound_at_pivot(&snd, 32, 128);
                }
            });
        });

        ui.add_space(4.0);

        if emitters.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                ui.label(RichText::new("No sound emitters or AmbientSound actors found in map.").weak());
                ui.label("Click 'Place AmbientSound at Pivot' to create an ambient positional audio node.");
            });
            return;
        }

        // Emitters Table
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Grid::new("sound_emitters_grid")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Actor Tag").strong());
                    ui.label(RichText::new("Sound Name").strong());
                    ui.label(RichText::new("Radius").strong());
                    ui.label(RichText::new("Volume").strong());
                    ui.label(RichText::new("Pitch").strong());
                    ui.label(RichText::new("Actions").strong());
                    ui.end_row();

                    for e in &emitters {
                        let is_sel = state.selected_sound_emitter_tag.as_deref() == Some(&e.actor_tag);
                        let name_col = if is_sel { Color32::from_rgb(255, 220, 80) } else { Color32::from_rgb(100, 200, 255) };

                        ui.label(RichText::new(&e.actor_tag).strong().color(name_col));
                        ui.label(RichText::new(&e.sound_name).weak());
                        ui.label(format!("{:.0} uu ({:.1}m)", e.sound_radius_units, e.sound_radius_units / 64.0));
                        ui.label(format!("{:.0}%", e.sound_volume_pct * 100.0));
                        ui.label(format!("{:.2}x", e.sound_pitch_ratio));

                        ui.horizontal(|ui| {
                            if ui.small_button("🔍 Select").clicked() {
                                state.selected_sound_emitter_tag = Some(e.actor_tag.clone());
                            }
                            if ui.small_button("🎯 Focus").clicked() {
                                state.focus_camera_on_sound_emitter(&e.actor_tag);
                            }
                        });
                        ui.end_row();
                    }
                });
        });

        ui.separator();

        // Selected Emitter Editor
        if let Some(ref sel_tag) = state.selected_sound_emitter_tag.clone() {
            if let Some(e) = emitters.iter().find(|em| em.actor_tag == *sel_tag) {
                ui.group(|ui| {
                    ui.label(RichText::new(format!("Editing Emitter: '{}' ({})", e.actor_tag, e.class_name)).strong().color(Color32::from_rgb(100, 220, 255)));

                    let mut snd_name = e.sound_name.clone();
                    let mut rad_byte = e.sound_radius_byte;
                    let mut vol_byte = e.sound_volume_byte;
                    let mut pitch_byte = e.sound_pitch_byte;
                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Sound Asset:");
                        if ui.add(egui::TextEdit::singleline(&mut snd_name).desired_width(260.0)).changed() { changed = true; }

                        if let Some(ref picked) = state.selected_sound_name {
                            if ui.button(format!("Use '{}'", picked)).clicked() {
                                snd_name = picked.clone();
                                changed = true;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sound Radius:");
                        let rad_str = format!("{:.0} uu ({:.1} m)", rad_byte as f32 * 40.0, rad_byte as f32 * 40.0 / 64.0);
                        if ui.add(egui::Slider::new(&mut rad_byte, 0..=255).text(rad_str)).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sound Volume:");
                        let vol_str = format!("{:.0}%", vol_byte as f32 / 255.0 * 100.0);
                        if ui.add(egui::Slider::new(&mut vol_byte, 0..=255).text(vol_str)).changed() {
                            changed = true;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sound Pitch:");
                        let pitch_str = format!("{:.2}x", pitch_byte as f32 / 64.0);
                        if ui.add(egui::Slider::new(&mut pitch_byte, 0..=255).text(pitch_str)).changed() {
                            changed = true;
                        }
                    });

                    if changed {
                        let _ = state.update_sound_emitter_properties(&e.actor_tag, &snd_name, rad_byte, vol_byte, pitch_byte);
                    }
                });
            }
        }
    });
}

fn render_profiler_tab(ui: &mut Ui, state: &mut EditorState) {
    let listener_pos = match state.acoustic_settings.listener_source {
        AcousticListenerSource::CameraPosition => state.camera.position,
        AcousticListenerSource::SelectedActor => {
            state.loaded_unr.as_ref()
                .and_then(|u| u.actors.iter().find(|a| a.class_name.eq_ignore_ascii_case("PlayerStart")))
                .map(|a| a.location)
                .unwrap_or(state.camera.position)
        }
    };

    let occlusions = state.compute_all_acoustic_occlusions(listener_pos);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Listener Anchor:").strong());
            ui.radio_value(&mut state.acoustic_settings.listener_source, AcousticListenerSource::CameraPosition, "Editor Camera");
            ui.radio_value(&mut state.acoustic_settings.listener_source, AcousticListenerSource::SelectedActor, "PlayerStart / Spawn");
            ui.label(RichText::new(format!("({:.0}, {:.0}, {:.0})", listener_pos.x, listener_pos.y, listener_pos.z)).weak().small());
        });

        ui.add_space(6.0);

        if occlusions.is_empty() {
            ui.label(RichText::new("No sound emitters active in the level.").weak());
            return;
        }

        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            egui::Grid::new("acoustics_profiler_grid")
                .striped(true)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Emitter").strong());
                    ui.label(RichText::new("Distance").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.label(RichText::new("Direct dB").strong());
                    ui.label(RichText::new("Walls").strong());
                    ui.label(RichText::new("Occlusion").strong());
                    ui.label(RichText::new("Perceived Level").strong());
                    ui.label(RichText::new("LPF Cutoff").strong());
                    ui.end_row();

                    for occ in &occlusions {
                        ui.label(RichText::new(&occ.emitter_tag).strong().color(Color32::from_rgb(100, 220, 255)));
                        ui.label(format!("{:.0} uu", occ.distance));

                        if !occ.is_in_range {
                            ui.label(RichText::new("Out of Range").weak());
                        } else if occ.walls_occluding_count > 0 {
                            ui.label(RichText::new("Occluded").color(Color32::from_rgb(255, 120, 80)));
                        } else {
                            ui.label(RichText::new("Line of Sight").color(Color32::from_rgb(100, 255, 120)));
                        }

                        ui.label(format!("{:.1} dB", occ.direct_volume_db));
                        ui.label(format!("{} solid", occ.walls_occluding_count));
                        ui.label(format!("-{:.1} dB", occ.occlusion_attenuation_db));

                        // Visual Perceived dB Meter
                        let pct = if occ.final_effective_volume_db <= -96.0 {
                            0.0
                        } else {
                            ((occ.final_effective_volume_db + 96.0) / 96.0).clamp(0.0, 1.0)
                        };
                        let bar_color = if pct > 0.75 {
                            Color32::from_rgb(255, 200, 50)
                        } else if pct > 0.25 {
                            Color32::from_rgb(80, 220, 120)
                        } else {
                            Color32::from_rgb(80, 150, 200)
                        };
                        ui.add(egui::ProgressBar::new(pct).desired_width(70.0).fill(bar_color).text(format!("{:.1} dB", occ.final_effective_volume_db)));

                        ui.label(format!("{:.0} Hz", occ.low_pass_cutoff_hz));
                        ui.end_row();
                    }
                });
        });
    });
}

fn render_settings_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.vertical(|ui| {
        ui.label(RichText::new("3D Acoustic Profiler Viewport Settings").strong());

        ui.checkbox(&mut state.acoustic_settings.enabled, "Enable Real-time Acoustic Simulation");
        ui.checkbox(&mut state.acoustic_settings.show_radii_spheres, "📐 Draw 3D Acoustic Radii Spheres (Max Cutoff, 50% Boundary, 100% Core)");
        ui.checkbox(&mut state.acoustic_settings.show_listener_rays, "⚡ Draw Line-of-Sight Acoustic Rays (Green=Clear, Red=Occluded)");

        ui.add_space(10.0);
        ui.separator();
        ui.label(RichText::new("Standard Unreal Engine 1 Acoustic Presets").strong());
        ui.label(RichText::new("Presets configure canonical SoundRadius and SoundVolume scales for common environmental soundscapes.").weak().small());

        let target_emitter = state.selected_sound_emitter_tag.clone();
        if let Some(ref tag) = target_emitter {
            ui.label(RichText::new(format!("Target Emitter: '{}'", tag)).color(Color32::from_rgb(100, 220, 255)).strong());
        } else {
            ui.label(RichText::new("Select an emitter in the Sound Emitters tab to apply presets.").weak());
        }

        ui.add_space(6.0);

        let presets = [
            ("Whispering Wind / Drip", "AmbAncient.Loop.Drip", 8u8, 64u8),
            ("Subtle Room Hum", "AmbAncient.Loop.Hum", 24u8, 96u8),
            ("Heavy Machinery / Generator", "AmbIndustrial.Machine.Gen1", 48u8, 160u8),
            ("Loud Industrial Siren", "AmbModern.Siren.Loop", 96u8, 220u8),
            ("Arena Roar / Thunderstorm", "AmbModern.Arena.Crowd", 160u8, 255u8),
        ];

        for (desc, snd, rad, vol) in presets {
            ui.horizontal(|ui| {
                ui.label(RichText::new(desc).strong());
                ui.label(format!("(Radius: {:.0} uu / Volume: {:.0}%)", rad as f32 * 40.0, vol as f32 / 255.0 * 100.0));
                if let Some(ref tag) = target_emitter {
                    if ui.small_button("Apply to Target").clicked() {
                        let _ = state.update_sound_emitter_properties(tag, snd, rad, vol, 64);
                    }
                }
                if ui.small_button("Place New").clicked() {
                    let _ = state.place_ambient_sound_at_pivot(snd, rad, vol);
                }
            });
        }
    });
}
