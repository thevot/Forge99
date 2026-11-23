use super::*;
use glam::Vec3;

impl EditorState {
    // --- Build & Compilation Pipeline (UnrealEd 2.0 Parity) ---
    pub fn build_geometry(&mut self) -> ut99_bsp::BspBuildReport {
        let brushes: Vec<Brush> = if self.bsp_config.only_visible_brushes {
            self.world_brushes.iter().enumerate()
                .filter(|(idx, _)| self.is_brush_group_visible(*idx))
                .map(|(_, b)| b.clone())
                .collect()
        } else {
            self.world_brushes.to_vec()
        };

        let (_node, report) = ut99_bsp::build_bsp_from_brushes(&brushes, &self.bsp_config);
        self.last_bsp_report = Some(report.clone());
        self.build_progress = Some((1.0, format!("BSP Geometry: {} nodes, {} cuts in {} ms", report.node_count, report.cut_count, report.elapsed_ms)));
        self.log(format!(
            "Build Geometry: Rebuilt BSP tree in {} ms. {} nodes, {} polygons, {} cuts, max depth {}, balance {:.1}%.",
            report.elapsed_ms, report.node_count, report.poly_count, report.cut_count, report.max_depth, report.balance_ratio_pct
        ));
        report
    }

    pub fn build_all_complete(&mut self) {
        let _checkpoint_msg = self.trigger_auto_save();
        self.build_geometry();
        if self.bsp_config.build_lighting {
            self.build_lighting();
        }
        if self.bsp_config.build_paths {
            self.build_ai_paths();
        }
        if self.bsp_config.build_decals {
            self.bake_projector_decals();
        }
        self.log("Build All: Pre-Build Safety Checkpoint saved. Geometry, Lighting, Bot AI Paths and Decals successfully compiled.");
    }

    pub(crate) fn actor_to_scene_light(actor: &ut99_format::ActorRecord) -> ut99_bsp::SceneLight {
        let is_special_lit = actor.properties.get("bSpecialLit")
            .and_then(|p| match p.value {
                TaggedValue::Bool(b) => Some(b),
                TaggedValue::Byte(b) => Some(b != 0),
                TaggedValue::Int(i) => Some(i != 0),
                _ => None,
            })
            .unwrap_or(false);
        let period = actor.properties.get("LightPeriod")
            .and_then(|p| match p.value {
                TaggedValue::Byte(b) => Some(b),
                TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            })
            .unwrap_or(32);
        let phase = actor.properties.get("LightPhase")
            .and_then(|p| match p.value {
                TaggedValue::Byte(b) => Some(b),
                TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            })
            .unwrap_or(0);
        let cone = actor.properties.get("LightCone")
            .and_then(|p| match p.value {
                TaggedValue::Byte(b) => Some(b),
                TaggedValue::Int(i) => Some(i as u8),
                _ => None,
            })
            .unwrap_or(64);

        ut99_bsp::SceneLight {
            name: actor.name.clone(),
            location: actor.location,
            brightness: actor.light_brightness,
            radius: actor.light_radius,
            hue: actor.light_hue,
            saturation: actor.light_saturation,
            light_type: actor.light_type,
            light_effect: actor.light_effect,
            period,
            phase,
            cone,
            is_special_lit,
        }
    }

    pub fn build_lighting(&mut self) {
        self.build_lighting_impl(true);
    }

    pub(super) fn build_lighting_without_history(&mut self) {
        self.build_lighting_impl(false);
    }

    fn build_lighting_impl(&mut self, record_history: bool) {
        let mut lights = Vec::new();
        let only_visible = self.bsp_config.only_visible_brushes;

        // 1. Gather lights from loaded .unr level (respecting Only Rebuild Visible Actors when enabled)
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if only_visible && !self.is_actor_group_visible(&actor.group) {
                    continue;
                }
                if actor.class_name.contains("Light") {
                    lights.push(Self::actor_to_scene_light(actor));
                }
            }
        }

        // If no lights in scene, add a sensible default light at room center so map isn't pitch black
        if lights.is_empty() {
            lights.push(ut99_bsp::SceneLight {
                name: "DefaultFillLight".to_string(),
                location: Vec3::new(0.0, 0.0, 200.0),
                brightness: 128,
                radius: 64,
                hue: 0,
                saturation: 255,
                light_type: 0,
                light_effect: 0,
                period: 32,
                phase: 0,
                cone: 64,
                is_special_lit: false,
            });
        }

        let old_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = self.world_brushes
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.polys.clone()))
            .collect();

        let mut stats = if only_visible {
            let visible_indices: Vec<usize> = (0..self.world_brushes.len())
                .filter(|&i| self.is_brush_group_visible(i))
                .collect();
            ut99_bsp::bake_lighting_selective(
                &mut self.world_brushes,
                &lights,
                &visible_indices,
                &self.lighting_config,
            )
        } else {
            ut99_bsp::bake_lighting_for_brushes(&mut self.world_brushes, &lights, &self.lighting_config)
        };

        // Also bake lighting onto all Mover brushes positioned at their BrushRaytraceKey / current_key keyframe (REQ-F05 & REQ-E05)
        for mover in &mut self.movers {
            let rt_key = mover.current_key.min(7);
            let mut temp_brush = mover.brush.clone();
            if let Some(Some(kf)) = mover.keyframes.get(rt_key) {
                temp_brush.location = kf.location;
                temp_brush.rotation = kf.rotation;
            }
            let mut single = vec![temp_brush];
            let m_stats = ut99_bsp::bake_lighting_for_brushes(&mut single, &lights, &self.lighting_config);
            if let Some(baked) = single.into_iter().next() {
                mover.brush.polys = baked.polys;
            }
            stats.total_polygons += m_stats.total_polygons;
            stats.total_luxels += m_stats.total_luxels;
            stats.shadow_rays_cast += m_stats.shadow_rays_cast;
        }

        let new_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = self.world_brushes
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.polys.clone()))
            .collect();

        if record_history { self.history.push(crate::history::Action::BakeLighting {
            old_polys,
            new_polys,
            description: if only_visible {
                "Visible-Only Scene Lighting Bake".to_string()
            } else {
                "Full Scene Lighting Bake".to_string()
            },
        }); }

        self.last_lighting_stats = Some(stats.clone());
        self.build_progress = Some((1.0, format!("Lighting Bake: {} luxels in {} ms", stats.total_luxels, stats.elapsed_ms)));
        self.map_is_dirty = true;
        self.log(format!(
            "Bake Lighting: Completed in {} ms across {} polygons. Baked {} luxels, cast {} shadow rays using {} lights.",
            stats.elapsed_ms, stats.total_polygons, stats.total_luxels, stats.shadow_rays_cast, lights.len()
        ));
    }

    /// Bakes lighting differentially only for brushes affected by currently selected/active lights
    pub fn bake_selected_lights_only(&mut self) -> Result<ut99_bsp::LightingStats, String> {
        let mut target_lights = Vec::new();

        // 1. Gather selected lights
        if let Some(ref unr) = self.loaded_unr {
            for (idx, actor) in unr.actors.iter().enumerate() {
                let is_selected = self.selected_actor_index == Some(idx)
                    || self.multi_selected_actors.contains(&idx);

                if is_selected && actor.class_name.contains("Light") {
                    target_lights.push(Self::actor_to_scene_light(actor));
                }
            }
        }

        if target_lights.is_empty() {
            if let Some(ref unr) = self.loaded_unr {
                if let Some(actor) = unr.actors.iter().find(|a| a.class_name.contains("Light")) {
                    target_lights.push(Self::actor_to_scene_light(actor));
                }
            }
        }

        if target_lights.is_empty() {
            return Err("No Light actors found in level or selected to bake.".to_string());
        }

        // 2. Determine affected brushes by spherical distance overlap in world space
        let mut affected_brush_indices = Vec::new();
        for (b_idx, brush) in self.world_brushes.iter().enumerate() {
            let mut affected = false;
            let world_polys = brush.to_world_polys();
            for light in &target_lights {
                let light_radius = light.radius as f32 * 25.0;
                for poly in &world_polys {
                    for world_v in &poly.vertices {
                        if world_v.distance(light.location) <= light_radius + 32.0 {
                            affected = true;
                            break;
                        }
                    }
                    if affected { break; }
                }
                if affected { break; }
            }
            if affected {
                affected_brush_indices.push(b_idx);
            }
        }

        if affected_brush_indices.is_empty() && !self.world_brushes.is_empty() {
            affected_brush_indices.push(0);
        }

        // 3. Snapshot for Undo
        let old_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = affected_brush_indices
            .iter()
            .map(|&i| (i, self.world_brushes[i].polys.clone()))
            .collect();

        // 4. Differential Bake
        let stats = ut99_bsp::bake_lighting_selective(
            &mut self.world_brushes,
            &target_lights,
            &affected_brush_indices,
            &self.lighting_config,
        );

        let new_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = affected_brush_indices
            .iter()
            .map(|&i| (i, self.world_brushes[i].polys.clone()))
            .collect();

        self.history.push(crate::history::Action::BakeLighting {
            old_polys,
            new_polys,
            description: format!("Differential Bake for {} lights", target_lights.len()),
        });

        self.last_lighting_stats = Some(stats.clone());
        self.map_is_dirty = true;
        self.log(format!(
            "⚡ Differential Lighting: Baked {} brushes in {} ms ({} luxels, {} lights)",
            affected_brush_indices.len(), stats.elapsed_ms, stats.total_luxels, target_lights.len()
        ));

        Ok(stats)
    }

    /// Scales `LightBrightness` of selected or all Light actors by a literal delta or percentage (`Deliverable B Categoria F.2` — UnrealEd 2.0 `Scale Lights...`).
    pub fn scale_lights_brightness(&mut self, value: f32, is_percentage: bool, selected_only: bool) -> usize {
        let mut modifications = Vec::new();
        if let Some(ref mut unr) = self.loaded_unr {
            let target_indices: Vec<usize> = if selected_only {
                if !self.multi_selected_actors.is_empty() {
                    self.multi_selected_actors.iter().copied().collect()
                } else if let Some(idx) = self.selected_actor_index {
                    vec![idx]
                } else {
                    Vec::new()
                }
            } else {
                (0..unr.actors.len()).collect()
            };

            for idx in target_indices {
                if let Some(actor) = unr.actors.get_mut(idx) {
                    if actor.class_name.contains("Light") {
                        let old_actor = actor.clone();
                        let old_b = actor.light_brightness as f32;
                        let new_b = if is_percentage {
                            (old_b * (value / 100.0)).round().clamp(0.0, 255.0) as u8
                        } else {
                            (old_b + value).round().clamp(0.0, 255.0) as u8
                        };
                        actor.light_brightness = new_b;
                        actor.properties.insert(
                            "LightBrightness".to_string(),
                            TaggedProperty {
                                name: "LightBrightness".to_string(),
                                array_index: 0,
                                value: TaggedValue::Byte(new_b),
                            },
                        );
                        modifications.push((idx, old_actor, actor.clone()));
                    }
                }
            }
        }

        let count = modifications.len();
        if count > 0 {
            let desc = if is_percentage {
                format!("Scale Lights ({:.0}%)", value)
            } else {
                format!("Scale Lights ({:+.0} literal)", value)
            };
            self.history.push(crate::history::Action::BatchUpdateLightEffects {
                modifications,
                description: desc.clone(),
            });
            self.map_is_dirty = true;
            self.log(format!("{}: modified {} Light actor(s).", desc, count));
        }
        count
    }

    /// Apply a canonical tournament color & intensity preset to all selected Light actors
    pub fn apply_light_preset_to_selected(&mut self, preset: TournamentLightPreset) -> usize {
        let (hue, sat, bright, rad) = preset.values();
        let mut modified_count = 0;

        if let Some(ref mut unr) = self.loaded_unr {
            let selected_set: Vec<usize> = if !self.multi_selected_actors.is_empty() {
                self.multi_selected_actors.iter().copied().collect()
            } else if let Some(idx) = self.selected_actor_index {
                vec![idx]
            } else {
                unr.actors.iter().enumerate()
                    .filter(|(_, a)| a.class_name.contains("Light"))
                    .map(|(i, _)| i)
                    .collect()
            };

            for &idx in &selected_set {
                if idx < unr.actors.len() && unr.actors[idx].class_name.contains("Light") {
                    let actor = &mut unr.actors[idx];
                    actor.light_hue = hue;
                    actor.light_saturation = sat;
                    actor.light_brightness = bright;
                    actor.light_radius = rad;
                    modified_count += 1;
                }
            }
        }

        if modified_count > 0 {
            self.map_is_dirty = true;
            self.log(format!("🎨 Applied preset '{}' to {} Light actor(s)", preset.label(), modified_count));
        }

        modified_count
    }

    /// Evaluate overall Luxmeter light levels and competitive coverage distribution
    pub fn compute_luxmeter_stats(&self) -> LuxmeterCoverageStats {
        let mut stats = LuxmeterCoverageStats::default();
        let mut total_lux_sum = 0.0;
        let mut sample_count = 0;

        for brush in &self.world_brushes {
            for poly in &brush.polys {
                stats.total_surfaces += 1;
                let mut poly_has_light = false;

                // Evaluate vertex colors
                for &c in &poly.vertex_colors {
                    let lux = (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) * 100.0;
                    total_lux_sum += lux;
                    sample_count += 1;

                    if lux < 15.0 {
                        stats.under_lit_count += 1;
                    } else if lux < 50.0 {
                        stats.shadow_count += 1;
                    } else if lux <= 100.0 {
                        stats.tournament_ideal_count += 1;
                    } else if lux <= 180.0 {
                        stats.bright_count += 1;
                    } else {
                        stats.overbright_count += 1;
                    }

                    if lux > 2.0 {
                        poly_has_light = true;
                    }
                }

                // Evaluate lightmap luxels
                if let Some(ref lm) = poly.lightmap {
                    stats.total_luxels += lm.luxels.len();
                    for &luxel in &lm.luxels {
                        let lux = (0.2126 * luxel[0] + 0.7152 * luxel[1] + 0.0722 * luxel[2]) * 100.0;
                        total_lux_sum += lux;
                        sample_count += 1;

                        if lux < 15.0 {
                            stats.under_lit_count += 1;
                        } else if lux < 50.0 {
                            stats.shadow_count += 1;
                        } else if lux <= 100.0 {
                            stats.tournament_ideal_count += 1;
                        } else if lux <= 180.0 {
                            stats.bright_count += 1;
                        } else {
                            stats.overbright_count += 1;
                        }

                        if lux > 2.0 {
                            poly_has_light = true;
                        }
                    }
                }

                if poly_has_light {
                    stats.lit_surfaces += 1;
                }
            }
        }

        if sample_count > 0 {
            stats.average_lux = total_lux_sum / sample_count as f32;
        }

        stats
    }

    // ==========================================
    // Real-time Zone Fog & Volumetric Attenuation Studio
    // ==========================================

    /// Scans all zone actors in the level and extracts volumetric fog properties and metrics
    pub fn collect_level_zone_fog_profiles(&self) -> Vec<ZoneFogProfile> {
        let mut profiles = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            for (idx, actor) in level.actors.iter().enumerate() {
                if !actor.class_name.contains("Zone") {
                    continue;
                }

                let is_water_zone = actor.class_name.contains("WaterZone");
                let is_slime_zone = actor.class_name.contains("SlimeZone");
                let is_lava_zone = actor.class_name.contains("LavaZone");

                // bFogZone: default true for water/slime/lava zones, or read from actor property
                let b_fog_zone = actor.properties.get("bFogZone")
                    .and_then(|p| match p.value {
                        TaggedValue::Bool(b) => Some(b),
                        TaggedValue::Byte(b) => Some(b != 0),
                        TaggedValue::Int(i) => Some(i != 0),
                        _ => None,
                    })
                    .unwrap_or(is_water_zone || is_slime_zone || is_lava_zone);

                // FogColor
                let fog_color = actor.properties.get("FogColor")
                    .and_then(|p| match p.value {
                        TaggedValue::Color { r, g, b, a } => {
                            Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0])
                        }
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        if is_water_zone {
                            [0.05, 0.25, 0.55, 1.0]
                        } else if is_slime_zone {
                            [0.20, 0.60, 0.10, 1.0]
                        } else if is_lava_zone {
                            [0.65, 0.25, 0.08, 1.0]
                        } else {
                            [0.12, 0.14, 0.18, 1.0]
                        }
                    });

                // FogDistance
                let fog_distance = actor.properties.get("FogDistance")
                    .and_then(|p| match p.value {
                        TaggedValue::Float(f) => Some(f),
                        TaggedValue::Int(i) => Some(i as f32),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        if is_water_zone { 900.0 }
                        else if is_slime_zone { 1200.0 }
                        else if is_lava_zone { 1600.0 }
                        else { 2500.0 }
                    });

                // FogStart
                let fog_start = actor.properties.get("FogStart")
                    .and_then(|p| match p.value {
                        TaggedValue::Float(f) => Some(f),
                        TaggedValue::Int(i) => Some(i as f32),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        (fog_distance * 0.15).min(fog_distance - 50.0).max(10.0)
                    });

                // FogDensity
                let fog_density = actor.properties.get("FogDensity")
                    .and_then(|p| match p.value {
                        TaggedValue::Float(f) => Some(f),
                        _ => None,
                    })
                    .unwrap_or_else(|| {
                        if is_water_zone { 1.5 }
                        else if is_slime_zone { 1.2 }
                        else if is_lava_zone { 1.1 }
                        else { 1.0 }
                    });

                let zone_name = actor.properties.get("ZoneName")
                    .and_then(|p| match &p.value {
                        TaggedValue::Str(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| actor.name.clone());

                profiles.push(ZoneFogProfile {
                    actor_index: idx,
                    actor_name: actor.name.clone(),
                    zone_name,
                    zone_tag: actor.tag.clone(),
                    class_name: actor.class_name.clone(),
                    location: actor.location,
                    b_fog_zone,
                    fog_color,
                    fog_distance,
                    fog_start,
                    fog_density,
                });
            }
        }
        profiles
    }

    /// Applies a canonical environmental fog preset to the specified zone actor with atomic undo/redo
    pub fn apply_fog_preset_to_zone(&mut self, zone_actor_idx: usize, preset: FogPresetId) -> bool {
        self.update_zone_fog_properties(
            zone_actor_idx,
            true,
            preset.color(),
            preset.distance(),
            preset.start(),
            preset.density(),
            format!("Applied fog preset '{}'", preset.label()),
        )
    }

    /// Updates zone fog parameters with atomic undo/redo and live viewport synchronization
    pub fn update_zone_fog_properties(
        &mut self,
        zone_actor_idx: usize,
        enabled: bool,
        color: [f32; 4],
        distance: f32,
        start: f32,
        density: f32,
        description: String,
    ) -> bool {
        let mut old_actor_opt = None;
        let mut new_actor_opt = None;

        let old_viewport = (
            self.distance_fog_enabled,
            self.fog_start,
            self.fog_end,
            self.fog_density,
            self.fog_color,
        );

        if let Some(ref mut level) = self.loaded_unr {
            if let Some(actor) = level.actors.get_mut(zone_actor_idx) {
                old_actor_opt = Some(actor.clone());

                actor.properties.insert(
                    "bFogZone".to_string(),
                    TaggedProperty {
                        name: "bFogZone".to_string(),
                        array_index: 0,
                        value: TaggedValue::Bool(enabled),
                    },
                );

                actor.properties.insert(
                    "FogDistance".to_string(),
                    TaggedProperty {
                        name: "FogDistance".to_string(),
                        array_index: 0,
                        value: TaggedValue::Float(distance),
                    },
                );

                actor.properties.insert(
                    "FogStart".to_string(),
                    TaggedProperty {
                        name: "FogStart".to_string(),
                        array_index: 0,
                        value: TaggedValue::Float(start),
                    },
                );

                actor.properties.insert(
                    "FogDensity".to_string(),
                    TaggedProperty {
                        name: "FogDensity".to_string(),
                        array_index: 0,
                        value: TaggedValue::Float(density),
                    },
                );

                let r_byte = (color[0].clamp(0.0, 1.0) * 255.0) as u8;
                let g_byte = (color[1].clamp(0.0, 1.0) * 255.0) as u8;
                let b_byte = (color[2].clamp(0.0, 1.0) * 255.0) as u8;
                let a_byte = (color[3].clamp(0.0, 1.0) * 255.0) as u8;

                actor.properties.insert(
                    "FogColor".to_string(),
                    TaggedProperty {
                        name: "FogColor".to_string(),
                        array_index: 0,
                        value: TaggedValue::Color {
                            r: r_byte,
                            g: g_byte,
                            b: b_byte,
                            a: a_byte,
                        },
                    },
                );

                new_actor_opt = Some(actor.clone());
            }
        }

        if let (Some(old_actor), Some(new_actor)) = (old_actor_opt, new_actor_opt) {
            // Synchronize viewport parameters
            self.distance_fog_enabled = enabled;
            self.fog_start = start;
            self.fog_end = distance;
            self.fog_density = density;
            self.fog_color = color;

            let new_viewport = (enabled, start, distance, density, color);

            self.history.push(Action::UpdateZoneFog {
                actor_index: zone_actor_idx,
                old_actor,
                new_actor,
                old_viewport_fog: old_viewport,
                new_viewport_fog: new_viewport,
                description: description.clone(),
            });

            self.map_is_dirty = true;
            let act_name = self.loaded_unr.as_ref().map(|l| l.actors[zone_actor_idx].name.clone()).unwrap_or_default();
            self.log(format!("{description} on Zone '{act_name}'"));
            true
        } else {
            false
        }
    }

    /// Spawns a new ZoneInfo actor at the current builder brush pivot with preset fog
    pub fn place_fog_zone_at_pivot(&mut self, preset: FogPresetId) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let name = format!("FogZone_{count}");
        let tag = format!("Fog_{count}");

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "ZoneInfo".to_string();
        actor.name = name.clone();
        actor.tag = tag;
        actor.location = spawn_pos;

        let col = preset.color();
        actor.properties.insert(
            "bFogZone".to_string(),
            TaggedProperty {
                name: "bFogZone".to_string(),
                array_index: 0,
                value: TaggedValue::Bool(true),
            },
        );
        actor.properties.insert(
            "FogDistance".to_string(),
            TaggedProperty {
                name: "FogDistance".to_string(),
                array_index: 0,
                value: TaggedValue::Float(preset.distance()),
            },
        );
        actor.properties.insert(
            "FogStart".to_string(),
            TaggedProperty {
                name: "FogStart".to_string(),
                array_index: 0,
                value: TaggedValue::Float(preset.start()),
            },
        );
        actor.properties.insert(
            "FogDensity".to_string(),
            TaggedProperty {
                name: "FogDensity".to_string(),
                array_index: 0,
                value: TaggedValue::Float(preset.density()),
            },
        );
        actor.properties.insert(
            "FogColor".to_string(),
            TaggedProperty {
                name: "FogColor".to_string(),
                array_index: 0,
                value: TaggedValue::Color {
                    r: (col[0] * 255.0) as u8,
                    g: (col[1] * 255.0) as u8,
                    b: (col[2] * 255.0) as u8,
                    a: (col[3] * 255.0) as u8,
                },
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

        // Sync viewport
        self.distance_fog_enabled = true;
        self.fog_start = preset.start();
        self.fog_end = preset.distance();
        self.fog_density = preset.density();
        self.fog_color = preset.color();
        self.selected_actor_index = Some(new_idx);
        self.zone_fog_settings.selected_zone_index = Some(new_idx);

        self.map_is_dirty = true;
        self.log(format!("Spawned new fog ZoneInfo '{name}' at ({:.1}, {:.1}, {:.1}) with preset '{}'", spawn_pos.x, spawn_pos.y, spawn_pos.z, preset.label()));
        new_idx
    }

    /// Evaluates weapon and combat sightline visibility percentages under given fog parameters
    pub fn compute_weapon_visibility_ratings(fog_start: f32, fog_end: f32, fog_density: f32) -> Vec<(&'static str, f32, f32)> {
        let weapons: &[(&'static str, f32)] = &[
            ("Melee & Enforcer", 256.0),
            ("Flak Cannon & Bio", 768.0),
            ("Shock Rifle & Pulse", 1536.0),
            ("Sniper Sightline", 3072.0),
            ("Cross-Arena Longshot", 5120.0),
        ];

        let span = (fog_end - fog_start).max(1.0);
        weapons
            .iter()
            .map(|&(name, dist)| {
                let factor = ((dist - fog_start) / span).clamp(0.0, 1.0);
                let vis = (1.0 - factor * fog_density.clamp(0.1, 2.0)).clamp(0.0, 1.0);
                (name, dist, vis)
            })
            .collect()
    }

    pub fn build_ai_paths(&mut self) {
        self.ai_path_links.clear();

        // Generate a marker for each item without an existing explicit association.
        // Proximity alone cannot establish an association: nearby items may each need one.
        let mut generated_spots: Vec<(usize, ut99_format::ActorRecord)> = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            let existing_spots: Vec<_> = unr
                .actors
                .iter()
                .filter(|a| a.class_name.rsplit('.').next().unwrap_or(&a.class_name).eq_ignore_ascii_case("InventorySpot"))
                .collect();
            let mut spot_idx = existing_spots.len();
            let mut used_names: std::collections::HashSet<String> = unr.actors.iter()
                .map(|a| a.name.to_ascii_lowercase())
                .collect();

            for (a_idx, actor) in unr.actors.iter().enumerate() {
                let cls = actor.class_name.as_str();
                let is_inventory = cls.contains("Weapon")
                    || cls.contains("Ammo")
                    || cls.contains("Pickup")
                    || cls.contains("Health")
                    || cls.contains("Armor")
                    || cls.contains("Shield")
                    || cls.contains("UDamage")
                    || cls.contains("JumpBoots")
                    || cls.contains("ThighPads")
                    || cls.contains("Vial")
                    || cls.contains("MedBox")
                    || cls.contains("Invisibility")
                    || cls.contains("ShockRifle")
                    || cls.contains("FlakCannon")
                    || cls.contains("PulseGun")
                    || cls.contains("Ripper")
                    || cls.contains("Minigun")
                    || cls.contains("Eightball")
                    || cls.contains("Rocket")
                    || cls.contains("SniperRifle")
                    || cls.contains("BioRifle")
                    || cls.contains("Enforcer")
                    || cls.contains("ImpactHammer")
                    || cls.contains("Redeemer")
                    || cls.contains("WarheadLauncher")
                    || cls.contains("Translocator")
                    || cls.contains("Chainsaw");
                if is_inventory && !cls.contains("InventorySpot") {
                    // Existing references can be unresolved binary object indices. Never
                    // overwrite them merely because they cannot be interpreted here.
                    let has_marker_reference = actor.properties.keys().chain(actor.object_paths.keys())
                        .any(|key| key.eq_ignore_ascii_case("myMarker"));
                    let spot_mentions_item = existing_spots.iter().any(|spot| {
                        let path = spot.object_paths.iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case("markedItem"))
                            .map(|(_, value)| value.as_str());
                        let property = spot.properties.iter()
                            .find(|(key, _)| key.eq_ignore_ascii_case("markedItem"))
                            .and_then(|(_, prop)| match &prop.value {
                                TaggedValue::Name(name) | TaggedValue::Str(name) => Some(name.as_str()),
                                _ => None,
                            });
                        path.or(property).and_then(|value| value.rsplit(|c| c == '.' || c == '\'').find(|part| !part.is_empty()))
                            .is_some_and(|name| name.eq_ignore_ascii_case(&actor.name))
                    });
                    let already_covered = has_marker_reference || spot_mentions_item;
                    if !already_covered {
                        let spot_name = loop {
                            let candidate = format!("InventorySpot{}", spot_idx);
                            spot_idx += 1;
                            if used_names.insert(candidate.to_ascii_lowercase()) {
                                break candidate;
                            }
                        };
                        let mut spot = ut99_format::ActorRecord::default();
                        spot.class_name = "InventorySpot".to_string();
                        spot.name = spot_name;
                        spot.location = actor.location;
                        spot.properties.insert(
                            "markedItem".to_string(),
                            TaggedProperty {
                                name: "markedItem".to_string(),
                                array_index: 0,
                                value: TaggedValue::Name(actor.name.clone()),
                            },
                        );
                        spot.object_paths
                            .insert("markedItem".to_string(), actor.name.clone());
                        generated_spots.push((a_idx, spot));
                    }
                }
            }
        }
        if !generated_spots.is_empty() {
            if let Some(ref mut unr) = self.loaded_unr {
                for (a_idx, spot) in generated_spots {
                    if let Some(item_actor) = unr.actors.get_mut(a_idx) {
                        item_actor.properties.insert(
                            "myMarker".to_string(),
                            TaggedProperty {
                                name: "myMarker".to_string(),
                                array_index: 0,
                                value: TaggedValue::Name(spot.name.clone()),
                            },
                        );
                        item_actor
                            .object_paths
                            .insert("myMarker".to_string(), spot.name.clone());
                    }
                    unr.actors.push(spot);
                }
                unr.sync_actor_views();
            }
        }

        self.refresh_ai_path_links();

        // Keep isolated nodes and their item references intact. Level Doctor must
        // report missing coverage rather than hide it by deleting navigation actors.
    }

    pub fn refresh_ai_path_links(&mut self) {
        self.ai_path_links.clear();
        struct LinkNodeMeta {
            location: Vec3,
            is_inv: bool,
            is_pathnode: bool,
            is_foot: bool,
            is_head: bool,
        }
        let mut link_nodes = Vec::new();
        self.nav_graph.nodes.clear();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                let is_nav = actor.class_name.contains("PathNode")
                    || actor.class_name.contains("PlayerStart")
                    || actor.class_name.contains("InventorySpot")
                    || actor.class_name.contains("Teleporter")
                    || actor.class_name.contains("Lift")
                    || actor.class_name.contains("JumpSpot")
                    || actor.class_name.contains("WarpZoneMarker");
                if is_nav {
                    link_nodes.push(LinkNodeMeta {
                        location: actor.location,
                        is_inv: actor.class_name.contains("InventorySpot"),
                        is_pathnode: actor.class_name.contains("PathNode"),
                        is_foot: actor.name.starts_with("PathNode_Foot_"),
                        is_head: actor.name.starts_with("PathNode_Head_"),
                    });
                    self.nav_graph.add_node(
                        actor.name.clone(),
                        actor.location,
                        actor.class_name.clone(),
                    );
                }
            }
        }

        let nav_index = ut99_bsp::NavBrushIndex::build(&self.world_brushes);

        let n = link_nodes.len();
        let max_walk_dist = 850.0f32;
        let max_jump_dist = 950.0f32;
        let max_step_height = 54.0f32;
        let max_jump_height = 180.0f32;
        let max_drop_height = 450.0f32;

        let mut link_count = 0;

        for i in 0..n {
            let ni = &link_nodes[i];
            let pos_a = ni.location;
            for j in (i + 1)..n {
                let nj = &link_nodes[j];
                let pos_b = nj.location;
                let dist = pos_a.distance(pos_b);
                if dist > max_jump_dist {
                    continue;
                }

                let delta_z = (pos_b.z - pos_a.z).abs();
                let step_dz = if ni.is_inv != nj.is_inv {
                    (delta_z - 32.0).max(0.0)
                } else {
                    delta_z
                };
                let is_ramp_pair = ni.is_pathnode
                    && nj.is_pathnode
                    && ((ni.is_foot && nj.is_head) || (ni.is_head && nj.is_foot));

                // 1. Walkable link (Bright Green), 2. Jump/Lift (Magenta), 3. Drop-down (Cyan)
                let link_color = if is_ramp_pair || (dist <= max_walk_dist && step_dz <= max_step_height) {
                    [0.20, 0.95, 0.35, 0.85]
                } else if delta_z <= max_jump_height {
                    [0.95, 0.25, 0.95, 0.85]
                } else if delta_z <= max_drop_height {
                    [0.20, 0.85, 1.0, 0.65]
                } else {
                    continue;
                };

                // Collision-aware line-of-sight check against solid CSG world brushes
                if nav_index.is_nav_link_blocked(pos_a, pos_b) {
                    continue;
                }

                self.ai_path_links.push((pos_a, pos_b, link_color));
                link_count += 1;
            }
        }

        let report = ut99_bsp::auto_build_navigation_reachspecs_with_index(
            &mut self.nav_graph,
            &nav_index,
            max_walk_dist,
            max_step_height,
            max_jump_height,
        );
        self.last_autopath_report = Some(report);

        self.log(format!("AI Path Build: Computed {} collision-verified navigation links across {} nodes.", link_count, n));
    }

    /// Clear compiled and estimated paths while retaining navigation actors and item references.
    pub fn clear_ai_paths(&mut self) -> usize {
        let had_links = self.ai_path_links.len();
        let compiled = self.loaded_unr.as_ref().map_or(0, |level| level.reach_specs.len());
        if had_links > 0 || compiled > 0 || !self.nav_graph.reach_specs.is_empty() || !self.nav_graph.adjacency.is_empty() || self.last_autopath_report.is_some() {
            self.history.push(Action::ClearAiPaths {
                old_graph: self.nav_graph.clone(),
                old_links: self.ai_path_links.to_vec(),
                old_reach_specs: self.loaded_unr.as_ref().map(|level| level.reach_specs.clone()),
                old_report: self.last_autopath_report.clone(),
            });
            self.clear_ai_path_links_without_history();
            self.map_is_dirty = true;
        }
        self.log(format!("Cleared AI Paths: {} compiled ReachSpecs and {} estimated display links; navigation actors retained.", compiled, had_links));
        self.toast_info("Cleared AI paths; navigation actors retained.");
        compiled + had_links
    }

    pub(super) fn clear_ai_path_links_without_history(&mut self) {
        if let Some(level) = &mut self.loaded_unr {
            level.reach_specs.clear();
        }
        self.ai_path_links.clear();
        self.nav_graph.reach_specs.clear();
        self.nav_graph.adjacency.clear();
        self.last_autopath_report = None;
    }

    /// BL-P3-02: Run "BSP Doctor" auto-repair on all world brushes.
    pub fn run_bsp_doctor(&mut self) -> ut99_bsp::BspDoctorReport {
        let old_brushes = self.world_brushes.to_vec();
        let report = ut99_bsp::repair_brushes_geometry(
            &mut self.world_brushes,
            self.grid_step.max(1.0),
            0.05,
            0.01,
        );
        if report.snapped_vertices_count > 0
            || report.welded_vertices_count > 0
            || report.triangulated_non_planar_polys_count > 0
            || report.removed_degenerate_polys_count > 0
        {
            self.history.push(Action::ReorderBrushes {
                old_brushes,
                new_brushes: self.world_brushes.to_vec(),
                old_selection: self.selected_brush_index,
                new_selection: self.selected_brush_index,
                description: "BSP Doctor Auto-Repair".to_string(),
            });
            self.log(format!(
                "🩺 BSP Doctor repaired world brushes: {} vertices snapped, {} welded, {} non-planar polygons triangulated, {} degenerate polygons removed",
                report.snapped_vertices_count,
                report.welded_vertices_count,
                report.triangulated_non_planar_polys_count,
                report.removed_degenerate_polys_count
            ));
        } else {
            self.log("🩺 BSP Doctor: All world brushes are clean and strictly planar.");
        }
        report
    }


    // --- SkyBox 3D Real-time Parallax & Multi-Zone Environmental Studio ---


    pub fn apply_lighting_preset(&mut self, hue: u8, sat: u8, brightness: u8, radius: u8, effect: u8) {
        let mut modified = 0;
        if let Some(ref mut unr) = self.loaded_unr {
            if let Some(sel_idx) = self.selected_actor_index {
                if let Some(actor) = unr.actors.get_mut(sel_idx) {
                    actor.light_hue = hue;
                    actor.light_saturation = sat;
                    actor.light_brightness = brightness;
                    actor.light_radius = radius;
                    actor.light_effect = effect;
                    modified += 1;
                }
            }
            for &idx in &self.multi_selected_actors {
                if let Some(actor) = unr.actors.get_mut(idx) {
                    actor.light_hue = hue;
                    actor.light_saturation = sat;
                    actor.light_brightness = brightness;
                    actor.light_radius = radius;
                    actor.light_effect = effect;
                    modified += 1;
                }
            }
        }
        if modified > 0 {
            self.log(format!("Applied Lighting Preset to {} light actor(s)", modified));
            self.toast_success(format!("Updated {} light(s)", modified));
        } else {
            // No light selected: spawn a new one with this preset at builder
            self.add_light_at_builder();
            if let Some(ref mut unr) = self.loaded_unr {
                if let Some(actor) = unr.actors.last_mut() {
                    actor.light_hue = hue;
                    actor.light_saturation = sat;
                    actor.light_brightness = brightness;
                    actor.light_radius = radius;
                    actor.light_effect = effect;
                }
            }
            self.toast_success("Spawned Light with Preset");
        }
    }


}

#[cfg(test)]
mod inventory_marker_tests {
    use super::*;

    #[test]
    fn clear_paths_preserves_actors_and_undo_restores_compiled_links() {
        let mut state = EditorState::default();
        let mut level = ut99_format::UnrLevel::default();
        let mut spot = ut99_format::ActorRecord::default();
        spot.name = "ManualSpot".into();
        spot.class_name = "InventorySpot".into();
        level.actors.push(spot);
        level.reach_specs.push(ut99_format::unr::level::ReachSpecRecord {
            start_actor_name: "ManualSpot".into(), end_actor_name: "Other".into(), ..Default::default()
        });
        let original_actors = level.actors.clone();
        let original_specs = level.reach_specs.clone();
        state.loaded_unr = Some(level);
        state.nav_graph.add_node("ManualSpot", Vec3::ZERO, "InventorySpot");
        state.ai_path_links.push((Vec3::ZERO, Vec3::X, [1.0; 4]));
        let original_graph = state.nav_graph.clone();
        let original_links = state.ai_path_links.clone();
        assert_eq!(state.clear_ai_paths(), 2);
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, original_actors);
        assert!(state.loaded_unr.as_ref().unwrap().reach_specs.is_empty());
        assert_eq!(state.nav_graph.nodes, original_graph.nodes);
        state.undo();
        assert_eq!(state.loaded_unr.as_ref().unwrap().reach_specs, original_specs);
        assert_eq!(state.nav_graph, original_graph);
        assert_eq!(state.ai_path_links, original_links);
        state.redo();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, original_actors);
        assert!(state.loaded_unr.as_ref().unwrap().reach_specs.is_empty());
        assert!(state.ai_path_links.is_empty());
    }

    #[test]
    fn nearby_items_receive_separate_markers() {
        let mut state = EditorState::default();
        let mut level = ut99_format::UnrLevel::default();
        for (name, x) in [("AmmoA", 0.0), ("AmmoB", 16.0)] {
            let mut item = ut99_format::ActorRecord::default();
            item.name = name.into();
            item.class_name = "ShockAmmo".into();
            item.location = Vec3::new(x, 0.0, 0.0);
            level.actors.push(item);
        }
        state.loaded_unr = Some(level);
        state.build_ai_paths();
        let actors = state.loaded_unr.as_ref().unwrap().actors.clone();
        let first = actors[0].object_paths.get("myMarker").unwrap();
        let second = actors[1].object_paths.get("myMarker").unwrap();
        assert_ne!(first, second);
        assert_eq!(actors.iter().filter(|a| a.class_name == "InventorySpot").count(), 2);
        state.run_level_doctor_audit();
        assert!(!state.map_audit_issues.iter().any(|issue| issue.category == "InventoryLinks"));
        state.build_ai_paths();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, actors);
    }

    #[test]
    fn existing_unresolved_and_manual_references_are_preserved() {
        let mut state = EditorState::default();
        let mut level = ut99_format::UnrLevel::default();
        let mut item = ut99_format::ActorRecord::default();
        item.name = "AmmoWithBinaryRef".into();
        item.class_name = "ShockAmmo".into();
        item.properties.insert("myMarker".into(), TaggedProperty {
            name: "myMarker".into(), array_index: 0, value: TaggedValue::Object(123),
        });
        let mut other = ut99_format::ActorRecord::default();
        other.name = "ManualAmmo".into();
        other.class_name = "ShockAmmo".into();
        let mut spot = ut99_format::ActorRecord::default();
        spot.name = "ManualSpot".into();
        spot.class_name = "Engine.InventorySpot".into();
        spot.object_paths.insert("markedItem".into(), "ShockAmmo'MyLevel.ManualAmmo'".into());
        level.actors = vec![item, other, spot].into();
        let original = level.actors.clone();
        state.loaded_unr = Some(level);
        state.build_ai_paths();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, original);
    }

    #[test]
    fn generated_marker_avoids_existing_names_and_path_build_is_idempotent() {
        let mut state = EditorState::default();
        let mut level = ut99_format::UnrLevel::default();
        let mut manual = ut99_format::ActorRecord::default();
        manual.name = "InventorySpot1".into();
        manual.class_name = "InventorySpot".into();
        manual.location = Vec3::new(1000.0, 0.0, 0.0);
        let mut other = ut99_format::ActorRecord::default();
        other.name = "inventoryspot2".into();
        other.class_name = "Light".into();
        let mut item = ut99_format::ActorRecord::default();
        item.name = "Rifle".into();
        item.class_name = "ShockRifle".into();
        level.actors = vec![manual.clone(), other.clone(), item].into();
        state.loaded_unr = Some(level);

        state.build_ai_paths();
        let actors = state.loaded_unr.as_ref().unwrap().actors.clone();
        assert_eq!(actors[0], manual);
        assert_eq!(actors[1], other);
        assert_eq!(actors[3].name, "InventorySpot3");
        assert_eq!(actors[2].object_paths.get("myMarker").map(String::as_str), Some("InventorySpot3"));
        assert_eq!(actors[3].object_paths.get("markedItem").map(String::as_str), Some("Rifle"));
        assert!(state.loaded_unr.as_ref().unwrap().other_actors.iter().any(|a| a.name == "InventorySpot3"));

        state.build_ai_paths();
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors, actors);
    }
}

#[cfg(test)]
mod navigation_coverage_tests {
    use super::*;

    #[test]
    fn isolated_inventory_marker_is_preserved_and_reported() {
        let mut state = EditorState::default();
        let mut weapon = ut99_format::ActorRecord::default();
        weapon.name = "ShockRifle0".into();
        weapon.class_name = "ShockRifle".into();
        weapon.location = Vec3::new(0.0, 0.0, 64.0);
        state.loaded_unr = Some(ut99_format::unr::UnrLevel {
            actors: vec![weapon].into(), ..Default::default()
        });
        state.build_ai_paths();
        let actors = &state.loaded_unr.as_ref().unwrap().actors;
        let weapon = actors.iter().find(|a| a.name == "ShockRifle0").unwrap();
        let marker = weapon.object_paths.get("myMarker").unwrap();
        assert!(actors.iter().any(|a| a.name == *marker && a.class_name == "InventorySpot"));
        assert!(state.nav_graph.nodes.iter().any(|n| n.name == *marker));
        assert!(ut99_bsp::LevelDoctor::audit_navigation_graph(&state.nav_graph)
            .iter().any(|i| i.object_name == *marker && i.severity == ut99_bsp::IssueSeverity::Error));
    }
}


impl EditorState {
    /// Appends a 3D world vertex to the active Freehand Polygon Drawing buffer (`Deliverable B Categoria A.3 Gruppo 1`).
    pub fn add_freehand_vertex(&mut self, pt: Vec3) {
        let snapped = if self.snap_to_grid && self.grid_step > 0.0 {
            (pt / self.grid_step).round() * self.grid_step
        } else {
            pt
        };
        self.freehand_points.push(snapped);
        self.log(format!(
            "Freehand Polygon: Added vertex #{} at ({:.0}, {:.0}, {:.0}).",
            self.freehand_points.len(),
            snapped.x,
            snapped.y,
            snapped.z
        ));
    }


    /// Clears the active Freehand Polygon Drawing buffer.
    pub fn clear_freehand_polygon(&mut self) {
        self.freehand_points.clear();
        self.log("Freehand Polygon: Cleared vertex buffer.");
    }


    /// Extrudes the current `freehand_points` closed polygon along its normal by `depth`
    /// into the active Red Builder Brush (`Deliverable B Categoria A.3 Gruppo 1`).
    pub fn extrude_freehand_polygon_to_builder(&mut self, depth: f32) -> Result<usize, String> {
        let n = self.freehand_points.len();
        if n < 3 {
            return Err(format!(
                "Freehand Polygon requires at least 3 vertices (current: {}).",
                n
            ));
        }
        if depth.abs() < 1.0 {
            return Err("Extrusion depth must be at least 1.0 UU.".to_string());
        }

        // Compute center of base polygon
        let base_center = self.freehand_points.iter().copied().sum::<Vec3>() / (n as f32);

        // Compute Newell normal of the polygon; fallback to +Z if degenerate
        let mut newell = Vec3::ZERO;
        for i in 0..n {
            let curr = self.freehand_points[i];
            let next = self.freehand_points[(i + 1) % n];
            newell.x += (curr.y - next.y) * (curr.z + next.z);
            newell.y += (curr.z - next.z) * (curr.x + next.x);
            newell.z += (curr.x - next.x) * (curr.y + next.y);
        }
        let extrude_dir = if newell.length_squared() > 1e-4 {
            newell.normalize()
        } else {
            Vec3::Z
        };
        let offset = extrude_dir * depth;
        let brush_origin = base_center + offset * 0.5;

        let local_base: Vec<Vec3> = self
            .freehand_points
            .iter()
            .map(|p| *p - brush_origin)
            .collect();
        let local_top: Vec<Vec3> = self
            .freehand_points
            .iter()
            .map(|p| *p + offset - brush_origin)
            .collect();

        let mut polys = Vec::with_capacity(n + 2);
        let tex = self
            .active_texture
            .clone()
            .unwrap_or_else(|| "DefaultTexture".to_string());

        let push_oriented = |polys: &mut Vec<ut99_bsp::Polygon>, mut verts: Vec<Vec3>| {
            if let Some(mut poly) = ut99_bsp::Polygon::new(verts.clone()) {
                // Local brush centroid is at Vec3::ZERO; outward normal should have dist_to_point(ZERO) <= 0
                if poly.plane.dist_to_point(Vec3::ZERO) > 0.0 {
                    verts.reverse();
                    if let Some(flipped) = ut99_bsp::Polygon::new(verts) {
                        poly = flipped;
                    }
                }
                poly.texture = tex.clone();
                polys.push(poly);
            }
        };

        // Bottom cap, Top cap, and N side quads
        push_oriented(&mut polys, local_base.clone());
        push_oriented(&mut polys, local_top.clone());
        for i in 0..n {
            let next = (i + 1) % n;
            let quad = vec![
                local_base[i],
                local_base[next],
                local_top[next],
                local_top[i],
            ];
            push_oriented(&mut polys, quad);
        }

        if polys.len() < 4 {
            return Err("Freehand Polygon extrusion produced degenerate geometry.".to_string());
        }

        let old_brush = self.builder_brush.clone();
        let mut new_brush = Brush::new("ActiveBuilder", CsgOperation::Active);
        new_brush.location = brush_origin;
        new_brush.polys = polys;
        let poly_count = new_brush.polys.len();
        self.builder_brush = new_brush.clone();

        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush,
            description: format!("Freehand Extrude ({} verts, depth {:.0})", n, depth),
        });
        self.freehand_extrude_depth = depth;
        self.log(format!(
            "Freehand Polygon extruded to Red Builder Brush ({} vertices -> {} polygons, depth {:.0}).",
            n, poly_count, depth
        ));

        Ok(poly_count)
    }


    pub fn set_builder_sheet_advanced(
        &mut self,
        width: f32,
        height: f32,
        axis: SheetAxis,
        preset: SheetPreset,
    ) {
        self.builder_brush = build_sheet_advanced(
            "BuilderSheet",
            CsgOperation::Add,
            width,
            height,
            axis,
            preset,
        );
        self.builder_sheet_params = (width, height);
        self.builder_sheet_axis = axis;
        self.builder_sheet_preset = preset;
        self.builder_shape = BuilderShape::Sheet;
        self.log(format!("Constructed {:?} sheet ({:.0} x {:.0}, preset: {:?})", axis, width, height, preset));
    }


    pub fn set_builder_arch(&mut self, config: ArchConfig) {
        self.builder_brush = build_arch(
            "BuilderArch",
            CsgOperation::Add,
            &config,
        );
        self.builder_arch_config = config;
        self.builder_shape = BuilderShape::Arch;
        self.log(format!(
            "Constructed {:?} arch (Span: {:.0}, Height: {:.0}, Segments: {})",
            self.builder_arch_config.arch_type,
            self.builder_arch_config.span_width,
            self.builder_arch_config.total_height,
            self.builder_arch_config.segments
        ));
    }


    /// Bakes raytraced lighting strictly onto the currently selected surfaces (`selected_surfaces` or `selected_brush_index`),
    /// leaving unselected polygons' lightmaps and vertex colors untouched (`REQ-F07` — Apply Lights to Selected).
    pub fn build_lighting_for_selected_surfaces(&mut self) -> usize {
        let mut target_surfaces: HashSet<(usize, usize)> = self.selected_surfaces.clone();
        if target_surfaces.is_empty() {
            if let Some(b_idx) = self.selected_brush_index {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    for p_idx in 0..b.polys.len() {
                        target_surfaces.insert((b_idx, p_idx));
                    }
                }
            }
            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    for p_idx in 0..b.polys.len() {
                        target_surfaces.insert((b_idx, p_idx));
                    }
                }
            }
        }
        if target_surfaces.is_empty() {
            self.log("Apply Lights to Selected: No surfaces or brushes selected.".to_string());
            return 0;
        }

        let mut lights = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if actor.class_name.contains("Light") {
                    lights.push(Self::actor_to_scene_light(actor));
                }
            }
        }
        if lights.is_empty() {
            lights.push(ut99_bsp::SceneLight {
                name: "DefaultFillLight".to_string(),
                location: Vec3::new(0.0, 0.0, 200.0),
                brightness: 128,
                radius: 64,
                hue: 0,
                saturation: 255,
                light_type: 0,
                light_effect: 0,
                period: 32,
                phase: 0,
                cone: 64,
                is_special_lit: false,
            });
        }

        let old_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = self
            .world_brushes
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.polys.clone()))
            .collect();

        let mut target_brush_indices: Vec<usize> = target_surfaces.iter().map(|&(b, _)| b).collect();
        target_brush_indices.sort_unstable();
        target_brush_indices.dedup();

        let stats = ut99_bsp::bake_lighting_selective(
            &mut self.world_brushes,
            &lights,
            &target_brush_indices,
            &self.lighting_config,
        );

        // Restore unselected polygons on those brushes so only the explicitly selected surfaces change
        for &b_idx in &target_brush_indices {
            if let (Some(brush), Some((_, orig_polys))) = (
                self.world_brushes.get_mut(b_idx),
                old_polys.iter().find(|(i, _)| *i == b_idx),
            ) {
                for (p_idx, poly) in brush.polys.iter_mut().enumerate() {
                    if !target_surfaces.contains(&(b_idx, p_idx)) {
                        if let Some(orig) = orig_polys.get(p_idx) {
                            poly.vertex_colors = orig.vertex_colors.clone();
                            poly.lightmap = orig.lightmap.clone();
                        }
                    }
                }
            }
        }

        let new_polys: Vec<(usize, Vec<ut99_bsp::Polygon>)> = self
            .world_brushes
            .iter()
            .enumerate()
            .map(|(i, b)| (i, b.polys.clone()))
            .collect();

        let baked_count = target_surfaces.len();
        self.history.push(Action::BakeLighting {
            old_polys,
            new_polys,
            description: format!("Apply Lights to {} Selected Surface(s)", baked_count),
        });
        self.last_lighting_stats = Some(stats);
        self.map_is_dirty = true;
        self.log(format!(
            "Apply Lights to Selected: Baked lighting onto {} selected surface(s).",
            baked_count
        ));
        baked_count
    }


    pub fn evaluate_selected_shadow_penumbra(&mut self) -> Option<PenumbraGradientProfile> {
        let light_pos = if let Some(ref unr) = self.loaded_unr {
            unr.actors.iter()
                .find(|a| a.class_name.eq_ignore_ascii_case("Light"))
                .map(|a| a.location)
                .unwrap_or(Vec3::new(0.0, 0.0, 300.0))
        } else {
            Vec3::new(0.0, 0.0, 300.0)
        };

        let surface_point = if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                b.location
            } else {
                Vec3::ZERO
            }
        } else {
            Vec3::ZERO
        };

        let profile = evaluate_brush_shadow_gradient(
            surface_point,
            Vec3::Z,
            light_pos,
            LightSourceShape::Sphere { radius: 24.0 },
            &self.world_brushes,
            32,
        );

        self.log(format!(
            "Shadow penumbra evaluated: factor {:.2}, in_penumbra: {}, softness: {:.1} uu.",
            profile.penumbra_factor,
            profile.is_in_penumbra,
            profile.penumbra_softness_uu
        ));
        self.shadow_penumbra_profile = Some(profile.clone());
        Some(profile)
    }


    /// UnrealEd 2.0 `Build Options -> Stats -> Refresh` (`REQ-I02` & `REQ-I03`):
    /// Recomputes the BSP tree statistics (`Nodes`, `Polys`, `Nodes:Polys Ratio`, `Max Depth`, `Cuts`)
    /// and 3D BSP Zone Flood-Fill (`Zones`) without mutating actors or lightmaps.
    pub fn refresh_build_stats(&mut self) -> ut99_bsp::BspBuildReport {
        let report = self.build_geometry();
        let flood = self.compute_3d_zone_flood_fill_report();
        self.log(format!(
            "Stats Refreshed: {} Brushes, {} Zones, {} Polygons, {} Nodes (Ratio {:.2}:1, optimal={}).",
            self.world_brushes.len(),
            flood.distinct_zone_count,
            report.poly_count,
            report.node_count,
            report.nodes_to_polys_ratio(),
            report.is_optimal_ratio()
        ));
        report
    }

}
