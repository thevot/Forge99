use crate::editor_state::EditorState;
use ut99_bsp::DispatcherPresetTemplate;
use egui::{Color32, RichText, Ui, Window};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatcherTab {
    TimelineEditor,
    Presets,
    ChainsAndHazards,
    RoundRobin,
}

/// Interactive Dispatcher Sequence & Trigger Timeline Studio
pub fn show_dispatcher_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_dispatcher_dialog {
        return;
    }

    if state.dispatcher_settings.is_playing {
        let dt = ctx.input(|i| i.stable_dt);
        state.update_dispatcher_timeline(dt);
        ctx.request_repaint();
    }

    let mut is_open = state.show_dispatcher_dialog;

    let tab_id = egui::Id::new("dispatcher_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<DispatcherTab>(tab_id)
            .unwrap_or(DispatcherTab::TimelineEditor)
    });

    Window::new("Dispatcher Sequence & Trigger Timeline Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(540.0)
        .min_width(620.0)
        .min_height(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            let sequences = state.get_dispatcher_sequences();

            // Header bar
            ui.horizontal(|ui| {
                ui.heading("⏱ Scripted Dispatcher Timeline");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📍 Place New Dispatcher at Pivot").clicked() {
                        let new_idx = state.place_dispatcher_at_pivot(Some(DispatcherPresetTemplate::TimedBlastAndBreach));
                        if let Some(ref l) = state.loaded_unr {
                            if let Some(act) = l.actors.get(new_idx) {
                                state.selected_dispatcher_name = Some(act.name.clone());
                                state.dispatcher_settings.selected_dispatcher_name = Some(act.name.clone());
                            }
                        }
                    }
                });
            });

            ui.separator();

            // Tab navigation
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == DispatcherTab::TimelineEditor, "🎛 Stage Editor & Flow").clicked() {
                    current_tab = DispatcherTab::TimelineEditor;
                }
                if ui.selectable_label(current_tab == DispatcherTab::Presets, "🎨 Scripted Sequence Presets").clicked() {
                    current_tab = DispatcherTab::Presets;
                }
                if ui.selectable_label(current_tab == DispatcherTab::ChainsAndHazards, "⛓ Cascading Chains & Hazards").clicked() {
                    current_tab = DispatcherTab::ChainsAndHazards;
                }
                if ui.selectable_label(current_tab == DispatcherTab::RoundRobin, "🔂 RoundRobin Step Sequencers").clicked() {
                    current_tab = DispatcherTab::RoundRobin;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            // Active sequence resolution
            let current_name = state.dispatcher_settings.selected_dispatcher_name.clone()
                .or_else(|| state.selected_dispatcher_name.clone())
                .unwrap_or_else(|| {
                    sequences.first().map(|s| s.dispatcher_name.clone()).unwrap_or_default()
                });

            let active_seq = sequences.iter().find(|s| s.dispatcher_name == current_name)
                .or_else(|| sequences.first());

            match current_tab {
                DispatcherTab::TimelineEditor => {
                    if sequences.is_empty() {
                        ui.colored_label(
                            Color32::from_rgb(180, 180, 180),
                            "No Dispatcher actors found in the active map.",
                        );
                        ui.label("Click 'Place New Dispatcher at Pivot' above or browse the 'Scripted Sequence Presets' tab to insert a Dispatcher actor to coordinate multi-stage doors, alarms, explosions, and puzzles.");
                    } else {
                        // Dispatcher selector & metrics
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Select Dispatcher:").strong());
                            egui::ComboBox::from_id_salt("dispatcher_combo")
                                .selected_text(&current_name)
                                .show_ui(ui, |ui| {
                                    for seq in &sequences {
                                        let is_sel = current_name == seq.dispatcher_name;
                                        if ui.selectable_label(is_sel, &seq.dispatcher_name).clicked() {
                                            state.selected_dispatcher_name = Some(seq.dispatcher_name.clone());
                                            state.dispatcher_settings.selected_dispatcher_name = Some(seq.dispatcher_name.clone());
                                            state.dispatcher_settings.timeline_time = 0.0;
                                        }
                                    }
                                });

                            if let Some(seq) = active_seq {
                                ui.separator();
                                ui.label(format!("Tag: '{}'", if seq.tag.is_empty() { "(None)" } else { &seq.tag }));
                                ui.separator();
                                ui.label(format!("Total: {:.2}s", seq.total_duration_secs));
                                ui.separator();
                                ui.label(format!("Stages: {}", seq.stages.len()));
                            }
                        });

                        ui.add_space(4.0);

                        // Transport Controls Bar
                        if let Some(seq) = active_seq {
                            let max_duration = seq.total_duration_secs.max(0.5);

                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    if state.dispatcher_settings.is_playing {
                                        if ui.button("⏸ Pause").clicked() {
                                            state.dispatcher_settings.is_playing = false;
                                        }
                                    } else {
                                        if ui.button("▶ Play").clicked() {
                                            if state.dispatcher_settings.timeline_time >= max_duration {
                                                state.dispatcher_settings.timeline_time = 0.0;
                                            }
                                            state.dispatcher_settings.is_playing = true;
                                        }
                                    }

                                    if ui.button("⏹ Stop").clicked() {
                                        state.dispatcher_settings.is_playing = false;
                                        state.dispatcher_settings.timeline_time = 0.0;
                                    }

                                    ui.separator();
                                    ui.label("Speed:");
                                    ui.selectable_value(&mut state.dispatcher_settings.play_speed, 0.5, "0.5x");
                                    ui.selectable_value(&mut state.dispatcher_settings.play_speed, 1.0, "1.0x");
                                    ui.selectable_value(&mut state.dispatcher_settings.play_speed, 2.0, "2.0x");

                                    ui.separator();
                                    ui.checkbox(&mut state.dispatcher_settings.loop_preview, "Loop");

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        ui.colored_label(
                                            Color32::from_rgb(100, 220, 255),
                                            format!("T = {:.2}s / {:.2}s", state.dispatcher_settings.timeline_time, max_duration),
                                        );
                                    });
                                });

                                // Interactive Scrubbing Timeline Canvas
                                let available_width = ui.available_width().max(200.0);
                                let (canvas_rect, response) = ui.allocate_exact_size(egui::vec2(available_width, 36.0), egui::Sense::click_and_drag());
                                
                                if response.dragged() || response.clicked() {
                                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                                        let frac = ((mouse_pos.x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0);
                                        state.dispatcher_settings.timeline_time = frac * max_duration;
                                    }
                                }

                                let painter = ui.painter_at(canvas_rect);
                                // Background track
                                painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(25, 28, 35));
                                painter.rect_stroke(canvas_rect, 4.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)), egui::StrokeKind::Inside);

                                // Stage markers
                                for stage in &seq.stages {
                                    let t_frac = (stage.cumulative_time_secs / max_duration).clamp(0.0, 1.0);
                                    let x = canvas_rect.min.x + t_frac * canvas_rect.width();

                                    // Vertical stage notch
                                    painter.line_segment(
                                        [egui::pos2(x, canvas_rect.min.y + 4.0), egui::pos2(x, canvas_rect.max.y - 4.0)],
                                        egui::Stroke::new(1.5_f32, Color32::from_rgb(120, 160, 240)),
                                    );

                                    // Stage notch text
                                    painter.text(
                                        egui::pos2(x, canvas_rect.min.y + 6.0),
                                        egui::Align2::CENTER_TOP,
                                        format!("#{}", stage.index),
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgb(180, 200, 240),
                                    );
                                }

                                // Playhead cursor
                                let playhead_frac = (state.dispatcher_settings.timeline_time / max_duration).clamp(0.0, 1.0);
                                let playhead_x = canvas_rect.min.x + playhead_frac * canvas_rect.width();
                                painter.line_segment(
                                    [egui::pos2(playhead_x, canvas_rect.min.y), egui::pos2(playhead_x, canvas_rect.max.y)],
                                    egui::Stroke::new(2.5_f32, Color32::from_rgb(255, 210, 40)),
                                );
                                painter.circle_filled(egui::pos2(playhead_x, canvas_rect.min.y + 3.0), 4.0, Color32::from_rgb(255, 210, 40));
                            });
                        }

                        ui.add_space(4.0);
                        render_timeline_editor_tab(ui, state, active_seq);
                    }
                }
                DispatcherTab::Presets => render_presets_tab(ui, state, active_seq),
                DispatcherTab::ChainsAndHazards => render_chains_tab(ui, state, active_seq),
                DispatcherTab::RoundRobin => render_round_robin_tab(ui, state),
            }
        });

    state.show_dispatcher_dialog = is_open;
}

fn render_timeline_editor_tab(ui: &mut Ui, state: &mut EditorState, active_seq: Option<&ut99_bsp::DispatcherSequence>) {
    if let Some(seq) = active_seq {
        ui.label(RichText::new("Chronological Event Execution Flow & Stage Timing Editor").strong().size(13.0));
        ui.label(RichText::new("Modify individual OutDelays and OutEvents with live timing evaluation and target resolution.").size(11.0).color(Color32::GRAY));
        ui.add_space(4.0);

        let disp_name = seq.dispatcher_name.clone();

        egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
            egui::Grid::new("dispatcher_stage_table")
                .striped(true)
                .min_col_width(70.0)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Stage").strong());
                    ui.label(RichText::new("Delay (Δt)").strong());
                    ui.label(RichText::new("Time (T)").strong());
                    ui.label(RichText::new("OutEvent (Target Tag)").strong());
                    ui.label(RichText::new("Matched Target Actors").strong());
                    ui.end_row();

                    for stage in &seq.stages {
                        let is_active = (state.dispatcher_settings.timeline_time >= stage.cumulative_time_secs - 0.2)
                            && (state.dispatcher_settings.timeline_time <= stage.cumulative_time_secs + 0.4);

                        let idx_text = if is_active {
                            RichText::new(format!("▶ #{}", stage.index)).strong().color(Color32::from_rgb(255, 210, 40))
                        } else {
                            RichText::new(format!("#{}", stage.index))
                        };
                        ui.label(idx_text);

                        // Editable delay
                        let mut delay = stage.delay_secs;
                        if ui.add(egui::DragValue::new(&mut delay).speed(0.1).range(0.0..=60.0).suffix("s")).changed() {
                            state.update_dispatcher_stage(&disp_name, stage.index, delay, &stage.event_name);
                        }

                        ui.colored_label(Color32::from_rgb(100, 200, 255), format!("T = {:.2}s", stage.cumulative_time_secs));

                        // Editable event string
                        let mut event_str = stage.event_name.clone();
                        if ui.text_edit_singleline(&mut event_str).changed() {
                            state.update_dispatcher_stage(&disp_name, stage.index, stage.delay_secs, &event_str);
                        }

                        // Target summary
                        if stage.matched_targets.is_empty() {
                            ui.colored_label(Color32::from_rgb(255, 100, 100), "❌ 0 actors (Broken Tag)");
                        } else {
                            let targets_str = stage
                                .matched_targets
                                .iter()
                                .map(|(name, _)| name.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            ui.colored_label(
                                Color32::from_rgb(80, 240, 120),
                                format!("✔ {} ({})", stage.matched_targets.len(), targets_str),
                            );
                        }
                        ui.end_row();
                    }
                });
        });
    }
}

fn render_presets_tab(ui: &mut Ui, state: &mut EditorState, active_seq: Option<&ut99_bsp::DispatcherSequence>) {
    ui.label(RichText::new("Canonical Scripted Sequence Templates").strong().size(13.0));
    ui.label(RichText::new("Pre-calibrated multi-stage trigger patterns for doors, airlocks, explosive breaches, and ambushes.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let active_name = active_seq.map(|s| s.dispatcher_name.clone());

    egui::ScrollArea::vertical().show(ui, |ui| {
        for &preset in DispatcherPresetTemplate::all() {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(preset.label()).strong().size(13.0));
                        ui.label(RichText::new(preset.description()).size(11.0).color(Color32::GRAY));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("📍 Spawn New at Pivot").clicked() {
                            let new_idx = state.place_dispatcher_at_pivot(Some(preset));
                            if let Some(ref l) = state.loaded_unr {
                                if let Some(act) = l.actors.get(new_idx) {
                                    state.selected_dispatcher_name = Some(act.name.clone());
                                    state.dispatcher_settings.selected_dispatcher_name = Some(act.name.clone());
                                }
                            }
                        }
                        if ui.button("Apply to Active Dispatcher").clicked() {
                            if let Some(ref name) = active_name {
                                state.apply_dispatcher_preset(name, preset);
                            }
                        }
                    });
                });

                ui.add_space(4.0);
                ui.horizontal_wrapped(|ui| {
                    for (i, (dt, ev)) in preset.stages().iter().enumerate() {
                        ui.label(RichText::new(format!("[#{}: +{:.1}s -> {}]", i, dt, ev)).size(10.5).color(Color32::LIGHT_GRAY));
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn render_chains_tab(ui: &mut Ui, state: &mut EditorState, active_seq: Option<&ut99_bsp::DispatcherSequence>) {
    ui.label(RichText::new("Multi-Dispatcher Cascading Chains & Analytical Hazards").strong().size(13.0));
    ui.label(RichText::new("Automated audit for cascading trigger workflows and recursion hazards (preventing server stack overflows).").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    // Hazard checks on active dispatcher
    if let Some(seq) = active_seq {
        if seq.has_recursive_loop {
            ui.colored_label(
                Color32::from_rgb(255, 75, 75),
                "⚠ HAZARD: Infinite loop detected! Dispatcher triggers its own Tag, which crashes Unreal Tournament with Script Call Stack Overflow.",
            );
        }
        if seq.has_unmatched_events {
            ui.colored_label(
                Color32::from_rgb(255, 180, 50),
                "⚠ WARNING: One or more OutEvents have no corresponding receiver actors in the level.",
            );
        }
    }

    ui.add_space(8.0);
    let chains = state.detect_cascading_dispatcher_chains();

    ui.label(RichText::new(format!("Detected Cascading Inter-Dispatcher Links ({})", chains.len())).strong());
    if chains.is_empty() {
        ui.label(RichText::new("No inter-dispatcher links detected (all dispatchers operate independently).").color(Color32::GRAY));
    } else {
        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
            egui::Grid::new("chains_grid")
                .striped(true)
                .min_col_width(90.0)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Source").strong());
                    ui.label(RichText::new("Stage").strong());
                    ui.label(RichText::new("Target Dispatcher").strong());
                    ui.label(RichText::new("Trigger Time").strong());
                    ui.label(RichText::new("Safety Status").strong());
                    ui.end_row();

                    for chain in &chains {
                        ui.label(&chain.source_dispatcher);
                        ui.label(format!("#{} ('{}')", chain.stage_index, chain.event_name));
                        ui.label(&chain.target_dispatcher);
                        ui.colored_label(Color32::from_rgb(100, 200, 255), format!("T = {:.2}s", chain.trigger_time));

                        if chain.is_circular {
                            ui.colored_label(Color32::from_rgb(255, 75, 75), "❌ CIRCULAR LOOP HAZARD");
                        } else {
                            ui.colored_label(Color32::from_rgb(80, 240, 120), "✔ Cascading OK");
                        }
                        ui.end_row();
                    }
                });
        });
    }
}

fn render_round_robin_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("RoundRobin Cyclic Step Trigger Sequencers (Engine.RoundRobin)").strong().size(13.0));
    ui.label(RichText::new("Step-by-step trigger routing: each trigger event advances execution to the next OutEvent slot.").size(11.0).color(Color32::GRAY));
    ui.add_space(4.0);

    let round_robins = state.get_round_robin_sequences();

    ui.horizontal(|ui| {
        if ui.button("📍 Place New RoundRobin at Pivot").clicked() {
            let new_idx = state.place_round_robin_at_pivot(true);
            if let Some(ref l) = state.loaded_unr {
                if l.actors.get(new_idx).is_some() {
                    state.selected_actor_index = Some(new_idx);
                }
            }
        }

        ui.label(format!("Found: {} RoundRobin actor(s)", round_robins.len()));
    });

    ui.separator();

    if round_robins.is_empty() {
        ui.group(|ui| {
            ui.label(RichText::new("No RoundRobin actors currently in the map.").color(Color32::from_rgb(200, 200, 200)));
            ui.add_space(4.0);
            ui.label("Click 'Place New RoundRobin at Pivot' to insert a RoundRobin actor. Unreal Tournament uses RoundRobin for:");
            ui.label(" • Multi-destination cycling teleporters");
            ui.label(" • Cycling security camera vantage monitors");
            ui.label(" • 3-stage switch puzzles & sequential door unlocking");
        });
        return;
    }

    // State for selected RoundRobin
    let rr_sel_id = egui::Id::new("selected_round_robin_name");
    let mut selected_rr_name = ui.ctx().data_mut(|d| {
        d.get_temp::<String>(rr_sel_id)
            .unwrap_or_else(|| round_robins[0].name.clone())
    });

    if !round_robins.iter().any(|r| r.name == selected_rr_name) {
        selected_rr_name = round_robins[0].name.clone();
    }

    let active_rr = round_robins.iter().find(|r| r.name == selected_rr_name).cloned();

    ui.horizontal(|ui| {
        ui.label(RichText::new("Select RoundRobin:").strong());
        egui::ComboBox::from_id_salt("round_robin_combo")
            .selected_text(&selected_rr_name)
            .show_ui(ui, |ui| {
                for rr in &round_robins {
                    let is_sel = selected_rr_name == rr.name;
                    if ui.selectable_label(is_sel, &rr.name).clicked() {
                        selected_rr_name = rr.name.clone();
                    }
                }
            });

        if let Some(ref rr) = active_rr {
            ui.separator();
            ui.label(format!("Tag: '{}'", if rr.tag.is_empty() { "(None)" } else { &rr.tag }));
            ui.separator();
            let mut b_loop = rr.b_loop;
            if ui.checkbox(&mut b_loop, "Loop (bLoop)").changed() {
                state.update_round_robin_loop(&rr.name, b_loop);
            }
        }
    });

    ui.ctx().data_mut(|d| d.insert_temp(rr_sel_id, selected_rr_name.clone()));
    ui.add_space(6.0);

    if let Some(rr) = active_rr {
        // Simulation / Trigger testing
        let sim_step_id = egui::Id::new(format!("rr_sim_step_{}", rr.name));
        let mut sim_step = ui.ctx().data_mut(|d| d.get_temp::<usize>(sim_step_id).unwrap_or(0));

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new("Trigger Simulation:").strong());
                let total_events = rr.out_events.len();
                let current_display = if total_events == 0 {
                    "0 / 0".to_string()
                } else {
                    format!("Slot #{}/{}", sim_step % total_events.max(1), total_events)
                };
                ui.colored_label(Color32::from_rgb(100, 220, 255), current_display);

                if ui.button("🔀 Simulate Trigger Pulse (Next Step)").clicked() && !rr.out_events.is_empty() {
                    let evt = &rr.out_events[sim_step % rr.out_events.len()];
                    state.log(format!("RoundRobin '{}' fired event: '{}'", rr.name, evt));
                    sim_step = if rr.b_loop {
                        (sim_step + 1) % rr.out_events.len()
                    } else {
                        (sim_step + 1).min(rr.out_events.len())
                    };
                    ui.ctx().data_mut(|d| d.insert_temp(sim_step_id, sim_step));
                }

                if ui.button("↺ Reset Simulation").clicked() {
                    sim_step = 0;
                    ui.ctx().data_mut(|d| d.insert_temp(sim_step_id, sim_step));
                }
            });
        });

        ui.add_space(6.0);

        // Grid of slots 0..15
        ui.label(RichText::new("OutEvents Execution Sequence (Slots 0..15)").strong());
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            egui::Grid::new("round_robin_slots_grid")
                .striped(true)
                .min_col_width(70.0)
                .spacing([10.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Slot").strong());
                    ui.label(RichText::new("OutEvent Name").strong());
                    ui.label(RichText::new("Target Receivers").strong());
                    ui.label(RichText::new("Status").strong());
                    ui.end_row();

                    // Retrieve existing properties from the actor for 16 slots
                    let existing_events: Vec<String> = if let Some(ref l) = state.loaded_unr {
                        if let Some(act) = l.actors.iter().find(|a| a.name == rr.name) {
                            (0..16).map(|i| {
                                let key = format!("OutEvents[{}]", i);
                                act.properties.get(&key).or_else(|| {
                                    if i == 0 { act.properties.get("OutEvents") } else { None }
                                }).and_then(|p| match &p.value {
                                    ut99_format::TaggedValue::Str(s) => Some(s.clone()),
                                    _ => None,
                                }).unwrap_or_default()
                            }).collect()
                        } else {
                            vec![String::new(); 16]
                        }
                    } else {
                        vec![String::new(); 16]
                    };

                    for i in 0..16 {
                        let is_active_step = !rr.out_events.is_empty() && (sim_step % rr.out_events.len() == i);
                        let slot_label = if is_active_step {
                            RichText::new(format!("▶ #{:02}", i)).strong().color(Color32::from_rgb(255, 210, 40))
                        } else {
                            RichText::new(format!("#{:02}", i))
                        };
                        ui.label(slot_label);

                        let mut event_str = existing_events.get(i).cloned().unwrap_or_default();
                        if ui.text_edit_singleline(&mut event_str).changed() {
                            state.update_round_robin_event(&rr.name, i, &event_str);
                        }

                        // Target count
                        if event_str.is_empty() || event_str.eq_ignore_ascii_case("None") {
                            ui.label(RichText::new("—").color(Color32::DARK_GRAY));
                            ui.colored_label(Color32::GRAY, "(Empty slot)");
                        } else {
                            let count = state.loaded_unr.as_ref().map(|l| {
                                l.actors.iter().filter(|a| {
                                    a.tag.eq_ignore_ascii_case(&event_str) || a.properties.get("Tag").map(|p| match &p.value {
                                        ut99_format::TaggedValue::Str(s) => s.eq_ignore_ascii_case(&event_str),
                                        _ => false,
                                    }).unwrap_or(false)
                                }).count()
                            }).unwrap_or(0);

                            if count == 0 {
                                ui.colored_label(Color32::from_rgb(255, 100, 100), "0 actors");
                                ui.colored_label(Color32::from_rgb(255, 100, 100), "⚠ No target");
                            } else {
                                ui.colored_label(Color32::from_rgb(80, 240, 120), format!("{} receiver(s)", count));
                                ui.colored_label(Color32::from_rgb(80, 240, 120), "✔ Linked");
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    }
}
