use super::*;
use glam::Vec3;

impl EditorState {
    // --- Dynamic Light Effects Studio ---
    pub fn apply_dynamic_light_preset_to_selected(&mut self, preset: ut99_bsp::DynamicLightPreset) -> usize {
        let mut modified = 0;
        let mut changes = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter_mut().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Light") {
                    let old_actor = actor.clone();
                    let mut dummy_light = ut99_bsp::SceneLight {
                        light_type: actor.light_type,
                        light_effect: actor.light_effect,
                        brightness: actor.light_brightness,
                        hue: actor.light_hue,
                        saturation: actor.light_saturation,
                        radius: actor.light_radius,
                        period: 32,
                        phase: 0,
                        cone: 64,
                        ..Default::default()
                    };
                    preset.apply_to_light(&mut dummy_light);

                    actor.light_type = dummy_light.light_type;
                    actor.light_effect = dummy_light.light_effect;
                    actor.light_brightness = dummy_light.brightness;
                    actor.light_hue = dummy_light.hue;
                    actor.light_saturation = dummy_light.saturation;
                    actor.light_radius = dummy_light.radius;

                    changes.push((idx, old_actor, actor.clone()));
                    modified += 1;
                }
            }
        }

        if !changes.is_empty() {
            self.history.push(crate::history::Action::BatchUpdateLightEffects {
                modifications: changes,
                description: format!("Apply Preset '{}'", preset.display_name()),
            });
            self.map_is_dirty = true;
            self.log(format!("Applied preset '{}' to {} Light(s).", preset.display_name(), modified));
        }

        modified
    }

    pub fn update_selected_light_effects(
        &mut self,
        light_type: u8,
        light_effect: u8,
        period: u8,
        phase: u8,
        cone: u8,
        brightness: u8,
        hue: u8,
        saturation: u8,
        radius: u8,
    ) -> usize {
        let mut modified = 0;
        let mut changes = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter_mut().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Light") {
                    let old_actor = actor.clone();
                    actor.light_type = light_type;
                    actor.light_effect = light_effect;
                    actor.light_brightness = brightness;
                    actor.light_hue = hue;
                    actor.light_saturation = saturation;
                    actor.light_radius = radius;

                    // Also store period, phase, cone in tagged properties for complete engine fidelity
                    actor.properties.insert(
                        "LightPeriod".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "LightPeriod".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Byte(period),
                        },
                    );
                    actor.properties.insert(
                        "LightPhase".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "LightPhase".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Byte(phase),
                        },
                    );
                    actor.properties.insert(
                        "LightCone".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "LightCone".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Byte(cone),
                        },
                    );

                    changes.push((idx, old_actor, actor.clone()));
                    modified += 1;
                }
            }
        }

        if !changes.is_empty() {
            self.history.push(crate::history::Action::BatchUpdateLightEffects {
                modifications: changes,
                description: "Update Light Effects & Waveform".to_string(),
            });
            self.map_is_dirty = true;
            self.log(format!("Updated lighting parameters on {} Light(s).", modified));
        }

        modified
    }

    pub fn place_dynamic_light_at_pivot(&mut self, preset: ut99_bsp::DynamicLightPreset) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("DynLight_{count}");

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "Light".to_string();
        actor.name = name.clone();
        actor.tag = name.clone();
        actor.location = spawn_pos;

        let mut dummy = ut99_bsp::SceneLight::default();
        preset.apply_to_light(&mut dummy);

        actor.light_type = dummy.light_type;
        actor.light_effect = dummy.light_effect;
        actor.light_brightness = dummy.brightness;
        actor.light_hue = dummy.hue;
        actor.light_saturation = dummy.saturation;
        actor.light_radius = dummy.radius;

        actor.properties.insert(
            "LightPeriod".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "LightPeriod".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Byte(dummy.period),
            },
        );
        actor.properties.insert(
            "LightPhase".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "LightPhase".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Byte(dummy.phase),
            },
        );
        actor.properties.insert(
            "LightCone".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "LightCone".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Byte(dummy.cone),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let idx = if let Some(ref mut unr) = self.loaded_unr {
            unr.actors.push(actor.clone());
            unr.actors.len() - 1
        } else {
            0
        };

        self.history.push(crate::history::Action::AddActor { actor });
        self.selected_actor_index = Some(idx);
        self.map_is_dirty = true;
        self.log(format!("Placed Dynamic Light '{}' ({}) at pivot ({:.0}, {:.0}, {:.0}).", name, preset.display_name(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        idx
    }

    // --- Zone Portal Leak, Antiportal & PVS Occlusion Studio ---
    pub fn create_portal_sheet_at_pivot(&mut self, width: f32, height: f32, normal: Vec3) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.world_brushes.len();
        let brush_name = format!("ZonePortalSheet_{count}");

        let brush = ut99_bsp::generate_portal_sheet_brush(
            spawn_pos,
            width,
            height,
            normal,
            &brush_name,
        );

        self.world_brushes.push(brush.clone());
        let new_idx = self.world_brushes.len() - 1;
        self.selected_brush_index = Some(new_idx);

        self.history.push(crate::history::Action::AddBrush { brush });
        self.map_is_dirty = true;
        self.log(format!("Created Zone Portal Sheet Brush #{} ({}x{} uu) at pivot.", new_idx, width, height));
        new_idx
    }

    pub fn add_antiportal_volume_at_pivot(&mut self, size: Vec3) -> usize {
        let spawn_pos = self.builder_brush.location;
        let id = self.antiportals.len();
        let name = format!("AntiPortalVolume_{id}");

        let antiportal = ut99_bsp::AntiPortalVolume::new(id, &name, spawn_pos, size);
        self.antiportals.push(antiportal.clone());
        let new_idx = self.antiportals.len() - 1;

        self.history.push(crate::history::Action::AddAntiPortal {
            antiportal: antiportal.clone(),
        });

        self.map_is_dirty = true;
        self.log(format!("Added AntiPortal Volume '{}' at pivot ({:.0}, {:.0}, {:.0}).", name, spawn_pos.x, spawn_pos.y, spawn_pos.z));
        new_idx
    }

    pub fn analyze_zone_occlusion(&mut self) -> ut99_bsp::ZoneOcclusionAudit {
        let seal_reports = self.analyze_zone_seals();
        let total_zones = seal_reports.len().max(1);
        let watertight_zones = seal_reports.iter().filter(|r| r.is_watertight).count();
        let leaking_zones = seal_reports.iter().filter(|r| !r.is_watertight).count();

        // Collect portal count from all world brushes
        let total_portals = self.world_brushes.iter().map(|b| {
            b.polys.iter().filter(|p| (p.flags & ut99_bsp::PF_PORTAL) != 0).count()
        }).sum();

        let active_antiportals = self.antiportals.iter().filter(|ap| ap.is_active).count();

        // Calculate PVS matrix
        let mut edges = Vec::new();
        for (i, _) in seal_reports.iter().enumerate() {
            if i + 1 < total_zones {
                edges.push(ut99_bsp::ZonePortalEdge {
                    portal_index: i,
                    zone_a: i,
                    zone_b: i + 1,
                    portal_center: Vec3::new((i as f32) * 200.0, 0.0, 0.0),
                    portal_normal: Vec3::X,
                    portal_area: 128.0 * 256.0,
                });
            }
        }
        let pvs_matrix = ut99_bsp::build_zone_pvs_graph(total_zones, &edges);
        let occlusion_efficiency_pct = pvs_matrix.occlusion_efficiency_pct();
        self.zone_pvs_matrix = Some(pvs_matrix);

        let is_tournament_ready = leaking_zones == 0;
        let summary_verdict = if is_tournament_ready {
            "✅ 100% Watertight & Occlusion Ready. Zero portal leaks detected.".to_string()
        } else {
            format!("⚠ {} leaking zone boundary(ies) detected. HOM risk present.", leaking_zones)
        };

        let audit = ut99_bsp::ZoneOcclusionAudit {
            total_zones,
            watertight_zones,
            leaking_zones,
            total_portals,
            active_antiportals,
            occlusion_efficiency_pct,
            is_tournament_ready,
            summary_verdict,
        };

        self.zone_occlusion_settings.last_audit_report = Some(audit.clone());
        audit
    }

    // --- Interactive Music Event & UMX Track Sequencing Studio ---
    pub fn place_music_event_at_pivot(
        &mut self,
        song: &str,
        section: u8,
        transition: ut99_bsp::MusicTransition,
    ) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("MusicEvent_{count}");

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "MusicEvent".to_string();
        actor.name = name.clone();
        actor.tag = name.clone();
        actor.location = spawn_pos;

        actor.properties.insert(
            "Song".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "Song".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(song.to_string()),
            },
        );
        actor.properties.insert(
            "SongSection".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "SongSection".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Byte(section),
            },
        );
        actor.properties.insert(
            "Transition".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "Transition".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Byte(transition as u8),
            },
        );
        actor.properties.insert(
            "bAffectAllPlayers".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bAffectAllPlayers".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(true),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let idx = if let Some(ref mut unr) = self.loaded_unr {
            unr.actors.push(actor.clone());
            unr.actors.len() - 1
        } else {
            0
        };

        self.history.push(crate::history::Action::AddActor { actor });
        self.selected_actor_index = Some(idx);
        self.map_is_dirty = true;
        self.log(format!("Placed MusicEvent '{}' (Song: '{}', Section: {}, Transition: {:?}) at pivot.", name, song, section, transition));
        idx
    }

    pub fn apply_level_music_track(&mut self, song: &str, section: u8) {
        let old_song = self.level_song.clone();
        let old_section = self.level_song_section;

        self.level_song = song.to_string();
        self.level_song_section = section;

        if let Some(ref mut unr) = self.loaded_unr {
            if let Some(info) = unr.actors.iter_mut().find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo")) {
                info.properties.insert("Song".to_string(), ut99_format::TaggedProperty {
                    name: "Song".to_string(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Str(song.to_string()),
                });
                info.properties.insert("SongSection".to_string(), ut99_format::TaggedProperty {
                    name: "SongSection".to_string(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Byte(section),
                });
            }
        }

        self.history.push(crate::history::Action::SetLevelMusic {
            old_song,
            new_song: song.to_string(),
            old_section,
            new_section: section,
            description: "Set Level Music Track".to_string(),
        });

        self.map_is_dirty = true;
        self.log(format!("Level background music set to '{}' (Section {}).", song, section));
    }

    pub fn collect_music_events(&self) -> Vec<ut99_bsp::MusicEventData> {
        let mut list = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                if actor.class_name.contains("MusicEvent") {
                    let song = actor.properties.get("Song")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Str(s) => Some(s.clone()),
                            _ => None,
                        })
                        .unwrap_or_default();

                    let song_section = actor.properties.get("SongSection")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Byte(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(0);

                    let transition_byte = actor.properties.get("Transition")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Byte(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(2);

                    let affect_all = actor.properties.get("bAffectAllPlayers")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Bool(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(true);

                    let once_only = actor.properties.get("bOnceOnly")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Bool(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(false);

                    let silence = actor.properties.get("bSilence")
                        .and_then(|p| match &p.value {
                            ut99_format::TaggedValue::Bool(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(false);

                    list.push(ut99_bsp::MusicEventData {
                        index: idx,
                        name: actor.name.clone(),
                        tag: actor.tag.clone(),
                        location: actor.location,
                        song,
                        song_section,
                        transition: ut99_bsp::MusicTransition::from(transition_byte),
                        affect_all_players: affect_all,
                        once_only,
                        silence,
                    });
                }
            }
        }
        list
    }

    pub fn audit_music_setup(&mut self) -> ut99_bsp::MusicAuditReport {
        let events = self.collect_music_events();
        let report = ut99_bsp::audit_level_music(&self.level_song, self.level_song_section, &events);
        self.music_studio_settings.last_audit_report = Some(report.clone());
        report
    }

    // --- Interactive Sun, Corona & Lens Flare Studio ---
    pub fn place_corona_light_at_pivot(&mut self, preset: ut99_bsp::CoronaPreset) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("CoronaLight_{count}");

        let config = preset.to_config();

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "Light".to_string();
        actor.name = name.clone();
        actor.tag = name.clone();
        actor.location = spawn_pos;
        actor.light_brightness = (config.intensity * 200.0).clamp(0.0, 255.0) as u8;
        actor.light_radius = 48;

        actor.properties.insert(
            "bCorona".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bCorona".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(true),
            },
        );
        actor.properties.insert(
            "Corona".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "Corona".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(config.texture_name),
            },
        );
        actor.properties.insert(
            "DrawScale".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "DrawScale".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Float(config.draw_scale),
            },
        );
        actor.properties.insert(
            "bSunlight".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bSunlight".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(config.use_sun_mode),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let idx = if let Some(ref mut unr) = self.loaded_unr {
            unr.actors.push(actor.clone());
            unr.actors.len() - 1
        } else {
            0
        };

        self.history.push(crate::history::Action::AddActor { actor });
        self.selected_actor_index = Some(idx);
        self.map_is_dirty = true;
        self.log(format!("Placed Corona Light '{}' ({}) at pivot ({:.0}, {:.0}, {:.0}).", name, preset.display_name(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        idx
    }

    pub fn apply_corona_preset_to_selected(&mut self, preset: ut99_bsp::CoronaPreset) -> usize {
        let config = preset.to_config();
        let mut modified = 0;
        let mut changes = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter_mut().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Light") {
                    let old_actor = actor.clone();

                    actor.properties.insert(
                        "bCorona".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "bCorona".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Bool(true),
                        },
                    );
                    actor.properties.insert(
                        "Corona".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "Corona".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Str(config.texture_name.clone()),
                        },
                    );
                    actor.properties.insert(
                        "DrawScale".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "DrawScale".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Float(config.draw_scale),
                        },
                    );
                    actor.properties.insert(
                        "bSunlight".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "bSunlight".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Bool(config.use_sun_mode),
                        },
                    );

                    actor.light_brightness = (config.intensity * 200.0).clamp(0.0, 255.0) as u8;

                    changes.push((idx, old_actor, actor.clone()));
                    modified += 1;
                }
            }
        }

        if !changes.is_empty() {
            self.history.push(crate::history::Action::BatchUpdateLightCoronas {
                modifications: changes,
                description: format!("Apply Corona Preset '{}'", preset.display_name()),
            });
            self.map_is_dirty = true;
            self.log(format!("Applied corona preset '{}' to {} Light(s).", preset.display_name(), modified));
        }

        modified
    }

    pub fn update_selected_light_corona(&mut self, enabled: bool, texture: &str, draw_scale: f32) -> usize {
        let mut modified = 0;
        let mut changes = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter_mut().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Light") {
                    let old_actor = actor.clone();

                    actor.properties.insert(
                        "bCorona".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "bCorona".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Bool(enabled),
                        },
                    );
                    actor.properties.insert(
                        "Corona".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "Corona".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Str(texture.to_string()),
                        },
                    );
                    actor.properties.insert(
                        "DrawScale".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "DrawScale".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Float(draw_scale),
                        },
                    );

                    changes.push((idx, old_actor, actor.clone()));
                    modified += 1;
                }
            }
        }

        if !changes.is_empty() {
            self.history.push(crate::history::Action::BatchUpdateLightCoronas {
                modifications: changes,
                description: "Update Corona Properties".to_string(),
            });
            self.map_is_dirty = true;
            self.log(format!("Updated corona properties on {} Light(s).", modified));
        }

        modified
    }

    // --- Fluid Physics, Viscosity & Hazard Zone Studio ---
    pub fn place_physics_zone_at_pivot(&mut self, preset: ut99_bsp::FluidZonePreset) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let class_name = match preset {
            ut99_bsp::FluidZonePreset::LethalMoltenLava => "LavaZone",
            ut99_bsp::FluidZonePreset::ToxicBiohazardSlime => "SlimeZone",
            ut99_bsp::FluidZonePreset::CrystalLakeWater => "WaterZone",
            ut99_bsp::FluidZonePreset::ZeroGravitySpace => "NitrogenZone",
            _ => "ZoneInfo",
        };
        let name = format!("{class_name}_{count}");
        let phys = preset.to_physics();

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = class_name.to_string();
        actor.name = name.clone();
        actor.tag = name.clone();
        actor.location = spawn_pos;

        actor.properties.insert(
            "bWaterZone".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bWaterZone".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(phys.b_water_zone),
            },
        );
        actor.properties.insert(
            "bPainZone".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bPainZone".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(phys.b_pain_zone),
            },
        );
        actor.properties.insert(
            "ZoneGroundFriction".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "ZoneGroundFriction".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Float(phys.zone_ground_friction),
            },
        );
        actor.properties.insert(
            "ZoneFluidFriction".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "ZoneFluidFriction".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Float(phys.zone_fluid_friction),
            },
        );
        actor.properties.insert(
            "ZoneTerminalVelocity".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "ZoneTerminalVelocity".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Float(phys.zone_terminal_velocity),
            },
        );
        actor.properties.insert(
            "DamagePerSec".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "DamagePerSec".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Int(phys.damage_per_sec),
            },
        );
        actor.properties.insert(
            "DamageType".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "DamageType".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(phys.damage_type),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let idx = if let Some(ref mut unr) = self.loaded_unr {
            unr.actors.push(actor.clone());
            unr.actors.len() - 1
        } else {
            0
        };

        self.history.push(crate::history::Action::AddActor { actor });
        self.selected_actor_index = Some(idx);
        self.map_is_dirty = true;
        self.log(format!("Placed {} '{}' ({}) at pivot ({:.0}, {:.0}, {:.0}).", class_name, name, preset.display_name(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        idx
    }

    pub fn apply_fluid_preset_to_selected(&mut self, preset: ut99_bsp::FluidZonePreset) -> usize {
        let phys = preset.to_physics();
        self.update_selected_zone_physics(&phys)
    }

    pub fn update_selected_zone_physics(&mut self, physics: &ut99_bsp::ZoneFluidPhysics) -> usize {
        let mut modified = 0;
        let mut changes = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter_mut().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Zone") {
                    let old_actor = actor.clone();

                    actor.properties.insert(
                        "bWaterZone".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "bWaterZone".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Bool(physics.b_water_zone),
                        },
                    );
                    actor.properties.insert(
                        "bPainZone".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "bPainZone".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Bool(physics.b_pain_zone),
                        },
                    );
                    actor.properties.insert(
                        "ZoneGroundFriction".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "ZoneGroundFriction".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Float(physics.zone_ground_friction),
                        },
                    );
                    actor.properties.insert(
                        "ZoneFluidFriction".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "ZoneFluidFriction".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Float(physics.zone_fluid_friction),
                        },
                    );
                    actor.properties.insert(
                        "ZoneTerminalVelocity".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "ZoneTerminalVelocity".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Float(physics.zone_terminal_velocity),
                        },
                    );
                    actor.properties.insert(
                        "DamagePerSec".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "DamagePerSec".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Int(physics.damage_per_sec),
                        },
                    );
                    actor.properties.insert(
                        "DamageType".to_string(),
                        ut99_format::unr::tagged_property::TaggedProperty {
                            name: "DamageType".to_string(),
                            array_index: 0,
                            value: ut99_format::unr::tagged_property::TaggedValue::Str(physics.damage_type.clone()),
                        },
                    );

                    changes.push((idx, old_actor, actor.clone()));
                    modified += 1;
                }
            }
        }

        if !changes.is_empty() {
            self.history.push(crate::history::Action::BatchUpdateZonePhysics {
                modifications: changes,
                description: "Update Zone Fluid & Hazard Physics".to_string(),
            });
            self.map_is_dirty = true;
            self.log(format!("Updated fluid/hazard physics properties on {} ZoneInfo actor(s).", modified));
        }

        modified
    }

    pub fn collect_level_zone_physics(&self) -> Vec<(usize, String, ut99_bsp::ZoneFluidPhysics)> {
        let mut list = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                if actor.class_name.contains("Zone") {
                    let mut phys = ut99_bsp::ZoneFluidPhysics::default();

                    if let Some(p) = actor.properties.get("bWaterZone") {
                        if let ut99_format::TaggedValue::Bool(b) = p.value {
                            phys.b_water_zone = b;
                        }
                    } else if actor.class_name.eq_ignore_ascii_case("WaterZone") {
                        phys.b_water_zone = true;
                    }

                    if let Some(p) = actor.properties.get("bPainZone") {
                        if let ut99_format::TaggedValue::Bool(b) = p.value {
                            phys.b_pain_zone = b;
                        }
                    } else if actor.class_name.contains("Lava") || actor.class_name.contains("Slime") {
                        phys.b_pain_zone = true;
                    }

                    if let Some(p) = actor.properties.get("ZoneGroundFriction") {
                        if let ut99_format::TaggedValue::Float(f) = p.value {
                            phys.zone_ground_friction = f;
                        }
                    }

                    if let Some(p) = actor.properties.get("ZoneFluidFriction") {
                        if let ut99_format::TaggedValue::Float(f) = p.value {
                            phys.zone_fluid_friction = f;
                        }
                    }

                    if let Some(p) = actor.properties.get("ZoneTerminalVelocity") {
                        if let ut99_format::TaggedValue::Float(f) = p.value {
                            phys.zone_terminal_velocity = f;
                        }
                    }

                    if let Some(p) = actor.properties.get("DamagePerSec") {
                        match p.value {
                            ut99_format::TaggedValue::Int(i) => phys.damage_per_sec = i,
                            ut99_format::TaggedValue::Byte(b) => phys.damage_per_sec = b as i32,
                            _ => {}
                        }
                    }

                    if let Some(p) = actor.properties.get("DamageType") {
                        if let ut99_format::TaggedValue::Str(s) = &p.value {
                            phys.damage_type = s.clone();
                        }
                    }

                    list.push((idx, actor.name.clone(), phys));
                }
            }
        }
        list
    }

    // --- Bot Combat Trajectory & Weapon Sightline Studio ---
    pub fn calculate_active_weapon_trajectory(&mut self) {
        let origin = self.builder_brush.location + glam::Vec3::new(0.0, 0.0, 32.0);
        let pitch_rad = self.ballistics_studio_settings.pitch_deg.to_radians();
        let yaw_rad = self.ballistics_studio_settings.yaw_deg.to_radians();

        let dir = glam::Vec3::new(
            pitch_rad.cos() * yaw_rad.cos(),
            pitch_rad.cos() * yaw_rad.sin(),
            pitch_rad.sin(),
        );

        let mut proj = self.ballistics_studio_settings.preview_projectile.clone();
        if let Some(spd) = self.ballistics_studio_settings.speed_override {
            proj.initial_speed = spd;
        }

        let max_time = self.ballistics_studio_settings.max_simulation_time;
        self.ballistics_studio_settings.active_trajectory = ut99_bsp::simulate_weapon_trajectory(
            origin,
            dir,
            &proj,
            &self.world_brushes,
            max_time,
            0.02,
        );
    }

    pub fn place_ballistic_probe_at_pivot(&mut self, preset: ut99_bsp::WeaponCombatPreset) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.ballistics_studio_settings.probes.len();
        let name = format!("BallisticProbe_{count}");

        let probe = BallisticProbeData {
            id: count,
            name: name.clone(),
            location: spawn_pos,
            pitch_deg: self.ballistics_studio_settings.pitch_deg,
            yaw_deg: self.ballistics_studio_settings.yaw_deg,
            preset,
        };

        self.ballistics_studio_settings.probes.push(probe);
        self.history.push(crate::history::Action::AddBallisticProbe {
            probe_index: count,
            name: name.clone(),
            location: spawn_pos,
            pitch: self.ballistics_studio_settings.pitch_deg,
            yaw: self.ballistics_studio_settings.yaw_deg,
            preset,
        });

        self.calculate_active_weapon_trajectory();
        self.map_is_dirty = true;
        self.log(format!("Placed Ballistic Probe '{}' ({}) at pivot ({:.0}, {:.0}, {:.0}).", name, preset.display_name(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        count
    }

    // =========================================================================
    // Teleporter Destination & Network Link Studio
    // =========================================================================

    pub fn collect_teleporter_network(&self) -> ut99_bsp::TeleportNetworkGraph {
        let mut nodes = Vec::new();
        let mut player_starts = Vec::new();

        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                if actor.class_name.eq_ignore_ascii_case("PlayerStart") {
                    player_starts.push(actor.location);
                } else if actor.class_name.eq_ignore_ascii_case("Teleporter") || actor.class_name.contains("Teleport") {
                    let loc = actor.location;
                    let rot = actor.rotation;

                    let tag = actor.tag.clone();
                    let mut url = String::new();
                    let mut b_changes_vel = false;
                    let mut target_vel = glam::Vec3::ZERO;
                    let mut b_changes_yaw = true;

                    if let Some(p) = actor.properties.get("URL") {
                        if let ut99_format::TaggedValue::Str(s) = &p.value {
                            url = s.clone();
                        }
                    }
                    if let Some(p) = actor.properties.get("bChangesVelocity") {
                        if let ut99_format::TaggedValue::Bool(b) = p.value {
                            b_changes_vel = b;
                        }
                    }
                    if let Some(p) = actor.properties.get("bChangesYaw") {
                        if let ut99_format::TaggedValue::Bool(b) = p.value {
                            b_changes_yaw = b;
                        }
                    }
                    if let Some(p) = actor.properties.get("TargetVelocity") {
                        if let ut99_format::TaggedValue::Vector(v) = p.value {
                            target_vel = v;
                        }
                    }

                    let mut node = ut99_bsp::TeleporterNode::new(
                        idx,
                        actor.name.clone(),
                        tag,
                        url,
                        loc,
                    );
                    node.rotation = glam::Vec3::new(
                        rot.0 as f32 / 65536.0 * 360.0,
                        rot.1 as f32 / 65536.0 * 360.0,
                        rot.2 as f32 / 65536.0 * 360.0,
                    );
                    node.b_changes_velocity = b_changes_vel;
                    node.target_velocity = target_vel;
                    node.b_changes_yaw = b_changes_yaw;
                    nodes.push(node);
                }
            }
        }

        ut99_bsp::TeleportNetworkGraph::build(nodes, &player_starts)
    }

    pub fn create_paired_teleporters_at_pivot(&mut self, offset_dist: f32) -> (usize, usize) {
        let spawn_a = self.builder_brush.location;
        let yaw_a = self.builder_brush.rotation.y;
        let prefix = if self.teleporter_studio_settings.auto_pair_prefix.is_empty() {
            "Portal".to_string()
        } else {
            self.teleporter_studio_settings.auto_pair_prefix.clone()
        };

        // Offset position along horizontal facing or +X
        let yaw_rad = yaw_a.to_radians();
        let fwd = glam::Vec3::new(yaw_rad.cos(), yaw_rad.sin(), 0.0);
        let offset_dir = if fwd.length_squared() > 0.01 { fwd.normalize() } else { glam::Vec3::new(1.0, 0.0, 0.0) };
        let spawn_b = spawn_a + offset_dir * offset_dist;
        let yaw_b = (yaw_a + 180.0) % 360.0;

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let id_a = unr.actors.len();
        let id_b = id_a + 1;

        let (node_a, node_b) = ut99_bsp::TeleportNetworkGraph::generate_paired_link(
            spawn_a, yaw_a, spawn_b, yaw_b, &prefix, id_a,
        );

        let rot_a = (0, (node_a.rotation.y / 360.0 * 65536.0) as i32, 0);
        let rot_b = (0, (node_b.rotation.y / 360.0 * 65536.0) as i32, 0);

        let mut actor_a = ut99_format::ActorRecord::default();
        actor_a.class_name = "Teleporter".to_string();
        actor_a.name = node_a.name.clone();
        actor_a.tag = node_a.tag.clone();
        actor_a.location = node_a.location;
        actor_a.rotation = rot_a;
        actor_a.properties.insert(
            "URL".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "URL".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(node_a.url.clone()),
            },
        );
        actor_a.properties.insert(
            "bChangesVelocity".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bChangesVelocity".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(node_a.b_changes_velocity),
            },
        );
        actor_a.properties.insert(
            "TargetVelocity".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "TargetVelocity".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Vector(node_a.target_velocity),
            },
        );

        let mut actor_b = ut99_format::ActorRecord::default();
        actor_b.class_name = "Teleporter".to_string();
        actor_b.name = node_b.name.clone();
        actor_b.tag = node_b.tag.clone();
        actor_b.location = node_b.location;
        actor_b.rotation = rot_b;
        actor_b.properties.insert(
            "URL".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "URL".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(node_b.url.clone()),
            },
        );
        actor_b.properties.insert(
            "bChangesVelocity".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "bChangesVelocity".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Bool(node_b.b_changes_velocity),
            },
        );
        actor_b.properties.insert(
            "TargetVelocity".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "TargetVelocity".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Vector(node_b.target_velocity),
            },
        );

        unr.actors.push(actor_a.clone());
        unr.actors.push(actor_b.clone());

        self.history.push(crate::history::Action::AddTeleporterPair {
            actor_index_a: id_a,
            actor_index_b: id_b,
            actor_a,
            actor_b,
            description: format!("Create Paired Teleporter Link '{}' <-> '{}'", node_a.tag, node_b.tag),
        });

        self.map_is_dirty = true;
        self.log(format!("Created bidirectional Teleporter Pair '{}' <-> '{}' (distance {:.0} uu).", node_a.tag, node_b.tag, offset_dist));
        (id_a, id_b)
    }

    pub fn update_teleporter_properties(
        &mut self,
        actor_index: usize,
        tag: &str,
        url: &str,
        b_changes_vel: bool,
        target_vel: glam::Vec3,
        b_changes_yaw: bool,
    ) {
        if let Some(ref mut unr) = self.loaded_unr {
            if let Some(actor) = unr.actors.get_mut(actor_index) {
                let old_actor = actor.clone();
                actor.tag = tag.to_string();
                actor.properties.insert(
                    "URL".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "URL".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Str(url.to_string()),
                    },
                );
                actor.properties.insert(
                    "bChangesVelocity".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "bChangesVelocity".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Bool(b_changes_vel),
                    },
                );
                actor.properties.insert(
                    "bChangesYaw".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "bChangesYaw".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Bool(b_changes_yaw),
                    },
                );
                actor.properties.insert(
                    "TargetVelocity".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "TargetVelocity".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Vector(target_vel),
                    },
                );

                let new_actor = actor.clone();
                self.history.push(crate::history::Action::UpdateTeleporterConfig {
                    actor_index,
                    old_actor,
                    new_actor,
                    description: format!("Update Teleporter #{} Config (Tag='{}', URL='{}')", actor_index, tag, url),
                });
                self.map_is_dirty = true;
                self.log(format!("Updated Teleporter #{} (Tag='{}', URL='{}').", actor_index, tag, url));
            }
        }
    }

    // =========================================================================
    // Translocator Trajectory & Teleport Beacon Studio
    // =========================================================================

    pub fn calculate_active_translocator_simulation(&mut self) {
        let origin = if let Some(probe) = self.translocator_studio_settings.probes.last() {
            probe.location
        } else {
            self.builder_brush.location + glam::Vec3::new(0.0, 0.0, 40.0)
        };

        // Collect all world polygons
        let mut world_polys = Vec::new();
        for b in &self.world_brushes {
            for p in &b.polys {
                world_polys.push(p.clone());
            }
        }

        let sim = ut99_bsp::simulate_translocator_flight(
            origin,
            &self.translocator_studio_settings.active_config,
            &world_polys,
        );

        self.translocator_studio_settings.active_simulation = Some(sim);
    }

    pub fn place_translocator_probe_at_pivot(&mut self) -> usize {
        let spawn_pos = self.builder_brush.location + glam::Vec3::new(0.0, 0.0, 30.0);
        let count = self.translocator_studio_settings.probes.len();
        let name = format!("TranslocatorProbe_{count}");
        let preset = self.translocator_studio_settings.selected_preset;

        let probe = TranslocatorProbeData {
            id: count,
            name: name.clone(),
            location: spawn_pos,
            pitch_deg: self.translocator_studio_settings.pitch_deg,
            yaw_deg: self.translocator_studio_settings.yaw_deg,
            preset,
        };

        self.translocator_studio_settings.probes.push(probe);
        self.history.push(crate::history::Action::AddTranslocatorProbe {
            probe_index: count,
            name: name.clone(),
            location: spawn_pos,
            pitch: self.translocator_studio_settings.pitch_deg,
            yaw: self.translocator_studio_settings.yaw_deg,
            preset,
        });

        self.calculate_active_translocator_simulation();
        self.map_is_dirty = true;
        self.log(format!("Placed Translocator Probe '{}' ({}) at pivot ({:.0}, {:.0}, {:.0}).", name, preset.display_name(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        count
    }

    // =========================================================================
    // LiftCenter, LiftExit & Dynamic Elevator Pathing Studio
    // =========================================================================

    pub fn collect_elevator_audit(&mut self) -> ut99_bsp::ElevatorAuditSummary {
        let mut mover_data_list = Vec::new();

        // 1. Collect from self.movers
        for m in &self.movers {
            let mut keys = Vec::new();
            for opt_k in &m.keyframes {
                if let Some(k) = opt_k {
                    keys.push(ut99_bsp::MoverKeyframe {
                        position: k.location,
                        rotation: k.rotation,
                    });
                }
            }
            if keys.is_empty() {
                keys.push(ut99_bsp::MoverKeyframe {
                    position: m.brush.location,
                    rotation: m.brush.rotation,
                });
            }

            // Estimate platform radius from brush polys
            let mut max_dist: f32 = 64.0;
            for poly in &m.brush.polys {
                for v in &poly.vertices {
                    let d = glam::Vec2::new(v.x, v.y).length();
                    if d > max_dist {
                        max_dist = d;
                    }
                }
            }

            mover_data_list.push(ut99_bsp::ElevatorMoverData {
                name: m.name.clone(),
                tag: if m.tag.is_empty() { m.name.clone() } else { m.tag.clone() },
                initial_state: m.glide_type.clone(),
                move_time: m.move_time.max(0.05),
                stay_open_time: m.stay_open_time,
                num_keys: keys.len(),
                keyframes: keys,
                platform_radius: max_dist,
            });
        }

        // Also check if there are movers in unr.actors that aren't in self.movers
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if a.class_name.contains("Mover") && !mover_data_list.iter().any(|m| m.name.eq_ignore_ascii_case(&a.name)) {
                    let keys = vec![ut99_bsp::MoverKeyframe {
                        position: a.location,
                        rotation: glam::Vec3::new(a.rotation.0 as f32, a.rotation.1 as f32, a.rotation.2 as f32),
                    }];

                    let mut move_time = 2.0;
                    let mut stay_time = 4.0;
                    let mut init_state = "StandOpenTimed".to_string();

                    for (k, prop) in &a.properties {
                        if k.eq_ignore_ascii_case("MoveTime") {
                            if let ut99_format::TaggedValue::Float(f) = prop.value {
                                move_time = f;
                            }
                        } else if k.eq_ignore_ascii_case("StayOpenTime") {
                            if let ut99_format::TaggedValue::Float(f) = prop.value {
                                stay_time = f;
                            }
                        } else if k.eq_ignore_ascii_case("InitialState") {
                            if let ut99_format::TaggedValue::Name(ref s) | ut99_format::TaggedValue::Str(ref s) = prop.value {
                                init_state = s.clone();
                            }
                        }
                    }

                    mover_data_list.push(ut99_bsp::ElevatorMoverData {
                        name: a.name.clone(),
                        tag: if a.tag.is_empty() { a.name.clone() } else { a.tag.clone() },
                        initial_state: init_state,
                        move_time,
                        stay_open_time: stay_time,
                        num_keys: keys.len(),
                        keyframes: keys,
                        platform_radius: 128.0,
                    });
                }
            }
        }

        // 2. Collect LiftCenters, LiftExits, and Triggers from unr.actors
        let mut lift_centers = Vec::new();
        let mut lift_exits = Vec::new();
        let mut triggers = Vec::new();

        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                let cls_lower = a.class_name.to_lowercase();
                if cls_lower.contains("liftcenter") {
                    let mut lift_tag = a.tag.clone();
                    let mut lift_trigger = String::new();
                    let mut max_dist = 128.0;

                    for (k, prop) in &a.properties {
                        if k.eq_ignore_ascii_case("LiftTag") {
                            if let ut99_format::TaggedValue::Str(ref s) | ut99_format::TaggedValue::Name(ref s) = prop.value {
                                lift_tag = s.clone();
                            }
                        } else if k.eq_ignore_ascii_case("LiftTrigger") {
                            if let ut99_format::TaggedValue::Str(ref s) | ut99_format::TaggedValue::Name(ref s) = prop.value {
                                lift_trigger = s.clone();
                            }
                        } else if k.eq_ignore_ascii_case("MaxDist2D") {
                            if let ut99_format::TaggedValue::Float(f) = prop.value {
                                max_dist = f;
                            }
                        }
                    }

                    lift_centers.push(ut99_bsp::LiftCenterData {
                        name: a.name.clone(),
                        lift_tag,
                        lift_trigger,
                        max_dist_2d: max_dist,
                        location: a.location,
                    });
                } else if cls_lower.contains("liftexit") {
                    let mut lift_tag = a.tag.clone();
                    let mut suggested_key = 0;
                    let mut key_frame = 0;

                    for (k, prop) in &a.properties {
                        if k.eq_ignore_ascii_case("LiftTag") {
                            if let ut99_format::TaggedValue::Str(ref s) | ut99_format::TaggedValue::Name(ref s) = prop.value {
                                lift_tag = s.clone();
                            }
                        } else if k.eq_ignore_ascii_case("SuggestedKeyFrame") {
                            if let ut99_format::TaggedValue::Byte(b) = prop.value {
                                suggested_key = b as usize;
                            } else if let ut99_format::TaggedValue::Int(i) = prop.value {
                                suggested_key = i.max(0) as usize;
                            }
                        } else if k.eq_ignore_ascii_case("KeyFrame") {
                            if let ut99_format::TaggedValue::Byte(b) = prop.value {
                                key_frame = b as usize;
                            } else if let ut99_format::TaggedValue::Int(i) = prop.value {
                                key_frame = i.max(0) as usize;
                            }
                        }
                    }

                    lift_exits.push(ut99_bsp::LiftExitData {
                        name: a.name.clone(),
                        lift_tag,
                        suggested_key_frame: suggested_key,
                        key_frame,
                        location: a.location,
                    });
                } else if cls_lower.contains("trigger") {
                    let mut evt = a.event.clone();
                    let mut rad = 40.0;
                    for (k, prop) in &a.properties {
                        if k.eq_ignore_ascii_case("Event") {
                            if let ut99_format::TaggedValue::Str(ref s) | ut99_format::TaggedValue::Name(ref s) = prop.value {
                                evt = s.clone();
                            }
                        } else if k.eq_ignore_ascii_case("CollisionRadius") {
                            if let ut99_format::TaggedValue::Float(f) = prop.value {
                                rad = f;
                            }
                        }
                    }
                    triggers.push(ut99_bsp::LiftTriggerData {
                        name: a.name.clone(),
                        event: evt,
                        location: a.location,
                        trigger_radius: rad,
                    });
                }
            }
        }

        let summary = ut99_bsp::audit_lift_systems(&mover_data_list, &lift_centers, &lift_exits, &triggers);
        self.lift_pathing_studio_settings.cached_audit = Some(summary.clone());
        self.calculate_active_lift_simulation();
        summary
    }

    pub fn calculate_active_lift_simulation(&mut self) {
        if let Some(ref audit) = self.lift_pathing_studio_settings.cached_audit {
            let idx = self.lift_pathing_studio_settings.selected_elevator_idx;
            if let Some(system) = audit.systems.get(idx) {
                let steps = ut99_bsp::simulate_elevator_bot_ride(system, 64);
                self.lift_pathing_studio_settings.cached_simulation = steps;
            } else if let Some(first) = audit.systems.first() {
                let steps = ut99_bsp::simulate_elevator_bot_ride(first, 64);
                self.lift_pathing_studio_settings.cached_simulation = steps;
            } else {
                self.lift_pathing_studio_settings.cached_simulation.clear();
            }
        } else {
            self.lift_pathing_studio_settings.cached_simulation.clear();
        }
    }

    pub fn step_elevator_simulation(&mut self, dt: f32) {
        if !self.lift_pathing_studio_settings.is_playing {
            return;
        }

        let speed = self.lift_pathing_studio_settings.playback_speed.clamp(0.1, 5.0);
        self.lift_pathing_studio_settings.playback_time += dt * speed;

        let total_cycle = if let Some(ref audit) = self.lift_pathing_studio_settings.cached_audit {
            let idx = self.lift_pathing_studio_settings.selected_elevator_idx;
            audit.systems.get(idx).map(|s| s.total_cycle_time).unwrap_or(4.0)
        } else {
            4.0
        };

        if self.lift_pathing_studio_settings.playback_time >= total_cycle {
            if self.lift_pathing_studio_settings.auto_loop {
                self.lift_pathing_studio_settings.playback_time %= total_cycle.max(0.1);
            } else {
                self.lift_pathing_studio_settings.playback_time = total_cycle;
                self.lift_pathing_studio_settings.is_playing = false;
            }
        }
    }

    pub fn create_synchronized_lift_setup_for_mover(&mut self, mover_idx: usize) -> bool {
        let (mover_name, mover_tag, keyframe_positions, platform_rad) = if let Some(m) = self.movers.get(mover_idx) {
            let mut keys = Vec::new();
            for opt_k in &m.keyframes {
                if let Some(k) = opt_k {
                    keys.push(k.location);
                }
            }
            if keys.is_empty() {
                keys.push(m.brush.location);
            }
            let rad = self.lift_pathing_studio_settings.wizard_platform_radius;
            (m.name.clone(), m.tag.clone(), keys, rad)
        } else if let Some(ref audit) = self.lift_pathing_studio_settings.cached_audit {
            if let Some(sys) = audit.systems.get(mover_idx) {
                let keys = sys.keyframes.iter().map(|k| k.position).collect();
                let rad = self.lift_pathing_studio_settings.wizard_platform_radius;
                (sys.mover_name.clone(), sys.mover_tag.clone(), keys, rad)
            } else {
                return false;
            }
        } else {
            return false;
        };

        let yaw = self.lift_pathing_studio_settings.wizard_exit_yaw_deg;
        let setup = ut99_bsp::generate_synchronized_lift_setup(&mover_name, &mover_tag, &keyframe_positions, platform_rad, yaw);

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let mut added_indices = Vec::new();
        let mut added_actors = Vec::new();

        // 1. Add LiftCenter
        let mut center_act = ut99_format::ActorRecord::default();
        center_act.class_name = "LiftCenter".to_string();
        center_act.name = setup.lift_center.name.clone();
        center_act.tag = setup.lift_center.lift_tag.clone();
        center_act.location = setup.lift_center.location;
        center_act.properties.insert(
            "LiftTag".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "LiftTag".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(setup.lift_center.lift_tag.clone()),
            },
        );
        center_act.properties.insert(
            "LiftTrigger".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "LiftTrigger".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(setup.lift_center.lift_trigger.clone()),
            },
        );
        let center_idx = unr.actors.len();
        added_indices.push(center_idx);
        added_actors.push(center_act.clone());
        unr.actors.push(center_act);

        // 2. Add LiftExits
        for exit in &setup.lift_exits {
            let mut exit_act = ut99_format::ActorRecord::default();
            exit_act.class_name = "LiftExit".to_string();
            exit_act.name = exit.name.clone();
            exit_act.tag = exit.lift_tag.clone();
            exit_act.location = exit.location;
            exit_act.properties.insert(
                "LiftTag".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "LiftTag".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Str(exit.lift_tag.clone()),
                },
            );
            exit_act.properties.insert(
                "SuggestedKeyFrame".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "SuggestedKeyFrame".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Byte(exit.suggested_key_frame as u8),
                },
            );
            exit_act.properties.insert(
                "KeyFrame".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "KeyFrame".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Byte(exit.key_frame as u8),
                },
            );
            let exit_idx = unr.actors.len();
            added_indices.push(exit_idx);
            added_actors.push(exit_act.clone());
            unr.actors.push(exit_act);
        }

        // 3. Add Triggers
        for trig in &setup.triggers {
            let mut trig_act = ut99_format::ActorRecord::default();
            trig_act.class_name = "Trigger".to_string();
            trig_act.name = trig.name.clone();
            trig_act.event = trig.event.clone();
            trig_act.location = trig.location;
            trig_act.properties.insert(
                "Event".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "Event".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Str(trig.event.clone()),
                },
            );
            let trig_idx = unr.actors.len();
            added_indices.push(trig_idx);
            added_actors.push(trig_act.clone());
            unr.actors.push(trig_act);
        }

        self.history.push(crate::history::Action::AddElevatorPathingSet {
            mover_tag: setup.lift_center.lift_tag.clone(),
            added_actor_indices: added_indices,
            added_actors,
            description: format!("Generated Synchronized Lift Pathing Set for Mover '{}' (LiftTag='{}')", mover_name, setup.lift_center.lift_tag),
        });

        self.map_is_dirty = true;
        self.collect_elevator_audit();
        self.log(format!("Generated synchronized LiftCenter and {} LiftExits for Mover '{}'.", setup.lift_exits.len(), mover_name));
        true
    }

    pub fn auto_fix_elevator_issues(&mut self, mover_tag: &str) -> usize {
        let mut fix_count = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            for actor in &mut unr.actors {
                let cls_lower = actor.class_name.to_lowercase();
                if cls_lower.contains("liftcenter") || cls_lower.contains("liftexit") {
                    let mut needs_fix = false;
                    for (k, prop) in &actor.properties {
                        if k.eq_ignore_ascii_case("LiftTag") {
                            if let ut99_format::TaggedValue::Str(ref s) | ut99_format::TaggedValue::Name(ref s) = prop.value {
                                if s.is_empty() || s != mover_tag {
                                    needs_fix = true;
                                }
                            }
                        }
                    }
                    if needs_fix {
                        actor.tag = mover_tag.to_string();
                        actor.properties.insert(
                            "LiftTag".to_string(),
                            ut99_format::unr::tagged_property::TaggedProperty {
                                name: "LiftTag".to_string(),
                                array_index: 0,
                                value: ut99_format::unr::tagged_property::TaggedValue::Str(mover_tag.to_string()),
                            },
                        );
                        fix_count += 1;
                    }
                }
            }
        }

        if fix_count > 0 {
            self.map_is_dirty = true;
            self.collect_elevator_audit();
            self.log(format!("Auto-fixed {} elevator tag/property discrepancies for tag '{}'.", fix_count, mover_tag));
        }
        fix_count
    }

    // =========================================================================
    // Dynamic Zone Water Surface & Wave Reflection Studio
    // =========================================================================

    pub fn step_water_surface_simulation(&mut self, dt: f32) {
        if !self.water_surface_studio_settings.is_playing {
            return;
        }
        let speed = self.water_surface_studio_settings.simulation_speed.clamp(0.1, 5.0);
        let dt_scaled = dt * speed;
        let config = self.water_surface_studio_settings.active_config.clone();
        self.water_surface_studio_settings.simulation.step(dt_scaled, &config);
    }

    pub fn inject_water_ripple_uv(&mut self, u: f32, v: f32, amplitude: f32) {
        let r = self.water_surface_studio_settings.ripple_brush_radius;
        self.water_surface_studio_settings.simulation.add_impulse_uv(u, v, r, amplitude);
    }

    pub fn inject_water_ripple_world(&mut self, world_pos: Vec3, amplitude: f32) {
        let center = self.water_surface_studio_settings.sheet_center;
        let sx = self.water_surface_studio_settings.sheet_size_x.max(16.0);
        let sy = self.water_surface_studio_settings.sheet_size_y.max(16.0);
        let rel_x = world_pos.x - (center.x - sx * 0.5);
        let rel_y = world_pos.y - (center.y - sy * 0.5);
        let u = (rel_x / sx).clamp(0.0, 1.0);
        let v = (rel_y / sy).clamp(0.0, 1.0);
        let r = self.water_surface_studio_settings.ripple_brush_radius;
        self.water_surface_studio_settings.simulation.add_impulse_uv(u, v, r, amplitude);
    }

    pub fn apply_water_preset(&mut self, preset: ut99_bsp::WaterPreset) {
        let old_config = self.water_surface_studio_settings.active_config.clone();
        let old_preset = self.water_surface_studio_settings.selected_preset;
        let new_config = preset.to_config();

        self.history.push(crate::history::Action::UpdateWaterSurfaceConfig {
            old_config,
            new_config: new_config.clone(),
            old_preset,
            new_preset: preset,
            description: format!("Applied Water Surface Preset '{}'", preset.display_name()),
        });

        self.water_surface_studio_settings.selected_preset = preset;
        self.water_surface_studio_settings.active_config = new_config;
        self.log(format!("Applied water preset: {}", preset.display_name()));
    }

    pub fn reset_water_simulation(&mut self) {
        self.water_surface_studio_settings.simulation.clear();
        self.log("Water surface heightfield simulation reset.");
    }

    pub fn generate_water_sheet_brush(&mut self) -> usize {
        let center = self.water_surface_studio_settings.sheet_center;
        let sx = self.water_surface_studio_settings.sheet_size_x;
        let sy = self.water_surface_studio_settings.sheet_size_y;
        let sub_x = self.water_surface_studio_settings.subdivisions_x;
        let sub_y = self.water_surface_studio_settings.subdivisions_y;
        let tex = self.water_surface_studio_settings.sheet_texture_name.clone();
        let name = format!("WaterSheet_{}", self.world_brushes.len());

        // Poly flags: PF_Translucent (0x00000004) | PF_TwoSided (0x00000100)
        let flags = 0x00000004 | 0x00000100;
        let sheet_brush = ut99_bsp::generate_watertight_water_sheet_brush(
            &name,
            center,
            sx,
            sy,
            sub_x,
            sub_y,
            flags,
            &tex,
        );

        let brush_index = self.world_brushes.len();
        self.world_brushes.push(sheet_brush.clone());
        self.map_is_dirty = true;

        self.history.push(crate::history::Action::AddWaterSheetBrush {
            brush_index,
            brush: sheet_brush,
            description: format!("Generated Watertight Water Sheet Brush '{}' ({}x{} quads)", name, sub_x, sub_y),
        });

        self.log(format!("Created watertight water sheet brush '{}' at ({:.1}, {:.1}, {:.1}) with {} quads.", name, center.x, center.y, center.z, sub_x * sub_y));
        brush_index
    }

    // =========================================================================
    // Mover Encroachment, Door Sequencing & Crushing Trap Studio
    // =========================================================================

    pub fn step_mover_studio_simulation(&mut self, dt: f32) {
        if !self.mover_studio_settings.is_playing {
            return;
        }

        let speed = self.mover_studio_settings.playback_speed.clamp(0.1, 5.0);
        self.mover_studio_settings.playback_time += dt * speed;

        let move_t = self.mover_studio_settings.active_config.move_time.max(0.1);
        let stay_t = self.mover_studio_settings.active_config.stay_open_time.max(0.0);
        let total_cycle = move_t * 2.0 + stay_t;

        if self.mover_studio_settings.playback_time >= total_cycle {
            if self.mover_studio_settings.auto_loop {
                self.mover_studio_settings.playback_time %= total_cycle.max(0.1);
            } else {
                self.mover_studio_settings.playback_time = total_cycle;
                self.mover_studio_settings.is_playing = false;
            }
        }
    }

    pub fn scrub_mover_timeline(&mut self, time: f32) {
        let move_t = self.mover_studio_settings.active_config.move_time.max(0.1);
        let stay_t = self.mover_studio_settings.active_config.stay_open_time.max(0.0);
        let total_cycle = move_t * 2.0 + stay_t;
        self.mover_studio_settings.playback_time = time.clamp(0.0, total_cycle);
    }

    pub fn apply_mover_template(&mut self, template: ut99_bsp::MoverTemplateArchetype) {
        let origin = if let Some(m) = self.movers.get(self.mover_studio_settings.selected_mover_idx) {
            m.brush.location
        } else {
            Vec3::ZERO
        };

        let old_config = self.mover_studio_settings.active_config.clone();
        let old_template = self.mover_studio_settings.selected_template;
        let new_config = template.to_config(origin);

        self.history.push(crate::history::Action::UpdateMoverSequenceConfig {
            mover_index: self.mover_studio_settings.selected_mover_idx,
            old_config,
            new_config: new_config.clone(),
            old_template,
            new_template: template,
            description: format!("Applied Mover Template '{}'", template.display_name()),
        });

        self.mover_studio_settings.selected_template = template;
        self.mover_studio_settings.active_config = new_config;
        self.mover_studio_settings.playback_time = 0.0;
        self.audit_active_mover_hazards();
        self.log(format!("Applied mover template: {}", template.display_name()));
    }

    pub fn audit_active_mover_hazards(&mut self) {
        let mut obstacles = Vec::new();
        // Sample nearby world brush vertex locations as potential obstacle walls
        for b in &self.world_brushes {
            for poly in &b.polys {
                for v in &poly.vertices {
                    obstacles.push(b.location + *v);
                }
            }
        }

        let mover_half = Vec3::new(32.0, 32.0, 48.0);
        let hazards = ut99_bsp::audit_mover_hazards(&self.mover_studio_settings.active_config, mover_half, &obstacles);
        self.mover_studio_settings.cached_hazards = hazards;
        self.simulate_active_mover_bot_encroachment();
    }

    pub fn simulate_active_mover_bot_encroachment(&mut self) {
        let bot_pos = self.mover_studio_settings.bot_test_position;
        let mover_half = Vec3::new(32.0, 32.0, 48.0);
        let sim = ut99_bsp::simulate_mover_encroachment(&self.mover_studio_settings.active_config, bot_pos, 17.0, mover_half);
        self.mover_studio_settings.cached_encroach_sim = sim;
    }

    pub fn wire_proximity_trigger_for_active_mover(&mut self) -> bool {
        let mover_name = self.mover_studio_settings.active_config.mover_name.clone();
        let mover_tag = self.mover_studio_settings.active_config.mover_tag.clone();
        let door_pos = self.mover_studio_settings.active_config.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO);
        let approach_dist = self.mover_studio_settings.trigger_approach_distance;
        let radius = self.mover_studio_settings.trigger_radius;

        let trig = ut99_bsp::generate_door_proximity_trigger(&mover_name, &mover_tag, door_pos, Vec3::X, approach_dist, radius);

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "Trigger".to_string();
        actor.name = trig.name.clone();
        actor.event = trig.event.clone();
        actor.location = trig.location;
        actor.properties.insert(
            "Event".to_string(),
            ut99_format::unr::tagged_property::TaggedProperty {
                name: "Event".to_string(),
                array_index: 0,
                value: ut99_format::unr::tagged_property::TaggedValue::Str(trig.event.clone()),
            },
        );

        let actor_index = unr.actors.len();
        unr.actors.push(actor.clone());
        self.map_is_dirty = true;

        self.history.push(crate::history::Action::AddDoorTriggerActor {
            actor_index,
            actor,
            description: format!("Generated Proximity Trigger '{}' (Event='{}') for Mover '{}'", trig.name, trig.event, mover_name),
        });

        self.audit_active_mover_hazards();
        self.log(format!("Wired proximity trigger '{}' at ({:.1}, {:.1}, {:.1}) for mover '{}'.", trig.name, trig.location.x, trig.location.y, trig.location.z, mover_name));
        true
    }

    // =========================================================================
    // Team Base Objective, CTF Flag Run Route & Defense AI Studio
    // =========================================================================

    pub fn scan_and_refresh_team_objectives(&mut self) {
        let mut objectives = Vec::new();
        let mut defense_points = Vec::new();

        // Scan loaded actors
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let class_lower = actor.class_name.to_lowercase();
                if class_lower.contains("flagbase") || class_lower.contains("ctfflag") {
                    let team = if class_lower.contains("red") {
                        Some(ut99_bsp::TeamColor::Red)
                    } else if class_lower.contains("blue") {
                        Some(ut99_bsp::TeamColor::Blue)
                    } else if class_lower.contains("green") {
                        Some(ut99_bsp::TeamColor::Green)
                    } else if class_lower.contains("gold") {
                        Some(ut99_bsp::TeamColor::Gold)
                    } else {
                        None
                    };
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        objective_type: ut99_bsp::GameObjectiveType::CaptureTheFlag,
                        team,
                        location: actor.location,
                        collision_radius: 60.0,
                        tag: actor.tag.clone(),
                    });
                } else if class_lower.contains("controlpoint") {
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        objective_type: ut99_bsp::GameObjectiveType::Domination,
                        team: None,
                        location: actor.location,
                        collision_radius: 60.0,
                        tag: actor.tag.clone(),
                    });
                } else if class_lower.contains("fortstandard") {
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        objective_type: ut99_bsp::GameObjectiveType::Assault,
                        team: Some(ut99_bsp::TeamColor::Blue),
                        location: actor.location,
                        collision_radius: 60.0,
                        tag: actor.tag.clone(),
                    });
                } else if class_lower.contains("defensepoint") {
                    let is_sniping = actor.properties.get("bSniping")
                        .and_then(|p| match &p.value {
                            ut99_format::unr::tagged_property::TaggedValue::Bool(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(false);
                    let team_num = actor.properties.get("team")
                        .and_then(|p| match &p.value {
                            ut99_format::unr::tagged_property::TaggedValue::Byte(b) => Some(*b),
                            _ => None,
                        })
                        .unwrap_or(0);
                    let team = match team_num {
                        1 => ut99_bsp::TeamColor::Blue,
                        2 => ut99_bsp::TeamColor::Green,
                        3 => ut99_bsp::TeamColor::Gold,
                        _ => ut99_bsp::TeamColor::Red,
                    };
                    defense_points.push(ut99_bsp::DefensePointData {
                        actor_name: actor.name.clone(),
                        team,
                        location: actor.location,
                        rotation_yaw_deg: (actor.rotation.1 as f32 / 65536.0) * 360.0,
                        fov_degrees: 90.0,
                        is_sniping,
                        priority: 200,
                        target_objective: None,
                        has_los_to_objective: true,
                        los_distance_uu: 0.0,
                        elevation_advantage_uu: 0.0,
                    });
                }
            }
        }

        // If no objectives detected, synthesize representative sample nodes
        if objectives.is_empty() {
            match self.team_objective_settings.active_game_mode {
                ut99_bsp::GameObjectiveType::CaptureTheFlag => {
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: "RedFlagBase0".to_string(),
                        class_name: "RedFlagBase".to_string(),
                        objective_type: ut99_bsp::GameObjectiveType::CaptureTheFlag,
                        team: Some(ut99_bsp::TeamColor::Red),
                        location: glam::Vec3::new(-1500.0, 0.0, 64.0),
                        collision_radius: 60.0,
                        tag: "RedFlag".to_string(),
                    });
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: "BlueFlagBase0".to_string(),
                        class_name: "BlueFlagBase".to_string(),
                        objective_type: ut99_bsp::GameObjectiveType::CaptureTheFlag,
                        team: Some(ut99_bsp::TeamColor::Blue),
                        location: glam::Vec3::new(1500.0, 0.0, 64.0),
                        collision_radius: 60.0,
                        tag: "BlueFlag".to_string(),
                    });
                }
                ut99_bsp::GameObjectiveType::Domination => {
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: "ControlPoint_Alpha".to_string(),
                        class_name: "ControlPoint".to_string(),
                        objective_type: ut99_bsp::GameObjectiveType::Domination,
                        team: None,
                        location: glam::Vec3::new(0.0, 1000.0, 64.0),
                        collision_radius: 60.0,
                        tag: "Alpha".to_string(),
                    });
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: "ControlPoint_Beta".to_string(),
                        class_name: "ControlPoint".to_string(),
                        objective_type: ut99_bsp::GameObjectiveType::Domination,
                        team: None,
                        location: glam::Vec3::new(866.0, -500.0, 64.0),
                        collision_radius: 60.0,
                        tag: "Beta".to_string(),
                    });
                    objectives.push(ut99_bsp::ObjectiveNode {
                        name: "ControlPoint_Gamma".to_string(),
                        class_name: "ControlPoint".to_string(),
                        objective_type: ut99_bsp::GameObjectiveType::Domination,
                        team: None,
                        location: glam::Vec3::new(-866.0, -500.0, 64.0),
                        collision_radius: 60.0,
                        tag: "Gamma".to_string(),
                    });
                }
                _ => {}
            }
        }

        // Compute flag routes if Red and Blue flags exist
        let red_flag = objectives.iter().find(|o| o.objective_type == ut99_bsp::GameObjectiveType::CaptureTheFlag && o.team == Some(ut99_bsp::TeamColor::Red));
        let blue_flag = objectives.iter().find(|o| o.objective_type == ut99_bsp::GameObjectiveType::CaptureTheFlag && o.team == Some(ut99_bsp::TeamColor::Blue));

        let obstacles = Vec::new();
        if let (Some(rf), Some(bf)) = (red_flag, blue_flag) {
            let routes = ut99_bsp::compute_flag_routes(rf.location, bf.location, None, &obstacles);
            let asym = ut99_bsp::evaluate_route_asymmetry(
                routes[0].total_distance_uu,
                routes[0].total_distance_uu,
            );
            self.team_objective_settings.routes = routes;
            self.team_objective_settings.asymmetry = Some(asym);
        } else {
            self.team_objective_settings.routes.clear();
            self.team_objective_settings.asymmetry = None;
        }

        // Domination triangle if 3 control points
        let cp_nodes: Vec<_> = objectives.iter().filter(|o| o.objective_type == ut99_bsp::GameObjectiveType::Domination).collect();
        if cp_nodes.len() >= 3 {
            let tri = ut99_bsp::evaluate_domination_triangle(
                &cp_nodes[0].name, cp_nodes[0].location,
                &cp_nodes[1].name, cp_nodes[1].location,
                &cp_nodes[2].name, cp_nodes[2].location,
            );
            self.team_objective_settings.dom_triangle = Some(tri);
        } else {
            self.team_objective_settings.dom_triangle = None;
        }

        // Audit line of sight for defense points
        for dp in &mut defense_points {
            if let Some(target_obj) = objectives.iter().find(|o| o.team == Some(dp.team)) {
                dp.target_objective = Some(target_obj.name.clone());
                let (has_los, _obs, dist) = ut99_bsp::check_defense_los(
                    dp.location,
                    target_obj.location,
                    dp.rotation_yaw_deg,
                    dp.fov_degrees,
                    &obstacles,
                );
                dp.has_los_to_objective = has_los;
                dp.los_distance_uu = dist;
                dp.elevation_advantage_uu = dp.location.z - target_obj.location.z;
            }
        }

        // Run audit
        let issues = ut99_bsp::audit_team_objectives(
            &objectives,
            &defense_points,
            &self.team_objective_settings.routes,
            self.team_objective_settings.asymmetry.as_ref(),
            self.team_objective_settings.dom_triangle.as_ref(),
        );

        self.team_objective_settings.detected_objectives = objectives;
        self.team_objective_settings.detected_defense_points = defense_points;
        self.team_objective_settings.audit_issues = issues;
    }

    pub fn generate_defense_points_for_active_objective(&mut self) -> usize {
        if self.team_objective_settings.detected_objectives.is_empty() {
            self.scan_and_refresh_team_objectives();
        }

        let idx = self.team_objective_settings.selected_objective_idx.min(
            self.team_objective_settings.detected_objectives.len().saturating_sub(1)
        );

        let obj = self.team_objective_settings.detected_objectives[idx].clone();
        let team = obj.team.unwrap_or(ut99_bsp::TeamColor::Red);
        let approach_dir = match team {
            ut99_bsp::TeamColor::Red => glam::Vec3::new(1.0, 0.0, 0.0),
            _ => glam::Vec3::new(-1.0, 0.0, 0.0),
        };

        let generated_dps = ut99_bsp::generate_coordinated_defense_points(&obj, team, approach_dir);
        let count = generated_dps.len();

        let mut added_indices = Vec::new();
        let mut added_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            for dp in &generated_dps {
                let mut actor = ut99_format::ActorRecord::default();
                actor.class_name = "DefensePoint".to_string();
                actor.name = dp.actor_name.clone();
                actor.location = dp.location;
                let yaw_raw = ((dp.rotation_yaw_deg / 360.0) * 65536.0) as i32;
                actor.rotation = (0, yaw_raw, 0);

                actor.properties.insert(
                    "team".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "team".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Byte(dp.team as u8),
                    },
                );
                actor.properties.insert(
                    "bSniping".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "bSniping".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Bool(dp.is_sniping),
                    },
                );

                let actor_idx = unr.actors.len();
                unr.actors.push(actor.clone());
                added_indices.push(actor_idx);
                added_actors.push(actor);
            }
        }

        self.history.push(crate::history::Action::AddDefensePointSet {
            objective_name: obj.name.clone(),
            added_actor_indices: added_indices,
            added_actors,
            description: format!("Deployed 3 Coordinated DefensePoints for '{}'", obj.name),
        });

        self.map_is_dirty = true;
        self.scan_and_refresh_team_objectives();
        self.log(format!("Deployed {} coordinated defense points for objective '{}'.", count, obj.name));
        count
    }

    pub fn apply_team_objective_preset(&mut self, preset: ut99_bsp::TeamObjectivePreset) {
        let old_mode = self.team_objective_settings.active_game_mode;
        let old_preset = self.team_objective_settings.selected_preset;
        let new_mode = preset.game_mode();

        self.history.push(crate::history::Action::UpdateTeamObjectiveSetup {
            old_mode,
            new_mode,
            old_preset,
            new_preset: Some(preset),
            description: format!("Applied Team Objective Preset '{}'", preset.display_name()),
        });

        self.team_objective_settings.active_game_mode = new_mode;
        self.team_objective_settings.selected_preset = Some(preset);
        self.scan_and_refresh_team_objectives();
        self.log(format!("Applied team objective preset: {}", preset.display_name()));
    }

    // =========================================================================
    // Destructible Architecture, Breakable Glass & Fracture Studio
    // =========================================================================

    pub fn regenerate_destructible_shards(&mut self) {
        let size = self.destructible_studio_settings.active_config.size;
        let pattern = self.destructible_studio_settings.selected_pattern;
        let count = self.destructible_studio_settings.active_config.shard_count;
        self.destructible_studio_settings.cached_shards = ut99_bsp::generate_fracture_shards(size, pattern, count);
        self.destructible_studio_settings.simulation.reset();
    }

    pub fn step_destructible_simulation(&mut self, dt: f32) {
        if self.destructible_studio_settings.is_animating {
            self.destructible_studio_settings.simulation.step(dt);
        }
    }

    pub fn trigger_destructible_shatter(&mut self) {
        let config = self.destructible_studio_settings.active_config.clone();
        let shards = self.destructible_studio_settings.cached_shards.clone();
        self.destructible_studio_settings.simulation.trigger_shatter(&config, &shards);
        self.log(format!("Triggered physical fracture simulation for barrier '{}' ({} shards).", config.name, shards.len()));
    }

    pub fn apply_destructible_archetype(&mut self, arch: ut99_bsp::DestructibleArchetype) {
        let center = self.destructible_studio_settings.active_config.center;
        let size = self.destructible_studio_settings.active_config.size;

        let old_config = self.destructible_studio_settings.active_config.clone();
        let old_archetype = self.destructible_studio_settings.selected_archetype;
        let new_config = arch.to_config(center, size);

        self.history.push(crate::history::Action::UpdateDestructibleConfig {
            old_config,
            new_config: new_config.clone(),
            old_archetype,
            new_archetype: arch,
            description: format!("Applied Destructible Archetype '{}'", arch.display_name()),
        });

        self.destructible_studio_settings.selected_archetype = arch;
        self.destructible_studio_settings.selected_pattern = new_config.fracture_pattern;
        self.destructible_studio_settings.active_config = new_config;
        self.regenerate_destructible_shards();
        self.audit_destructible_hazards();
        self.log(format!("Applied destructible archetype: {}", arch.display_name()));
    }

    pub fn audit_destructible_hazards(&mut self) {
        self.destructible_studio_settings.cached_hazards = ut99_bsp::audit_destructible_hazards(
            &self.destructible_studio_settings.active_config
        );
    }

    pub fn generate_destructible_barrier(&mut self) -> usize {
        let cfg = self.destructible_studio_settings.active_config.clone();
        let name = format!("{}_{}", cfg.name, self.world_brushes.len());

        // 1. Generate planar sheet brush with proper polyflags
        let poly_flags = match cfg.archetype {
            ut99_bsp::DestructibleArchetype::BreakableWindowGlass | ut99_bsp::DestructibleArchetype::FragileSkylightDome => {
                0x00000004 | 0x00000100 // PF_Translucent | PF_TwoSided
            }
            ut99_bsp::DestructibleArchetype::VentGratingShatter => {
                0x00000002 | 0x00000100 // PF_Masked | PF_TwoSided
            }
            _ => 0, // Solid wall/door
        };

        let sheet_brush = ut99_bsp::generate_watertight_water_sheet_brush(
            &name,
            cfg.center,
            cfg.size.x,
            cfg.size.y,
            2,
            2,
            poly_flags,
            &cfg.texture_intact,
        );

        let brush_index = self.world_brushes.len();
        self.world_brushes.push(sheet_brush.clone());

        // 2. Generate corresponding Actor (BreakingGlass or ExplodingWall)
        let mut added_actor_indices = Vec::new();
        let mut added_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            let mut actor = ut99_format::ActorRecord::default();
            actor.class_name = match cfg.archetype {
                ut99_bsp::DestructibleArchetype::BreakableWindowGlass | ut99_bsp::DestructibleArchetype::FragileSkylightDome => {
                    "BreakingGlass".to_string()
                }
                ut99_bsp::DestructibleArchetype::VentGratingShatter => {
                    "SpecialEvent".to_string()
                }
                _ => "ExplodingWall".to_string(),
            };
            actor.name = format!("{}_Actor", name);
            actor.location = cfg.center;
            actor.event = cfg.event_on_break.clone();

            actor.properties.insert(
                "Health".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "Health".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Int(cfg.health as i32),
                },
            );
            actor.properties.insert(
                "Event".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "Event".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Str(cfg.event_on_break.clone()),
                },
            );

            let act_idx = unr.actors.len();
            unr.actors.push(actor.clone());
            added_actor_indices.push(act_idx);
            added_actors.push(actor);

            // 3. Auto-wire bot reachability PathNode if requested
            if cfg.auto_wire_bot_reachability {
                let mut path_node = ut99_format::ActorRecord::default();
                path_node.class_name = "PathNode".to_string();
                path_node.name = format!("{}_ApertureNode", name);
                path_node.location = cfg.center;
                let pn_idx = unr.actors.len();
                unr.actors.push(path_node.clone());
                added_actor_indices.push(pn_idx);
                added_actors.push(path_node);
            }
        }

        self.history.push(crate::history::Action::AddDestructibleBarrier {
            brush_index,
            brush: sheet_brush,
            added_actor_indices,
            added_actors,
            description: format!("Created Destructible Barrier '{}' ({:.0} HP)", name, cfg.health),
        });

        self.map_is_dirty = true;
        self.audit_destructible_hazards();
        self.log(format!("Generated destructible barrier '{}' at ({:.1}, {:.1}, {:.1}) with {} shards.", name, cfg.center.x, cfg.center.y, cfg.center.z, cfg.shard_count));
        brush_index
    }

    // =========================================================================
    // Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio
    // =========================================================================
    pub fn recalculate_kicker_trajectory(&mut self) {
        let analysis = ut99_bsp::calculate_kicker_trajectory(&self.kicker_studio_settings.active_config);
        let hazards = ut99_bsp::audit_kicker_hazards(&self.kicker_studio_settings.active_config, &analysis);
        self.kicker_studio_settings.trajectory_analysis = analysis;
        self.kicker_studio_settings.cached_hazards = hazards;
    }

    pub fn solve_kicker_velocity_to_target(&mut self) {
        let origin = self.kicker_studio_settings.active_config.pad_location;
        let target = self.kicker_studio_settings.active_config.target_location;
        let clearance = self.kicker_studio_settings.target_apex_clearance_z;
        let gravity = self.kicker_studio_settings.active_config.gravity_z;

        let solved_vel = ut99_bsp::solve_kick_velocity_for_target(origin, target, clearance, gravity);

        let old_config = self.kicker_studio_settings.active_config.clone();
        let mut new_config = old_config.clone();
        new_config.kick_velocity = solved_vel;

        self.history.push(crate::history::Action::UpdateKickerConfig {
            old_config,
            new_config: new_config.clone(),
            old_archetype: self.kicker_studio_settings.selected_archetype,
            new_archetype: self.kicker_studio_settings.selected_archetype,
            description: format!("Solved KickVelocity ({:.0}, {:.0}, {:.0}) to target", solved_vel.x, solved_vel.y, solved_vel.z),
        });

        self.kicker_studio_settings.active_config = new_config;
        self.recalculate_kicker_trajectory();
        self.log(format!("Auto-solved KickVelocity: ({:.1}, {:.1}, {:.1})", solved_vel.x, solved_vel.y, solved_vel.z));
    }

    pub fn apply_kicker_archetype(&mut self, arch: ut99_bsp::KickerArchetype) {
        let old_config = self.kicker_studio_settings.active_config.clone();
        let old_arch = self.kicker_studio_settings.selected_archetype;
        let pad_pos = old_config.pad_location;
        let mut new_config = arch.to_config(pad_pos);
        new_config.name = format!("{}_{}", new_config.actor_class, self.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0));

        self.history.push(crate::history::Action::UpdateKickerConfig {
            old_config,
            new_config: new_config.clone(),
            old_archetype: old_arch,
            new_archetype: arch,
            description: format!("Applied Kicker Archetype '{}'", arch.display_name()),
        });

        self.kicker_studio_settings.selected_archetype = arch;
        self.kicker_studio_settings.active_config = new_config;
        self.recalculate_kicker_trajectory();
        self.log(format!("Applied kicker archetype '{}'.", arch.display_name()));
    }

    pub fn generate_kicker_setup(&mut self) -> (usize, Option<usize>) {
        let cfg = self.kicker_studio_settings.active_config.clone();
        let mut kicker_idx = 0;
        let mut jump_spot_idx = None;
        let mut kicker_actor = ut99_format::ActorRecord::default();
        let mut jump_spot_actor = None;

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. Create the Kicker / UT_JumpPad actor
            kicker_actor.class_name = cfg.actor_class.clone();
            kicker_actor.name = format!("{}_{}", cfg.actor_class, unr.actors.len());
            kicker_actor.location = cfg.pad_location;
            let kick_tag = format!("{}_Event", kicker_actor.name);
            kicker_actor.event = kick_tag.clone();

            kicker_actor.properties.insert(
                "KickVelocity".to_string(),
                ut99_format::unr::tagged_property::TaggedProperty {
                    name: "KickVelocity".to_string(),
                    array_index: 0,
                    value: ut99_format::unr::tagged_property::TaggedValue::Vector(cfg.kick_velocity),
                },
            );

            kicker_idx = unr.actors.len();
            unr.actors.push(kicker_actor.clone());

            // 2. Optionally create the JumpSpot companion actor for bot AI
            if cfg.auto_place_jump_spot {
                let mut js = ut99_format::ActorRecord::default();
                js.class_name = "JumpSpot".to_string();
                js.name = format!("JumpSpot_{}", unr.actors.len());
                js.location = cfg.target_location;
                js.tag = kick_tag;

                js.properties.insert(
                    "bSpecialCost".to_string(),
                    ut99_format::unr::tagged_property::TaggedProperty {
                        name: "bSpecialCost".to_string(),
                        array_index: 0,
                        value: ut99_format::unr::tagged_property::TaggedValue::Bool(true),
                    },
                );

                let js_idx = unr.actors.len();
                unr.actors.push(js.clone());
                jump_spot_idx = Some(js_idx);
                jump_spot_actor = Some(js);
            }
        }

        self.history.push(crate::history::Action::AddJumpPadSetup {
            kicker_index: kicker_idx,
            jump_spot_index: jump_spot_idx,
            kicker_actor,
            jump_spot_actor,
            description: format!("Created {} Setup ({:.0}, {:.0}, {:.0})", cfg.actor_class, cfg.kick_velocity.x, cfg.kick_velocity.y, cfg.kick_velocity.z),
        });

        self.map_is_dirty = true;
        self.recalculate_kicker_trajectory();
        self.log(format!("Generated {} at ({:.1}, {:.1}, {:.1}) with target ({:.1}, {:.1}, {:.1})",
            cfg.actor_class, cfg.pad_location.x, cfg.pad_location.y, cfg.pad_location.z,
            cfg.target_location.x, cfg.target_location.y, cfg.target_location.z));
        (kicker_idx, jump_spot_idx)
    }

    // =========================================================================
    // BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio
    // =========================================================================

    pub fn run_bsp_quality_audit(&mut self) {
        let (root_node, _report) = ut99_bsp::build_bsp_from_brushes(&self.world_brushes, &self.bsp_config);
        let metrics = ut99_bsp::analyze_bsp_tree_quality(root_node.as_ref(), &self.world_brushes);
        let slivers = ut99_bsp::detect_sliver_polygons(
            &self.world_brushes,
            self.bsp_quality_studio_settings.min_sliver_area,
            self.bsp_quality_studio_settings.max_aspect_ratio,
        );
        let coplanar = ut99_bsp::detect_coplanar_overlaps(&self.world_brushes, 0.1);
        let offenders = ut99_bsp::analyze_brush_cut_offenders(&self.world_brushes);

        self.bsp_quality_studio_settings.quality_metrics = metrics;
        self.bsp_quality_studio_settings.detected_slivers = slivers;
        self.bsp_quality_studio_settings.detected_coplanar = coplanar;
        self.bsp_quality_studio_settings.cut_offenders = offenders;

        self.log(format!(
            "BSP Quality Audit: {} world polys (Cut Ratio: {:.2}), Max Depth: {}, Balance: {:.1}%, {} slivers, {} coplanar overlaps, {} cut offenders",
            self.bsp_quality_studio_settings.quality_metrics.total_world_bsp_polys,
            self.bsp_quality_studio_settings.quality_metrics.cut_ratio,
            self.bsp_quality_studio_settings.quality_metrics.max_tree_depth,
            self.bsp_quality_studio_settings.quality_metrics.tree_balance_pct,
            self.bsp_quality_studio_settings.detected_slivers.len(),
            self.bsp_quality_studio_settings.detected_coplanar.len(),
            self.bsp_quality_studio_settings.cut_offenders.len(),
        ));
    }

    pub fn purge_all_sliver_polygons(&mut self) -> usize {
        let min_area = self.bsp_quality_studio_settings.min_sliver_area;
        let mut modifications = Vec::new();
        let mut total_purged = 0;

        for (b_idx, brush) in self.world_brushes.iter_mut().enumerate() {
            let old_polys = brush.polys.clone();
            let purged = ut99_bsp::purge_sliver_triangles(brush, min_area);
            if purged > 0 {
                total_purged += purged;
                modifications.push((b_idx, old_polys, brush.polys.clone()));
            }
        }

        if !modifications.is_empty() {
            self.history.push(crate::history::Action::PurgeSliverPolygons {
                modifications,
                description: format!("Purged {} sliver micro-polygons across level", total_purged),
            });
            self.map_is_dirty = true;
            self.run_bsp_quality_audit();
            self.log(format!("Purged {} sliver micro-polygons across brushes.", total_purged));
        } else {
            self.log("No sliver micro-polygons found to purge.");
        }

        total_purged
    }

    pub fn convert_top_cut_offenders_to_semisolid(&mut self, top_n: usize) -> usize {
        let mut conversions = Vec::new();
        let mut converted_count = 0;

        for offender in &self.bsp_quality_studio_settings.cut_offenders {
            if converted_count >= top_n {
                break;
            }
            if offender.can_be_semisolid && offender.csg_oper == ut99_bsp::CsgOperation::Add {
                if let Some(b) = self.world_brushes.get_mut(offender.brush_index) {
                    let old_polys = b.polys.clone();
                    for p in &mut b.polys {
                        p.flags |= ut99_bsp::PF_SEMISOLID;
                    }
                    let new_polys = b.polys.clone();
                    conversions.push((offender.brush_index, old_polys, new_polys));
                    converted_count += 1;
                }
            }
        }

        if !conversions.is_empty() {
            self.history.push(crate::history::Action::BatchConvertSemiSolid {
                conversions,
                description: format!("Converted {} high-cut additive brushes to Semi-Solid", converted_count),
            });
            self.map_is_dirty = true;
            self.run_bsp_quality_audit();
            self.log(format!("Converted {} high-cut brushes to Semi-Solid to eliminate BSP cuts.", converted_count));
        } else {
            self.log("No additive cut offenders suitable for Semi-Solid conversion.");
        }

        converted_count
    }

    // =========================================================================
    // 2D Shape Loft, Curved Pipe & Architectural Molding Studio
    // =========================================================================

    pub fn apply_loft_archetype(&mut self, arch: ut99_bsp::LoftProfileArchetype) {
        let old_config = self.loft_studio_settings.active_config.clone();
        let old_keyframes = self.loft_studio_settings.path_keyframes.clone();

        self.loft_studio_settings.active_config.archetype = arch;
        self.loft_studio_settings.custom_profile = arch.to_profile(self.loft_studio_settings.active_config.profile_radius);

        self.recalculate_loft_mesh();

        self.history.push(crate::history::Action::UpdateLoftConfig {
            old_config,
            new_config: self.loft_studio_settings.active_config.clone(),
            old_keyframes,
            new_keyframes: self.loft_studio_settings.path_keyframes.clone(),
            description: format!("Applied Loft Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Loft Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_loft_mesh(&mut self) {
        let frames = ut99_bsp::generate_rmf_frames(
            &self.loft_studio_settings.path_keyframes,
            self.loft_studio_settings.active_config.tessellation_steps,
        );
        let hazards = ut99_bsp::audit_loft_hazards(
            &self.loft_studio_settings.path_keyframes,
            self.loft_studio_settings.active_config.profile_radius,
        );
        let brush = ut99_bsp::generate_loft_brush(
            &self.loft_studio_settings.custom_profile,
            &self.loft_studio_settings.path_keyframes,
            &self.loft_studio_settings.active_config,
        );

        self.loft_studio_settings.cached_frames = frames;
        self.loft_studio_settings.cached_hazards = hazards;
        self.loft_studio_settings.cached_preview_brush = Some(brush);
    }

    pub fn add_loft_keyframe(&mut self, pos: glam::Vec3) {
        let old_config = self.loft_studio_settings.active_config.clone();
        let old_keyframes = self.loft_studio_settings.path_keyframes.clone();

        self.loft_studio_settings.path_keyframes.push(ut99_bsp::LoftPathKeyframe::new(pos));
        self.loft_studio_settings.selected_keyframe_idx = Some(self.loft_studio_settings.path_keyframes.len() - 1);
        self.recalculate_loft_mesh();

        self.history.push(crate::history::Action::UpdateLoftConfig {
            old_config,
            new_config: self.loft_studio_settings.active_config.clone(),
            old_keyframes,
            new_keyframes: self.loft_studio_settings.path_keyframes.clone(),
            description: format!("Added Loft Keyframe at ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z),
        });
        self.log(format!("Added Loft Keyframe at ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z));
    }

    pub fn remove_loft_keyframe(&mut self, idx: usize) {
        if self.loft_studio_settings.path_keyframes.len() <= 2 {
            self.log("Cannot remove keyframe: minimum 2 keyframes required for loft path.");
            return;
        }
        if idx < self.loft_studio_settings.path_keyframes.len() {
            let old_config = self.loft_studio_settings.active_config.clone();
            let old_keyframes = self.loft_studio_settings.path_keyframes.clone();

            self.loft_studio_settings.path_keyframes.remove(idx);
            self.loft_studio_settings.selected_keyframe_idx = Some(idx.saturating_sub(1));
            self.recalculate_loft_mesh();

            self.history.push(crate::history::Action::UpdateLoftConfig {
                old_config,
                new_config: self.loft_studio_settings.active_config.clone(),
                old_keyframes,
                new_keyframes: self.loft_studio_settings.path_keyframes.clone(),
                description: format!("Removed Loft Keyframe #{}", idx),
            });
            self.log(format!("Removed Loft Keyframe #{}", idx));
        }
    }

    pub fn commit_loft_to_builder_brush(&mut self) {
        self.recalculate_loft_mesh();
        if let Some(ref brush) = self.loft_studio_settings.cached_preview_brush {
            let old_builder = self.builder_brush.clone();
            let mut new_builder = brush.clone();
            new_builder.name = "ActiveBuilder".to_string();
            new_builder.csg_oper = ut99_bsp::CsgOperation::Active;
            self.builder_brush = new_builder.clone();

            self.history.push(crate::history::Action::CommitLoftedBrush {
                brush_index: None,
                brush: new_builder,
                is_builder_brush: true,
                old_builder_brush: Some(old_builder),
                description: "Committed Lofted Shape to Active Red Builder Brush".to_string(),
            });
            self.map_is_dirty = true;
            self.log("Committed lofted 3D geometry to Active Red Builder Brush.");
        }
    }

    pub fn commit_loft_to_world_brush(&mut self, csg: ut99_bsp::CsgOperation) -> usize {
        self.recalculate_loft_mesh();
        let mut new_brush = if let Some(ref brush) = self.loft_studio_settings.cached_preview_brush {
            brush.clone()
        } else {
            ut99_bsp::Brush::new("Loft_Pipe", csg)
        };

        new_brush.csg_oper = csg;
        new_brush.name = format!("Loft_{}_{}", self.loft_studio_settings.active_config.archetype.display_name().replace(' ', "_"), self.world_brushes.len());

        let idx = self.world_brushes.len();
        self.world_brushes.push(new_brush.clone());
        self.selected_brush_index = Some(idx);

        self.history.push(crate::history::Action::CommitLoftedBrush {
            brush_index: Some(idx),
            brush: new_brush,
            is_builder_brush: false,
            old_builder_brush: None,
            description: format!("Created World Loft Brush #{} ({:?})", idx, csg),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed lofted geometry as World Brush #{} with CSG {:?}", idx, csg));
        idx
    }

    // =========================================================================
    // Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio
    // =========================================================================

    pub fn apply_stair_archetype(&mut self, arch: ut99_bsp::StairArchetype) {
        let old_config = self.stairway_studio_settings.active_config.clone();
        let mut new_config = arch.default_config();
        new_config.texture_name = old_config.texture_name.clone();
        self.stairway_studio_settings.active_config = new_config.clone();

        self.recalculate_stair_geometry();

        self.history.push(crate::history::Action::UpdateStairConfig {
            old_config,
            new_config,
            description: format!("Applied Stair Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Stair Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_stair_geometry(&mut self) {
        let cfg = &self.stairway_studio_settings.active_config;
        let stair_brush = ut99_bsp::generate_stair_brush(cfg);
        let ramp_brush = if cfg.add_smooth_collision_hull {
            Some(ut99_bsp::generate_smooth_collision_ramp_brush(cfg))
        } else {
            None
        };
        let handrail_brush = if cfg.add_handrail_banister {
            Some(ut99_bsp::generate_handrail_brush(cfg))
        } else {
            None
        };
        let waypoints = ut99_bsp::generate_bot_path_nodes(cfg, glam::Vec3::ZERO);
        let metrics = ut99_bsp::calculate_stair_metrics(cfg);
        let hazards = ut99_bsp::audit_stair_safety(cfg);

        self.stairway_studio_settings.cached_stair_brush = Some(stair_brush);
        self.stairway_studio_settings.cached_ramp_brush = ramp_brush;
        self.stairway_studio_settings.cached_handrail_brush = handrail_brush;
        self.stairway_studio_settings.cached_waypoints = waypoints;
        self.stairway_studio_settings.cached_metrics = metrics;
        self.stairway_studio_settings.cached_hazards = hazards;
    }

    pub fn commit_stair_to_builder_brush(&mut self) {
        self.recalculate_stair_geometry();
        if let Some(ref brush) = self.stairway_studio_settings.cached_stair_brush {
            let old_builder = self.builder_brush.clone();
            let mut new_builder = brush.clone();
            new_builder.name = "ActiveBuilder".to_string();
            new_builder.csg_oper = ut99_bsp::CsgOperation::Active;
            self.builder_brush = new_builder.clone();

            self.history.push(crate::history::Action::CommitStairSetup {
                stair_brush_index: None,
                stair_brush: new_builder,
                ramp_brush_index: None,
                ramp_brush: None,
                is_builder_brush: true,
                old_builder_brush: Some(old_builder),
                description: "Committed Stairway Geometry to Active Red Builder Brush".to_string(),
            });
            self.map_is_dirty = true;
            self.log("Committed stairway geometry to Active Red Builder Brush.");
        }
    }

    pub fn commit_stair_and_ramp_to_world(&mut self, csg: ut99_bsp::CsgOperation) -> (usize, Option<usize>) {
        self.recalculate_stair_geometry();

        let mut stair_brush = if let Some(ref brush) = self.stairway_studio_settings.cached_stair_brush {
            brush.clone()
        } else {
            ut99_bsp::Brush::new("Stair_Brush", csg)
        };
        stair_brush.csg_oper = csg;
        let arch_name = self.stairway_studio_settings.active_config.archetype.display_name().replace(' ', "_");
        let stair_idx = self.world_brushes.len();
        stair_brush.name = format!("Stair_{}_{}", arch_name, stair_idx);
        self.world_brushes.push(stair_brush.clone());

        let mut ramp_idx = None;
        let mut committed_ramp = None;

        if let Some(ref ramp) = self.stairway_studio_settings.cached_ramp_brush {
            let mut r = ramp.clone();
            r.csg_oper = ut99_bsp::CsgOperation::Add;
            let ridx = self.world_brushes.len();
            r.name = format!("StairRamp_Hull_{}", ridx);
            self.world_brushes.push(r.clone());
            ramp_idx = Some(ridx);
            committed_ramp = Some(r);
        }

        self.selected_brush_index = Some(stair_idx);

        self.history.push(crate::history::Action::CommitStairSetup {
            stair_brush_index: Some(stair_idx),
            stair_brush,
            ramp_brush_index: ramp_idx,
            ramp_brush: committed_ramp,
            is_builder_brush: false,
            old_builder_brush: None,
            description: format!("Created World Stairway #{} with Ramp {:?}", stair_idx, ramp_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Stairway as World Brush #{} (Ramp: {:?})", stair_idx, ramp_idx));
        (stair_idx, ramp_idx)
    }

    // =========================================================================
    // Surface Clutter, Procedural Foliage & Decoration Scatter Studio
    // =========================================================================

    pub fn apply_scatter_archetype(&mut self, arch: ut99_bsp::ScatterArchetype) {
        let old_config = self.scatter_studio_settings.active_config.clone();
        let new_config = arch.default_config();
        self.scatter_studio_settings.active_config = new_config.clone();

        self.recalculate_scatter_preview();

        self.history.push(crate::history::Action::UpdateScatterConfig {
            old_config,
            new_config,
            description: format!("Applied Scatter Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Scatter Archetype: {}.", arch.display_name()));
    }

    pub fn reroll_scatter_seed(&mut self) {
        let mut old_cfg = self.scatter_studio_settings.active_config.clone();
        old_cfg.random_seed = old_cfg.random_seed.wrapping_add(1337);
        self.scatter_studio_settings.active_config = old_cfg;
        self.recalculate_scatter_preview();
    }

    pub fn clear_scatter_preview(&mut self) {
        self.scatter_studio_settings.preview_instances.clear();
        self.scatter_studio_settings.cached_metrics = ut99_bsp::ScatterMetrics {
            total_instances: 0,
            valid_clearance_count: 0,
            blocked_clearance_count: 0,
            average_spacing: 0.0,
            sampled_surface_area: 0.0,
        };
        self.scatter_studio_settings.cached_hazards.clear();
    }

    pub fn recalculate_scatter_preview(&mut self) {
        let mut target_polys = Vec::new();

        // 1. Gather polygons from selected surfaces or brushes
        if !self.selected_surfaces.is_empty() {
            for &(b_idx, s_idx) in &self.selected_surfaces {
                if let Some(brush) = self.world_brushes.get(b_idx) {
                    if let Some(poly) = brush.polys.get(s_idx) {
                        target_polys.push(poly.clone());
                    }
                }
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get(b_idx) {
                target_polys.extend(brush.polys.clone());
            }
        } else {
            // Collect all polygons across world brushes
            for brush in &self.world_brushes {
                target_polys.extend(brush.polys.clone());
            }
        }

        // 2. Gather bot navigation waypoints and player starts
        let mut bot_nodes = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let cn = actor.class_name.to_lowercase();
                if cn.contains("pathnode") || cn.contains("playerstart") || cn.contains("jumpspot") || cn.contains("inventoryspot") {
                    bot_nodes.push(actor.location);
                }
            }
        }

        let cfg = &self.scatter_studio_settings.active_config;
        let instances = ut99_bsp::generate_scatter_instances(&target_polys, cfg, &bot_nodes);
        let metrics = ut99_bsp::evaluate_scatter_metrics(&instances, &target_polys, cfg);
        let hazards = ut99_bsp::audit_scatter_hazards(&instances, cfg);

        self.scatter_studio_settings.preview_instances = instances;
        self.scatter_studio_settings.cached_metrics = metrics;
        self.scatter_studio_settings.cached_hazards = hazards;
    }

    pub fn commit_scatter_instances_to_world(&mut self) -> Vec<usize> {
        self.recalculate_scatter_preview();

        let instances: Vec<ut99_bsp::ScatterInstance> = self.scatter_studio_settings.preview_instances
            .iter()
            .filter(|i| i.is_valid_clearance)
            .cloned()
            .collect();

        if instances.is_empty() {
            self.log("No valid clutter instances to commit (all blocked or none generated).");
            return Vec::new();
        }

        let mut added_indices = Vec::new();
        let mut added_actors = Vec::new();

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let cfg = self.scatter_studio_settings.active_config.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            let start_id = unr.actors.len();
            for (idx, inst) in instances.iter().enumerate() {
                let mut act = ut99_format::ActorRecord::default();
                act.name = format!("{}_{}", cfg.mesh_name, start_id + idx);
                act.class_name = cfg.class_name.clone();
                act.location = inst.position;

                // Convert degrees to Unreal angle units (0..65535)
                let deg_to_unr = 65536.0 / 360.0;
                let pitch = (inst.rotation.x * deg_to_unr) as i32;
                let yaw = (inst.rotation.y * deg_to_unr) as i32;
                let roll = (inst.rotation.z * deg_to_unr) as i32;
                act.rotation = (pitch, yaw, roll);

                act.main_scale = inst.scale;
                act.group = "ScatteredClutter".to_string();

                let new_idx = unr.actors.len();
                unr.actors.push(act.clone());
                added_indices.push(new_idx);
                added_actors.push(act);
            }
        }

        self.history.push(crate::history::Action::CommitScatterBatch {
            added_actor_indices: added_indices.clone(),
            actors: added_actors,
            description: format!("Committed {} Scattered Clutter Instances ({})", added_indices.len(), cfg.archetype.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed {} scattered actors to level (Group: 'ScatteredClutter').", added_indices.len()));
        self.clear_scatter_preview();

        added_indices
    }

    // =========================================================================
    // Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio
    // =========================================================================

    pub fn apply_light_shaft_archetype(&mut self, arch: ut99_bsp::LightShaftArchetype) {
        let old_config = self.light_shaft_settings.active_config.clone();
        let new_config = arch.default_config();
        self.light_shaft_settings.active_config = new_config.clone();

        self.recalculate_light_shaft_geometry();

        self.history.push(crate::history::Action::UpdateLightShaftConfig {
            old_config,
            new_config,
            description: format!("Applied Light Shaft Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Light Shaft Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_light_shaft_geometry(&mut self) {
        let cfg = &self.light_shaft_settings.active_config;
        let brush = ut99_bsp::generate_light_shaft_brush(cfg);
        let motes = ut99_bsp::generate_dust_motes(cfg, 35);
        let hazards = ut99_bsp::audit_light_shaft_safety(cfg);

        self.light_shaft_settings.cached_preview_brush = Some(brush);
        self.light_shaft_settings.cached_dust_motes = motes;
        self.light_shaft_settings.cached_hazards = hazards;
    }

    pub fn commit_light_shaft_to_builder_brush(&mut self) {
        self.recalculate_light_shaft_geometry();
        if let Some(ref brush) = self.light_shaft_settings.cached_preview_brush {
            let old_builder = self.builder_brush.clone();
            self.builder_brush = brush.clone();

            self.history.push(crate::history::Action::CommitLightShaftBrush {
                brush_index: None,
                brush: brush.clone(),
                is_builder_brush: true,
                old_builder_brush: Some(old_builder),
                description: format!("Set Active Builder Brush to Volumetric Light Shaft ({})", self.light_shaft_settings.active_config.archetype.display_name()),
            });

            self.map_is_dirty = true;
            self.log(format!("Committed Volumetric Light Shaft to Red Active Builder Brush ({} polygons).", brush.polys.len()));
        }
    }

    pub fn commit_light_shaft_to_world(&mut self) -> usize {
        self.recalculate_light_shaft_geometry();
        let brush = self.light_shaft_settings.cached_preview_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_light_shaft_brush(&self.light_shaft_settings.active_config));

        let new_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        self.history.push(crate::history::Action::CommitLightShaftBrush {
            brush_index: Some(new_idx),
            brush: brush.clone(),
            is_builder_brush: false,
            old_builder_brush: None,
            description: format!("Added Volumetric Light Shaft Brush #{} ({})", new_idx, self.light_shaft_settings.active_config.archetype.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Added Volumetric Light Shaft to Level as World Brush #{} ({} polygons, Non-Solid Translucent).", new_idx, brush.polys.len()));

        new_idx
    }

    // =========================================================================
    // Spline Rail, Kinetic Track Mover & Rollercoaster Studio
    // =========================================================================

    pub fn apply_track_archetype(&mut self, arch: ut99_bsp::TrackArchetype) {
        let old_config = self.track_studio_settings.active_config.clone();
        let new_config = arch.default_config();
        self.track_studio_settings.active_config = new_config.clone();

        self.recalculate_track_preview();

        self.history.push(crate::history::Action::UpdateTrackConfig {
            old_config,
            new_config,
            description: format!("Applied Track Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Track Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_track_preview(&mut self) {
        let cfg = &self.track_studio_settings.active_config;
        let (rails, ties) = ut99_bsp::generate_track_brushes(cfg);
        let keyframes = ut99_bsp::bake_mover_keyframes(cfg);
        let waypoints = ut99_bsp::generate_station_bot_waypoints(cfg);
        let hazards = ut99_bsp::audit_track_safety(cfg);

        self.track_studio_settings.cached_rails_brush = Some(rails);
        self.track_studio_settings.cached_ties_brush = Some(ties);
        self.track_studio_settings.cached_keyframes = keyframes;
        self.track_studio_settings.cached_waypoints = waypoints;
        self.track_studio_settings.cached_hazards = hazards;
    }

    pub fn scrub_track_timeline(&mut self, t: f32) {
        self.track_studio_settings.timeline_t = t.clamp(0.0, 1.0);
    }

    pub fn commit_track_setup(&mut self) -> (usize, usize, Option<usize>, Vec<usize>) {
        self.recalculate_track_preview();

        let rails_brush = self.track_studio_settings.cached_rails_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_track_brushes(&self.track_studio_settings.active_config).0);
        let ties_brush = self.track_studio_settings.cached_ties_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_track_brushes(&self.track_studio_settings.active_config).1);

        let rails_idx = self.world_brushes.len();
        self.world_brushes.push(rails_brush.clone());

        let ties_idx = self.world_brushes.len();
        self.world_brushes.push(ties_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_actor = None;
        let mut waypoint_indices = Vec::new();
        let mut waypoint_actors = Vec::new();

        let cfg = self.track_studio_settings.active_config.clone();
        let keyframes = self.track_studio_settings.cached_keyframes.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. Spawn Mover Actor at Key0 and register EditorMover with brush & keyframes
            if let Some(k0) = keyframes.first() {
                let mut mover = ut99_format::ActorRecord::default();
                mover.name = format!("TrackMover_{}", unr.actors.len());
                mover.class_name = "Engine.Mover".to_string();
                mover.location = k0.position;
                mover.rotation = k0.unreal_rotation;
                mover.group = "TrackMovers".to_string();

                let mut carriage_brush = build_cube(
                    &mover.name,
                    CsgOperation::Add,
                    Vec3::new(128.0, 128.0, 24.0),
                );
                carriage_brush.location = k0.position;
                let mut ed_mover = EditorMover::new(carriage_brush);
                ed_mover.name = mover.name.clone();
                ed_mover.mover_class = "Mover".to_string();
                const ROT_TO_DEG: f32 = 360.0 / 65536.0;
                for (k_i, kf) in keyframes.iter().take(8).enumerate() {
                    ed_mover.keyframes[k_i] = Some(MoverKeyframe {
                        location: kf.position,
                        rotation: Vec3::new(
                            kf.unreal_rotation.0 as f32 * ROT_TO_DEG,
                            kf.unreal_rotation.1 as f32 * ROT_TO_DEG,
                            kf.unreal_rotation.2 as f32 * ROT_TO_DEG,
                        ),
                    });
                }
                self.movers.push(ed_mover);

                let idx = unr.actors.len();
                unr.actors.push(mover.clone());
                mover_idx = Some(idx);
                mover_actor = Some(mover);
            }

            // 2. Spawn Station Bot Waypoints (LiftExit / LiftCenter)
            for wp in &self.track_studio_settings.cached_waypoints {
                let mut act = ut99_format::ActorRecord::default();
                act.name = format!("{}_{}", wp.name, unr.actors.len());
                act.class_name = wp.class_name.clone();
                act.location = wp.location;
                act.group = "TrackWaypoints".to_string();

                let idx = unr.actors.len();
                unr.actors.push(act.clone());
                waypoint_indices.push(idx);
                waypoint_actors.push(act);
            }
            unr.sync_actor_views();
        }

        self.history.push(crate::history::Action::CommitTrackSetup {
            rails_brush_index: Some(rails_idx),
            ties_brush_index: Some(ties_idx),
            rails_brush,
            ties_brush,
            mover_actor_index: mover_idx,
            mover_actor,
            waypoint_actor_indices: waypoint_indices.clone(),
            waypoint_actors,
            description: format!("Committed Track Setup ({}, {} Keyframes)", cfg.archetype.display_name(), keyframes.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Spline Track to World: Rails Brush #{}, Ties Brush #{}, Mover #{} ({} Keyframes).",
            rails_idx, ties_idx, mover_idx.unwrap_or(0), keyframes.len()));

        (rails_idx, ties_idx, mover_idx, waypoint_indices)
    }

    // =========================================================================
    // Security CCTV Camera, Live Video Monitor & Surveillance Studio
    // =========================================================================

    pub fn apply_surveillance_archetype(&mut self, arch: ut99_bsp::SurveillanceArchetype) {
        let old_config = self.surveillance_settings.active_config.clone();
        let new_config = arch.default_config();
        self.surveillance_settings.active_config = new_config.clone();

        self.recalculate_surveillance_preview();

        self.history.push(crate::history::Action::UpdateSurveillanceConfig {
            old_config,
            new_config,
            description: format!("Applied Surveillance Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Surveillance Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_surveillance_preview(&mut self) {
        let cfg = &self.surveillance_settings.active_config;
        let brush = ut99_bsp::generate_monitor_brush(cfg);

        let (cur_yaw, cur_pitch) = ut99_bsp::sample_patrol_orientation(
            &cfg.patrol,
            cfg.camera_mount.base_yaw_deg,
            cfg.camera_mount.base_pitch_deg,
            self.surveillance_settings.simulated_time_seconds,
        );

        let aspect = cfg.monitor.aspect_ratio_w as f32 / cfg.monitor.aspect_ratio_h.max(1) as f32;
        let frustum = ut99_bsp::calculate_camera_frustum(&cfg.camera_mount, cur_yaw, cur_pitch, aspect);
        let hazards = ut99_bsp::audit_surveillance_safety(cfg, &self.world_brushes);

        self.surveillance_settings.cached_monitor_brush = Some(brush);
        self.surveillance_settings.cached_frustum = Some(frustum);
        self.surveillance_settings.cached_hazards = hazards;
    }

    pub fn scrub_surveillance_time(&mut self, t: f32) {
        self.surveillance_settings.simulated_time_seconds = t.max(0.0);
        self.recalculate_surveillance_preview();
    }

    pub fn commit_surveillance_setup_to_world(&mut self) -> (usize, Option<usize>, Option<usize>) {
        self.recalculate_surveillance_preview();

        let monitor_brush = self.surveillance_settings.cached_monitor_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_monitor_brush(&self.surveillance_settings.active_config));

        let monitor_idx = self.world_brushes.len();
        self.world_brushes.push(monitor_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut camera_idx = None;
        let mut camera_actor = None;
        let mut screen_idx = None;
        let mut screen_actor = None;

        let cfg = self.surveillance_settings.active_config.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. Spawn Camera Actor
            let deg_to_unr = 65536.0 / 360.0;
            let pitch = (cfg.camera_mount.base_pitch_deg * deg_to_unr) as i32;
            let yaw = (cfg.camera_mount.base_yaw_deg * deg_to_unr) as i32;
            let roll = (cfg.camera_mount.base_roll_deg * deg_to_unr) as i32;

            let mut cam = ut99_format::ActorRecord::default();
            cam.name = format!("Camera_{}", unr.actors.len());
            cam.class_name = "Engine.Camera".to_string();
            cam.location = cfg.camera_mount.position;
            cam.rotation = (pitch, yaw, roll);
            cam.tag = cfg.camera_tag.clone();
            cam.group = "SurveillanceCameras".to_string();

            let c_idx = unr.actors.len();
            unr.actors.push(cam.clone());
            camera_idx = Some(c_idx);
            camera_actor = Some(cam);

            // 2. Spawn Screen / ScriptedTexture Actor
            let mut scr = ut99_format::ActorRecord::default();
            scr.name = format!("Screen_{}", unr.actors.len());
            scr.class_name = "Screen.Screen".to_string();
            scr.location = Vec3::ZERO;
            scr.event = cfg.screen_event.clone();
            scr.tag = cfg.camera_tag.clone();
            scr.group = "SurveillanceMonitors".to_string();

            let s_idx = unr.actors.len();
            unr.actors.push(scr.clone());
            screen_idx = Some(s_idx);
            screen_actor = Some(scr);
        }

        self.history.push(crate::history::Action::CommitSurveillanceSetup {
            monitor_brush_index: Some(monitor_idx),
            monitor_brush,
            camera_actor_index: camera_idx,
            camera_actor,
            screen_actor_index: screen_idx,
            screen_actor,
            description: format!("Committed Surveillance Network ({}, Tag: {})", cfg.archetype.display_name(), cfg.camera_tag),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Surveillance Setup to World: Monitor Brush #{}, Camera Actor #{}, Screen Actor #{}.",
            monitor_idx, camera_idx.unwrap_or(0), screen_idx.unwrap_or(0)));

        (monitor_idx, camera_idx, screen_idx)
    }

    // =========================================================================
    // Security Laser Grid, Tripwire Alarm & Energy Barrier Studio
    // =========================================================================

    pub fn apply_laser_archetype(&mut self, arch: ut99_bsp::LaserArchetype) {
        let old_config = self.laser_grid_settings.active_config.clone();
        let new_config = arch.default_config();
        self.laser_grid_settings.active_config = new_config.clone();

        self.recalculate_laser_preview();

        self.history.push(crate::history::Action::UpdateLaserConfig {
            old_config,
            new_config,
            description: format!("Applied Laser Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Laser Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_laser_preview(&mut self) {
        let cfg = &self.laser_grid_settings.active_config;
        let beams = ut99_bsp::generate_laser_beams(cfg, self.laser_grid_settings.simulated_time_seconds);
        let (beam_b, diode_b) = ut99_bsp::generate_laser_brush_geometry(cfg, &beams);
        let triggers = ut99_bsp::generate_beam_trigger_specs(cfg, &beams);
        let hazards = ut99_bsp::audit_laser_grid_safety(cfg, &beams, &self.world_brushes);

        self.laser_grid_settings.cached_beams = beams;
        self.laser_grid_settings.cached_beam_brush = Some(beam_b);
        self.laser_grid_settings.cached_diode_brush = Some(diode_b);
        self.laser_grid_settings.cached_trigger_specs = triggers;
        self.laser_grid_settings.cached_hazards = hazards;
    }

    pub fn scrub_laser_time(&mut self, t: f32) {
        self.laser_grid_settings.simulated_time_seconds = t.max(0.0);
        self.recalculate_laser_preview();
    }

    pub fn commit_laser_grid_to_world(&mut self) -> (usize, usize, Vec<usize>) {
        self.recalculate_laser_preview();

        let beam_brush = self.laser_grid_settings.cached_beam_brush
            .clone()
            .unwrap_or_else(|| {
                let beams = ut99_bsp::generate_laser_beams(&self.laser_grid_settings.active_config, 0.0);
                ut99_bsp::generate_laser_brush_geometry(&self.laser_grid_settings.active_config, &beams).0
            });

        let diode_brush = self.laser_grid_settings.cached_diode_brush
            .clone()
            .unwrap_or_else(|| {
                let beams = ut99_bsp::generate_laser_beams(&self.laser_grid_settings.active_config, 0.0);
                ut99_bsp::generate_laser_brush_geometry(&self.laser_grid_settings.active_config, &beams).1
            });

        let beam_idx = self.world_brushes.len();
        self.world_brushes.push(beam_brush.clone());

        let diode_idx = self.world_brushes.len();
        self.world_brushes.push(diode_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut trigger_indices = Vec::new();
        let mut trigger_actors = Vec::new();

        let cfg = self.laser_grid_settings.active_config.clone();
        let trigger_specs = self.laser_grid_settings.cached_trigger_specs.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            for spec in &trigger_specs {
                let mut trig = ut99_format::ActorRecord::default();
                trig.name = format!("{}_{}", spec.name, unr.actors.len());
                trig.class_name = "Engine.Trigger".to_string();
                trig.location = spec.location;
                trig.event = spec.event.clone();
                trig.tag = "LaserGridTrigger".to_string();
                trig.group = "LaserSecurity".to_string();

                let idx = unr.actors.len();
                unr.actors.push(trig.clone());
                trigger_indices.push(idx);
                trigger_actors.push(trig);
            }
        }

        self.history.push(crate::history::Action::CommitLaserGrid {
            beam_brush_index: Some(beam_idx),
            diode_brush_index: Some(diode_idx),
            beam_brush,
            diode_brush,
            trigger_actor_indices: trigger_indices.clone(),
            trigger_actors,
            description: format!("Committed Laser Security Grid ({}, {} Beams)", cfg.archetype.display_name(), trigger_specs.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Laser Grid to World: Ribbon Brush #{}, Diode Brush #{}, {} Trigger Actors.",
            beam_idx, diode_idx, trigger_indices.len()));

        (beam_idx, diode_idx, trigger_indices)
    }

    // =========================================================================
    // Environmental Ventilation Fan, Air Current & Wind Tunnel Studio
    // =========================================================================

    pub fn apply_ventilator_archetype(&mut self, arch: ut99_bsp::VentilatorArchetype) {
        let old_config = self.ventilator_settings.active_config.clone();
        let new_config = arch.default_config();
        self.ventilator_settings.active_config = new_config.clone();

        self.recalculate_ventilator_preview();

        self.history.push(crate::history::Action::UpdateVentilatorConfig {
            old_config,
            new_config,
            description: format!("Applied Ventilator Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Ventilator Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_ventilator_preview(&mut self) {
        let cfg = &self.ventilator_settings.active_config;
        let (fan_b, house_b) = ut99_bsp::generate_ventilator_brush(cfg, self.ventilator_settings.simulated_time_seconds);
        let lift = ut99_bsp::calculate_aerodynamic_lift(cfg);
        let streams = ut99_bsp::generate_wind_streamlines(cfg, self.ventilator_settings.simulated_time_seconds);
        let hazards = ut99_bsp::audit_ventilator_safety(cfg, &self.world_brushes);

        self.ventilator_settings.cached_fan_brush = Some(fan_b);
        self.ventilator_settings.cached_housing_brush = Some(house_b);
        self.ventilator_settings.cached_lift = lift;
        self.ventilator_settings.cached_streamlines = streams;
        self.ventilator_settings.cached_hazards = hazards;
    }

    pub fn scrub_ventilator_time(&mut self, t: f32) {
        self.ventilator_settings.simulated_time_seconds = t.max(0.0);
        self.recalculate_ventilator_preview();
    }

    pub fn commit_ventilator_to_world(&mut self) -> (usize, usize, Option<usize>, Option<usize>, Option<usize>) {
        self.recalculate_ventilator_preview();

        let fan_brush = self.ventilator_settings.cached_fan_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_ventilator_brush(&self.ventilator_settings.active_config, 0.0).0
            });

        let housing_brush = self.ventilator_settings.cached_housing_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_ventilator_brush(&self.ventilator_settings.active_config, 0.0).1
            });

        let fan_idx = self.world_brushes.len();
        self.world_brushes.push(fan_brush.clone());

        let housing_idx = self.world_brushes.len();
        self.world_brushes.push(housing_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_act = None;
        let mut zone_idx = None;
        let mut zone_act = None;
        let mut trig_idx = None;
        let mut trig_act = None;

        let cfg = self.ventilator_settings.active_config.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. RotatingMover actor for spinning the fan brush + EditorMover registration
            let mut mover = ut99_format::ActorRecord::default();
            mover.name = format!("FanMover_{}", unr.actors.len());
            mover.class_name = "Engine.RotatingMover".to_string();
            mover.location = cfg.position;
            mover.tag = "VentilatorFan".to_string();
            mover.group = "Ventilation".to_string();

            let mut ed_mover = EditorMover::new(fan_brush.clone());
            ed_mover.name = mover.name.clone();
            ed_mover.mover_class = "RotatingMover".to_string();
            ed_mover.tag = mover.tag.clone();
            self.movers.push(ed_mover);

            let m_idx = unr.actors.len();
            unr.actors.push(mover.clone());
            mover_idx = Some(m_idx);
            mover_act = Some(mover);

            // 2. ZoneInfo actor with ZoneVelocity for aerodynamic airflow
            let flow_sign = match cfg.air_current.flow_direction {
                ut99_bsp::AirFlowDirection::Blowing => 1.0,
                ut99_bsp::AirFlowDirection::Sucking => -1.0,
                ut99_bsp::AirFlowDirection::Neutral => 0.0,
            };
            let flow_vec = cfg.rotation_axis.normalize_or_zero() * (cfg.air_current.zone_velocity_strength * flow_sign);
            let mut zone = ut99_format::ActorRecord::default();
            zone.name = format!("FanZoneInfo_{}", unr.actors.len());
            zone.class_name = "Engine.ZoneInfo".to_string();
            zone.location = cfg.position + flow_vec.normalize_or_zero() * (cfg.air_current.effective_range * 0.5);
            zone.tag = "FanAirZone".to_string();
            zone.group = "Ventilation".to_string();
            let z_idx = unr.actors.len();
            unr.actors.push(zone.clone());
            zone_idx = Some(z_idx);
            zone_act = Some(zone);

            // 3. Optional Lethal Trigger actor at the blade plane
            if cfg.lethality.is_lethal_contact {
                let mut trig = ut99_format::ActorRecord::default();
                trig.name = format!("BladeHazard_{}", unr.actors.len());
                trig.class_name = "Engine.Trigger".to_string();
                trig.location = cfg.position;
                trig.tag = "FanHazard".to_string();
                trig.group = "Ventilation".to_string();
                let t_idx = unr.actors.len();
                unr.actors.push(trig.clone());
                trig_idx = Some(t_idx);
                trig_act = Some(trig);
            }
            unr.sync_actor_views();
        }

        self.history.push(crate::history::Action::CommitVentilatorSetup {
            fan_brush_index: Some(fan_idx),
            housing_brush_index: Some(housing_idx),
            fan_brush,
            housing_brush,
            mover_actor_index: mover_idx,
            mover_actor: mover_act,
            zone_actor_index: zone_idx,
            zone_actor: zone_act,
            trigger_actor_index: trig_idx,
            trigger_actor: trig_act,
            description: format!("Committed Industrial Ventilator ({}, {} RPM)", cfg.archetype.display_name(), cfg.spin_rpm),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Ventilator Setup to World: Fan Brush #{}, Housing Brush #{}, Mover Actor #{:?}, Zone Actor #{:?}.",
            fan_idx, housing_idx, mover_idx, zone_idx));

        (fan_idx, housing_idx, mover_idx, zone_idx, trig_idx)
    }

    // =========================================================================
    // Industrial Conveyor Belt, Treadmill & Surface Velocity Studio
    // =========================================================================

    pub fn apply_conveyor_archetype(&mut self, arch: ut99_bsp::ConveyorArchetype) {
        let old_config = self.conveyor_settings.active_config.clone();
        let new_config = arch.default_config();
        self.conveyor_settings.active_config = new_config.clone();

        self.recalculate_conveyor_preview();

        self.history.push(crate::history::Action::UpdateConveyorConfig {
            old_config,
            new_config,
            description: format!("Applied Conveyor Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Conveyor Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_conveyor_preview(&mut self) {
        let cfg = &self.conveyor_settings.active_config;
        let (deck_b, rollers_b, frame_b) = ut99_bsp::generate_conveyor_brushes(cfg);
        let metrics = ut99_bsp::calculate_conveyor_sync_metrics(cfg);
        let waypoints = ut99_bsp::generate_conveyor_bot_waypoints(cfg);
        let hazards = ut99_bsp::audit_conveyor_safety(cfg, &self.world_brushes);

        self.conveyor_settings.cached_deck_brush = Some(deck_b);
        self.conveyor_settings.cached_rollers_brush = Some(rollers_b);
        self.conveyor_settings.cached_frame_brush = Some(frame_b);
        self.conveyor_settings.cached_metrics = metrics;
        self.conveyor_settings.cached_waypoints = waypoints;
        self.conveyor_settings.cached_hazards = hazards;
    }

    pub fn scrub_conveyor_time(&mut self, t: f32) {
        self.conveyor_settings.simulated_time_seconds = t.max(0.0);
        self.recalculate_conveyor_preview();
    }

    pub fn commit_conveyor_to_world(&mut self) -> (usize, usize, usize, Option<usize>, Vec<usize>) {
        self.recalculate_conveyor_preview();

        let deck_brush = self.conveyor_settings.cached_deck_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_conveyor_brushes(&self.conveyor_settings.active_config).0
            });

        let rollers_brush = self.conveyor_settings.cached_rollers_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_conveyor_brushes(&self.conveyor_settings.active_config).1
            });

        let frame_brush = self.conveyor_settings.cached_frame_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_conveyor_brushes(&self.conveyor_settings.active_config).2
            });

        let deck_idx = self.world_brushes.len();
        self.world_brushes.push(deck_brush.clone());

        let rollers_idx = self.world_brushes.len();
        self.world_brushes.push(rollers_brush.clone());

        let frame_idx = self.world_brushes.len();
        self.world_brushes.push(frame_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut zone_idx = None;
        let mut zone_act = None;
        let mut waypoint_indices = Vec::new();
        let mut waypoint_actors = Vec::new();

        let cfg = self.conveyor_settings.active_config.clone();
        let _metrics = self.conveyor_settings.cached_metrics.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. ZoneInfo actor with ZoneVelocity matching synchronized linear velocity
            let mut zone = ut99_format::ActorRecord::default();
            zone.name = format!("ConveyorZoneInfo_{}", unr.actors.len());
            zone.class_name = "Engine.ZoneInfo".to_string();
            zone.location = cfg.position + Vec3::new(0.0, 0.0, cfg.deck.thickness * 0.5 + 24.0);
            zone.tag = "ConveyorZone".to_string();
            zone.group = "Conveyors".to_string();
            let z_idx = unr.actors.len();
            unr.actors.push(zone.clone());
            zone_idx = Some(z_idx);
            zone_act = Some(zone);

            // 2. Bot Waypoint PathNodes at intake and discharge
            if cfg.bot_nav.generate_bot_waypoints {
                let (intake, discharge) = &self.conveyor_settings.cached_waypoints;

                let mut in_act = ut99_format::ActorRecord::default();
                in_act.name = format!("{}_{}", intake.name, unr.actors.len());
                in_act.class_name = "Engine.PathNode".to_string();
                in_act.location = intake.location;
                in_act.tag = "ConveyorIntake".to_string();
                in_act.group = "Conveyors".to_string();
                let in_idx = unr.actors.len();
                unr.actors.push(in_act.clone());
                waypoint_indices.push(in_idx);
                waypoint_actors.push(in_act);

                let mut out_act = ut99_format::ActorRecord::default();
                out_act.name = format!("{}_{}", discharge.name, unr.actors.len());
                out_act.class_name = "Engine.PathNode".to_string();
                out_act.location = discharge.location;
                out_act.tag = "ConveyorDischarge".to_string();
                out_act.group = "Conveyors".to_string();
                let out_idx = unr.actors.len();
                unr.actors.push(out_act.clone());
                waypoint_indices.push(out_idx);
                waypoint_actors.push(out_act);
            }
        }

        self.history.push(crate::history::Action::CommitConveyorSetup {
            deck_brush_index: Some(deck_idx),
            rollers_brush_index: Some(rollers_idx),
            frame_brush_index: Some(frame_idx),
            deck_brush,
            rollers_brush,
            frame_brush,
            zone_actor_index: zone_idx,
            zone_actor: zone_act,
            waypoint_actor_indices: waypoint_indices.clone(),
            waypoint_actors,
            description: format!("Committed Industrial Conveyor ({}, {:.0} uu/s)", cfg.archetype.display_name(), cfg.kinematics.speed_uu_per_sec),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Conveyor Setup to World: Deck Brush #{}, Rollers Brush #{}, Frame Brush #{}, Zone Actor #{:?}, {} Waypoints.",
            deck_idx, rollers_idx, frame_idx, zone_idx, waypoint_indices.len()));

        (deck_idx, rollers_idx, frame_idx, zone_idx, waypoint_indices)
    }

    // =========================================================================
    // Automated Defense Turret, Sentry Cannon & Emplacement Studio
    // =========================================================================

    pub fn apply_turret_archetype(&mut self, arch: ut99_bsp::TurretArchetype) {
        let old_config = self.turret_settings.active_config.clone();
        let new_config = arch.default_config();
        self.turret_settings.active_config = new_config.clone();

        self.recalculate_turret_preview();

        self.history.push(crate::history::Action::UpdateTurretConfig {
            old_config,
            new_config,
            description: format!("Applied Turret Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Turret Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_turret_preview(&mut self) {
        let cfg = &self.turret_settings.active_config;
        let (mount_b, shield_b) = ut99_bsp::generate_turret_mount_brushes(cfg);
        let metrics = ut99_bsp::calculate_turret_tracking_metrics(cfg, self.turret_settings.simulated_time_seconds);
        let waypoints = ut99_bsp::generate_turret_bot_waypoints(cfg, &self.world_brushes);
        let hazards = ut99_bsp::audit_turret_safety(cfg, &self.world_brushes);

        self.turret_settings.cached_mount_brush = Some(mount_b);
        self.turret_settings.cached_shield_brush = Some(shield_b);
        self.turret_settings.cached_metrics = metrics;
        self.turret_settings.cached_waypoints = waypoints;
        self.turret_settings.cached_hazards = hazards;
    }

    pub fn scrub_turret_time(&mut self, t: f32) {
        self.turret_settings.simulated_time_seconds = t.max(0.0);
        self.recalculate_turret_preview();
    }

    pub fn commit_turret_to_world(&mut self) -> (usize, usize, Option<usize>, Vec<usize>) {
        self.recalculate_turret_preview();

        let mount_brush = self.turret_settings.cached_mount_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_turret_mount_brushes(&self.turret_settings.active_config).0
            });

        let shield_brush = self.turret_settings.cached_shield_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_turret_mount_brushes(&self.turret_settings.active_config).1
            });

        let mount_idx = self.world_brushes.len();
        self.world_brushes.push(mount_brush.clone());

        let shield_idx = self.world_brushes.len();
        self.world_brushes.push(shield_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut turret_idx = None;
        let mut turret_act = None;
        let mut waypoint_indices = Vec::new();
        let mut waypoint_actors = Vec::new();

        let cfg = self.turret_settings.active_config.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. Botpack.TeamCannon stationary pawn actor
            let mut turret = ut99_format::ActorRecord::default();
            turret.name = format!("{}_{}", cfg.tag, unr.actors.len());
            turret.class_name = "Botpack.TeamCannon".to_string();
            let mount_up = match cfg.mount.mount_type {
                ut99_bsp::TurretMountType::FloorPedestal => Vec3::Z,
                ut99_bsp::TurretMountType::CeilingSuspended => -Vec3::Z,
                ut99_bsp::TurretMountType::WallBracket => Vec3::Z,
            };
            turret.location = cfg.position + mount_up * cfg.mount.pedestal_height;
            turret.tag = cfg.tag.clone();
            turret.group = "Defenses".to_string();
            let t_idx = unr.actors.len();
            unr.actors.push(turret.clone());
            turret_idx = Some(t_idx);
            turret_act = Some(turret);

            // 2. Bot Evasion & Vantage PathNodes
            for wp in &self.turret_settings.cached_waypoints {
                let mut node = ut99_format::ActorRecord::default();
                node.name = format!("{}_{}", wp.name, unr.actors.len());
                node.class_name = "Engine.PathNode".to_string();
                node.location = wp.location;
                node.tag = format!("{}_Waypoint", cfg.tag);
                node.group = "Defenses".to_string();
                let n_idx = unr.actors.len();
                unr.actors.push(node.clone());
                waypoint_indices.push(n_idx);
                waypoint_actors.push(node);
            }
        }

        self.history.push(crate::history::Action::CommitTurretSetup {
            mount_brush_index: Some(mount_idx),
            shield_brush_index: Some(shield_idx),
            mount_brush,
            shield_brush,
            turret_actor_index: turret_idx,
            turret_actor: turret_act,
            waypoint_actor_indices: waypoint_indices.clone(),
            waypoint_actors,
            description: format!("Committed Auto Defense Turret ({}, {})", cfg.archetype.display_name(), cfg.team.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Turret Setup to World: Mount Brush #{}, Shield Brush #{}, Turret Actor #{:?}, {} Waypoints.",
            mount_idx, shield_idx, turret_idx, waypoint_indices.len()));

        (mount_idx, shield_idx, turret_idx, waypoint_indices)
    }

    // =========================================================================
    // Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
    // =========================================================================

    pub fn apply_soundscape_archetype(&mut self, arch: ut99_bsp::SoundscapeArchetype) {
        let old_config = self.soundscape_settings.active_config.clone();
        let new_config = arch.default_config();
        self.soundscape_settings.active_config = new_config.clone();

        self.recalculate_soundscape_preview();

        self.history.push(crate::history::Action::UpdateSoundscapeConfig {
            old_config,
            new_config,
            description: format!("Applied Soundscape Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Soundscape Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_soundscape_preview(&mut self) {
        let cfg = &self.soundscape_settings.active_config;
        let metrics = ut99_bsp::calculate_soundscape_metrics(cfg);
        let zone_extent = (metrics.outer_radius_uu * 0.75).max(256.0);
        let zone_b = ut99_bsp::generate_acoustic_volume_brush(cfg, zone_extent);
        let hazards = ut99_bsp::audit_soundscape_safety(cfg, &self.world_brushes);

        let probe_loc = cfg.position + Vec3::new(self.soundscape_settings.probe_listener_distance_uu, 0.0, 0.0);
        let probe = ut99_bsp::sample_sound_propagation(cfg, probe_loc, &self.world_brushes);

        self.soundscape_settings.cached_zone_brush = Some(zone_b);
        self.soundscape_settings.cached_metrics = metrics;
        self.soundscape_settings.cached_hazards = hazards;
        self.soundscape_settings.cached_probe_sample = probe;
    }

    pub fn scrub_soundscape_probe(&mut self, dist_uu: f32) {
        self.soundscape_settings.probe_listener_distance_uu = dist_uu.max(0.0);
        let cfg = &self.soundscape_settings.active_config;
        let probe_loc = cfg.position + Vec3::new(self.soundscape_settings.probe_listener_distance_uu, 0.0, 0.0);
        self.soundscape_settings.cached_probe_sample = ut99_bsp::sample_sound_propagation(cfg, probe_loc, &self.world_brushes);
    }

    pub fn commit_soundscape_to_world(&mut self) -> (usize, Option<usize>, Option<usize>) {
        self.recalculate_soundscape_preview();

        let metrics = &self.soundscape_settings.cached_metrics;
        let zone_extent = (metrics.outer_radius_uu * 0.75).max(256.0);
        let zone_brush = self.soundscape_settings.cached_zone_brush
            .clone()
            .unwrap_or_else(|| {
                ut99_bsp::generate_acoustic_volume_brush(&self.soundscape_settings.active_config, zone_extent)
            });

        let zone_b_idx = self.world_brushes.len();
        self.world_brushes.push(zone_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut ambient_idx = None;
        let mut ambient_act = None;
        let mut zone_idx = None;
        let mut zone_act = None;

        let cfg = self.soundscape_settings.active_config.clone();

        if let Some(ref mut unr) = self.loaded_unr {
            // 1. Engine.AmbientSound actor
            let mut snd = ut99_format::ActorRecord::default();
            snd.name = format!("{}_{}", cfg.tag, unr.actors.len());
            snd.class_name = "Engine.AmbientSound".to_string();
            snd.location = cfg.position;
            snd.tag = cfg.tag.clone();
            snd.group = "Acoustics".to_string();
            let s_idx = unr.actors.len();
            unr.actors.push(snd.clone());
            ambient_idx = Some(s_idx);
            ambient_act = Some(snd);

            // 2. Engine.ZoneInfo acoustic room actor
            let mut zinfo = ut99_format::ActorRecord::default();
            zinfo.name = format!("{}_{}", cfg.zone_tag, unr.actors.len());
            zinfo.class_name = "Engine.ZoneInfo".to_string();
            zinfo.location = cfg.position + Vec3::new(0.0, 0.0, 32.0);
            zinfo.tag = cfg.zone_tag.clone();
            zinfo.group = "Acoustics".to_string();
            let z_idx = unr.actors.len();
            unr.actors.push(zinfo.clone());
            zone_idx = Some(z_idx);
            zone_act = Some(zinfo);
        }

        self.history.push(crate::history::Action::CommitSoundscapeSetup {
            zone_brush_index: Some(zone_b_idx),
            zone_brush,
            ambient_actor_index: ambient_idx,
            ambient_actor: ambient_act,
            zone_actor_index: zone_idx,
            zone_actor: zone_act,
            description: format!("Committed Soundscape ({}, {})", cfg.archetype.display_name(), cfg.acoustics.reverb_preset.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Soundscape to World: Zone Brush #{}, AmbientSound Actor #{:?}, ZoneInfo Actor #{:?}.",
            zone_b_idx, ambient_idx, zone_idx));

        (zone_b_idx, ambient_idx, zone_idx)
    }

    // =========================================================================
    // Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio
    // =========================================================================

    pub fn apply_hazard_chamber_archetype(&mut self, arch: ut99_bsp::HazardChamberArchetype) {
        let old_config = self.hazard_chamber_settings.active_config.clone();
        let new_config = arch.default_config();
        self.hazard_chamber_settings.active_config = new_config.clone();

        self.recalculate_hazard_chamber_preview();

        self.history.push(crate::history::Action::UpdateHazardChamberConfig {
            old_config,
            new_config,
            description: format!("Applied Hazard Chamber Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Hazard Chamber Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_hazard_chamber_preview(&mut self) {
        let cfg = &self.hazard_chamber_settings.active_config;
        let brushes = ut99_bsp::generate_hazard_chamber_brushes(cfg);
        let metrics = ut99_bsp::calculate_chamber_metrics(cfg);
        let waypoints = ut99_bsp::generate_chamber_bot_waypoints(cfg);
        let hazards = ut99_bsp::audit_hazard_chamber_safety(cfg, &self.world_brushes);

        self.hazard_chamber_settings.cached_brushes = Some(brushes);
        self.hazard_chamber_settings.cached_metrics = metrics;
        self.hazard_chamber_settings.cached_waypoints = waypoints;
        self.hazard_chamber_settings.cached_hazards = hazards;
    }

    pub fn scrub_hazard_chamber_time(&mut self, time_s: f32) {
        self.hazard_chamber_settings.simulated_time_seconds = time_s.max(0.0);
    }

    pub fn commit_hazard_chamber_to_world(&mut self) -> (usize, usize, usize, Option<usize>, usize, Option<usize>, Option<usize>, Vec<usize>) {
        self.recalculate_hazard_chamber_preview();

        let cfg = self.hazard_chamber_settings.active_config.clone();
        let brushes = self.hazard_chamber_settings.cached_brushes
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_hazard_chamber_brushes(&cfg));

        // 1. Commit Brushes
        let chamber_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.chamber_brush.clone());

        let door_l_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.door_left_brush.clone());

        let door_r_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.door_right_brush.clone());

        let mut win_idx = None;
        if let Some(ref win_b) = brushes.window_glass_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(win_b.clone());
            win_idx = Some(idx);
        }

        let console_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.console_brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut zone_idx = None;
        let mut zone_act = None;
        let mut trig_idx = None;
        let mut trig_act = None;
        let mut waypoint_indices = Vec::new();
        let mut waypoint_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            // 2. ZoneInfo Hazard Actor inside chamber
            let mut zinfo = ut99_format::ActorRecord::default();
            zinfo.name = format!("{}_HazardZone_{}", cfg.tag, unr.actors.len());
            zinfo.class_name = "Engine.ZoneInfo".to_string();
            zinfo.location = cfg.position;
            zinfo.tag = cfg.tag.clone();
            zinfo.group = "HazardChamber".to_string();
            let z_i = unr.actors.len();
            unr.actors.push(zinfo.clone());
            zone_idx = Some(z_i);
            zone_act = Some(zinfo);

            // 3. Trigger Button Actor at Console
            let mut trig = ut99_format::ActorRecord::default();
            trig.name = format!("{}_ConsoleBtn_{}", cfg.tag, unr.actors.len());
            trig.class_name = "Engine.Trigger".to_string();
            trig.location = cfg.position + cfg.console_offset + Vec3::new(0.0, 0.0, -cfg.chamber_height * 0.5 + 40.0);
            trig.event = cfg.event_name.clone();
            trig.tag = format!("{}_Button", cfg.tag);
            trig.group = "HazardChamber".to_string();
            let t_i = unr.actors.len();
            unr.actors.push(trig.clone());
            trig_idx = Some(t_i);
            trig_act = Some(trig);

            // 4. Bot Navigation Waypoints (3 nodes: bait inside, console outside, escape)
            let wp = &self.hazard_chamber_settings.cached_waypoints;
            for (w_name, pos) in [
                ("BaitTrap", wp.bait_inside),
                ("ConsoleActivator", wp.console_activator),
                ("SafeEscape", wp.safe_outside_door),
            ] {
                let mut node = ut99_format::ActorRecord::default();
                node.name = format!("{}_{}_{}", cfg.tag, w_name, unr.actors.len());
                node.class_name = "Engine.PathNode".to_string();
                node.location = pos;
                node.tag = format!("{}_{}", cfg.tag, w_name);
                node.group = "HazardChamber".to_string();
                let n_idx = unr.actors.len();
                unr.actors.push(node.clone());
                waypoint_indices.push(n_idx);
                waypoint_actors.push(node);
            }
        }

        self.history.push(crate::history::Action::CommitHazardChamberSetup {
            chamber_brush_index: Some(chamber_idx),
            door_l_brush_index: Some(door_l_idx),
            door_r_brush_index: Some(door_r_idx),
            window_brush_index: win_idx,
            console_brush_index: Some(console_idx),
            chamber_brush: brushes.chamber_brush,
            door_l_brush: brushes.door_left_brush,
            door_r_brush: brushes.door_right_brush,
            window_brush: brushes.window_glass_brush,
            console_brush: brushes.console_brush,
            zone_actor_index: zone_idx,
            zone_actor: zone_act,
            trigger_actor_index: trig_idx,
            trigger_actor: trig_act,
            waypoint_actor_indices: waypoint_indices.clone(),
            waypoint_actors,
            description: format!("Committed Hazard Chamber ({}, {})", cfg.archetype.display_name(), cfg.damage_type.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Hazard Chamber to World: Chamber #{} (Doors #{}/#{}, Glass #{:?}, Console #{}), ZoneInfo #{:?}, Trigger #{:?}, {} Waypoints.",
            chamber_idx, door_l_idx, door_r_idx, win_idx, console_idx, zone_idx, trig_idx, waypoint_indices.len()));

        (chamber_idx, door_l_idx, door_r_idx, win_idx, console_idx, zone_idx, trig_idx, waypoint_indices)
    }

    // =========================================================================
    // Industrial Pipe Network, Steam Valve & Conduit Junction Studio
    // =========================================================================

    pub fn apply_pipe_archetype(&mut self, arch: ut99_bsp::PipeArchetype) {
        let old_config = self.pipe_conduit_settings.active_config.clone();
        let new_config = arch.default_config();
        self.pipe_conduit_settings.active_config = new_config.clone();

        self.recalculate_pipe_preview();

        self.history.push(crate::history::Action::UpdatePipeConduitConfig {
            old_config,
            new_config,
            description: format!("Applied Pipe Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Pipe Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_pipe_preview(&mut self) {
        let cfg = &self.pipe_conduit_settings.active_config;
        let brushes = ut99_bsp::generate_pipe_conduit_brushes(cfg);
        let metrics = ut99_bsp::calculate_pipe_metrics(cfg);
        let hazards = ut99_bsp::audit_pipe_conduit_safety(cfg, &self.world_brushes);

        self.pipe_conduit_settings.cached_brushes = Some(brushes);
        self.pipe_conduit_settings.cached_metrics = metrics;
        self.pipe_conduit_settings.cached_hazards = hazards;
    }

    pub fn scrub_pipe_valve_position(&mut self, open_frac: f32) {
        self.pipe_conduit_settings.active_config.valve_open_fraction = open_frac.clamp(0.0, 1.0);
        self.recalculate_pipe_preview();
    }

    pub fn commit_pipe_conduit_to_world(&mut self) -> (usize, Option<usize>, Vec<usize>, Option<usize>, Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
        self.recalculate_pipe_preview();

        let cfg = self.pipe_conduit_settings.active_config.clone();
        let brushes = self.pipe_conduit_settings.cached_brushes
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_pipe_conduit_brushes(&cfg));

        // 1. Straight run brush
        let straight_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.straight_run_brush.clone());

        // 2. Elbow bend brush
        let mut elbow_idx = None;
        if let Some(ref eb) = brushes.elbow_bend_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(eb.clone());
            elbow_idx = Some(idx);
        }

        // 3. Flange brushes
        let mut flange_indices = Vec::new();
        for fb in &brushes.flange_brushes {
            let idx = self.world_brushes.len();
            self.world_brushes.push(fb.clone());
            flange_indices.push(idx);
        }

        // 4. Wall collar brush
        let mut collar_idx = None;
        if let Some(ref cb) = brushes.wall_collar_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(cb.clone());
            collar_idx = Some(idx);
        }

        // 5. Valve body brush
        let mut valve_body_idx = None;
        if let Some(ref vb) = brushes.valve_body_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(vb.clone());
            valve_body_idx = Some(idx);
        }

        // 6. Valve handwheel brush
        let mut valve_wheel_idx = None;
        if let Some(ref wb) = brushes.valve_handwheel_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(wb.clone());
            valve_wheel_idx = Some(idx);
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_act = None;
        let mut trig_idx = None;
        let mut trig_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            // Handwheel RotatingMover actor if valve exists
            if cfg.has_relief_valve {
                let mut mover = ut99_format::ActorRecord::default();
                mover.name = format!("{}_HandwheelMover_{}", cfg.tag, unr.actors.len());
                mover.class_name = "Engine.Mover".to_string();
                let v_pos = cfg.position + Vec3::new(0.0, cfg.total_run_length_uu * 0.5, cfg.pipe_outer_radius + 24.0);
                mover.location = v_pos;
                mover.tag = format!("{}_ValveWheel", cfg.tag);
                mover.event = cfg.event_name.clone();
                mover.group = "PipeNetwork".to_string();
                if let Some(ref wb) = brushes.valve_handwheel_brush {
                    let mut ed_mover = crate::editor_state::EditorMover::new(wb.clone());
                    ed_mover.name = mover.name.clone();
                    ed_mover.mover_class = "RotatingMover".to_string();
                    ed_mover.tag = mover.tag.clone();
                    ed_mover.event = mover.event.clone();
                    ed_mover.keyframes[1] = Some(crate::editor_state::MoverKeyframe {
                        location: wb.location,
                        rotation: wb.rotation + Vec3::new(0.0, 180.0, 0.0),
                    });
                    self.movers.push(ed_mover);
                }
                let m_i = unr.actors.len();
                unr.actors.push(mover.clone());
                mover_idx = Some(m_i);
                mover_act = Some(mover);
            }

            // Steam Hazard Relief Trigger
            if cfg.steam_hazard_enabled {
                let mut trig = ut99_format::ActorRecord::default();
                trig.name = format!("{}_SteamHazardTrigger_{}", cfg.tag, unr.actors.len());
                trig.class_name = "Engine.Trigger".to_string();
                let vent_pos = cfg.position + Vec3::new(cfg.pipe_outer_radius + 12.0, cfg.total_run_length_uu * 0.5, 0.0);
                trig.location = vent_pos;
                trig.event = cfg.event_name.clone();
                trig.tag = format!("{}_SteamVent", cfg.tag);
                trig.group = "PipeNetwork".to_string();
                let t_i = unr.actors.len();
                unr.actors.push(trig.clone());
                trig_idx = Some(t_i);
                trig_act = Some(trig);
            }
        }

        self.history.push(crate::history::Action::CommitPipeConduitSetup {
            straight_brush_index: Some(straight_idx),
            straight_brush: brushes.straight_run_brush,
            elbow_brush_index: elbow_idx,
            elbow_brush: brushes.elbow_bend_brush,
            flange_brush_indices: flange_indices.clone(),
            flange_brushes: brushes.flange_brushes,
            collar_brush_index: collar_idx,
            collar_brush: brushes.wall_collar_brush,
            valve_body_brush_index: valve_body_idx,
            valve_body_brush: brushes.valve_body_brush,
            valve_wheel_brush_index: valve_wheel_idx,
            valve_wheel_brush: brushes.valve_handwheel_brush,
            valve_mover_actor_index: mover_idx,
            valve_mover_actor: mover_act,
            hazard_trigger_actor_index: trig_idx,
            hazard_trigger_actor: trig_act,
            description: format!("Committed Pipe Conduit ({}, {} flanges)", cfg.archetype.display_name(), flange_indices.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Pipe Conduit to World: Straight #{}, Elbow #{:?}, {} Flanges, Collar #{:?}, Valve #{:?}/{:?}, Mover #{:?}, Trigger #{:?}.",
            straight_idx, elbow_idx, flange_indices.len(), collar_idx, valve_body_idx, valve_wheel_idx, mover_idx, trig_idx));

        (straight_idx, elbow_idx, flange_indices, collar_idx, valve_body_idx, valve_wheel_idx, mover_idx, trig_idx)
    }

    // =========================================================================
    // Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
    // =========================================================================

    pub fn apply_hologram_archetype(&mut self, arch: ut99_bsp::HologramArchetype) {
        let old_config = self.hologram_settings.active_config.clone();
        let new_config = arch.default_config();
        self.hologram_settings.active_config = new_config.clone();

        self.recalculate_hologram_preview();

        self.history.push(crate::history::Action::UpdateHologramConfig {
            old_config,
            new_config,
            description: format!("Applied Hologram Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Hologram Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_hologram_preview(&mut self) {
        let cfg = &self.hologram_settings.active_config;
        let brushes = ut99_bsp::generate_hologram_brushes(cfg);
        let metrics = ut99_bsp::evaluate_hologram_kinematics(cfg, self.hologram_settings.simulated_time_seconds);
        let hazards = ut99_bsp::audit_hologram_safety(cfg, &self.world_brushes);

        self.hologram_settings.cached_brushes = Some(brushes);
        self.hologram_settings.cached_metrics = metrics;
        self.hologram_settings.cached_hazards = hazards;
    }

    pub fn scrub_hologram_time(&mut self, time_secs: f32) {
        self.hologram_settings.simulated_time_seconds = time_secs.max(0.0);
        let cfg = &self.hologram_settings.active_config;
        self.hologram_settings.cached_metrics = ut99_bsp::evaluate_hologram_kinematics(cfg, self.hologram_settings.simulated_time_seconds);
    }

    pub fn commit_hologram_to_world(&mut self) -> (usize, Option<usize>, Option<usize>, Option<usize>, Option<usize>) {
        self.recalculate_hologram_preview();

        let cfg = self.hologram_settings.active_config.clone();
        let brushes = self.hologram_settings.cached_brushes
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_hologram_brushes(&cfg));

        // 1. Display surface brush
        let disp_idx = self.world_brushes.len();
        self.world_brushes.push(brushes.display_surface_brush.clone());

        // 2. Emitter pedestal brush
        let mut ped_idx = None;
        if let Some(ref pb) = brushes.emitter_pedestal_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(pb.clone());
            ped_idx = Some(idx);
        }

        // 3. Projection cone brush
        let mut cone_idx = None;
        if let Some(ref cb) = brushes.projection_cone_brush {
            let idx = self.world_brushes.len();
            self.world_brushes.push(cb.clone());
            cone_idx = Some(idx);
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_act = None;
        let mut light_idx = None;
        let mut light_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            // RotatingMover actor if rotation is enabled
            if cfg.rotation_speed_deg_s.abs() > 0.001 {
                let mut mover = ut99_format::ActorRecord::default();
                mover.name = format!("{}_RotatorMover_{}", cfg.tag, unr.actors.len());
                mover.class_name = "Engine.Mover".to_string();
                let center_z = cfg.position.z + cfg.elevation_offset_uu;
                mover.location = Vec3::new(cfg.position.x, cfg.position.y, center_z);
                mover.tag = cfg.tag.clone();
                mover.event = cfg.event_name.clone();
                mover.group = "HologramDisplay".to_string();
                let mut ed_mover = crate::editor_state::EditorMover::new(brushes.display_surface_brush.clone());
                ed_mover.name = mover.name.clone();
                ed_mover.mover_class = "RotatingMover".to_string();
                ed_mover.tag = mover.tag.clone();
                ed_mover.event = mover.event.clone();
                ed_mover.keyframes[1] = Some(crate::editor_state::MoverKeyframe {
                    location: brushes.display_surface_brush.location,
                    rotation: brushes.display_surface_brush.rotation + Vec3::new(0.0, cfg.rotation_speed_deg_s, 0.0),
                });
                self.movers.push(ed_mover);
                let m_i = unr.actors.len();
                unr.actors.push(mover.clone());
                mover_idx = Some(m_i);
                mover_act = Some(mover);
            }

            // Projector Light Actor at emitter lens
            let mut light = ut99_format::ActorRecord::default();
            light.name = format!("{}_ProjectorLight_{}", cfg.tag, unr.actors.len());
            light.class_name = "Engine.Light".to_string();
            let lens_z = cfg.position.z + if cfg.has_emitter_pedestal { cfg.pedestal_height } else { 0.0 };
            light.location = Vec3::new(cfg.position.x, cfg.position.y, lens_z);
            light.tag = format!("{}_LensLight", cfg.tag);
            light.group = "HologramDisplay".to_string();
            let l_i = unr.actors.len();
            unr.actors.push(light.clone());
            light_idx = Some(l_i);
            light_act = Some(light);
        }

        self.history.push(crate::history::Action::CommitHologramSetup {
            display_brush_index: Some(disp_idx),
            display_brush: brushes.display_surface_brush,
            pedestal_brush_index: ped_idx,
            pedestal_brush: brushes.emitter_pedestal_brush,
            projection_cone_brush_index: cone_idx,
            projection_cone_brush: brushes.projection_cone_brush,
            rotator_mover_actor_index: mover_idx,
            rotator_mover_actor: mover_act,
            projector_light_actor_index: light_idx,
            projector_light_actor: light_act,
            description: format!("Committed Hologram ({}, {})", cfg.archetype.display_name(), cfg.display_shape.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Hologram to World: Display #{}, Pedestal #{:?}, Cone #{:?}, Mover #{:?}, Light #{:?}.",
            disp_idx, ped_idx, cone_idx, mover_idx, light_idx));

        (disp_idx, ped_idx, cone_idx, mover_idx, light_idx)
    }

    // =========================================================================
    // Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
    // =========================================================================

    pub fn apply_weather_archetype(&mut self, arch: ut99_bsp::WeatherArchetype) {
        let old_config = self.weather_settings.active_config.clone();
        let new_config = arch.to_config();
        self.weather_settings.active_config = new_config.clone();

        self.recalculate_weather_preview();

        self.history.push(crate::history::Action::UpdateWeatherConfig {
            old_config,
            new_config,
            description: format!("Applied Weather Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Weather Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_weather_preview(&mut self) {
        let cfg = &self.weather_settings.active_config;
        let volume_b = ut99_bsp::generate_weather_volume_brush(cfg);
        let metrics = ut99_bsp::calculate_weather_metrics(cfg, self.weather_settings.simulated_time_seconds, self.weather_settings.listener_probe_pos);
        let hazards = ut99_bsp::audit_weather_safety(cfg);

        let cloud_top = cfg.position + glam::Vec3::new(0.0, 0.0, cfg.volume_extents.z * 0.5);
        let ground = cfg.position - glam::Vec3::new(0.0, 0.0, cfg.volume_extents.z * 0.5);
        let lightning = ut99_bsp::generate_lightning_bolt(cloud_top, ground, 4, 120.0, 42);

        self.weather_settings.cached_volume_brush = Some(volume_b);
        self.weather_settings.cached_lightning_branch = lightning;
        self.weather_settings.cached_metrics = metrics;
        self.weather_settings.cached_hazards = hazards;
    }

    pub fn scrub_weather_time(&mut self, time_secs: f32) {
        self.weather_settings.simulated_time_seconds = time_secs.max(0.0);
        let cfg = &self.weather_settings.active_config;
        self.weather_settings.cached_metrics = ut99_bsp::calculate_weather_metrics(cfg, self.weather_settings.simulated_time_seconds, self.weather_settings.listener_probe_pos);
    }

    pub fn step_weather_simulation(&mut self, dt: f32) {
        if self.weather_settings.is_animating {
            self.weather_settings.simulated_time_seconds += dt;
            let cfg = &self.weather_settings.active_config;
            self.weather_settings.cached_metrics = ut99_bsp::calculate_weather_metrics(cfg, self.weather_settings.simulated_time_seconds, self.weather_settings.listener_probe_pos);
        }
    }

    pub fn commit_weather_to_world(&mut self) -> (usize, Option<usize>, Option<usize>, Option<usize>) {
        self.recalculate_weather_preview();

        let cfg = self.weather_settings.active_config.clone();
        let volume_b = self.weather_settings.cached_volume_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_weather_volume_brush(&cfg));

        // 1. Precipitation Volume World brush
        let vol_idx = self.world_brushes.len();
        self.world_brushes.push(volume_b.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut emitter_idx = None;
        let mut emitter_act = None;
        let mut light_idx = None;
        let mut light_act = None;
        let mut audio_idx = None;
        let mut audio_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            // WeatherEmitter / ZoneInfo Actor
            let mut emitter = ut99_format::ActorRecord::default();
            emitter.name = format!("{}_Emitter_{}", cfg.tag, unr.actors.len());
            emitter.class_name = "Engine.WeatherEmitter".to_string();
            emitter.location = cfg.position;
            emitter.tag = cfg.tag.clone();
            emitter.group = "WeatherStorm".to_string();
            let e_i = unr.actors.len();
            unr.actors.push(emitter.clone());
            emitter_idx = Some(e_i);
            emitter_act = Some(emitter);

            // Lightning Flash Light Actor
            if cfg.lightning_enabled {
                let mut light = ut99_format::ActorRecord::default();
                light.name = format!("{}_LightningLight_{}", cfg.tag, unr.actors.len());
                light.class_name = "Engine.Light".to_string();
                let flash_z = cfg.position.z + cfg.volume_extents.z * 0.45;
                light.location = glam::Vec3::new(cfg.position.x, cfg.position.y, flash_z);
                light.tag = format!("{}_LightningFlash", cfg.tag);
                light.group = "WeatherStorm".to_string();
                let l_i = unr.actors.len();
                unr.actors.push(light.clone());
                light_idx = Some(l_i);
                light_act = Some(light);
            }

            // Ambient Precipitation / Thunder Sound Actor
            if !cfg.ambient_sound_name.is_empty() {
                let mut audio = ut99_format::ActorRecord::default();
                audio.name = format!("{}_WeatherAudio_{}", cfg.tag, unr.actors.len());
                audio.class_name = "Engine.AmbientSound".to_string();
                audio.location = cfg.position;
                audio.tag = format!("{}_AudioLoop", cfg.tag);
                audio.group = "WeatherStorm".to_string();
                let a_i = unr.actors.len();
                unr.actors.push(audio.clone());
                audio_idx = Some(a_i);
                audio_act = Some(audio);
            }
        }

        self.history.push(crate::history::Action::CommitWeatherSetup {
            volume_brush_index: Some(vol_idx),
            volume_brush: volume_b,
            zone_emitter_actor_index: emitter_idx,
            zone_emitter_actor: emitter_act,
            lightning_light_actor_index: light_idx,
            lightning_light_actor: light_act,
            thunder_audio_actor_index: audio_idx,
            thunder_audio_actor: audio_act,
            description: format!("Committed Weather Storm ({}, {})", cfg.archetype.display_name(), cfg.precipitation_type.display_name()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Weather Storm to World: Volume #{}, Emitter #{:?}, Lightning #{:?}, Audio #{:?}.",
            vol_idx, emitter_idx, light_idx, audio_idx));

        (vol_idx, emitter_idx, light_idx, audio_idx)
    }

    // =========================================================================
    // Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
    // =========================================================================

    pub fn apply_arena_rules_archetype(&mut self, arch: ut99_bsp::ArenaRulesArchetype) {
        let old_config = self.arena_rules_settings.active_config.clone();
        let new_config = arch.to_config();
        self.arena_rules_settings.active_config = new_config.clone();

        self.recalculate_arena_rules_preview();

        self.history.push(crate::history::Action::UpdateArenaRulesConfig {
            old_config,
            new_config,
            description: format!("Applied Arena Rules Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Arena Rules Archetype: {}.", arch.display_name()));
    }

    pub fn toggle_arena_mutator(&mut self, mutator: ut99_bsp::TournamentMutator) {
        let old_config = self.arena_rules_settings.active_config.clone();
        let mut new_config = old_config.clone();
        if let Some(pos) = new_config.active_mutators.iter().position(|m| *m == mutator) {
            new_config.active_mutators.remove(pos);
        } else {
            new_config.active_mutators.push(mutator);
        }
        self.arena_rules_settings.active_config = new_config.clone();
        self.recalculate_arena_rules_preview();

        self.history.push(crate::history::Action::UpdateArenaRulesConfig {
            old_config,
            new_config,
            description: format!("Toggled Tournament Mutator: {}", mutator.display_name()),
        });
    }

    pub fn recalculate_arena_rules_preview(&mut self) {
        let mut weapon_count = 0;
        let mut ammo_count = 0;

        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                if lower.contains("weapon") || lower.contains("shockrifle") || lower.contains("enforcer")
                    || lower.contains("pulsegun") || lower.contains("minigun") || lower.contains("flakcannon")
                    || lower.contains("ut_eightball") || lower.contains("sniperrifle") || lower.contains("warheadlauncher")
                    || lower.contains("chainsaw") || lower.contains("biorifle") {
                    weapon_count += 1;
                } else if lower.contains("ammo") || lower.contains("clip") || lower.contains("bulletbox")
                    || lower.contains("shellbox") || lower.contains("rocketpack") || lower.contains("flakshellammo")
                    || lower.contains("shockcore") || lower.contains("bioammo") {
                    ammo_count += 1;
                }
            }
        }

        self.arena_rules_settings.world_weapon_count = weapon_count;
        self.arena_rules_settings.world_ammo_count = ammo_count;

        let cfg = &self.arena_rules_settings.active_config;
        let metrics = ut99_bsp::evaluate_arena_rules_metrics(cfg, weapon_count, ammo_count);
        let hazards = ut99_bsp::audit_arena_rules_safety(cfg, weapon_count, ammo_count);

        self.arena_rules_settings.cached_metrics = metrics;
        self.arena_rules_settings.cached_hazards = hazards;
    }

    pub fn commit_arena_rules_to_world(&mut self) -> (Option<usize>, Vec<usize>) {
        self.recalculate_arena_rules_preview();

        let cfg = self.arena_rules_settings.active_config.clone();
        let old_level_game_type = self.level_game_type.clone();
        let new_level_game_type = cfg.game_type.class_path().to_string();
        self.level_game_type = new_level_game_type.clone();

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut level_info_idx = None;
        let mut old_level_info = None;
        let mut new_level_info = None;
        let mut mutator_indices = Vec::new();
        let mut mutator_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            // Find or create LevelInfo
            if let Some(pos) = unr.actors.iter().position(|a| a.class_name.eq_ignore_ascii_case("Engine.LevelInfo")) {
                level_info_idx = Some(pos);
                old_level_info = Some(unr.actors[pos].clone());
                unr.actors[pos].tag = cfg.tag.clone();
                new_level_info = Some(unr.actors[pos].clone());
            } else {
                let mut info = ut99_format::ActorRecord::default();
                info.name = "LevelInfo0".to_string();
                info.class_name = "Engine.LevelInfo".to_string();
                info.tag = cfg.tag.clone();
                info.group = "LevelProperties".to_string();
                let pos = unr.actors.len();
                unr.actors.push(info.clone());
                level_info_idx = Some(pos);
                new_level_info = Some(info);
            }

            // Spawn mutator actors for active tournament mutators
            for mutator in &cfg.active_mutators {
                let mut act = ut99_format::ActorRecord::default();
                let class_name = match mutator {
                    ut99_bsp::TournamentMutator::InstagibDM => "Botpack.InstaGibDM",
                    ut99_bsp::TournamentMutator::RocketArena => "Botpack.RocketArena",
                    ut99_bsp::TournamentMutator::FlakArena => "Botpack.FlakArena",
                    ut99_bsp::TournamentMutator::ShockArena => "Botpack.ShockArena",
                    ut99_bsp::TournamentMutator::SniperArena => "Botpack.SniperArena",
                    ut99_bsp::TournamentMutator::LowGrav => "Botpack.LowGrav",
                    ut99_bsp::TournamentMutator::NoPowerups => "Botpack.NoPowerups",
                    ut99_bsp::TournamentMutator::FatBoy => "Botpack.FatBoy",
                    ut99_bsp::TournamentMutator::Chameleon => "Botpack.Chameleon",
                    ut99_bsp::TournamentMutator::Relics => "Relics.RelicStrength",
                };
                act.name = format!("Mutator_{}_{}", mutator.display_name().replace(' ', ""), unr.actors.len());
                act.class_name = class_name.to_string();
                act.tag = format!("Mutator_{}", cfg.tag);
                act.group = "ArenaMutators".to_string();
                let idx = unr.actors.len();
                unr.actors.push(act.clone());
                mutator_indices.push(idx);
                mutator_actors.push(act);
            }
        }

        self.history.push(crate::history::Action::CommitArenaRulesSetup {
            level_info_actor_index: level_info_idx,
            old_level_info,
            new_level_info,
            old_level_game_type,
            new_level_game_type,
            mutator_actor_indices: mutator_indices.clone(),
            mutator_actors,
            description: format!("Committed Arena Rules ({}, FragLimit:{}, MaxPlayers:{})",
                cfg.archetype.display_name(), cfg.frag_limit, cfg.max_players),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Arena Match Rules: GameType={}, FragLimit={}, TimeLimit={}m, Mutators={}.",
            cfg.game_type.display_name(), cfg.frag_limit, cfg.time_limit_mins, cfg.active_mutators.len()));

        (level_info_idx, mutator_indices)
    }

    // =========================================================================
    // Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
    // =========================================================================

    pub fn apply_armor_archetype(&mut self, arch: ut99_bsp::ArmorArchetype) {
        let old_config = self.armor_stacking_settings.active_config.clone();
        let new_config = arch.to_config();
        self.armor_stacking_settings.active_config = new_config.clone();

        self.recalculate_armor_stacking_preview();

        self.history.push(crate::history::Action::UpdateArmorStackingConfig {
            old_config,
            new_config,
            description: format!("Applied Armor Stacking Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Armor Stacking Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_armor_stacking_preview(&mut self) {
        let mut world_pickups = Vec::new();

        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                let is_major = lower.contains("shieldbelt") || lower.contains("armor2") || lower.contains("keg") || lower.contains("healthpack");
                if lower.contains("shieldbelt") || lower.contains("armor") || lower.contains("thighpads")
                    || lower.contains("health") || lower.contains("medbox") || lower.contains("vial") {
                    world_pickups.push(ut99_bsp::PickupMarker {
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        position: actor.location,
                        is_major_armor_or_keg: is_major,
                    });
                }
            }
        }

        let cfg = &self.armor_stacking_settings.active_config;
        let brush = ut99_bsp::generate_armor_pedestal_brush(cfg);
        let metrics = ut99_bsp::evaluate_armor_stacking_metrics(cfg, &world_pickups);
        let hazards = ut99_bsp::audit_armor_stacking_safety(cfg, &world_pickups);

        let dmg = self.armor_stacking_settings.test_incoming_damage;
        let dmg_res = ut99_bsp::simulate_damage_absorption(
            dmg,
            self.armor_stacking_settings.simulated_damage_probe_hp,
            self.armor_stacking_settings.simulated_damage_probe_shield,
            self.armor_stacking_settings.simulated_damage_probe_body,
            self.armor_stacking_settings.simulated_damage_probe_thigh,
        );

        self.armor_stacking_settings.cached_pedestal_brush = Some(brush);
        self.armor_stacking_settings.cached_metrics = metrics;
        self.armor_stacking_settings.cached_hazards = hazards;
        self.armor_stacking_settings.cached_damage_result = dmg_res;
    }

    pub fn simulate_armor_test_damage(&mut self, damage: f32) {
        self.armor_stacking_settings.test_incoming_damage = damage.max(0.0);
        let dmg_res = ut99_bsp::simulate_damage_absorption(
            self.armor_stacking_settings.test_incoming_damage,
            self.armor_stacking_settings.simulated_damage_probe_hp,
            self.armor_stacking_settings.simulated_damage_probe_shield,
            self.armor_stacking_settings.simulated_damage_probe_body,
            self.armor_stacking_settings.simulated_damage_probe_thigh,
        );
        self.armor_stacking_settings.cached_damage_result = dmg_res;
    }

    pub fn commit_armor_stacking_to_world(&mut self) -> (usize, Vec<usize>) {
        self.recalculate_armor_stacking_preview();

        let cfg = self.armor_stacking_settings.active_config.clone();
        let brush = self.armor_stacking_settings.cached_pedestal_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_armor_pedestal_brush(&cfg));

        // 1. Add pedestal brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut pickup_indices = Vec::new();
        let mut pickup_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            let base_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 16.0);

            // Spawn ShieldBelt actor if configured
            if cfg.shield_belt_count > 0 {
                let mut act = ut99_format::ActorRecord::default();
                act.name = format!("UT_ShieldBelt_{}", unr.actors.len());
                act.class_name = "Botpack.UT_ShieldBelt".to_string();
                act.location = base_pos;
                act.tag = format!("{}_ShieldBelt", cfg.tag);
                act.group = "ArmorStack".to_string();
                let idx = unr.actors.len();
                unr.actors.push(act.clone());
                pickup_indices.push(idx);
                pickup_actors.push(act);
            } else if cfg.body_armor_count > 0 {
                let mut act = ut99_format::ActorRecord::default();
                act.name = format!("Armor2_{}", unr.actors.len());
                act.class_name = "Botpack.Armor2".to_string();
                act.location = base_pos;
                act.tag = format!("{}_BodyArmor", cfg.tag);
                act.group = "ArmorStack".to_string();
                let idx = unr.actors.len();
                unr.actors.push(act.clone());
                pickup_indices.push(idx);
                pickup_actors.push(act);
            } else if cfg.keg_of_health_count > 0 {
                let mut act = ut99_format::ActorRecord::default();
                act.name = format!("HealthPack_{}", unr.actors.len());
                act.class_name = "Botpack.HealthPack".to_string();
                act.location = base_pos;
                act.tag = format!("{}_KegHealth", cfg.tag);
                act.group = "ArmorStack".to_string();
                let idx = unr.actors.len();
                unr.actors.push(act.clone());
                pickup_indices.push(idx);
                pickup_actors.push(act);
            }

            // Flanking Health Vials if configured
            if cfg.health_vial_count >= 2 {
                for side in [-1.0_f32, 1.0_f32] {
                    let mut act = ut99_format::ActorRecord::default();
                    act.name = format!("HealthVial_{}", unr.actors.len());
                    act.class_name = "Botpack.HealthVial".to_string();
                    act.location = base_pos + Vec3::new(side * 48.0, 0.0, 0.0);
                    act.tag = format!("{}_FlankVial", cfg.tag);
                    act.group = "ArmorStack".to_string();
                    let idx = unr.actors.len();
                    unr.actors.push(act.clone());
                    pickup_indices.push(idx);
                    pickup_actors.push(act);
                }
            }
        }

        self.history.push(crate::history::Action::CommitArmorStackingSetup {
            pedestal_brush_index: Some(brush_idx),
            pedestal_brush: brush,
            pickup_actor_indices: pickup_indices.clone(),
            pickup_actors,
            description: format!("Committed Armor Station ({}, Pickups:{})", cfg.archetype.display_name(), pickup_indices.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Armor Station to World: Pedestal #{}, Pickups={:?}.", brush_idx, pickup_indices));

        (brush_idx, pickup_indices)
    }

    // =========================================================================
    // Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
    // =========================================================================

    pub fn apply_jump_route_archetype(&mut self, arch: ut99_bsp::BotJumpArchetype) {
        let old_config = self.bot_jump_routes_settings.active_config.clone();
        let new_config = arch.to_config();
        self.bot_jump_routes_settings.active_config = new_config.clone();

        self.recalculate_jump_route_preview();

        self.history.push(crate::history::Action::UpdateBotJumpRoutesConfig {
            old_config,
            new_config,
            description: format!("Applied Bot Jump Route Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Bot Jump Route Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_jump_route_preview(&mut self) {
        let cfg = &self.bot_jump_routes_settings.active_config;
        let solution = ut99_bsp::solve_jump_trajectory(cfg);
        let metrics = ut99_bsp::evaluate_jump_metrics(cfg, &solution);
        let hazards = ut99_bsp::audit_jump_route_safety(cfg, &solution);
        let brush = ut99_bsp::generate_jump_plate_brush(cfg);

        self.bot_jump_routes_settings.cached_solution = solution;
        self.bot_jump_routes_settings.cached_metrics = metrics;
        self.bot_jump_routes_settings.cached_hazards = hazards;
        self.bot_jump_routes_settings.cached_plate_brush = Some(brush);
    }

    pub fn commit_jump_route_to_world(&mut self) -> (usize, Vec<usize>) {
        self.recalculate_jump_route_preview();

        let cfg = self.bot_jump_routes_settings.active_config.clone();
        let brush = self.bot_jump_routes_settings.cached_plate_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_jump_plate_brush(&cfg));

        // 1. Add launch jump plate brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_indices = Vec::new();
        let mut actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            // 2. Launch PathNode Actor
            let mut start_node = ut99_format::ActorRecord::default();
            start_node.name = format!("PathNode_{}", unr.actors.len());
            start_node.class_name = "Engine.PathNode".to_string();
            start_node.location = cfg.start_pos + Vec3::new(0.0, 0.0, cfg.plate_height + 20.0);
            start_node.tag = format!("{}_Start", cfg.tag);
            start_node.event = format!("{}_JumpEvent", cfg.tag);
            start_node.group = "BotJumpRoutes".to_string();
            let idx0 = unr.actors.len();
            unr.actors.push(start_node.clone());
            actor_indices.push(idx0);
            actors.push(start_node);

            // 3. Destination JumpSpot Actor
            let mut dest_spot = ut99_format::ActorRecord::default();
            dest_spot.name = format!("JumpSpot_{}", unr.actors.len());
            dest_spot.class_name = "Botpack.JumpSpot".to_string();
            dest_spot.location = cfg.target_pos + Vec3::new(0.0, 0.0, 20.0);
            dest_spot.tag = format!("{}_JumpEvent", cfg.tag);
            dest_spot.group = "BotJumpRoutes".to_string();
            let idx1 = unr.actors.len();
            unr.actors.push(dest_spot.clone());
            actor_indices.push(idx1);
            actors.push(dest_spot);

            // 4. TranslocTarget Actor if translocator is enabled
            if cfg.b_always_translocate {
                let mut tt = ut99_format::ActorRecord::default();
                tt.name = format!("TranslocTarget_{}", unr.actors.len());
                tt.class_name = "Botpack.TranslocTarget".to_string();
                tt.location = cfg.target_pos + Vec3::new(0.0, 0.0, 32.0);
                tt.tag = format!("{}_TranslocTarget", cfg.tag);
                tt.group = "BotJumpRoutes".to_string();
                let idx2 = unr.actors.len();
                unr.actors.push(tt.clone());
                actor_indices.push(idx2);
                actors.push(tt);
            }
        }

        self.history.push(crate::history::Action::CommitBotJumpRouteSetup {
            jump_plate_brush_index: Some(brush_idx),
            jump_plate_brush: brush,
            actor_indices: actor_indices.clone(),
            actors,
            description: format!("Committed Bot Jump Route ({}, Actors:{})", cfg.archetype.display_name(), actor_indices.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Bot Jump Route to World: Plate #{}, Actors={:?}.", brush_idx, actor_indices));

        (brush_idx, actor_indices)
    }

    // =========================================================================
    // Interactive Tournament Weapon Placement & Ammo Circuit Studio
    // =========================================================================

    pub fn apply_weapon_circuit_archetype(&mut self, arch: ut99_bsp::WeaponCircuitArchetype) {
        let old_config = self.weapon_circuit_settings.active_config.clone();
        let new_config = arch.to_config();
        self.weapon_circuit_settings.active_config = new_config.clone();

        self.recalculate_weapon_circuit_preview();

        self.history.push(crate::history::Action::UpdateWeaponCircuitConfig {
            old_config,
            new_config,
            description: format!("Applied Weapon Circuit Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Weapon Circuit Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_weapon_circuit_preview(&mut self) {
        let cfg = &self.weapon_circuit_settings.active_config;

        let mut markers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                let is_w = lower.contains("weapon")
                    || lower.contains("shockrifle")
                    || lower.contains("flakcannon")
                    || lower.contains("eightball")
                    || lower.contains("sniperrifle")
                    || lower.contains("minigun")
                    || lower.contains("biorifle")
                    || lower.contains("pulsegun")
                    || lower.contains("ripper")
                    || lower.contains("enforcer")
                    || lower.contains("warheadlauncher");
                let is_a = lower.contains("ammo")
                    || lower.contains("shockcore")
                    || lower.contains("flakshell")
                    || lower.contains("rocketpack")
                    || lower.contains("bulletbox")
                    || lower.contains("eclip")
                    || lower.contains("bladehopper");

                if is_w || is_a {
                    markers.push(ut99_bsp::LevelWeaponMarker {
                        name: actor.name.clone(),
                        class_name: actor.class_name.clone(),
                        position: actor.location,
                        is_weapon: is_w,
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_weapon_circuit_metrics(cfg, &markers);
        let hazards = ut99_bsp::audit_weapon_circuit_safety(cfg, &markers);
        let brush = ut99_bsp::generate_weapon_altar_brush(cfg);

        self.weapon_circuit_settings.cached_metrics = metrics;
        self.weapon_circuit_settings.cached_hazards = hazards;
        self.weapon_circuit_settings.cached_altar_brush = Some(brush);
    }

    pub fn commit_weapon_station_to_world(&mut self) -> (usize, Vec<usize>) {
        self.recalculate_weapon_circuit_preview();

        let cfg = self.weapon_circuit_settings.active_config.clone();
        let brush = self.weapon_circuit_settings.cached_altar_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_weapon_altar_brush(&cfg));

        // 1. Add weapon altar brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_indices = Vec::new();
        let mut actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            // 2. Primary Weapon Actor
            let mut weapon_act = ut99_format::ActorRecord::default();
            weapon_act.name = format!("{}_{}", cfg.primary_weapon.actor_class_name().split('.').last().unwrap_or("Weapon"), unr.actors.len());
            weapon_act.class_name = cfg.primary_weapon.actor_class_name().to_string();
            weapon_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height + 24.0);
            weapon_act.tag = cfg.tag.clone();
            weapon_act.group = "WeaponCircuit".to_string();
            let w_idx = unr.actors.len();
            unr.actors.push(weapon_act.clone());
            actor_indices.push(w_idx);
            actors.push(weapon_act);

            // If DualEnforcers, add second Enforcer close by for akimbo pickup
            if cfg.primary_weapon == ut99_bsp::TournamentWeaponType::DualEnforcers {
                let mut enf2 = ut99_format::ActorRecord::default();
                enf2.name = format!("EnforcerAkimbo_{}", unr.actors.len());
                enf2.class_name = "Botpack.Enforcer".to_string();
                enf2.location = cfg.position + Vec3::new(24.0, 0.0, cfg.altar_height + 24.0);
                enf2.tag = cfg.tag.clone();
                enf2.group = "WeaponCircuit".to_string();
                let e2_idx = unr.actors.len();
                unr.actors.push(enf2.clone());
                actor_indices.push(e2_idx);
                actors.push(enf2);
            }

            // 3. Flanking Ammunition Boxes
            let ammo_offsets = [
                Vec3::new(-cfg.altar_width * 0.45, 0.0, cfg.altar_height + 12.0),
                Vec3::new(cfg.altar_width * 0.45, 0.0, cfg.altar_height + 12.0),
                Vec3::new(0.0, cfg.altar_depth * 0.45, cfg.altar_height + 12.0),
            ];
            for i in 0..(cfg.flanking_ammo_count as usize).min(ammo_offsets.len()) {
                let mut ammo_act = ut99_format::ActorRecord::default();
                ammo_act.name = format!("{}_{}", cfg.primary_weapon.ammo_class_name().split('.').last().unwrap_or("Ammo"), unr.actors.len());
                ammo_act.class_name = cfg.primary_weapon.ammo_class_name().to_string();
                ammo_act.location = cfg.position + ammo_offsets[i];
                ammo_act.tag = cfg.tag.clone();
                ammo_act.group = "WeaponCircuit".to_string();
                let a_idx = unr.actors.len();
                unr.actors.push(ammo_act.clone());
                actor_indices.push(a_idx);
                actors.push(ammo_act);
            }
        }

        self.history.push(crate::history::Action::CommitWeaponCircuitSetup {
            altar_brush_index: Some(brush_idx),
            altar_brush: brush,
            actor_indices: actor_indices.clone(),
            actors,
            description: format!("Committed Weapon Station ({}, Actors:{})", cfg.archetype.display_name(), actor_indices.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Weapon Station to World: Altar #{}, Actors={:?}.", brush_idx, actor_indices));

        (brush_idx, actor_indices)
    }

    // =========================================================================
    // Interactive Tournament Powerup Control & Relic Altar Studio
    // =========================================================================

    pub fn apply_powerup_archetype(&mut self, arch: ut99_bsp::PowerupArchetype) {
        let old_config = self.powerup_control_settings.active_config.clone();
        let new_config = arch.to_config();
        self.powerup_control_settings.active_config = new_config.clone();

        self.recalculate_powerup_preview();

        self.history.push(crate::history::Action::UpdatePowerupControlConfig {
            old_config,
            new_config,
            description: format!("Applied Powerup Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Powerup Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_powerup_preview(&mut self) {
        let cfg = &self.powerup_control_settings.active_config;

        let mut actors = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                actors.push(ut99_bsp::NearbyLevelActor {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_powerup_metrics(cfg, &actors);
        let hazards = ut99_bsp::audit_powerup_safety(cfg, &actors);
        let brush = ut99_bsp::generate_powerup_altar_brush(cfg);

        self.powerup_control_settings.cached_metrics = metrics;
        self.powerup_control_settings.cached_hazards = hazards;
        self.powerup_control_settings.cached_altar_brush = Some(brush);
    }

    pub fn commit_powerup_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_powerup_preview();

        let cfg = self.powerup_control_settings.active_config.clone();
        let brush = self.powerup_control_settings.cached_altar_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_powerup_altar_brush(&cfg));

        // 1. Add powerup altar brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut powerup_act = ut99_format::ActorRecord::default();
            powerup_act.name = format!("{}_{}", cfg.powerup_type.actor_class_name().split('.').last().unwrap_or("Powerup"), unr.actors.len());
            powerup_act.class_name = cfg.powerup_type.actor_class_name().to_string();
            powerup_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height + 24.0);
            powerup_act.tag = cfg.tag.clone();
            powerup_act.group = "Powerups".to_string();
            let idx = unr.actors.len();
            unr.actors.push(powerup_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(powerup_act);
        }

        self.history.push(crate::history::Action::CommitPowerupSetup {
            altar_brush_index: Some(brush_idx),
            altar_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Powerup Altar ({}, Actor:{:?})", cfg.archetype.display_name(), actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Powerup Altar to World: Altar #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Tournament Spawn Point Balancing & Telefrag Prevention Studio
    // =========================================================================

    pub fn apply_spawn_archetype(&mut self, arch: ut99_bsp::SpawnDistributionArchetype) {
        let old_config = self.spawn_distribution_settings.active_config.clone();
        let new_config = arch.to_config();
        self.spawn_distribution_settings.active_config = new_config.clone();

        self.recalculate_spawn_preview();

        self.history.push(crate::history::Action::UpdateSpawnDistributionConfig {
            old_config,
            new_config,
            description: format!("Applied Spawn Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Spawn Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_spawn_preview(&mut self) {
        let cfg = &self.spawn_distribution_settings.active_config;

        let mut spawns = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                let is_ps = lower.contains("playerstart");
                let is_w = lower.contains("weapon") || lower.contains("shock") || lower.contains("flak") || lower.contains("eightball") || lower.contains("sniper");
                if is_ps || is_w {
                    spawns.push(ut99_bsp::LevelPlayerStartRecord {
                        name: actor.name.clone(),
                        position: actor.location,
                        team_number: 255, // default
                        is_player_start: is_ps,
                        is_weapon: is_w,
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_spawn_metrics(cfg, &spawns);
        let hazards = ut99_bsp::audit_spawn_safety(cfg, &spawns);
        let brush = ut99_bsp::generate_spawn_pad_brush(cfg);

        self.spawn_distribution_settings.cached_metrics = metrics;
        self.spawn_distribution_settings.cached_hazards = hazards;
        self.spawn_distribution_settings.cached_pad_brush = Some(brush);
    }

    pub fn commit_spawn_pad_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_spawn_preview();

        let cfg = self.spawn_distribution_settings.active_config.clone();
        let brush = self.spawn_distribution_settings.cached_pad_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_spawn_pad_brush(&cfg));

        // 1. Add spawn pad brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut start_act = ut99_format::ActorRecord::default();
            start_act.name = format!("PlayerStart_{}", unr.actors.len());
            start_act.class_name = "Engine.PlayerStart".to_string();
            // Player height offset: pad_height + cylinder half height (40.0)
            start_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.pad_height + 40.0);
            start_act.tag = cfg.tag.clone();
            start_act.group = "Navigation".to_string();
            start_act.properties.insert(
                "TeamNumber".to_string(),
                TaggedProperty {
                    name: "TeamNumber".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(cfg.team_number as u8),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(start_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(start_act);
        }

        self.history.push(crate::history::Action::CommitSpawnPointSetup {
            pad_brush_index: Some(brush_idx),
            pad_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Spawn Point ({}, Actor:{:?})", cfg.archetype.display_name(), actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Spawn Pad to World: Pad #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Domination Control Point & Point-Weight Triangle Studio
    // =========================================================================

    pub fn apply_domination_archetype(&mut self, arch: ut99_bsp::DominationArchetype) {
        let old_config = self.domination_settings.active_config.clone();
        let new_config = arch.to_config();
        self.domination_settings.active_config = new_config.clone();

        self.recalculate_domination_preview();

        self.history.push(crate::history::Action::UpdateDominationConfig {
            old_config,
            new_config,
            description: format!("Applied Domination Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Domination Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_domination_preview(&mut self) {
        let cfg = &self.domination_settings.active_config;

        let mut points = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                if lower.contains("controlpoint") || lower.contains("dominationpoint") {
                    points.push(ut99_bsp::LevelControlPointRecord {
                        name: actor.name.clone(),
                        point_id: ut99_bsp::DominationPointId::PointB, // default secondary
                        position: actor.location,
                        score_cadence_secs: 5.0,
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_domination_metrics(cfg, &points);
        let hazards = ut99_bsp::audit_domination_safety(cfg, &points);
        let brush = ut99_bsp::generate_domination_pedestal_brush(cfg);

        self.domination_settings.cached_metrics = metrics;
        self.domination_settings.cached_hazards = hazards;
        self.domination_settings.cached_pedestal_brush = Some(brush);
    }

    pub fn commit_domination_point_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_domination_preview();

        let cfg = self.domination_settings.active_config.clone();
        let brush = self.domination_settings.cached_pedestal_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_domination_pedestal_brush(&cfg));

        // 1. Add pedestal brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut cp_act = ut99_format::ActorRecord::default();
            cp_act.name = format!("ControlPoint_{}", cfg.active_point.letter());
            cp_act.class_name = "Botpack.ControlPoint".to_string();
            // Position above pedestal: pedestal_height + 28 uu
            cp_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 28.0);
            cp_act.tag = cfg.tag.clone();
            cp_act.group = "Domination".to_string();
            cp_act.properties.insert(
                "PointName".to_string(),
                TaggedProperty {
                    name: "PointName".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(format!("Point {}", cfg.active_point.letter())),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(cp_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(cp_act);
        }

        self.history.push(crate::history::Action::CommitDominationPointSetup {
            pedestal_brush_index: Some(brush_idx),
            pedestal_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed ControlPoint ({}, Actor:{:?})", cfg.archetype.display_name(), actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Domination Pedestal to World: Pedestal #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Assault Objective & Objective Chain Studio
    // =========================================================================

    pub fn apply_assault_archetype(&mut self, arch: ut99_bsp::AssaultArchetype) {
        let old_config = self.assault_settings.active_config.clone();
        let new_config = arch.to_config();
        self.assault_settings.active_config = new_config.clone();

        self.recalculate_assault_preview();

        self.history.push(crate::history::Action::UpdateAssaultConfig {
            old_config,
            new_config,
            description: format!("Applied Assault Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Assault Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_assault_preview(&mut self) {
        let cfg = &self.assault_settings.active_config;

        let mut chain = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                if lower.contains("fortstandard") || lower.contains("assault") {
                    let parsed_order = if let Some(pos) = actor.name.rfind('_') {
                        actor.name[pos + 1..].parse::<u8>().unwrap_or(1)
                    } else {
                        1
                    };
                    chain.push(ut99_bsp::LevelAssaultObjectiveRecord {
                        name: actor.name.clone(),
                        order: parsed_order,
                        position: actor.location,
                        objective_type: ut99_bsp::AssaultObjectiveType::DestroyableTarget,
                        event_on_complete: actor.event.clone(),
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_assault_metrics(cfg, &chain);
        let hazards = ut99_bsp::audit_assault_safety(cfg, &chain);
        let brush = ut99_bsp::generate_blast_door_brush(cfg);

        self.assault_settings.cached_metrics = metrics;
        self.assault_settings.cached_hazards = hazards;
        self.assault_settings.cached_door_brush = Some(brush);
    }

    pub fn commit_assault_objective_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_assault_preview();

        let cfg = self.assault_settings.active_config.clone();
        let brush = self.assault_settings.cached_door_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_blast_door_brush(&cfg));

        // 1. Add door brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut obj_act = ut99_format::ActorRecord::default();
            obj_act.name = format!("AssaultObj_{}", cfg.objective_order);
            obj_act.class_name = cfg.objective_type.default_actor_class().to_string();
            // Position above door: door_height * 0.5 + 24 uu
            obj_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.door_height * 0.5 + 24.0);
            obj_act.tag = cfg.tag.clone();
            obj_act.event = cfg.event_on_complete.clone();
            obj_act.group = "Assault".to_string();
            obj_act.properties.insert(
                "DefensePriority".to_string(),
                TaggedProperty {
                    name: "DefensePriority".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(cfg.objective_order),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(obj_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(obj_act);
        }

        self.history.push(crate::history::Action::CommitAssaultObjectiveSetup {
            door_brush_index: Some(brush_idx),
            door_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Assault Objective ({}, Actor:{:?})", cfg.archetype.display_name(), actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Assault Blast Door to World: Door #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Team Base Radar, Jammer & Holographic Threat Map Studio
    // =========================================================================

    pub fn apply_radar_archetype(&mut self, arch: ut99_bsp::RadarStudioArchetype) {
        let old_config = self.radar_settings.active_config.clone();
        let new_config = arch.to_config();
        self.radar_settings.active_config = new_config.clone();

        self.recalculate_radar_preview();

        self.history.push(crate::history::Action::UpdateRadarConfig {
            old_config,
            new_config,
            description: format!("Applied Radar Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Radar Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_radar_preview(&mut self) {
        let cfg = &self.radar_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                if lower.contains("radar") || lower.contains("jammer") {
                    let is_jam = lower.contains("jammer");
                    let team = if lower.contains("blue") { 1 } else { 0 };
                    peers.push(ut99_bsp::LevelRadarNodeRecord {
                        name: actor.name.clone(),
                        team,
                        position: actor.location,
                        is_jammer: is_jam,
                        radius: 2000.0,
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_radar_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_radar_safety(cfg, &peers);
        let brush = ut99_bsp::generate_radar_mast_and_dish_brush(cfg);

        self.radar_settings.cached_metrics = metrics;
        self.radar_settings.cached_hazards = hazards;
        self.radar_settings.cached_antenna_brush = Some(brush);
    }

    pub fn commit_radar_station_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_radar_preview();

        let cfg = self.radar_settings.active_config.clone();
        let brush = self.radar_settings.cached_antenna_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_radar_mast_and_dish_brush(&cfg));

        // 1. Add mast brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut rad_act = ut99_format::ActorRecord::default();
            let prefix = if cfg.is_jammer { "JammerNode" } else { "RadarStation" };
            rad_act.name = format!("{}_{}", prefix, unr.actors.len());
            rad_act.class_name = if cfg.is_jammer {
                "Botpack.RadarJammer".to_string()
            } else {
                "Botpack.RadarStation".to_string()
            };
            // Position atop mast: mast_height + 16 uu
            rad_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.mast_height + 16.0);
            rad_act.tag = cfg.tag.clone();
            rad_act.group = "Radar".to_string();
            let idx = unr.actors.len();
            unr.actors.push(rad_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(rad_act);
        }

        self.history.push(crate::history::Action::CommitRadarStationSetup {
            mast_brush_index: Some(brush_idx),
            mast_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Radar Station ({}, Actor:{:?})", cfg.archetype.display_name(), actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Radar Mast & Dish to World: Mast #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Capture The Flag Base & Transit Routes Studio
    // =========================================================================

    pub fn apply_ctf_base_archetype(&mut self, arch: ut99_bsp::CtfBaseArchetype) {
        let old_config = self.ctf_base_settings.active_config.clone();
        let new_config = arch.to_config();
        self.ctf_base_settings.active_config = new_config.clone();

        self.recalculate_ctf_base_preview();

        self.history.push(crate::history::Action::UpdateCtfBaseConfig {
            old_config,
            new_config,
            description: format!("Applied CTF Base Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied CTF Base Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_ctf_base_preview(&mut self) {
        let cfg = &self.ctf_base_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower = actor.class_name.to_lowercase();
                if lower.contains("flagbase") {
                    let team = if lower.contains("blue") {
                        ut99_bsp::CTFTeam::Blue
                    } else if lower.contains("green") {
                        ut99_bsp::CTFTeam::Green
                    } else if lower.contains("gold") {
                        ut99_bsp::CTFTeam::Gold
                    } else {
                        ut99_bsp::CTFTeam::Red
                    };
                    peers.push(ut99_bsp::LevelFlagBaseRecord {
                        name: actor.name.clone(),
                        team,
                        position: actor.location,
                    });
                }
            }
        }

        let metrics = ut99_bsp::evaluate_ctf_base_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_ctf_base_safety(cfg, &peers);
        let brush = ut99_bsp::generate_ctf_flag_base_brush(cfg);

        self.ctf_base_settings.cached_metrics = metrics;
        self.ctf_base_settings.cached_hazards = hazards;
        self.ctf_base_settings.cached_pedestal_brush = Some(brush);
    }

    pub fn commit_ctf_base_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_ctf_base_preview();

        let cfg = self.ctf_base_settings.active_config.clone();
        let brush = self.ctf_base_settings.cached_pedestal_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_ctf_flag_base_brush(&cfg));

        // 1. Add pedestal brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut flag_act = ut99_format::ActorRecord::default();
            flag_act.name = format!("{}_{}", cfg.team.actor_class_name().rsplit('.').next().unwrap_or("FlagBase"), unr.actors.len());
            flag_act.class_name = cfg.team.actor_class_name().to_string();
            // Position atop pedestal: pedestal_height + 12 uu
            flag_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 12.0);
            flag_act.tag = cfg.tag.clone();
            flag_act.group = "CTF".to_string();
            let team_byte = match cfg.team {
                ut99_bsp::CTFTeam::Red => 0,
                ut99_bsp::CTFTeam::Blue => 1,
                ut99_bsp::CTFTeam::Green => 2,
                ut99_bsp::CTFTeam::Gold => 3,
            };
            flag_act.properties.insert(
                "Team".to_string(),
                TaggedProperty {
                    name: "Team".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(team_byte),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(flag_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(flag_act);
        }

        self.history.push(crate::history::Action::CommitCtfBaseSetup {
            pedestal_brush_index: Some(brush_idx),
            pedestal_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed CTF Base ({:?}, Actor:{:?})", cfg.team, actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed CTF Flag Base to World: Pedestal #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Team Cannon & Automated Turret Defense Studio
    // =========================================================================

    pub fn apply_team_cannon_archetype(&mut self, arch: ut99_bsp::TeamCannonArchetype) {
        let old_config = self.team_cannon_settings.active_config.clone();
        let new_config = arch.to_config();
        self.team_cannon_settings.active_config = new_config.clone();

        self.recalculate_team_cannon_preview();

        self.history.push(crate::history::Action::UpdateTeamCannonConfig {
            old_config,
            new_config,
            description: format!("Applied Team Cannon Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Team Cannon Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_team_cannon_preview(&mut self) {
        let cfg = &self.team_cannon_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let lower_name = actor.name.to_lowercase();
                let lower_tag = actor.tag.to_lowercase();
                let lower_grp = actor.group.to_lowercase();
                let lower_cls = actor.class_name.to_lowercase();
                let team = if lower_name.contains("blue") || lower_tag.contains("blue") || lower_grp.contains("blue") || lower_cls.contains("blue") || lower_name.contains("team1") {
                    1
                } else if lower_name.contains("green") || lower_tag.contains("green") || lower_grp.contains("green") || lower_cls.contains("green") {
                    2
                } else if lower_name.contains("gold") || lower_tag.contains("gold") || lower_grp.contains("gold") || lower_cls.contains("gold") {
                    3
                } else if lower_name.contains("red") || lower_tag.contains("red") || lower_grp.contains("red") || lower_cls.contains("red") || lower_name.contains("team0") {
                    0
                } else {
                    1 // default to opposing team if unassigned
                };

                peers.push(ut99_bsp::LevelCombatActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                    team,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_team_cannon_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_team_cannon_safety(cfg, &peers);
        let brush = ut99_bsp::generate_team_cannon_bunker_brush(cfg);

        self.team_cannon_settings.cached_metrics = metrics;
        self.team_cannon_settings.cached_hazards = hazards;
        self.team_cannon_settings.cached_bunker_brush = Some(brush);
    }

    pub fn commit_team_cannon_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_team_cannon_preview();

        let cfg = self.team_cannon_settings.active_config.clone();
        let brush = self.team_cannon_settings.cached_bunker_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_team_cannon_bunker_brush(&cfg));

        // 1. Add bunker mount brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut cannon_act = ut99_format::ActorRecord::default();
            cannon_act.name = format!("TeamCannon_{}", unr.actors.len());
            cannon_act.class_name = "Botpack.TeamCannon".to_string();
            // Position atop bunker mount: mount_height + 16 uu
            let z_offset = if cfg.is_inverted {
                -(cfg.bunker_mount_height + 16.0)
            } else {
                cfg.bunker_mount_height + 16.0
            };
            cannon_act.location = cfg.position + Vec3::new(0.0, 0.0, z_offset);
            cannon_act.tag = cfg.tag.clone();
            cannon_act.group = "Defense".to_string();
            cannon_act.properties.insert(
                "MyTeam".to_string(),
                TaggedProperty {
                    name: "MyTeam".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(cfg.team),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(cannon_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(cannon_act);
        }

        self.history.push(crate::history::Action::CommitTeamCannonSetup {
            mount_brush_index: Some(brush_idx),
            mount_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Team Cannon ({}, Actor:{:?})", cfg.tag, actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Team Cannon to World: Mount #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
    // =========================================================================

    pub fn apply_bio_trap_archetype(&mut self, arch: ut99_bsp::BioTrapArchetype) {
        let old_config = self.bio_trap_settings.active_config.clone();
        let new_config = arch.to_config();
        self.bio_trap_settings.active_config = new_config.clone();

        self.recalculate_bio_trap_preview();

        self.history.push(crate::history::Action::UpdateBioTrapConfig {
            old_config,
            new_config,
            description: format!("Applied Bio-Trap Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Bio-Trap Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_bio_trap_preview(&mut self) {
        let cfg = &self.bio_trap_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelBioContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_bio_trap_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_bio_trap_safety(cfg, &peers);
        let brush = ut99_bsp::generate_bio_drain_grate_brush(cfg);

        self.bio_trap_settings.cached_metrics = metrics;
        self.bio_trap_settings.cached_hazards = hazards;
        self.bio_trap_settings.cached_grate_brush = Some(brush);
    }

    pub fn commit_bio_trap_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_bio_trap_preview();

        let cfg = self.bio_trap_settings.active_config.clone();
        let brush = self.bio_trap_settings.cached_grate_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_bio_drain_grate_brush(&cfg));

        // 1. Add chemical drain grate brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut bio_act = ut99_format::ActorRecord::default();
            bio_act.name = format!("BioGelTrap_{}", unr.actors.len());
            bio_act.class_name = "Botpack.BioGel".to_string();
            // Position slightly offset along surface normal
            bio_act.location = cfg.position + cfg.surface_normal.normalize_or_zero() * (cfg.grate_height + 4.0);
            bio_act.tag = cfg.tag.clone();
            bio_act.group = "Hazard".to_string();
            let idx = unr.actors.len();
            unr.actors.push(bio_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(bio_act);
        }

        self.history.push(crate::history::Action::CommitBioTrapSetup {
            grate_brush_index: Some(brush_idx),
            grate_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Bio Trap ({}, Actor:{:?})", cfg.tag, actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Bio-Rifle Slime Trap to World: Grate #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Pressure Plate & Proximity Trigger Network Studio
    // =========================================================================

    pub fn apply_trigger_network_archetype(&mut self, arch: ut99_bsp::TriggerNetworkArchetype) {
        let old_config = self.trigger_network_settings.active_config.clone();
        let new_config = arch.to_config();
        self.trigger_network_settings.active_config = new_config.clone();

        self.recalculate_trigger_network_preview();

        self.history.push(crate::history::Action::UpdateTriggerNetworkConfig {
            old_config,
            new_config,
            description: format!("Applied Trigger Network Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Trigger Network Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_trigger_network_preview(&mut self) {
        let cfg = &self.trigger_network_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelTriggerContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    event: actor.event.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_trigger_network_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_trigger_network_safety(cfg, &peers);
        let brush = ut99_bsp::generate_pressure_plate_brush(cfg);

        self.trigger_network_settings.cached_metrics = metrics;
        self.trigger_network_settings.cached_hazards = hazards;
        self.trigger_network_settings.cached_plate_brush = Some(brush);
    }

    pub fn commit_trigger_network_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_trigger_network_preview();

        let cfg = self.trigger_network_settings.active_config.clone();
        let brush = self.trigger_network_settings.cached_plate_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_pressure_plate_brush(&cfg));

        // 1. Add pressure plate mount frame brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut actor_idx = None;
        let mut actor_record = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut trig_act = ut99_format::ActorRecord::default();
            trig_act.name = format!("Trigger_{}", unr.actors.len());
            trig_act.class_name = "Engine.Trigger".to_string();
            trig_act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.plate_thickness_uu + 8.0);
            trig_act.event = cfg.event_name.clone();
            trig_act.tag = format!("Switch_{}", unr.actors.len());
            trig_act.group = "Logic".to_string();
            trig_act.properties.insert(
                "CollisionRadius".to_string(),
                TaggedProperty {
                    name: "CollisionRadius".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.activation_radius_uu),
                },
            );
            trig_act.properties.insert(
                "CollisionHeight".to_string(),
                TaggedProperty {
                    name: "CollisionHeight".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.activation_height_uu),
                },
            );
            trig_act.properties.insert(
                "ReTriggerDelay".to_string(),
                TaggedProperty {
                    name: "ReTriggerDelay".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.retrigger_delay_secs),
                },
            );
            let idx = unr.actors.len();
            unr.actors.push(trig_act.clone());
            actor_idx = Some(idx);
            actor_record = Some(trig_act);
        }

        self.history.push(crate::history::Action::CommitTriggerNetworkSetup {
            plate_brush_index: Some(brush_idx),
            plate_brush: brush,
            actor_index: actor_idx,
            actor: actor_record,
            description: format!("Committed Trigger Network (Event:{}, Actor:{:?})", cfg.event_name, actor_idx),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Trigger Network to World: Plate #{}, Actor={:?}.", brush_idx, actor_idx));

        (brush_idx, actor_idx)
    }

    // =========================================================================
    // Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
    // =========================================================================

    pub fn apply_elevator_archetype(&mut self, arch: ut99_bsp::ElevatorArchetype) {
        let old_config = self.elevator_settings.active_config.clone();
        let new_config = arch.to_config();
        self.elevator_settings.active_config = new_config.clone();

        self.recalculate_elevator_preview();

        self.history.push(crate::history::Action::UpdateElevatorConfig {
            old_config,
            new_config,
            description: format!("Applied Elevator Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Elevator Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_elevator_preview(&mut self) {
        let cfg = &self.elevator_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelElevatorContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    lift_tag: actor.event.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_elevator_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_elevator_safety(cfg, &peers);
        let brush = ut99_bsp::generate_elevator_platform_brush(cfg);

        self.elevator_settings.cached_metrics = metrics;
        self.elevator_settings.cached_hazards = hazards;
        self.elevator_settings.cached_platform_brush = Some(brush);
    }

    pub fn commit_elevator_to_world(&mut self) -> (usize, Option<usize>, Vec<usize>) {
        self.recalculate_elevator_preview();

        let cfg = self.elevator_settings.active_config.clone();
        let brush = self.elevator_settings.cached_platform_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_elevator_platform_brush(&cfg));

        // 1. Add elevator platform brush to world and register EditorMover with Key0 & Key1
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        let mut ed_mover = EditorMover::new(brush.clone());
        ed_mover.name = if brush.name.is_empty() {
            format!("ElevatorMover_{}", self.movers.len())
        } else {
            brush.name.clone()
        };
        ed_mover.tag = cfg.tag.clone();
        ed_mover.initial_state = "StandOpenTimed".to_string();
        ed_mover.keyframes[0] = Some(MoverKeyframe {
            location: cfg.position,
            rotation: Vec3::ZERO,
        });
        ed_mover.keyframes[1] = Some(MoverKeyframe {
            location: cfg.position + Vec3::new(0.0, 0.0, cfg.travel_height_uu),
            rotation: Vec3::ZERO,
        });
        self.movers.push(ed_mover);

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut lift_center_idx = None;
        let mut lift_center_act = None;
        let mut lift_exit_indices = Vec::new();
        let mut lift_exit_actors = Vec::new();

        if let Some(ref mut unr) = self.loaded_unr {
            let lift_tag_prop = TaggedProperty {
                name: "LiftTag".to_string(),
                array_index: 0,
                value: TaggedValue::Name(cfg.tag.clone()),
            };
            // 2. Add LiftCenter placed directly on moving platform
            let mut center = ut99_format::ActorRecord::default();
            center.name = format!("LiftCenter_{}", unr.actors.len());
            center.class_name = "Botpack.LiftCenter".to_string();
            center.location = cfg.position + Vec3::new(0.0, 0.0, cfg.platform_thickness_uu + 20.0);
            center.tag = cfg.tag.clone();
            center.event = cfg.tag.clone();
            center.group = "Navigation".to_string();
            center.properties.insert("LiftTag".to_string(), lift_tag_prop.clone());
            let c_idx = unr.actors.len();
            unr.actors.push(center.clone());
            lift_center_idx = Some(c_idx);
            lift_center_act = Some(center);

            // 3. Add LiftExit 0 at Bottom Landing (Key 0)
            let mut exit0 = ut99_format::ActorRecord::default();
            exit0.name = format!("LiftExit_Bottom_{}", unr.actors.len());
            exit0.class_name = "Botpack.LiftExit".to_string();
            exit0.location = cfg.position + Vec3::new(cfg.platform_width_uu * 0.5 + 48.0, 0.0, 16.0);
            exit0.tag = cfg.tag.clone();
            exit0.event = cfg.tag.clone();
            exit0.group = "Navigation".to_string();
            exit0.properties.insert("LiftTag".to_string(), lift_tag_prop.clone());
            exit0.properties.insert(
                "SuggestedKeyFrame".to_string(),
                TaggedProperty {
                    name: "SuggestedKeyFrame".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(0),
                },
            );
            let e0_idx = unr.actors.len();
            unr.actors.push(exit0.clone());
            lift_exit_indices.push(e0_idx);
            lift_exit_actors.push(exit0);

            // 4. Add LiftExit 1 at Top Landing (Key 1)
            let mut exit1 = ut99_format::ActorRecord::default();
            exit1.name = format!("LiftExit_Top_{}", unr.actors.len());
            exit1.class_name = "Botpack.LiftExit".to_string();
            exit1.location = cfg.position + Vec3::new(cfg.platform_width_uu * 0.5 + 48.0, 0.0, cfg.travel_height_uu + 16.0);
            exit1.tag = cfg.tag.clone();
            exit1.event = cfg.tag.clone();
            exit1.group = "Navigation".to_string();
            exit1.properties.insert("LiftTag".to_string(), lift_tag_prop);
            exit1.properties.insert(
                "SuggestedKeyFrame".to_string(),
                TaggedProperty {
                    name: "SuggestedKeyFrame".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(1),
                },
            );
            let e1_idx = unr.actors.len();
            unr.actors.push(exit1.clone());
            lift_exit_indices.push(e1_idx);
            lift_exit_actors.push(exit1);
            unr.sync_actor_views();
        }

        self.history.push(crate::history::Action::CommitElevatorSetup {
            platform_brush_index: Some(brush_idx),
            platform_brush: brush,
            lift_center_actor_index: lift_center_idx,
            lift_center_actor: lift_center_act,
            lift_exit_actor_indices: lift_exit_indices.clone(),
            lift_exit_actors: lift_exit_actors.clone(),
            description: format!("Committed Elevator ({}, Exits:{})", cfg.tag, lift_exit_indices.len()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Elevator to World: Platform #{}, Center={:?}, Exits={:?}.", brush_idx, lift_center_idx, lift_exit_indices));

        (brush_idx, lift_center_idx, lift_exit_indices)
    }

    // =========================================================================
    // Interactive Dynamic Zone Velocity, Convection & Vacuum Studio
    // =========================================================================

    pub fn apply_zone_velocity_archetype(&mut self, arch: ut99_bsp::ZoneVelocityArchetype) {
        let old_config = self.zone_velocity_settings.active_config.clone();
        let new_config = arch.to_config();
        self.zone_velocity_settings.active_config = new_config.clone();

        self.recalculate_zone_velocity_preview();

        self.history.push(crate::history::Action::UpdateZoneVelocityConfig {
            old_config,
            new_config,
            description: format!("Applied Zone Velocity Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Zone Velocity Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_zone_velocity_preview(&mut self) {
        let cfg = &self.zone_velocity_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelZoneContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_zone_velocity_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_zone_velocity_safety(cfg, &peers);
        let brush = ut99_bsp::generate_zone_portal_sheet_brush(cfg);

        self.zone_velocity_settings.cached_metrics = metrics;
        self.zone_velocity_settings.cached_hazards = hazards;
        self.zone_velocity_settings.cached_portal_brush = Some(brush);
    }

    pub fn commit_zone_velocity_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_zone_velocity_preview();

        let cfg = self.zone_velocity_settings.active_config.clone();
        let brush = self.zone_velocity_settings.cached_portal_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_zone_portal_sheet_brush(&cfg));

        // 1. Add Zone Portal Sheet Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut zone_info_idx = None;
        let mut zone_info_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            // 2. Add or configure ZoneInfo / WaterZone actor
            let mut zone_act = ut99_format::ActorRecord::default();
            let class_name = match cfg.medium {
                ut99_bsp::ZoneMediumType::SubmergedWater => "Botpack.WaterZone",
                ut99_bsp::ZoneMediumType::CorrosiveAcidSlime => "Botpack.SlimeZone",
                ut99_bsp::ZoneMediumType::MoltenLava => "Botpack.LavaZone",
                _ => "Engine.ZoneInfo",
            };
            zone_act.name = format!("ZoneInfo_{}", unr.actors.len());
            zone_act.class_name = class_name.to_string();
            zone_act.location = cfg.position + Vec3::new(0.0, 0.0, 32.0);
            zone_act.tag = cfg.zone_tag.clone();
            zone_act.group = "ZoneInfo".to_string();
            zone_act.properties.insert(
                "ZoneVelocity".to_string(),
                TaggedProperty {
                    name: "ZoneVelocity".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(cfg.zone_velocity),
                },
            );
            let zone_gravity = if cfg.is_zero_gravity {
                Vec3::ZERO
            } else {
                Vec3::new(0.0, 0.0, -950.0)
            };
            zone_act.properties.insert(
                "ZoneGravity".to_string(),
                TaggedProperty {
                    name: "ZoneGravity".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(zone_gravity),
                },
            );
            zone_act.properties.insert(
                "ZoneFluidFriction".to_string(),
                TaggedProperty {
                    name: "ZoneFluidFriction".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.fluid_friction),
                },
            );

            let z_idx = unr.actors.len();
            unr.actors.push(zone_act.clone());
            zone_info_idx = Some(z_idx);
            zone_info_act = Some(zone_act);
        }

        self.history.push(crate::history::Action::CommitZoneVelocitySetup {
            portal_brush_index: Some(brush_idx),
            portal_brush: brush,
            zone_info_actor_index: zone_info_idx,
            zone_info_actor: zone_info_act,
            description: format!("Committed Zone Velocity (Tag:{}, Vel:{:.0})", cfg.zone_tag, cfg.zone_velocity.length()),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Zone Velocity to World: Portal Sheet #{}, ZoneInfo={:?}.", brush_idx, zone_info_idx));

        (brush_idx, zone_info_idx)
    }

    // =========================================================================
    // Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
    // =========================================================================

    pub fn apply_hazard_barrel_archetype(&mut self, arch: ut99_bsp::BarrelHazardType) {
        let old_config = self.hazard_barrel_settings.active_config.clone();
        let new_config = arch.to_config();
        self.hazard_barrel_settings.active_config = new_config.clone();

        self.recalculate_hazard_barrel_preview();

        self.history.push(crate::history::Action::UpdateHazardBarrelConfig {
            old_config,
            new_config,
            description: format!("Applied Hazard Barrel Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Hazard Barrel Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_hazard_barrel_preview(&mut self) {
        let cfg = &self.hazard_barrel_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelBarrelContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_hazard_barrel_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_hazard_barrel_safety(cfg, &peers);
        let brush = ut99_bsp::generate_hazard_barrel_brush(cfg);

        self.hazard_barrel_settings.cached_metrics = metrics;
        self.hazard_barrel_settings.cached_hazards = hazards;
        self.hazard_barrel_settings.cached_barrel_brush = Some(brush);
    }

    pub fn commit_hazard_barrel_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_hazard_barrel_preview();

        let cfg = self.hazard_barrel_settings.active_config.clone();
        let brush = self.hazard_barrel_settings.cached_barrel_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_hazard_barrel_brush(&cfg));

        // 1. Add Barrel Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut barrel_idx = None;
        let mut barrel_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("UT_SteelBarrel_{}", unr.actors.len());
            act.class_name = "Botpack.UT_SteelBarrel".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.barrel_height_uu * 0.5);
            act.tag = cfg.tag.clone();
            act.event = cfg.event_on_explode.clone();
            act.group = "Hazard".to_string();
            act.properties.insert(
                "Health".to_string(),
                TaggedProperty {
                    name: "Health".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(cfg.health.round() as i32),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            barrel_idx = Some(idx);
            barrel_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitHazardBarrelSetup {
            barrel_brush_index: Some(brush_idx),
            barrel_brush: brush,
            barrel_actor_index: barrel_idx,
            barrel_actor: barrel_act,
            description: format!("Committed Hazard Barrel ({}, Dmg:{:.0})", cfg.tag, cfg.blast_damage),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Hazard Barrel to World: Brush #{}, Actor={:?}.", brush_idx, barrel_idx));

        (brush_idx, barrel_idx)
    }

    // =========================================================================
    // Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
    // =========================================================================

    pub fn apply_special_lit_archetype(&mut self, arch: ut99_bsp::SpecialLitArchetype) {
        let old_config = self.special_lit_settings.active_config.clone();
        let new_config = arch.to_config();
        self.special_lit_settings.active_config = new_config.clone();

        self.recalculate_special_lit_preview();

        self.history.push(crate::history::Action::UpdateSpecialLitConfig {
            old_config,
            new_config,
            description: format!("Applied SpecialLit Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied SpecialLit Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_special_lit_preview(&mut self) {
        let cfg = &self.special_lit_settings.active_config;

        let mut surfs = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            for (p_idx, poly) in brush.polys.iter().enumerate() {
                let center = if !poly.vertices.is_empty() {
                    poly.vertices.iter().copied().sum::<Vec3>() / poly.vertices.len() as f32 + brush.location
                } else {
                    brush.location
                };
                let is_special = (poly.flags & ut99_bsp::PF_SPECIAL_LIT) != 0;
                surfs.push(ut99_bsp::LevelLightContextSurfaceRecord {
                    poly_index: b_idx * 1000 + p_idx,
                    is_special_lit: is_special,
                    center,
                    normal: poly.plane.normal,
                    area: 1024.0,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_special_lit_metrics(cfg, &surfs);
        let hazards = ut99_bsp::audit_special_lit_safety(cfg, &surfs);
        let brush = ut99_bsp::generate_special_lit_fixture_brush(cfg);

        self.special_lit_settings.cached_metrics = metrics;
        self.special_lit_settings.cached_hazards = hazards;
        self.special_lit_settings.cached_fixture_brush = Some(brush);
    }

    pub fn commit_special_lit_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_special_lit_preview();

        let cfg = self.special_lit_settings.active_config.clone();
        let brush = self.special_lit_settings.cached_fixture_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_special_lit_fixture_brush(&cfg));

        // 1. Add Fixture Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut light_idx = None;
        let mut light_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("SpecialLight_{}", unr.actors.len());
            act.class_name = "Engine.Light".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, 8.0);
            act.tag = cfg.tag.clone();
            act.group = "Lighting".to_string();
            act.light_brightness = cfg.light_brightness;
            act.light_hue = cfg.light_hue;
            act.light_saturation = cfg.light_saturation;
            act.light_radius = (cfg.light_radius_uu / 25.0).clamp(1.0, 255.0) as u8;
            act.properties.insert(
                "bSpecialLit".to_string(),
                TaggedProperty {
                    name: "bSpecialLit".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            light_idx = Some(idx);
            light_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitSpecialLitSetup {
            fixture_brush_index: Some(brush_idx),
            fixture_brush: brush,
            light_actor_index: light_idx,
            light_actor: light_act,
            description: format!("Committed SpecialLit Light ({}, Radius:{:.0})", cfg.tag, cfg.light_radius_uu),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed SpecialLit Light to World: Fixture #{}, Light={:?}.", brush_idx, light_idx));

        (brush_idx, light_idx)
    }

    // =========================================================================
    // Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
    // =========================================================================

    pub fn apply_sniper_vantage_archetype(&mut self, arch: ut99_bsp::SniperVantageArchetype) {
        let old_config = self.sniper_vantage_settings.active_config.clone();
        let new_config = arch.to_config();
        self.sniper_vantage_settings.active_config = new_config.clone();

        self.recalculate_sniper_vantage_preview();

        self.history.push(crate::history::Action::UpdateSniperVantageConfig {
            old_config,
            new_config,
            description: format!("Applied Sniper Vantage Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Sniper Vantage Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_sniper_vantage_preview(&mut self) {
        let cfg = &self.sniper_vantage_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelSniperContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_sniper_vantage_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_sniper_vantage_safety(cfg, &peers);
        let brush = ut99_bsp::generate_sniper_parapet_brush(cfg);

        self.sniper_vantage_settings.cached_metrics = metrics;
        self.sniper_vantage_settings.cached_hazards = hazards;
        self.sniper_vantage_settings.cached_parapet_brush = Some(brush);
    }

    pub fn commit_sniper_vantage_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_sniper_vantage_preview();

        let cfg = self.sniper_vantage_settings.active_config.clone();
        let brush = self.sniper_vantage_settings.cached_parapet_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_sniper_parapet_brush(&cfg));

        // 1. Add Parapet Balcony Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut sniper_idx = None;
        let mut sniper_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("SniperRifle_{}", unr.actors.len());
            act.class_name = "Botpack.SniperRifle".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, 16.0);
            act.tag = cfg.tag.clone();
            act.group = "Weapons".to_string();

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            sniper_idx = Some(idx);
            sniper_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitSniperVantageSetup {
            parapet_brush_index: Some(brush_idx),
            parapet_brush: brush,
            sniper_actor_index: sniper_idx,
            sniper_actor: sniper_act,
            description: format!("Committed Sniper Vantage ({}, Range:{:.0})", cfg.tag, cfg.max_effective_range_uu),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Sniper Vantage to World: Parapet #{}, Weapon={:?}.", brush_idx, sniper_idx));

        (brush_idx, sniper_idx)
    }

    // =========================================================================
    // Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
    // =========================================================================

    pub fn apply_secret_door_archetype(&mut self, arch: ut99_bsp::SecretDoorArchetype) {
        let old_config = self.secret_door_settings.active_config.clone();
        let new_config = arch.to_config();
        self.secret_door_settings.active_config = new_config.clone();

        self.recalculate_secret_door_preview();

        self.history.push(crate::history::Action::UpdateSecretDoorConfig {
            old_config,
            new_config,
            description: format!("Applied Secret Door Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Secret Door Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_secret_door_preview(&mut self) {
        let cfg = &self.secret_door_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelSecretContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_secret_door_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_secret_door_safety(cfg, &peers);
        let brush = ut99_bsp::generate_secret_door_brush(cfg);

        self.secret_door_settings.cached_metrics = metrics;
        self.secret_door_settings.cached_hazards = hazards;
        self.secret_door_settings.cached_door_brush = Some(brush);
    }

    pub fn commit_secret_door_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_secret_door_preview();

        let cfg = self.secret_door_settings.active_config.clone();
        let brush = self.secret_door_settings.cached_door_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_secret_door_brush(&cfg));

        // 1. Add Secret Door Brush to world and register EditorMover with Key0 & Key1
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        let mut ed_mover = EditorMover::new(brush.clone());
        ed_mover.name = if brush.name.is_empty() {
            format!("SecretDoorMover_{}", self.movers.len())
        } else {
            brush.name.clone()
        };
        ed_mover.tag = cfg.event.clone();
        ed_mover.stay_open_time = cfg.stay_open_time_secs;
        ed_mover.keyframes[0] = Some(MoverKeyframe {
            location: brush.location,
            rotation: brush.rotation,
        });
        ed_mover.keyframes[1] = Some(MoverKeyframe {
            location: brush.location + Vec3::new(0.0, 0.0, cfg.door_height_uu),
            rotation: brush.rotation,
        });
        self.movers.push(ed_mover);

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut trigger_idx = None;
        let mut trigger_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("Trigger_Secret_{}", unr.actors.len());
            act.class_name = "Engine.Trigger".to_string();
            act.location = cfg.position;
            act.event = cfg.event.clone();
            act.tag = cfg.tag.clone();
            act.group = "Triggers".to_string();
            act.properties.insert(
                "ReTriggerDelay".to_string(),
                TaggedProperty {
                    name: "ReTriggerDelay".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.stay_open_time_secs),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            unr.sync_actor_views();
            trigger_idx = Some(idx);
            trigger_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitSecretDoorSetup {
            door_brush_index: Some(brush_idx),
            door_brush: brush,
            trigger_actor_index: trigger_idx,
            trigger_actor: trigger_act,
            description: format!("Committed Secret Door ({}, Event:{})", cfg.tag, cfg.event),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Secret Door to World: Door #{}, Trigger={:?}.", brush_idx, trigger_idx));

        (brush_idx, trigger_idx)
    }

    // =========================================================================
    // Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
    // =========================================================================

    pub fn apply_teleporter_arch_archetype(&mut self, arch: ut99_bsp::TeleporterArchArchetype) {
        let old_config = self.teleporter_arch_settings.active_config.clone();
        let new_config = arch.to_config();
        self.teleporter_arch_settings.active_config = new_config.clone();

        self.recalculate_teleporter_arch_preview();

        self.history.push(crate::history::Action::UpdateTeleporterArchConfig {
            old_config,
            new_config,
            description: format!("Applied Teleporter Arch Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Teleporter Arch Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_teleporter_arch_preview(&mut self) {
        let cfg = &self.teleporter_arch_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelTeleportContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    tag: actor.tag.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_teleporter_arch_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_teleporter_arch_safety(cfg, &peers);
        let brush = ut99_bsp::generate_teleporter_arch_brush(cfg);

        self.teleporter_arch_settings.cached_metrics = metrics;
        self.teleporter_arch_settings.cached_hazards = hazards;
        self.teleporter_arch_settings.cached_arch_brush = Some(brush);
    }

    pub fn commit_teleporter_arch_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_teleporter_arch_preview();

        let cfg = self.teleporter_arch_settings.active_config.clone();
        let brush = self.teleporter_arch_settings.cached_arch_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_teleporter_arch_brush(&cfg));

        // 1. Add Archway Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut tele_idx = None;
        let mut tele_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("Teleporter_{}", unr.actors.len());
            act.class_name = "Engine.Teleporter".to_string();
            act.location = cfg.position;
            act.event = cfg.url.clone();
            act.tag = cfg.tag.clone();
            act.group = "Teleporters".to_string();
            act.properties.insert(
                "URL".to_string(),
                TaggedProperty {
                    name: "URL".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(cfg.url.clone()),
                },
            );
            act.properties.insert(
                "bChangesYaw".to_string(),
                TaggedProperty {
                    name: "bChangesYaw".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(cfg.b_changes_yaw),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            tele_idx = Some(idx);
            tele_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitTeleporterArchSetup {
            arch_brush_index: Some(brush_idx),
            arch_brush: brush,
            teleporter_actor_index: tele_idx,
            teleporter_actor: tele_act,
            description: format!("Committed Teleporter Arch ({}, URL:{})", cfg.tag, cfg.url),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Teleporter Arch to World: Arch #{}, Actor={:?}.", brush_idx, tele_idx));

        (brush_idx, tele_idx)
    }

    // =========================================================================
    // Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
    // =========================================================================

    pub fn apply_fluid_basin_archetype(&mut self, arch: ut99_bsp::FluidBasinArchetype) {
        let old_config = self.fluid_basin_settings.active_config.clone();
        let new_config = arch.to_config();
        self.fluid_basin_settings.active_config = new_config.clone();

        self.recalculate_fluid_basin_preview();

        self.history.push(crate::history::Action::UpdateFluidBasinConfig {
            old_config,
            new_config,
            description: format!("Applied Fluid Basin Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Fluid Basin Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_fluid_basin_preview(&mut self) {
        let cfg = &self.fluid_basin_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelBasinContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_fluid_basin_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_fluid_basin_safety(cfg, &peers);
        let brush = ut99_bsp::generate_fluid_basin_brush(cfg);

        self.fluid_basin_settings.cached_metrics = metrics;
        self.fluid_basin_settings.cached_hazards = hazards;
        self.fluid_basin_settings.cached_basin_brush = Some(brush);
    }

    pub fn commit_fluid_basin_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_fluid_basin_preview();

        let cfg = self.fluid_basin_settings.active_config.clone();
        let brush = self.fluid_basin_settings.cached_basin_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_fluid_basin_brush(&cfg));

        // 1. Add Basin Trough Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut zone_idx = None;
        let mut zone_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("Zone_Fluid_{}", unr.actors.len());
            act.class_name = if cfg.dps > 0.0 {
                "Botpack.PainZone".to_string()
            } else {
                "Botpack.WaterZone".to_string()
            };
            act.location = cfg.position - Vec3::new(0.0, 0.0, cfg.depth_uu * 0.5);
            act.tag = cfg.tag.clone();
            act.group = "Zones".to_string();
            act.properties.insert(
                "DamagePerSec".to_string(),
                TaggedProperty {
                    name: "DamagePerSec".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(cfg.dps.round() as i32),
                },
            );
            act.properties.insert(
                "bWaterZone".to_string(),
                TaggedProperty {
                    name: "bWaterZone".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            zone_idx = Some(idx);
            zone_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitFluidBasinSetup {
            basin_brush_index: Some(brush_idx),
            basin_brush: brush,
            zone_actor_index: zone_idx,
            zone_actor: zone_act,
            description: format!("Committed Fluid Basin ({}, DPS:{:.0})", cfg.tag, cfg.dps),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Fluid Basin to World: Basin #{}, ZoneActor={:?}.", brush_idx, zone_idx));

        (brush_idx, zone_idx)
    }

    // =========================================================================
    // Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
    // =========================================================================

    pub fn apply_alarm_beacon_archetype(&mut self, arch: ut99_bsp::AlarmBeaconArchetype) {
        let old_config = self.alarm_beacon_settings.active_config.clone();
        let new_config = arch.to_config();
        self.alarm_beacon_settings.active_config = new_config.clone();

        self.recalculate_alarm_beacon_preview();

        self.history.push(crate::history::Action::UpdateAlarmBeaconConfig {
            old_config,
            new_config,
            description: format!("Applied Alarm Beacon Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Alarm Beacon Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_alarm_beacon_preview(&mut self) {
        let cfg = &self.alarm_beacon_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelAlarmContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_alarm_beacon_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_alarm_beacon_safety(cfg, &peers);
        let brush = ut99_bsp::generate_alarm_beacon_brush(cfg);

        self.alarm_beacon_settings.cached_metrics = metrics;
        self.alarm_beacon_settings.cached_hazards = hazards;
        self.alarm_beacon_settings.cached_beacon_brush = Some(brush);
    }

    pub fn commit_alarm_beacon_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_alarm_beacon_preview();

        let cfg = self.alarm_beacon_settings.active_config.clone();
        let brush = self.alarm_beacon_settings.cached_beacon_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_alarm_beacon_brush(&cfg));

        // 1. Add Alarm Beacon Mount Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut light_idx = None;
        let mut light_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("AlarmLight_{}", unr.actors.len());
            act.class_name = "Engine.Light".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, 16.0);
            act.light_radius = (cfg.light_radius_uu / 16.0).clamp(1.0, 255.0) as u8;
            act.light_brightness = cfg.light_brightness;
            act.light_hue = cfg.light_hue;
            act.light_saturation = cfg.light_saturation;
            act.light_type = 1; // LT_Steady
            act.light_effect = if cfg.strobe_frequency_hz > 0.0 {
                3 // LE_Strobe
            } else if cfg.rotor_rpm > 0.0 {
                1 // LE_Rotor
            } else {
                0 // LE_None
            };
            act.tag = cfg.tag.clone();
            act.event = cfg.event.clone();
            act.group = "Alarms".to_string();

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            light_idx = Some(idx);
            light_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitAlarmBeaconSetup {
            beacon_brush_index: Some(brush_idx),
            beacon_brush: brush,
            light_actor_index: light_idx,
            light_actor: light_act,
            description: format!("Committed Alarm Beacon ({}, Event:{})", cfg.tag, cfg.event),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Alarm Beacon to World: Beacon #{}, LightActor={:?}.", brush_idx, light_idx));

        (brush_idx, light_idx)
    }

    // =========================================================================
    // Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
    // =========================================================================

    pub fn apply_supply_locker_archetype(&mut self, arch: ut99_bsp::SupplyLockerArchetype) {
        let old_config = self.supply_locker_settings.active_config.clone();
        let new_config = arch.to_config();
        self.supply_locker_settings.active_config = new_config.clone();

        self.recalculate_supply_locker_preview();

        self.history.push(crate::history::Action::UpdateSupplyLockerConfig {
            old_config,
            new_config,
            description: format!("Applied Supply Locker Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Supply Locker Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_supply_locker_preview(&mut self) {
        let cfg = &self.supply_locker_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelLockerContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_supply_locker_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_supply_locker_safety(cfg, &peers);
        let brush = ut99_bsp::generate_supply_locker_brush(cfg);

        self.supply_locker_settings.cached_metrics = metrics;
        self.supply_locker_settings.cached_hazards = hazards;
        self.supply_locker_settings.cached_locker_brush = Some(brush);
    }

    pub fn commit_supply_locker_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_supply_locker_preview();

        let cfg = self.supply_locker_settings.active_config.clone();
        let brush = self.supply_locker_settings.cached_locker_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_supply_locker_brush(&cfg));

        // 1. Add Supply Locker Cabinet Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut weapon_idx = None;
        let mut weapon_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("LockerWeapon_{}", unr.actors.len());
            act.class_name = if !cfg.primary_weapon_class.is_empty() {
                cfg.primary_weapon_class.clone()
            } else {
                cfg.primary_ammo_class.clone()
            };
            act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.locker_height_uu * 0.4);
            act.tag = cfg.tag.clone();
            act.group = "ArmoryLockers".to_string();

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            weapon_idx = Some(idx);
            weapon_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitSupplyLockerSetup {
            locker_brush_index: Some(brush_idx),
            locker_brush: brush,
            weapon_actor_index: weapon_idx,
            weapon_actor: weapon_act,
            description: format!("Committed Supply Locker ({}, Items:{})", cfg.tag, cfg.ammo_pack_count),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Supply Locker to World: Locker #{}, WeaponActor={:?}.", brush_idx, weapon_idx));

        (brush_idx, weapon_idx)
    }

    // =========================================================================
    // Interactive Glass Skylight, Observation Dome & Structural Truss Studio
    // =========================================================================

    pub fn apply_skylight_archetype(&mut self, arch: ut99_bsp::SkylightArchetype) {
        let old_config = self.skylight_settings.active_config.clone();
        let new_config = arch.to_config();
        self.skylight_settings.active_config = new_config.clone();

        self.recalculate_skylight_preview();

        self.history.push(crate::history::Action::UpdateSkylightConfig {
            old_config,
            new_config,
            description: format!("Applied Skylight Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Skylight Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_skylight_preview(&mut self) {
        let cfg = &self.skylight_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelSkylightContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_skylight_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_skylight_safety(cfg, &peers);
        let brush = ut99_bsp::generate_skylight_brush(cfg);

        self.skylight_settings.cached_metrics = metrics;
        self.skylight_settings.cached_hazards = hazards;
        self.skylight_settings.cached_skylight_brush = Some(brush);
    }

    pub fn commit_skylight_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_skylight_preview();

        let cfg = self.skylight_settings.active_config.clone();
        let brush = self.skylight_settings.cached_skylight_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_skylight_brush(&cfg));

        // 1. Add Skylight Frame & Glass Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut light_idx = None;
        let mut light_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("Sunlight_{}", unr.actors.len());
            act.class_name = "Engine.Light".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.height_or_rise_uu * 0.5);
            act.light_radius = (cfg.width_uu.max(cfg.length_uu) / 16.0).clamp(1.0, 255.0) as u8;
            act.light_brightness = cfg.sunlight_brightness;
            act.light_hue = cfg.sunlight_hue;
            act.light_saturation = 32;
            act.light_type = 1; // LT_Steady
            act.tag = cfg.tag.clone();
            act.group = "Skylights".to_string();

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            light_idx = Some(idx);
            light_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitSkylightSetup {
            skylight_brush_index: Some(brush_idx),
            skylight_brush: brush,
            sunlight_actor_index: light_idx,
            sunlight_actor: light_act,
            description: format!("Committed Skylight ({}, Trans:{:.0}%)", cfg.tag, cfg.glass_transmittance_pct),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Skylight to World: Brush #{}, SunlightActor={:?}.", brush_idx, light_idx));

        (brush_idx, light_idx)
    }

    // =========================================================================
    // Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
    // =========================================================================

    pub fn apply_vent_duct_archetype(&mut self, arch: ut99_bsp::VentDuctArchetype) {
        let old_config = self.vent_duct_settings.active_config.clone();
        let new_config = arch.to_config();
        self.vent_duct_settings.active_config = new_config.clone();

        self.recalculate_vent_duct_preview();

        self.history.push(crate::history::Action::UpdateVentDuctConfig {
            old_config,
            new_config,
            description: format!("Applied Vent Duct Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Vent Duct Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_vent_duct_preview(&mut self) {
        let cfg = &self.vent_duct_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelVentContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_vent_duct_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_vent_duct_safety(cfg, &peers);
        let brush = ut99_bsp::generate_vent_duct_brush(cfg);

        self.vent_duct_settings.cached_metrics = metrics;
        self.vent_duct_settings.cached_hazards = hazards;
        self.vent_duct_settings.cached_duct_brush = Some(brush);
    }

    pub fn commit_vent_duct_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_vent_duct_preview();

        let cfg = self.vent_duct_settings.active_config.clone();
        let brush = self.vent_duct_settings.cached_duct_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_vent_duct_brush(&cfg));

        // 1. Add Vent Duct Conduit Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("VentGrate_{}", unr.actors.len());
            act.class_name = if cfg.grate_breakable {
                "Engine.Mover".to_string()
            } else {
                "Engine.Trigger".to_string()
            };
            act.location = cfg.position;
            act.tag = cfg.tag.clone();
            act.event = cfg.event.clone();
            act.group = "VentShafts".to_string();
            act.properties.insert(
                "bDamageTriggered".to_string(),
                TaggedProperty {
                    name: "bDamageTriggered".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(cfg.grate_breakable),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            mover_idx = Some(idx);
            mover_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitVentDuctSetup {
            duct_brush_index: Some(brush_idx),
            duct_brush: brush,
            grate_actor_index: mover_idx,
            grate_actor: mover_act,
            description: format!("Committed Vent Duct ({}, Length:{:.0} uu)", cfg.tag, cfg.duct_length_uu),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Vent Duct to World: Duct #{}, GrateActor={:?}.", brush_idx, mover_idx));

        (brush_idx, mover_idx)
    }

    // =========================================================================
    // Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
    // =========================================================================

    pub fn apply_piston_archetype(&mut self, arch: ut99_bsp::PistonArchetype) {
        let old_config = self.piston_settings.active_config.clone();
        let new_config = arch.to_config();
        self.piston_settings.active_config = new_config.clone();

        self.recalculate_piston_preview();

        self.history.push(crate::history::Action::UpdatePistonConfig {
            old_config,
            new_config,
            description: format!("Applied Piston Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Piston Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_piston_preview(&mut self) {
        let cfg = &self.piston_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelPistonContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_piston_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_piston_safety(cfg, &peers);
        let brush = ut99_bsp::generate_piston_brush(cfg);

        self.piston_settings.cached_metrics = metrics;
        self.piston_settings.cached_hazards = hazards;
        self.piston_settings.cached_piston_brush = Some(brush);
    }

    pub fn commit_piston_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_piston_preview();

        let cfg = self.piston_settings.active_config.clone();
        let brush = self.piston_settings.cached_piston_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_piston_brush(&cfg));

        // 1. Add Piston Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut mover_idx = None;
        let mut mover_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("PistonMover_{}", unr.actors.len());
            act.class_name = "Engine.Mover".to_string();
            act.location = cfg.position;
            act.tag = cfg.tag.clone();
            act.event = cfg.event.clone();
            act.group = "PistonTraps".to_string();
            let move_time = (cfg.stroke_length_uu / cfg.stroke_velocity_uu_s.max(1.0)).max(0.05);
            act.properties.insert(
                "MoveTime".to_string(),
                TaggedProperty {
                    name: "MoveTime".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(move_time),
                },
            );
            act.properties.insert(
                "StayOpenTime".to_string(),
                TaggedProperty {
                    name: "StayOpenTime".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.wait_time_s.max(0.0)),
                },
            );
            act.properties.insert(
                "EncroachDamage".to_string(),
                TaggedProperty {
                    name: "EncroachDamage".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(cfg.squish_damage),
                },
            );

            let mut ed_mover = EditorMover::new(brush.clone());
            ed_mover.name = act.name.clone();
            ed_mover.tag = cfg.tag.clone();
            ed_mover.event = cfg.event.clone();
            ed_mover.move_time = move_time;
            ed_mover.stay_open_time = cfg.wait_time_s.max(0.0);
            ed_mover.keyframes[0] = Some(MoverKeyframe {
                location: cfg.position,
                rotation: Vec3::ZERO,
            });
            ed_mover.keyframes[1] = Some(MoverKeyframe {
                location: cfg.position + Vec3::new(0.0, 0.0, -cfg.stroke_length_uu),
                rotation: Vec3::ZERO,
            });
            self.movers.push(ed_mover);

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            unr.sync_actor_views();
            mover_idx = Some(idx);
            mover_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitPistonSetup {
            piston_brush_index: Some(brush_idx),
            piston_brush: brush,
            mover_actor_index: mover_idx,
            mover_actor: mover_act,
            description: format!("Committed Piston ({}, Stroke:{:.0} uu)", cfg.tag, cfg.stroke_length_uu),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Piston to World: Brush #{}, MoverActor={:?}.", brush_idx, mover_idx));

        (brush_idx, mover_idx)
    }

    // =========================================================================
    // Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
    // =========================================================================

    pub fn apply_jump_boots_archetype(&mut self, arch: ut99_bsp::JumpBootsArchetype) {
        let old_config = self.jump_boots_settings.active_config.clone();
        let new_config = arch.to_config();
        self.jump_boots_settings.active_config = new_config.clone();

        self.recalculate_jump_boots_preview();

        self.history.push(crate::history::Action::UpdateJumpBootsConfig {
            old_config,
            new_config,
            description: format!("Applied Jump Boots Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Jump Boots Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_jump_boots_preview(&mut self) {
        let cfg = &self.jump_boots_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelJumpBootsContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_jump_boots_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_jump_boots_safety(cfg, &peers);
        let brush = ut99_bsp::generate_jump_boots_brush(cfg);

        self.jump_boots_settings.cached_metrics = metrics;
        self.jump_boots_settings.cached_hazards = hazards;
        self.jump_boots_settings.cached_cradle_brush = Some(brush);
    }

    pub fn commit_jump_boots_to_world(&mut self) -> (usize, Option<usize>) {
        self.recalculate_jump_boots_preview();

        let cfg = self.jump_boots_settings.active_config.clone();
        let brush = self.jump_boots_settings.cached_cradle_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_jump_boots_brush(&cfg));

        // 1. Add Cradle Brush to world
        let brush_idx = self.world_brushes.len();
        self.world_brushes.push(brush.clone());

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut boots_idx = None;
        let mut boots_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("JumpBoots_{}", unr.actors.len());
            act.class_name = "Botpack.UT_JumpBoots".to_string();
            act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.cradle_height_uu + 20.0);
            act.tag = cfg.tag.clone();
            act.event = cfg.event.clone();
            act.group = "JumpBootsCaches".to_string();
            act.properties.insert(
                "Charge".to_string(),
                TaggedProperty {
                    name: "Charge".to_string(),
                    array_index: 0,
                    value: TaggedValue::Int(cfg.charge_count as i32),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            boots_idx = Some(idx);
            boots_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitJumpBootsSetup {
            cradle_brush_index: Some(brush_idx),
            cradle_brush: brush,
            boots_actor_index: boots_idx,
            boots_actor: boots_act,
            description: format!("Committed Jump Boots ({}, Boost:{:.0} uu/s)", cfg.tag, cfg.launch_boost_vz),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Jump Boots to World: Cradle #{}, BootsActor={:?}.", brush_idx, boots_idx));

        (brush_idx, boots_idx)
    }

    // =========================================================================
    // Interactive Force Field, Energy Barrier & Power Core Overload Studio
    // =========================================================================

    pub fn apply_force_field_archetype(&mut self, arch: ut99_bsp::ForceFieldArchetype) {
        let old_config = self.force_field_settings.active_config.clone();
        let new_config = arch.to_config();
        self.force_field_settings.active_config = new_config.clone();

        self.recalculate_force_field_preview();

        self.history.push(crate::history::Action::UpdateForceFieldConfig {
            old_config,
            new_config,
            description: format!("Applied Force Field Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Force Field Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_force_field_preview(&mut self) {
        let cfg = &self.force_field_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelForceFieldContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_force_field_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_force_field_safety(cfg, &peers);
        let (field_brush, pylon_brushes) = ut99_bsp::generate_force_field_brushes(cfg);

        self.force_field_settings.cached_metrics = metrics;
        self.force_field_settings.cached_hazards = hazards;
        self.force_field_settings.cached_field_brush = Some(field_brush);
        self.force_field_settings.cached_pylon_brushes = pylon_brushes;
    }

    pub fn commit_force_field_to_world(&mut self) -> (usize, Vec<usize>, Option<usize>) {
        self.recalculate_force_field_preview();

        let cfg = self.force_field_settings.active_config.clone();
        let field_brush = self.force_field_settings.cached_field_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_force_field_brushes(&cfg).0);
        let pylon_brushes = self.force_field_settings.cached_pylon_brushes.clone();

        // 1. Add field brush
        let field_idx = self.world_brushes.len();
        self.world_brushes.push(field_brush.clone());

        // 2. Add pylon brushes
        let mut pylon_indices = Vec::new();
        for pylon in &pylon_brushes {
            let idx = self.world_brushes.len();
            self.world_brushes.push(pylon.clone());
            pylon_indices.push(idx);
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut gen_idx = None;
        let mut gen_act = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("ForceFieldGen_{}", unr.actors.len());
            act.class_name = if cfg.generator_linked { "Engine.Trigger".to_string() } else { "Engine.Light".to_string() };
            act.location = cfg.position + Vec3::new(0.0, 0.0, cfg.height_uu * 0.5);
            act.tag = if !cfg.generator_tag.is_empty() { cfg.generator_tag.clone() } else { cfg.tag.clone() };
            act.event = cfg.event.clone();
            act.group = "ForceFields".to_string();
            act.properties.insert(
                "DamageThreshold".to_string(),
                TaggedProperty {
                    name: "DamageThreshold".to_string(),
                    array_index: 0,
                    value: TaggedValue::Float(cfg.voltage_kv),
                },
            );

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            gen_idx = Some(idx);
            gen_act = Some(act);
        }

        self.history.push(crate::history::Action::CommitForceFieldSetup {
            field_brush_index: Some(field_idx),
            field_brush,
            pylon_brush_indices: pylon_indices.clone(),
            pylon_brushes,
            generator_actor_index: gen_idx,
            generator_actor: gen_act,
            description: format!("Committed Force Field ({}, Voltage:{:.0} kV)", cfg.tag, cfg.voltage_kv),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Force Field to World: Field #{}, Pylons={:?}, GenActor={:?}.", field_idx, pylon_indices, gen_idx));

        (field_idx, pylon_indices, gen_idx)
    }

    // =========================================================================
    // Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
    // =========================================================================

    pub fn apply_reactor_core_archetype(&mut self, arch: ut99_bsp::ReactorCoreArchetype) {
        let old_config = self.reactor_core_settings.active_config.clone();
        let new_config = arch.to_config();
        self.reactor_core_settings.active_config = new_config.clone();

        self.recalculate_reactor_core_preview();

        self.history.push(crate::history::Action::UpdateReactorCoreConfig {
            old_config,
            new_config,
            description: format!("Applied Reactor Core Archetype: {}", arch.display_name()),
        });
        self.log(format!("Applied Reactor Core Archetype: {}.", arch.display_name()));
    }

    pub fn recalculate_reactor_core_preview(&mut self) {
        let cfg = &self.reactor_core_settings.active_config;

        let mut peers = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                peers.push(ut99_bsp::LevelReactorContextActorRecord {
                    name: actor.name.clone(),
                    class_name: actor.class_name.clone(),
                    position: actor.location,
                });
            }
        }

        let metrics = ut99_bsp::evaluate_reactor_core_metrics(cfg, &peers);
        let hazards = ut99_bsp::audit_reactor_core_safety(cfg, &peers);
        let (core_brush, ring_brushes) = ut99_bsp::generate_reactor_core_brushes(cfg);

        self.reactor_core_settings.cached_metrics = metrics;
        self.reactor_core_settings.cached_hazards = hazards;
        self.reactor_core_settings.cached_core_brush = Some(core_brush);
        self.reactor_core_settings.cached_ring_brushes = ring_brushes;
    }

    pub fn commit_reactor_core_to_world(&mut self) -> (usize, Vec<usize>, Option<usize>) {
        self.recalculate_reactor_core_preview();

        let cfg = self.reactor_core_settings.active_config.clone();
        let core_brush = self.reactor_core_settings.cached_core_brush
            .clone()
            .unwrap_or_else(|| ut99_bsp::generate_reactor_core_brushes(&cfg).0);
        let ring_brushes = self.reactor_core_settings.cached_ring_brushes.clone();

        // 1. Add Core brush
        let core_idx = self.world_brushes.len();
        self.world_brushes.push(core_brush.clone());

        // 2. Add Ring brushes
        let mut ring_indices = Vec::new();
        for ring in &ring_brushes {
            let idx = self.world_brushes.len();
            self.world_brushes.push(ring.clone());
            ring_indices.push(idx);
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let mut act_idx = None;
        let mut act_rec = None;

        if let Some(ref mut unr) = self.loaded_unr {
            let mut act = ut99_format::ActorRecord::default();
            act.name = format!("ReactorCore_{}", unr.actors.len());
            act.class_name = "Engine.RotatingMover".to_string();
            act.location = cfg.position;
            act.tag = cfg.tag.clone();
            act.event = cfg.event.clone();
            act.group = "ReactorCores".to_string();
            let yaw_rate = ((cfg.outer_ring_rpm / 60.0) * 65536.0).round() as i32;
            let pitch_rate = ((cfg.mid_ring_rpm / 60.0) * 65536.0).round() as i32;
            let roll_rate = ((cfg.outer_ring_rpm / 120.0) * 65536.0).round() as i32;
            act.properties.insert(
                "RotationRate".to_string(),
                TaggedProperty {
                    name: "RotationRate".to_string(),
                    array_index: 0,
                    value: TaggedValue::Rotator {
                        pitch: pitch_rate,
                        yaw: yaw_rate,
                        roll: roll_rate,
                    },
                },
            );
            act.properties.insert(
                "bFixedRotationDir".to_string(),
                TaggedProperty {
                    name: "bFixedRotationDir".to_string(),
                    array_index: 0,
                    value: TaggedValue::Bool(true),
                },
            );

            let mover_brush = ring_brushes.first().cloned().unwrap_or_else(|| core_brush.clone());
            let mut ed_mover = crate::editor_state::EditorMover::new(mover_brush.clone());
            ed_mover.name = act.name.clone();
            ed_mover.mover_class = "RotatingMover".to_string();
            ed_mover.tag = act.tag.clone();
            ed_mover.event = act.event.clone();
            ed_mover.keyframes[1] = Some(crate::editor_state::MoverKeyframe {
                location: mover_brush.location,
                rotation: mover_brush.rotation + Vec3::new(0.0, 90.0, 0.0),
            });
            self.movers.push(ed_mover);

            let idx = unr.actors.len();
            unr.actors.push(act.clone());
            act_idx = Some(idx);
            act_rec = Some(act);
        }

        self.history.push(crate::history::Action::CommitReactorCoreSetup {
            core_brush_index: Some(core_idx),
            core_brush,
            ring_brush_indices: ring_indices.clone(),
            ring_brushes,
            reactor_actor_index: act_idx,
            reactor_actor: act_rec,
            description: format!("Committed Reactor Core ({}, OuterRPM:{:.1})", cfg.tag, cfg.outer_ring_rpm),
        });

        self.map_is_dirty = true;
        self.log(format!("Committed Reactor Core to World: Core #{}, Rings={:?}, MoverActor={:?}.", core_idx, ring_indices, act_idx));

        (core_idx, ring_indices, act_idx)
    }
}
