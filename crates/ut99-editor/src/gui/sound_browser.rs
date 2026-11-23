use crate::editor_state::EditorState;
use egui::{Color32, Pos2, Rect, RichText, Stroke};

pub fn render_sound_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_sound_browser {
        return;
    }

    let mut open = state.show_sound_browser;

    crate::gui::modal::standard_dialog_window(ctx, "Sound Browser (.uax)", &mut open, [420.0, 520.0], [340.0, 320.0])
        .show(ctx, |ui| {
            if state.sound_packages.is_empty() { ui.label("No sound packages found in the active UT installation."); }
            // Package Selector Bar
            let mut switch_pkg = None;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Package:").strong());
                egui::ComboBox::from_id_salt("sound_package_combo")
                    .selected_text(&state.selected_sound_package)
                    .show_ui(ui, |ui| {
                        for pkg in &state.sound_packages {
                            if ui.selectable_value(&mut state.selected_sound_package, pkg.clone(), pkg).clicked() {
                                switch_pkg = Some(pkg.clone());
                            }
                        }
                    });

                if ui.button("🔄 Reload").clicked() {
                    state.scan_sound_packages();
                }

                if ui.button("💾 Save .uax").on_hover_text("Export current sound package as a binary .uax package").clicked() {
                    let pkg_name = if state.selected_sound_package.is_empty() {
                        "CustomSounds".to_string()
                    } else {
                        state.selected_sound_package.clone()
                    };
                    if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{pkg_name}.uax"))
                        .add_filter("Unreal Sound", &["uax"]).save_file() {
                        match state.export_sound_package_uax(&path) {
                            Ok(()) => state.toast_success(format!("Exported {}", path.display())),
                            Err(error) => state.audio_error(format!("Sound export to '{}' failed: {error:#}", path.display())),
                        }
                    }
                }
            });

            if let Some(pkg) = switch_pkg {
                state.load_sound_package(&pkg);
            }

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Filter:");
                ui.text_edit_singleline(&mut state.sound_browser_search);
                if ui.button("✖").clicked() {
                    state.sound_browser_search.clear();
                }
            });
            ui.separator();

            let search = state.sound_browser_search.to_lowercase();
            let mut select_sound = None;
            let mut double_clicked_sound = None;

            // Sounds List
            ui.label(format!("Sounds in package ({} total):", state.loaded_sounds.len()));
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                for sound in &state.loaded_sounds {
                    if !search.is_empty() && !sound.name.to_lowercase().contains(&search) {
                        continue;
                    }
                    let is_sel = state.selected_sound_name.as_deref() == Some(&sound.name);
                    let chan_str = if sound.channels == 1 { "Mono" } else { "Stereo" };
                    let label = format!("🔊 {}  ({:.2}s, {}Hz {})", sound.name, sound.duration_seconds, sound.sample_rate, chan_str);

                    let item = if is_sel {
                        ui.selectable_label(true, RichText::new(label).color(Color32::from_rgb(255, 230, 80)).strong())
                    } else {
                        ui.selectable_label(false, label)
                    };

                    if item.double_clicked() {
                        select_sound = Some(sound.name.clone());
                        double_clicked_sound = Some(sound.name.clone());
                    } else if item.clicked() {
                        select_sound = Some(sound.name.clone());
                    }
                }
            });

            if let Some(name) = select_sound {
                state.selected_sound_name = Some(name);
            }

            if let Some(sound_name) = double_clicked_sound {
                let full_name = format!("{}.{}", state.selected_sound_package, sound_name);
                if let Some(m_idx) = state.selected_mover_index {
                    if let Some(mover) = state.movers.get_mut(m_idx) {
                        let old_mover = mover.clone();
                        mover.opening_sound = full_name.clone();
                        let new_mover = mover.clone();
                        let m_name = mover.name.clone();
                        state.history.push(crate::history::Action::UpdateElevatorMoverConfig {
                            mover_index: m_idx,
                            old_mover,
                            new_mover,
                            description: format!("Set Mover '{}' OpeningSound to '{}'", m_name, full_name),
                        });
                        state.map_is_dirty = true;
                        state.log(format!("Assigned opening sound '{}' to Mover '{}'.", full_name, m_name));
                        state.toast_success(format!("Assigned sound to Mover '{}'", m_name));
                    }
                } else {
                    state.log(format!("Selected sound '{}'.", full_name));
                    state.toast_info(format!("Selected active sound '{}'", full_name));
                }
            }

            ui.add_space(8.0);
            ui.separator();

            // Selected Sound Details & Waveform Visualizer
            let mut play_req: Option<(String, Vec<u8>)> = None;
            let mut stop_req = false;
            let mut assign_sound_msg = None;
            let mut assign_closed_sound_msg = None;
            let mut place_ambient_req = None;
            let mut embed_mylevel_req = None;
            let mut delete_sound_req = None;

            if let Some(ref sel_name) = state.selected_sound_name.clone() {
                if let Some(sound) = state.loaded_sounds.iter().find(|s| &s.name == sel_name) {
                    let s_name = sound.name.clone();
                    let s_dur = sound.duration_seconds;
                    let s_size = sound.data_size;
                    let wav_bytes = sound.wav_bytes.clone();
                    let pkg_name = state.selected_sound_package.clone();
                    let full_ref = format!("{}.{}", pkg_name, s_name);

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Selected:").strong());
                        ui.label(RichText::new(&s_name).color(Color32::from_rgb(100, 220, 255)).strong());
                        ui.label(format!("({:.2} seconds, {} bytes)", s_dur, s_size));
                    });

                    // Waveform Canvas
                    let (canvas_rect, _) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 64.0), egui::Sense::hover());
                    ui.painter().rect_filled(canvas_rect, 4.0, Color32::from_rgb(20, 22, 28));

                    render_waveform(ui, canvas_rect, &wav_bytes);

                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("▶ Play Sound").on_hover_text("Play WAV sound sample asynchronously").clicked() {
                            play_req = Some((full_ref.clone(), wav_bytes.clone()));
                        }

                        if ui.button("⏹ Stop").on_hover_text("Stop active sound preview").clicked() {
                            stop_req = true;
                        }

                        if ui.button("🔊 Place AmbientSound").on_hover_text("Spawn an AmbientSound actor at the Red Builder Brush with this sound").clicked() {
                            place_ambient_req = Some(full_ref.clone());
                        }

                        if ui.button("📌 Mover OpeningSound").on_hover_text("Assign as OpeningSound on the currently selected Mover").clicked() {
                            assign_sound_msg = Some(full_ref.clone());
                        }

                        if ui.button("📌 Mover ClosedSound").on_hover_text("Assign as ClosedSound on the currently selected Mover").clicked() {
                            assign_closed_sound_msg = Some(full_ref);
                        }

                        if ui.button("📦 Embed in MyLevel").on_hover_text("Embed this USound into the MyLevel pseudo-package inside the .unr map").clicked() {
                            embed_mylevel_req = Some(s_name.clone());
                        }

                        if ui.button("🗑 Delete").on_hover_text("Delete this sound from the active package").clicked() {
                            delete_sound_req = Some(s_name);
                        }
                    });
                }
            } else {
                ui.label("Select a sound from the list above to view waveform and play.");
            }

            if let Some((full_ref, wav_bytes)) = play_req {
                let _ = state.play_sound_preview(&full_ref, &wav_bytes);
            }
            if stop_req {
                state.stop_audio_preview();
            }
            if let Some(full_ref) = place_ambient_req {
                state.place_ambient_sound_at_builder(&full_ref);
            }
            if let Some(s_name) = embed_mylevel_req {
                if state.embed_sound_in_mylevel(&s_name) {
                    state.toast_success(format!("Embedded '{}' in MyLevel", s_name));
                }
            }
            if let Some(s_name) = delete_sound_req {
                if state.delete_sound_from_package(&s_name) {
                    state.toast_info(format!("Deleted sound '{}'", s_name));
                }
            }
            if let Some(full_name) = assign_sound_msg {
                if let Some(m_idx) = state.selected_mover_index {
                    if let Some(mover) = state.movers.get_mut(m_idx) {
                        let old_mover = mover.clone();
                        mover.opening_sound = full_name.clone();
                        let new_mover = mover.clone();
                        let m_name = mover.name.clone();
                        state.history.push(crate::history::Action::UpdateElevatorMoverConfig {
                            mover_index: m_idx,
                            old_mover,
                            new_mover,
                            description: format!("Set Mover '{}' OpeningSound to '{}'", m_name, full_name),
                        });
                        state.map_is_dirty = true;
                        state.log(format!("Assigned opening sound '{}' to Mover '{}'.", full_name, m_name));
                    }
                } else {
                    state.log(format!("Selected sound '{}'.", full_name));
                }
            }
            if let Some(full_name) = assign_closed_sound_msg {
                if let Some(m_idx) = state.selected_mover_index {
                    if let Some(mover) = state.movers.get_mut(m_idx) {
                        let old_mover = mover.clone();
                        mover.closed_sound = full_name.clone();
                        let new_mover = mover.clone();
                        let m_name = mover.name.clone();
                        state.history.push(crate::history::Action::UpdateElevatorMoverConfig {
                            mover_index: m_idx,
                            old_mover,
                            new_mover,
                            description: format!("Set Mover '{}' ClosedSound to '{}'", m_name, full_name),
                        });
                        state.map_is_dirty = true;
                        state.log(format!("Assigned closed sound '{}' to Mover '{}'.", full_name, m_name));
                    }
                } else {
                    state.log(format!("Selected sound '{}'.", full_name));
                }
            }
        });

    state.show_sound_browser = open;
}

/// Renders audio waveform amplitudes onto the canvas (supports both 8-bit unsigned and 16-bit signed LE PCM)
fn render_waveform(ui: &egui::Ui, rect: Rect, wav_bytes: &[u8]) {
    let painter = ui.painter();
    let center_y = rect.center().y;
    let width = rect.width();

    let bits_per_sample = if wav_bytes.len() >= 36 && &wav_bytes[0..4] == b"RIFF" {
        u16::from_le_bytes([wav_bytes[34], wav_bytes[35]])
    } else {
        8
    };

    // Find PCM data chunk
    let data_pos = wav_bytes.windows(4).position(|w| w == b"data").map(|p| p + 8).unwrap_or(44);
    let pcm = if data_pos < wav_bytes.len() {
        &wav_bytes[data_pos..]
    } else {
        &[]
    };

    if pcm.is_empty() {
        painter.line_segment(
            [Pos2::new(rect.min.x, center_y), Pos2::new(rect.max.x, center_y)],
            Stroke::new(1.0_f32, Color32::from_rgb(60, 60, 70)),
        );
        return;
    }

    let sample_bytes = if bits_per_sample == 16 { 2 } else { 1 };
    let total_samples = pcm.len() / sample_bytes;
    if total_samples == 0 {
        return;
    }

    let points_count = (width as usize).min(300).max(32);
    let step = (total_samples / points_count).max(1);
    let wave_color = Color32::from_rgb(80, 200, 255);

    let mut prev_pt: Option<Pos2> = None;
    for i in 0..points_count {
        let sample_idx = (i * step).min(total_samples - 1);
        let norm_val = if bits_per_sample == 16 {
            let byte_idx = sample_idx * 2;
            if byte_idx + 1 < pcm.len() {
                let s = i16::from_le_bytes([pcm[byte_idx], pcm[byte_idx + 1]]);
                (s as f32 / 32768.0).clamp(-1.0, 1.0)
            } else {
                0.0
            }
        } else {
            let raw_val = pcm[sample_idx] as f32;
            ((raw_val - 128.0) / 128.0).clamp(-1.0, 1.0)
        };
        let px = rect.min.x + (i as f32 / points_count as f32) * width;
        let py = center_y - norm_val * (rect.height() * 0.42);

        let pt = Pos2::new(px, py);
        if let Some(prev) = prev_pt {
            painter.line_segment([prev, pt], Stroke::new(1.5_f32, wave_color));
        }
        prev_pt = Some(pt);
    }
}
