use super::*;

impl EditorState {
    // ==========================================
    // Dispatcher Sequence & Timeline
    // ==========================================
    pub fn get_dispatcher_sequences(&self) -> Vec<DispatcherSequence> {
        let Some(ref level) = self.loaded_unr else {
            return Vec::new();
        };
        if !level.actors.iter().any(|a| a.class_name.contains("Dispatcher")) {
            return Vec::new();
        }

        let mut sequences = Vec::new();
        {
            let mut actor_triplets = Vec::new();
            for a in &level.actors {
                let tag = if !a.tag.is_empty() {
                    a.tag.clone()
                } else {
                    a.properties.get("Tag").and_then(|p| match &p.value {
                        TaggedValue::Str(s) => Some(s.clone()),
                        _ => None,
                    }).unwrap_or_default()
                };
                actor_triplets.push((a.name.clone(), tag, a.location));
            }
            for m in &self.movers {
                actor_triplets.push((m.name.clone(), m.tag.clone(), m.brush.location));
            }

            for actor in &level.actors {
                if actor.class_name.contains("Dispatcher") {
                    let tag = if !actor.tag.is_empty() {
                        actor.tag.clone()
                    } else {
                        actor.properties.get("Tag").and_then(|p| match &p.value {
                            TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        }).unwrap_or_default()
                    };

                    let mut out_delays = Vec::new();
                    let mut out_events = Vec::new();

                    for i in 0..8 {
                        let delay_key = format!("OutDelays[{}]", i);
                        let delay = actor.properties.get(&delay_key).or_else(|| {
                            if i == 0 { actor.properties.get("OutDelays") } else { None }
                        }).and_then(|p| match &p.value {
                            TaggedValue::Float(f) => Some(*f),
                            TaggedValue::Int(val) => Some(*val as f32),
                            _ => None,
                        }).unwrap_or(0.0);
                        out_delays.push(delay);

                        let event_key = format!("OutEvents[{}]", i);
                        let event = actor.properties.get(&event_key).or_else(|| {
                            if i == 0 { actor.properties.get("OutEvents") } else { None }
                        }).and_then(|p| match &p.value {
                            TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        }).unwrap_or_default();
                        out_events.push(event);
                    }

                    sequences.push(build_dispatcher_sequence(
                        &actor.name,
                        &tag,
                        actor.location,
                        &out_delays,
                        &out_events,
                        &actor_triplets,
                    ));
                }
            }
        }
        sequences
    }

    /// Updates a single stage (0..7) in a Dispatcher actor's timeline with atomic undo/redo
    pub fn update_dispatcher_stage(
        &mut self,
        dispatcher_name: &str,
        stage_idx: usize,
        delay: f32,
        event: &str,
    ) -> bool {
        if stage_idx >= 8 {
            return false;
        }

        let mut old_actor_opt = None;
        let mut new_actor_opt = None;

        if let Some(ref mut level) = self.loaded_unr {
            if let Some(actor) = level.actors.iter_mut().find(|a| a.name == dispatcher_name) {
                old_actor_opt = Some(actor.clone());

                let delay_key = format!("OutDelays[{}]", stage_idx);
                let event_key = format!("OutEvents[{}]", stage_idx);

                actor.properties.insert(
                    delay_key,
                    TaggedProperty {
                        name: "OutDelays".to_string(),
                        array_index: stage_idx as i32,
                        value: TaggedValue::Float(delay.max(0.0)),
                    },
                );

                actor.properties.insert(
                    event_key,
                    TaggedProperty {
                        name: "OutEvents".to_string(),
                        array_index: stage_idx as i32,
                        value: TaggedValue::Str(event.to_string()),
                    },
                );

                new_actor_opt = Some(actor.clone());
            }
        }

        if let (Some(old_actor), Some(new_actor)) = (old_actor_opt, new_actor_opt) {
            self.history.push(Action::UpdateDispatcherSequence {
                actor_name: dispatcher_name.to_string(),
                old_actor,
                new_actor,
                description: format!("Updated stage #{} (Δt={:.2}s, event='{}')", stage_idx, delay, event),
            });
            self.map_is_dirty = true;
            self.log(format!("Updated Dispatcher '{}' stage #{} (Δt={:.2}s -> '{}')", dispatcher_name, stage_idx, delay, event));
            true
        } else {
            false
        }
    }

    /// Applies a canonical scripted sequence template to the specified Dispatcher with atomic undo/redo
    pub fn apply_dispatcher_preset(
        &mut self,
        dispatcher_name: &str,
        preset: DispatcherPresetTemplate,
    ) -> bool {
        let mut old_actor_opt = None;
        let mut new_actor_opt = None;

        if let Some(ref mut level) = self.loaded_unr {
            if let Some(actor) = level.actors.iter_mut().find(|a| a.name == dispatcher_name) {
                old_actor_opt = Some(actor.clone());

                let stages = preset.stages();
                for i in 0..8 {
                    let delay_key = format!("OutDelays[{}]", i);
                    let event_key = format!("OutEvents[{}]", i);

                    if i < stages.len() {
                        let (delay, event) = stages[i];
                        actor.properties.insert(
                            delay_key,
                            TaggedProperty {
                                name: "OutDelays".to_string(),
                                array_index: i as i32,
                                value: TaggedValue::Float(delay),
                            },
                        );
                        actor.properties.insert(
                            event_key,
                            TaggedProperty {
                                name: "OutEvents".to_string(),
                                array_index: i as i32,
                                value: TaggedValue::Str(event.to_string()),
                            },
                        );
                    } else {
                        actor.properties.insert(
                            delay_key,
                            TaggedProperty {
                                name: "OutDelays".to_string(),
                                array_index: i as i32,
                                value: TaggedValue::Float(0.0),
                            },
                        );
                        actor.properties.insert(
                            event_key,
                            TaggedProperty {
                                name: "OutEvents".to_string(),
                                array_index: i as i32,
                                value: TaggedValue::Str(String::new()),
                            },
                        );
                    }
                }

                new_actor_opt = Some(actor.clone());
            }
        }

        if let (Some(old_actor), Some(new_actor)) = (old_actor_opt, new_actor_opt) {
            self.history.push(Action::UpdateDispatcherSequence {
                actor_name: dispatcher_name.to_string(),
                old_actor,
                new_actor,
                description: format!("Applied preset '{}'", preset.label()),
            });
            self.map_is_dirty = true;
            self.log(format!("Applied preset '{}' to Dispatcher '{}'", preset.label(), dispatcher_name));
            true
        } else {
            false
        }
    }

    /// Spawns a new Dispatcher actor at the builder brush pivot with an optional sequence template
    pub fn place_dispatcher_at_pivot(&mut self, preset: Option<DispatcherPresetTemplate>) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("Dispatcher_{}", count);
        let tag = format!("TriggerSequence_{}", count);

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "Dispatcher".to_string();
        actor.name = name.clone();
        actor.tag = tag;
        actor.location = spawn_pos;

        if let Some(template) = preset {
            let stages = template.stages();
            for i in 0..8 {
                let delay_key = format!("OutDelays[{}]", i);
                let event_key = format!("OutEvents[{}]", i);
                if i < stages.len() {
                    let (delay, event) = stages[i];
                    actor.properties.insert(
                        delay_key,
                        TaggedProperty {
                            name: "OutDelays".to_string(),
                            array_index: i as i32,
                            value: TaggedValue::Float(delay),
                        },
                    );
                    actor.properties.insert(
                        event_key,
                        TaggedProperty {
                            name: "OutEvents".to_string(),
                            array_index: i as i32,
                            value: TaggedValue::Str(event.to_string()),
                        },
                    );
                }
            }
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let new_idx = if let Some(ref mut level) = self.loaded_unr {
            level.actors.push(actor.clone());
            level.actors.len() - 1
        } else {
            0
        };

        self.history.push(Action::AddActor {
            actor: actor.clone(),
        });

        self.selected_actor_index = Some(new_idx);
        self.selected_dispatcher_name = Some(name.clone());
        self.dispatcher_settings.selected_dispatcher_name = Some(name.clone());

        self.map_is_dirty = true;
        self.log(format!("Spawned Dispatcher '{name}' at ({:.1}, {:.1}, {:.1})", spawn_pos.x, spawn_pos.y, spawn_pos.z));
        new_idx
    }

    /// Evaluates cascading multi-dispatcher trigger chains and circular dependencies across the level
    pub fn detect_cascading_dispatcher_chains(&self) -> Vec<DispatcherChainLink> {
        let sequences = self.get_dispatcher_sequences();
        detect_cascading_dispatcher_chains(&sequences)
    }

    /// Advances the interactive timeline simulation clock for Dispatcher scrubbing and live preview
    pub fn update_dispatcher_timeline(&mut self, dt: f32) {
        if !self.dispatcher_settings.is_playing {
            return;
        }

        let sequences = self.get_dispatcher_sequences();
        let target_name = self.dispatcher_settings.selected_dispatcher_name.as_deref()
            .or(self.selected_dispatcher_name.as_deref());

        let max_duration = if let Some(name) = target_name {
            sequences.iter().find(|s| s.dispatcher_name == name)
                .map(|s| s.total_duration_secs)
                .unwrap_or(5.0)
        } else {
            5.0
        }.max(0.5);

        self.dispatcher_settings.timeline_time += dt * self.dispatcher_settings.play_speed;
        if self.dispatcher_settings.timeline_time >= max_duration {
            if self.dispatcher_settings.loop_preview {
                self.dispatcher_settings.timeline_time %= max_duration;
            } else {
                self.dispatcher_settings.timeline_time = max_duration;
                self.dispatcher_settings.is_playing = false;
            }
        }
    }

    // ==========================================
    // RoundRobin Sequencer (Engine.RoundRobin)
    // ==========================================
    pub fn get_round_robin_sequences(&self) -> Vec<RoundRobinSequence> {
        let mut result = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            for actor in &level.actors {
                if actor.class_name.contains("RoundRobin") {
                    let tag = if !actor.tag.is_empty() {
                        actor.tag.clone()
                    } else {
                        actor.properties.get("Tag").and_then(|p| match &p.value {
                            TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        }).unwrap_or_default()
                    };

                    let b_loop = actor.properties.get("bLoop").and_then(|p| match &p.value {
                        TaggedValue::Bool(b) => Some(*b),
                        TaggedValue::Byte(b) => Some(*b != 0),
                        TaggedValue::Int(i) => Some(*i != 0),
                        _ => None,
                    }).unwrap_or(true);

                    let mut out_events = Vec::new();
                    for i in 0..16 {
                        let key = format!("OutEvents[{}]", i);
                        let ev = actor.properties.get(&key).or_else(|| {
                            if i == 0 { actor.properties.get("OutEvents") } else { None }
                        }).and_then(|p| match &p.value {
                            TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        }).unwrap_or_default();
                        if !ev.is_empty() && !ev.eq_ignore_ascii_case("None") {
                            out_events.push(ev);
                        }
                    }

                    result.push(RoundRobinSequence::new(
                        &actor.name,
                        &tag,
                        out_events,
                        b_loop,
                    ));
                }
            }
        }
        result
    }

    pub fn place_round_robin_at_pivot(&mut self, b_loop: bool) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("RoundRobin_{}", count);
        let tag = format!("StepSequence_{}", count);

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "RoundRobin".to_string();
        actor.name = name.clone();
        actor.tag = tag;
        actor.location = spawn_pos;
        actor.properties.insert(
            "bLoop".to_string(),
            TaggedProperty {
                name: "bLoop".to_string(),
                array_index: 0,
                value: TaggedValue::Bool(b_loop),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let new_idx = if let Some(ref mut level) = self.loaded_unr {
            level.actors.push(actor.clone());
            level.actors.len() - 1
        } else {
            0
        };

        self.history.push(Action::AddActor {
            actor: actor.clone(),
        });

        self.selected_actor_index = Some(new_idx);
        self.map_is_dirty = true;
        self.log(format!("Spawned RoundRobin '{name}' at ({:.1}, {:.1}, {:.1})", spawn_pos.x, spawn_pos.y, spawn_pos.z));
        new_idx
    }

    pub fn update_round_robin_event(
        &mut self,
        rr_name: &str,
        slot_idx: usize,
        event: &str,
    ) -> bool {
        if slot_idx >= 16 {
            return false;
        }

        let mut old_actor_opt = None;
        let mut new_actor_opt = None;

        if let Some(ref mut level) = self.loaded_unr {
            if let Some(actor) = level.actors.iter_mut().find(|a| a.name == rr_name) {
                old_actor_opt = Some(actor.clone());

                let event_key = format!("OutEvents[{}]", slot_idx);
                actor.properties.insert(
                    event_key,
                    TaggedProperty {
                        name: "OutEvents".to_string(),
                        array_index: slot_idx as i32,
                        value: TaggedValue::Str(event.to_string()),
                    },
                );

                new_actor_opt = Some(actor.clone());
            }
        }

        if let (Some(old_actor), Some(new_actor)) = (old_actor_opt, new_actor_opt) {
            self.history.push(Action::UpdateDispatcherSequence {
                actor_name: rr_name.to_string(),
                old_actor,
                new_actor,
                description: format!("Updated RoundRobin slot #{} (event='{}')", slot_idx, event),
            });
            self.map_is_dirty = true;
            self.log(format!("Updated RoundRobin '{}' slot #{} -> '{}'", rr_name, slot_idx, event));
            true
        } else {
            false
        }
    }

    pub fn update_round_robin_loop(
        &mut self,
        rr_name: &str,
        b_loop: bool,
    ) -> bool {
        let mut old_actor_opt = None;
        let mut new_actor_opt = None;

        if let Some(ref mut level) = self.loaded_unr {
            if let Some(actor) = level.actors.iter_mut().find(|a| a.name == rr_name) {
                old_actor_opt = Some(actor.clone());

                actor.properties.insert(
                    "bLoop".to_string(),
                    TaggedProperty {
                        name: "bLoop".to_string(),
                        array_index: 0,
                        value: TaggedValue::Bool(b_loop),
                    },
                );

                new_actor_opt = Some(actor.clone());
            }
        }

        if let (Some(old_actor), Some(new_actor)) = (old_actor_opt, new_actor_opt) {
            self.history.push(Action::UpdateDispatcherSequence {
                actor_name: rr_name.to_string(),
                old_actor,
                new_actor,
                description: format!("Updated RoundRobin bLoop={}", b_loop),
            });
            self.map_is_dirty = true;
            true
        } else {
            false
        }
    }
}
