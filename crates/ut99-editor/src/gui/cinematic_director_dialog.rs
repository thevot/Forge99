use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use ut99_bsp::CinematicPreset;

/// Interactive Cinematic Matinee Camera Director Studio
pub fn render_cinematic_director_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_cinematic_studio_dialog {
        return;
    }

    let mut is_open = state.show_cinematic_studio_dialog;

    Window::new("Cinematic Matinee Camera Director Studio")
        .open(&mut is_open)
        .default_width(840.0)
        .default_height(580.0)
        .min_width(680.0)
        .min_height(450.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.heading("🎬 Camera Director Studio");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚡ Export to InterpolationPoints").clicked() {
                        state.export_cinematic_to_level_actors();
                    }
                    let pilot_btn_col = if state.cinematic_settings.pilot_camera {
                        Color32::from_rgb(255, 100, 100)
                    } else {
                        Color32::from_rgb(100, 200, 255)
                    };
                    let pilot_label = if state.cinematic_settings.pilot_camera {
                        "🎥 Piloting Active (Click to Disengage)"
                    } else {
                        "🎥 Pilot Viewport Camera"
                    };
                    if ui.button(RichText::new(pilot_label).color(pilot_btn_col)).clicked() {
                        state.cinematic_settings.pilot_camera = !state.cinematic_settings.pilot_camera;
                    }
                });
            });

            ui.separator();

            // Playback Transport Controls
            let est_dur = state.cinematic_settings.active_track.estimated_duration();
            let cur_sec = state.cinematic_settings.playhead_t * est_dur;

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    let play_label = if state.cinematic_settings.is_playing { "⏸ Pause" } else { "▶ Play" };
                    if ui.button(play_label).clicked() {
                        state.cinematic_settings.is_playing = !state.cinematic_settings.is_playing;
                    }
                    if ui.button("⏪ Rewind").clicked() {
                        state.cinematic_settings.playhead_t = 0.0;
                    }

                    ui.separator();

                    ui.label(format!("{:.1}s / {:.1}s", cur_sec, est_dur));
                    ui.add(egui::Slider::new(&mut state.cinematic_settings.playhead_t, 0.0..=1.0)
                        .show_value(false)
                        .text("Playhead"));

                    ui.separator();

                    // Playback speed buttons
                    let cur_speed = state.cinematic_settings.playback_speed;
                    if ui.selectable_label((cur_speed - 0.5).abs() < 1e-2, "0.5x").clicked() {
                        state.cinematic_settings.playback_speed = 0.5;
                    }
                    if ui.selectable_label((cur_speed - 1.0).abs() < 1e-2, "1.0x").clicked() {
                        state.cinematic_settings.playback_speed = 1.0;
                    }
                    if ui.selectable_label((cur_speed - 2.0).abs() < 1e-2, "2.0x").clicked() {
                        state.cinematic_settings.playback_speed = 2.0;
                    }

                    ui.separator();

                    ui.checkbox(&mut state.cinematic_settings.active_track.is_looping, "Loop");
                    ui.checkbox(&mut state.cinematic_settings.show_spline_path, "3D Spline");
                    ui.checkbox(&mut state.cinematic_settings.show_camera_frustum, "Frustum");
                });
            });

            ui.add_space(4.0);

            // Main Editor Layout: Left Column = Keyframes Sequence, Right Column = Presets
            ui.horizontal(|ui| {
                // Left Column: Keyframes Timeline
                ui.vertical(|ui| {
                    ui.set_width(480.0);
                    render_keyframes_timeline(ui, state);
                });

                ui.separator();

                // Right Column: Presets Library
                ui.vertical(|ui| {
                    ui.label(RichText::new("⭐ Canonical UT99 Flyby Presets").strong().size(13.0));
                    ui.label(RichText::new("Pre-calibrated camera sequences for tournaments & trailers.").size(11.0).color(Color32::GRAY));
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical().id_salt("cinematic_presets_scroll").max_height(400.0).show(ui, |ui| {
                        for preset in CinematicPreset::all() {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(preset.title()).strong().color(Color32::from_rgb(255, 200, 80)));
                                    ui.label(RichText::new(preset.description()).size(11.0).color(Color32::LIGHT_GRAY));
                                    ui.add_space(4.0);
                                    if ui.button("Apply Preset").clicked() {
                                        state.apply_cinematic_preset(*preset);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                });
            });
        });

    state.show_cinematic_studio_dialog = is_open;
}

fn render_keyframes_timeline(ui: &mut Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("Spline Camera Nodes").strong().size(13.0));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📸 Capture Viewport Camera").clicked() {
                state.add_keyframe_at_camera_position();
            }
        });
    });

    ui.add_space(4.0);

    let kf_count = state.cinematic_settings.active_track.keyframes.len();
    if kf_count == 0 {
        ui.label(RichText::new("No keyframes in active track. Click 'Capture Viewport Camera' or pick a preset.").italics().color(Color32::GRAY));
        return;
    }

    egui::ScrollArea::vertical().id_salt("cinematic_keyframes_scroll").max_height(420.0).show(ui, |ui| {
        let mut to_remove = None;
        let mut to_swap_up = None;

        for i in 0..kf_count {
            let kf = &mut state.cinematic_settings.active_track.keyframes[i];

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("#{i} {}", kf.label)).strong().color(Color32::from_rgb(100, 220, 255)));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("🗑").clicked() {
                            to_remove = Some(i);
                        }
                        if i > 0 && ui.button("▲").clicked() {
                            to_swap_up = Some(i);
                        }
                    });
                });

                ui.horizontal(|ui| {
                    ui.label("Pos:");
                    ui.add(egui::DragValue::new(&mut kf.position.x).speed(2.0).prefix("X: "));
                    ui.add(egui::DragValue::new(&mut kf.position.y).speed(2.0).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut kf.position.z).speed(2.0).prefix("Z: "));
                });

                ui.horizontal(|ui| {
                    ui.label("Rot:");
                    ui.add(egui::DragValue::new(&mut kf.rotation.x).speed(1.0).prefix("P: "));
                    ui.add(egui::DragValue::new(&mut kf.rotation.y).speed(1.0).prefix("Y: "));
                    ui.add(egui::DragValue::new(&mut kf.rotation.z).speed(1.0).prefix("R: "));
                    ui.separator();
                    ui.label("FOV:");
                    ui.add(egui::DragValue::new(&mut kf.fov).range(30.0..=140.0).speed(0.5).suffix("°"));
                });

                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.add(egui::DragValue::new(&mut kf.speed).range(50.0..=1500.0).speed(5.0).suffix(" uu/s"));
                    ui.separator();
                    ui.label("Pause:");
                    ui.add(egui::DragValue::new(&mut kf.pause_time).range(0.0..=10.0).speed(0.1).suffix(" s"));
                });
            });
            ui.add_space(2.0);
        }

        if let Some(i) = to_remove {
            state.remove_cinematic_keyframe(i);
        }
        if let Some(i) = to_swap_up {
            state.cinematic_settings.active_track.keyframes.swap(i, i - 1);
            state.map_is_dirty = true;
        }
    });
}
