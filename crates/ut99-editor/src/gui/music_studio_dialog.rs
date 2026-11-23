use crate::editor_state::EditorState;
use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{
    simulate_crossfade, MusicIssueSeverity, MusicTrackPreset, MusicTransition,
};

/// Interactive Music Event, Dynamic Audio Ambiance & UMX Track Sequencing Studio
pub fn render_music_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_music_studio_dialog {
        return;
    }

    let mut is_open = state.show_music_studio_dialog;

    Window::new("Music Event & UMX Track Sequencing Studio")
        .open(&mut is_open)
        .default_width(880.0)
        .default_height(620.0)
        .min_width(720.0)
        .min_height(500.0)
        .resizable(true)
        .show(ctx, |ui| {
            render_music_studio_content(ui, state);
        });

    state.show_music_studio_dialog = is_open;
}

fn render_music_studio_content(ui: &mut Ui, state: &mut EditorState) {
    // Header Bar with Quick Action Buttons
    ui.horizontal(|ui| {
        ui.heading("🎵 Music Event & UMX Sequencing Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("📍 Place MusicEvent at Pivot").clicked() {
                let song = state.music_studio_settings.current_track_info.package.clone();
                let section = state.music_studio_settings.selected_section;
                let trans = state.music_studio_settings.crossfade_transition;
                state.place_music_event_at_pivot(&song, section, trans);
            }

            if ui.button("🏆 Set as Level Background Song").clicked() {
                let song = state.music_studio_settings.current_track_info.package.clone();
                let section = state.music_studio_settings.selected_section;
                state.apply_level_music_track(&song, section);
            }
        });
    });

    ui.label(
        RichText::new("Preview tracker modules (.umx/IT/XM/S3M), simulate transition cross-fades, and sequence interactive MusicEvents.")
            .size(11.0)
            .color(Color32::from_rgb(170, 170, 180)),
    );

    ui.separator();

    // 1. Presets Selector Bar
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("Track Presets:").strong());
        for &preset in MusicTrackPreset::all() {
            let is_sel = state.music_studio_settings.selected_preset == preset;
            let btn_text = match preset {
                MusicTrackPreset::ForegoneDestruction => "🌌 Foregone (Face)",
                MusicTrackPreset::RunDeck16 => "🧪 Run (Deck16)",
                MusicTrackPreset::MechanismEight => "⚙ Mech8",
                MusicTrackPreset::BotpackTheme => "🏆 Botpack",
                MusicTrackPreset::NetherAnimal => "🦇 Nether",
                MusicTrackPreset::Colossus => "🪐 Colossus",
            };

            let resp = ui.selectable_label(is_sel, btn_text);
            if resp.clicked() {
                state.music_studio_settings.selected_preset = preset;
                state.music_studio_settings.current_track_info = preset.to_tracker_info();
                state.music_studio_settings.selected_section = 0;
                state.music_studio_settings.playback_time_sec = 0.0;
            }
        }
    });

    ui.add_space(4.0);

    let info = state.music_studio_settings.current_track_info.clone();

    // 2. Track Metadata & Live Transport Controls
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&info.title).size(16.0).strong().color(Color32::from_rgb(100, 220, 255)));
            ui.label(RichText::new(format!("by {}", info.author)).color(Color32::from_rgb(200, 200, 210)));
            ui.separator();
            ui.label(RichText::new(format!("Package: {}.umx", info.package)).color(Color32::from_rgb(255, 210, 100)));
            ui.label(RichText::new(format!("Format: {}", info.format)).color(Color32::from_rgb(120, 240, 160)));
            ui.label(RichText::new(format!("{} Channels", info.channels)).color(Color32::from_rgb(180, 140, 255)));
            ui.label(RichText::new(format!("{} BPM", info.default_bpm)).color(Color32::from_rgb(255, 140, 140)));
        });

        ui.add_space(6.0);

        // Player Bar
        ui.horizontal(|ui| {
            if state.music_studio_settings.is_playing {
                if ui.button("⏸ Pause").clicked() {
                    state.music_studio_settings.is_playing = false;
                }
            } else {
                if ui.button("▶ Play Preview").clicked() {
                    state.music_studio_settings.is_playing = true;
                }
            }

            if ui.button("⏹ Stop").clicked() {
                state.music_studio_settings.is_playing = false;
                state.music_studio_settings.playback_time_sec = 0.0;
            }

            ui.add_space(10.0);
            ui.label("Volume:");
            ui.add(egui::Slider::new(&mut state.music_studio_settings.volume, 0.0..=1.0).text(""));

            ui.add_space(10.0);
            ui.label(format!("Time: {:.1}s", state.music_studio_settings.playback_time_sec));

            if state.music_studio_settings.is_playing {
                state.music_studio_settings.playback_time_sec += 1.0 / 60.0;
                ui.ctx().request_repaint();
            }
        });
    });

    ui.add_space(8.0);

    // 3. Sections Timeline & Crossfade Transition Simulator
    ui.columns(2, |cols| {
        // Left Column: Pattern Sections
        cols[0].group(|ui| {
            ui.heading("🎼 Song Pattern Sections");
            ui.label(
                RichText::new("Sections trigger different mood arrangements within the same tracker module.")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 160, 170)),
            );
            ui.separator();

            for sec in &info.sections {
                let is_active = state.music_studio_settings.selected_section == sec.section_index;
                let bg_color = if is_active {
                    Color32::from_rgb(30, 50, 80)
                } else {
                    Color32::from_rgb(20, 20, 25)
                };

                egui::Frame::NONE
                    .fill(bg_color)
                    .corner_radius(4.0)
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let badge = format!("Sec #{}", sec.section_index);
                            ui.label(RichText::new(badge).strong().color(Color32::from_rgb(255, 200, 80)));
                            ui.label(RichText::new(&sec.name).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(if is_active { "Active ✓" } else { "Select" }).clicked() {
                                    state.music_studio_settings.selected_section = sec.section_index;
                                    state.music_studio_settings.playback_time_sec = 0.0;
                                }
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(&sec.mood).italics().color(Color32::from_rgb(140, 220, 180)).size(10.5));
                            ui.label(RichText::new(format!("Order #{} | {} BPM", sec.order_start, sec.bpm)).size(10.5).color(Color32::from_rgb(180, 180, 180)));
                        });
                    });
                ui.add_space(3.0);
            }
        });

        // Right Column: Transition & Cross-fade Simulator Canvas
        cols[1].group(|ui| {
            ui.heading("🎚 Cross-fade Transition Simulator");
            ui.horizontal(|ui| {
                ui.label("Transition Mode:");
                egui::ComboBox::from_id_salt("music_transition_combo")
                    .selected_text(state.music_studio_settings.crossfade_transition.display_name())
                    .show_ui(ui, |ui| {
                        for &tr in MusicTransition::all() {
                            ui.selectable_value(
                                &mut state.music_studio_settings.crossfade_transition,
                                tr,
                                tr.display_name(),
                            );
                        }
                    });
            });

            let trans = state.music_studio_settings.crossfade_transition;
            let duration = trans.fade_duration_secs();
            ui.label(
                RichText::new(format!("Cross-fade duration: {:.1} seconds", duration))
                    .size(11.0)
                    .color(Color32::from_rgb(200, 200, 140)),
            );

            // Draw Cross-fade Curve Graph Canvas
            let (canvas_rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 110.0), egui::Sense::hover());
            let painter = ui.painter_at(canvas_rect);

            painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(16, 16, 22));
            painter.rect_stroke(
                canvas_rect,
                4.0,
                Stroke::new(1.0_f32, Color32::from_rgb(50, 50, 70)),
                StrokeKind::Outside,
            );

            let steps = 60;
            let max_time = duration.max(2.0);
            let mut out_points = Vec::with_capacity(steps);
            let mut in_points = Vec::with_capacity(steps);

            for i in 0..=steps {
                let t = (i as f32 / steps as f32) * max_time;
                let (v_out, v_in) = simulate_crossfade(1.0, 1.0, trans, t);

                let x = canvas_rect.min.x + (i as f32 / steps as f32) * canvas_rect.width();
                let y_out = canvas_rect.max.y - 10.0 - v_out * (canvas_rect.height() - 20.0);
                let y_in = canvas_rect.max.y - 10.0 - v_in * (canvas_rect.height() - 20.0);

                out_points.push(pos2(x, y_out));
                in_points.push(pos2(x, y_in));
            }

            for i in 0..steps {
                painter.line_segment(
                    [out_points[i], out_points[i + 1]],
                    Stroke::new(2.0_f32, Color32::from_rgb(255, 90, 90)), // Outgoing fade
                );
                painter.line_segment(
                    [in_points[i], in_points[i + 1]],
                    Stroke::new(2.0_f32, Color32::from_rgb(80, 220, 120)), // Incoming fade
                );
            }

            ui.horizontal(|ui| {
                ui.colored_label(Color32::from_rgb(255, 90, 90), "■ Outgoing Section");
                ui.colored_label(Color32::from_rgb(80, 220, 120), "■ Incoming Section");
            });
        });
    });

    ui.add_space(8.0);

    // 4. MusicEvents in Level & Audit
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading("🎬 Level Music Events & Triggers");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("🔍 Run Full Music Audit").clicked() {
                    state.audit_music_setup();
                }
            });
        });

        let events = state.collect_music_events();
        if events.is_empty() {
            ui.label(
                RichText::new("No MusicEvent actors found in this level. Use 'Place MusicEvent at Pivot' above to add dynamic soundtrack cues.")
                    .color(Color32::from_rgb(180, 180, 190)),
            );
        } else {
            egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                egui::Grid::new("music_events_grid").striped(true).num_columns(6).show(ui, |ui| {
                    ui.label(RichText::new("Actor").strong());
                    ui.label(RichText::new("Tag").strong());
                    ui.label(RichText::new("Target Song").strong());
                    ui.label(RichText::new("Sec").strong());
                    ui.label(RichText::new("Transition").strong());
                    ui.label(RichText::new("Action").strong());
                    ui.end_row();

                    for ev in &events {
                        ui.label(&ev.name);
                        ui.label(if ev.tag.is_empty() {
                            RichText::new("(none)").color(Color32::from_rgb(255, 120, 120))
                        } else {
                            RichText::new(&ev.tag).color(Color32::from_rgb(140, 210, 255))
                        });
                        ui.label(if ev.silence {
                            RichText::new("🔇 Silence").color(Color32::from_rgb(255, 180, 100))
                        } else {
                            RichText::new(format!("{}.umx", ev.song)).strong()
                        });
                        ui.label(format!("#{}", ev.song_section));
                        ui.label(ev.transition.display_name());

                        let target_loc = ev.location;
                        if ui.button("🎯 Focus").clicked() {
                            state.camera.position = target_loc + glam::Vec3::new(0.0, -150.0, 50.0);
                        }
                        ui.end_row();
                    }
                });
            });
        }

        // Audit Report Results
        if let Some(ref audit) = state.music_studio_settings.last_audit_report {
            ui.add_space(4.0);
            ui.separator();
            ui.horizontal(|ui| {
                if audit.is_valid {
                    ui.colored_label(Color32::from_rgb(80, 240, 120), "✅ Music Configuration Valid");
                } else {
                    ui.colored_label(Color32::from_rgb(255, 80, 80), "⚠ Music Configuration Issues Detected");
                }
                ui.label(format!("Default Song: '{}' | Referenced Tracks: {}", audit.level_song, audit.unique_songs_referenced.len()));
            });

            for issue in &audit.issues {
                let col = match issue.severity {
                    MusicIssueSeverity::Critical => Color32::from_rgb(255, 80, 80),
                    MusicIssueSeverity::Warning => Color32::from_rgb(255, 180, 60),
                    MusicIssueSeverity::Info => Color32::from_rgb(100, 200, 255),
                };
                ui.horizontal(|ui| {
                    ui.colored_label(col, "•");
                    ui.label(RichText::new(&issue.title).strong().color(col));
                    ui.label(&issue.description);
                });
            }
        }
    });
}
