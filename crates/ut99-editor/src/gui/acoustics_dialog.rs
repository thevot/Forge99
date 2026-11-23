use crate::editor_state::EditorState;
use egui::{Color32, RichText};
use ut99_bsp::{get_eax_preset, simulate_reverb_impulse_samples, EaxEnvironment};

pub fn render_acoustics_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_acoustics_dialog {
        return;
    }

    let mut open = state.show_acoustics_dialog;

    crate::gui::modal::standard_dialog_window(ctx, "Zone Acoustics, EAX Reverb & Distance Fog", &mut open, [540.0, 500.0], [380.0, 320.0])
        .show(ctx, |ui| {
            ui.heading("Zone Acoustics & EAX Environmental Audio");
            ui.label(
                RichText::new("Configure 3D sound propagation, EAX 2.0 room reverb presets, and volumetric distance fog.")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            // Section 1: Viewport Overlays & Distance Fog
            egui::CollapsingHeader::new(RichText::new("🌫 Distance Fog & Viewport Simulation").strong())
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut state.show_audio_zones, "Render Acoustic Resonance Cages & Sound Radii in 3D View");
                    ui.checkbox(&mut state.distance_fog_enabled, "Enable Real-Time Distance Fog (WGSL Hardware Shaders)");

                    ui.add_enabled_ui(state.distance_fog_enabled, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Fog Start (uu):");
                            ui.add(egui::Slider::new(&mut state.fog_start, 0.0..=5000.0).step_by(10.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog End (uu):  ");
                            ui.add(egui::Slider::new(&mut state.fog_end, 100.0..=15000.0).step_by(50.0));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog Density:   ");
                            ui.add(egui::Slider::new(&mut state.fog_density, 0.1..=4.0).step_by(0.1));
                        });
                        ui.horizontal(|ui| {
                            ui.label("Fog Color (RGB):");
                            ui.color_edit_button_rgb(&mut state.fog_color[0..3].try_into().unwrap_or([0.1, 0.1, 0.1]));
                        });
                    });

                    if ui.button("🔄 Sync Fog Parameters from Selected ZoneInfo").clicked() {
                        if !state.sync_fog_from_selected_zone() {
                            state.log("No ZoneInfo actor selected with valid fog parameters.");
                        }
                    }
                });

            ui.add_space(6.0);
            ui.separator();

            // Section 2: EAX Preset Configuration & Inspection
            let mut active_preset = EaxEnvironment::Generic;
            if let Some(sel_idx) = state.selected_actor_index {
                if let Some(ref level) = state.loaded_unr {
                    if let Some(actor) = level.actors.get(sel_idx) {
                        if actor.class_name.contains("WaterZone") {
                            active_preset = EaxEnvironment::Underwater;
                        } else if let Some(prop) = actor.properties.get("ZoneEffect") {
                            if let ut99_format::TaggedValue::Str(s) = &prop.value {
                                if let Some(p) = ut99_bsp::parse_eax_preset(s) {
                                    active_preset = p;
                                }
                            }
                        }
                    }
                }
            }

            ui.label(RichText::new("EAX 2.0 Reverb Environment Presets").strong());

            let mut selected_preset = active_preset;
            egui::ComboBox::from_label("Active Preset")
                .selected_text(selected_preset.name())
                .show_ui(ui, |ui| {
                    for preset in EaxEnvironment::ALL {
                        ui.selectable_value(&mut selected_preset, *preset, preset.name());
                    }
                });

            let acoustics = get_eax_preset(selected_preset);

            ui.group(|ui| {
                ui.label(RichText::new(format!("Preset: {}", selected_preset.name())).strong().color(Color32::from_rgb(100, 220, 255)));
                ui.label(RichText::new(acoustics.description).italics().size(11.0));
                ui.add_space(4.0);

                egui::Grid::new("acoustics_grid").num_columns(2).striped(true).show(ui, |ui| {
                    ui.label("Reverb Decay Time (T60):");
                    ui.label(format!("{:.2} s", acoustics.reverb_decay_time));
                    ui.end_row();

                    ui.label("Environment Dimensions:");
                    ui.label(format!("{:.0} Unreal Units", acoustics.environment_size));
                    ui.end_row();

                    ui.label("High-Frequency Damping:");
                    ui.label(format!("{:.2}x HF ratio", acoustics.high_frequency_decay_ratio));
                    ui.end_row();

                    ui.label("Early Reflections Delay:");
                    ui.label(format!("{:.1} ms", acoustics.reflections_delay));
                    ui.end_row();

                    ui.label("Late Reverb Delay:");
                    ui.label(format!("{:.1} ms", acoustics.reverb_delay));
                    ui.end_row();

                    ui.label("Room Attenuation:");
                    ui.label(format!("{:.1} dB", acoustics.room_attenuation_db));
                    ui.end_row();
                });
            });

            // Impulse response waveform mini-preview
            let waveform_samples = simulate_reverb_impulse_samples(&acoustics, 24);
            ui.add_space(4.0);
            ui.label(RichText::new("Simulated Impulse Response Envelope:").size(10.0));
            ui.horizontal(|ui| {
                for sample in waveform_samples {
                    let height = (sample.abs() * 28.0).clamp(2.0, 30.0);
                    let (rect, _response) = ui.allocate_exact_size(egui::vec2(14.0, 32.0), egui::Sense::hover());
                    let color = if sample >= 0.0 {
                        Color32::from_rgb(180, 100, 255)
                    } else {
                        Color32::from_rgb(120, 60, 200)
                    };
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.min.x + 2.0, rect.max.y - height),
                        egui::vec2(10.0, height),
                    );
                    ui.painter().rect_filled(bar_rect, 2.0, color);
                }
            });

            ui.add_space(6.0);
            if ui.button(RichText::new(format!("⚡ Apply '{}' to Selected ZoneInfo", selected_preset.name())).strong()).clicked() {
                if !state.apply_eax_preset_to_selected_zone(selected_preset) {
                    state.log("Please select a ZoneInfo actor in the 3D viewport or Outliner.");
                }
            }

            ui.add_space(8.0);
            ui.separator();

            // Section 3: Map Audio Zones Report Table
            ui.label(RichText::new("Zone Audio & Ambient Sound Actors in Map").strong());
            let reports = state.get_zone_audio_reports();

            egui::ScrollArea::vertical()
                .max_height(140.0)
                .show(ui, |ui| {
                    egui::Grid::new("audio_zones_list").num_columns(5).striped(true).show(ui, |ui| {
                        ui.label(RichText::new("Zone / Sound").strong());
                        ui.label(RichText::new("Type").strong());
                        ui.label(RichText::new("Preset").strong());
                        ui.label(RichText::new("T60 Decay").strong());
                        ui.label(RichText::new("Radius").strong());
                        ui.end_row();

                        for rep in reports {
                            ui.label(&rep.zone_name);
                            if rep.is_reverb {
                                ui.colored_label(Color32::from_rgb(200, 120, 255), "Reverb Zone");
                            } else {
                                ui.colored_label(Color32::from_rgb(255, 180, 50), "Ambient Sound");
                            }
                            ui.label(rep.preset.name());
                            ui.label(format!("{:.2} s", rep.reverb_time));
                            ui.label(format!("{:.0} uu", rep.sound_radius));
                            ui.end_row();
                        }
                    });
                });
        });

    state.show_acoustics_dialog = open;
}
