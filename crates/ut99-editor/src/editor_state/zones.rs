use super::*;
use glam::Vec3;

impl EditorState {
    pub fn find_active_sky_zone(&self) -> Option<(usize, glam::Vec3, String)> {
        if let Some(ref unr) = self.loaded_unr {
            for (idx, a) in unr.actors.iter().enumerate() {
                if a.class_name.eq_ignore_ascii_case("SkyZoneInfo") {
                    return Some((idx, a.location, a.tag.clone()));
                }
            }
        }
        None
    }

    /// Calculate the real-time SkyBox camera projection derived from the viewport camera
    pub fn calculate_skybox_camera(&self, main_camera: &crate::camera::Camera) -> Option<crate::camera::Camera> {
        if !self.sky_zone_settings.enabled {
            return None;
        }
        let (_, sky_pos, _) = self.find_active_sky_zone()?;
        let mut sky_cam = main_camera.clone();

        // In UE1, parallax is calculated relative to map origin or anchor:
        let anchor = self.sky_zone_settings.custom_anchor.unwrap_or(glam::Vec3::ZERO);
        let delta = main_camera.position - anchor;
        sky_cam.position = sky_pos + delta * self.sky_zone_settings.parallax_scale;
        sky_cam.yaw = main_camera.yaw;
        sky_cam.pitch = main_camera.pitch;
        Some(sky_cam)
    }

    /// Place a new SkyZoneInfo actor at the builder brush pivot / camera focus
    pub fn place_sky_zone_anchor_at_pivot(&mut self) -> Result<String, String> {
        let spawn_pos = self.builder_brush.location;

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let sky_count = unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case("SkyZoneInfo")).count();
        let name = if sky_count == 0 { "SkyZoneInfo0".to_string() } else { format!("SkyZoneInfo{}", sky_count) };
        let tag = "SkyZone".to_string();

        let mut actor = ut99_format::ActorRecord::default();
        actor.name = name.clone();
        actor.class_name = "SkyZoneInfo".to_string();
        actor.tag = tag;
        actor.location = spawn_pos;
        actor.properties.insert(
            "bHidden".into(),
            ut99_format::TaggedProperty {
                name: "bHidden".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Bool(true),
            },
        );
        actor.properties.insert(
            "bStatic".into(),
            ut99_format::TaggedProperty {
                name: "bStatic".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Bool(true),
            },
        );

        unr.actors.push(actor.clone());
        self.history.push(crate::history::Action::AddActor { actor });
        self.map_is_dirty = true;
        self.log(format!("🌌 Placed SkyZoneInfo anchor at ({:.0}, {:.0}, {:.0})", spawn_pos.x, spawn_pos.y, spawn_pos.z));
        Ok(name)
    }

    /// Mark selected surfaces (or current brush) with PF_FAKE_BACKDROP
    pub fn mark_selected_surfaces_as_sky(&mut self, is_sky: bool) -> usize {
        let mut modified = 0;
        let mut brush_map: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();

        if !self.selected_surfaces.is_empty() {
            for &(b_idx, p_idx) in &self.selected_surfaces {
                brush_map.entry(b_idx).or_default().push(p_idx);
            }
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                brush_map.insert(b_idx, (0..b.polys.len()).collect());
            }
        }

        for (b_idx, poly_indices) in brush_map {
            if let Some(b) = self.world_brushes.get_mut(b_idx) {
                let old_polys = b.polys.clone();
                for p_idx in poly_indices {
                    if let Some(poly) = b.polys.get_mut(p_idx) {
                        if is_sky {
                            poly.flags |= ut99_bsp::PF_FAKE_BACKDROP | ut99_bsp::PF_UNLIT;
                        } else {
                            poly.flags &= !ut99_bsp::PF_FAKE_BACKDROP;
                        }
                        modified += 1;
                    }
                }
                self.history.push(crate::history::Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys: b.polys.clone(),
                    description: if is_sky { "Mark Sky Surfaces (PF_FAKE_BACKDROP)".into() } else { "Unmark Sky Surfaces".into() },
                });
            }
        }

        if modified > 0 {
            self.map_is_dirty = true;
            let action_str = if is_sky { "Marked" } else { "Unmarked" };
            self.log(format!("{} {} surface(s) as Sky Backdrop (PF_FAKE_BACKDROP)", action_str, modified));
        }

        modified
    }

    /// Scan and collect all zone profiles in the level
    pub fn collect_level_zone_profiles(&self) -> Vec<ZoneProfile> {
        let mut profiles = Vec::new();

        let check_actor = |a: &ut99_format::ActorRecord, profiles: &mut Vec<ZoneProfile>| {
            let is_zone = a.class_name.eq_ignore_ascii_case("ZoneInfo")
                || a.class_name.eq_ignore_ascii_case("SkyZoneInfo")
                || a.class_name.eq_ignore_ascii_case("WaterZone")
                || a.class_name.eq_ignore_ascii_case("SlimeZone")
                || a.class_name.eq_ignore_ascii_case("LavaZone")
                || a.properties.contains_key("ZoneGravity")
                || a.properties.contains_key("bWaterZone");

            if is_zone {
                let is_sky_zone = a.class_name.eq_ignore_ascii_case("SkyZoneInfo");
                let is_water_zone = a.class_name.eq_ignore_ascii_case("WaterZone")
                    || a.class_name.eq_ignore_ascii_case("SlimeZone")
                    || a.class_name.eq_ignore_ascii_case("LavaZone")
                    || a.properties.get("bWaterZone").map(|p| match p.value {
                        ut99_format::TaggedValue::Bool(b) => b,
                        _ => false,
                    }).unwrap_or(false);

                let gravity = a.properties.get("ZoneGravity").map(|p| match p.value {
                    ut99_format::TaggedValue::Vector(v) => v,
                    _ => if is_water_zone { glam::Vec3::new(0.0, 0.0, -400.0) } else { glam::Vec3::new(0.0, 0.0, -950.0) },
                }).unwrap_or_else(|| if is_water_zone { glam::Vec3::new(0.0, 0.0, -400.0) } else { glam::Vec3::new(0.0, 0.0, -950.0) });

                let velocity = a.properties.get("ZoneVelocity").map(|p| match p.value {
                    ut99_format::TaggedValue::Vector(v) => v,
                    _ => glam::Vec3::ZERO,
                }).unwrap_or(glam::Vec3::ZERO);

                let fluid_friction = a.properties.get("ZoneFluidFriction").map(|p| match p.value {
                    ut99_format::TaggedValue::Float(f) => f,
                    _ => if is_water_zone { 1.2 } else { 0.0 },
                }).unwrap_or_else(|| if is_water_zone { 1.2 } else { 0.0 });

                let damage_per_sec = a.properties.get("DamagePerSec").map(|p| match p.value {
                    ut99_format::TaggedValue::Int(i) => i,
                    ut99_format::TaggedValue::Byte(b) => b as i32,
                    _ => 0,
                }).unwrap_or(0);

                let damage_type = a.properties.get("DamageType").map(|p| match &p.value {
                    ut99_format::TaggedValue::Name(s) | ut99_format::TaggedValue::Str(s) => s.clone(),
                    _ => String::new(),
                }).unwrap_or_default();

                let zone_name = a.properties.get("ZoneName").map(|p| match &p.value {
                    ut99_format::TaggedValue::Str(s) | ut99_format::TaggedValue::Name(s) => s.clone(),
                    _ => a.tag.clone(),
                }).unwrap_or_else(|| if !a.tag.is_empty() { a.tag.clone() } else { a.name.clone() });

                profiles.push(ZoneProfile {
                    actor_tag: if !a.tag.is_empty() { a.tag.clone() } else { a.name.clone() },
                    class_name: a.class_name.clone(),
                    location: a.location,
                    zone_name,
                    is_sky_zone,
                    is_water_zone,
                    gravity,
                    velocity,
                    fluid_friction,
                    damage_per_sec,
                    damage_type,
                    ambient_brightness: a.light_brightness,
                    ambient_hue: a.light_hue,
                    ambient_saturation: a.light_saturation,
                });
            }
        };

        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                check_actor(a, &mut profiles);
            }
        }

        profiles
    }

    /// Apply an environmental preset to a specific zone actor
    pub fn apply_zone_physics_preset(&mut self, actor_tag: &str, preset: ZonePhysicsPreset) -> Result<(), String> {
        let (gravity, friction, is_water, dmg, dmg_type) = match preset {
            ZonePhysicsPreset::StandardEarth => (glam::Vec3::new(0.0, 0.0, -950.0), 0.0, false, 0, ""),
            ZonePhysicsPreset::WaterSwimming => (glam::Vec3::new(0.0, 0.0, -400.0), 1.2, true, 0, ""),
            ZonePhysicsPreset::MuddySwamp => (glam::Vec3::new(0.0, 0.0, -500.0), 2.5, true, 0, ""),
            ZonePhysicsPreset::ToxicSlime => (glam::Vec3::new(0.0, 0.0, -400.0), 1.5, true, 10, "Corroded"),
            ZonePhysicsPreset::VolcanicLava => (glam::Vec3::new(0.0, 0.0, -300.0), 2.0, true, 40, "Burned"),
            ZonePhysicsPreset::LowGravity => (glam::Vec3::new(0.0, 0.0, -200.0), 0.0, false, 0, ""),
            ZonePhysicsPreset::SpaceVacuum => (glam::Vec3::ZERO, 0.0, false, 5, "Drowned"),
        };

        self.update_zone_properties(actor_tag, gravity, friction, is_water, dmg, dmg_type)
    }

    /// Update physics and environmental properties on a zone actor with Undo/Redo
    pub fn update_zone_properties(
        &mut self,
        actor_tag: &str,
        gravity: glam::Vec3,
        friction: f32,
        is_water: bool,
        damage: i32,
        damage_type: &str,
    ) -> Result<(), String> {
        let unr = self.loaded_unr.as_mut().ok_or_else(|| "No level loaded".to_string())?;

        let actor_idx = unr.actors.iter().position(|a| a.tag == actor_tag || a.name == actor_tag)
            .ok_or_else(|| format!("Zone actor '{}' not found", actor_tag))?;

        let old_actor = unr.actors[actor_idx].clone();
        let mut new_actor = old_actor.clone();

        new_actor.properties.insert(
            "ZoneGravity".into(),
            ut99_format::TaggedProperty {
                name: "ZoneGravity".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Vector(gravity),
            },
        );
        new_actor.properties.insert(
            "ZoneFluidFriction".into(),
            ut99_format::TaggedProperty {
                name: "ZoneFluidFriction".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Float(friction),
            },
        );
        new_actor.properties.insert(
            "bWaterZone".into(),
            ut99_format::TaggedProperty {
                name: "bWaterZone".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Bool(is_water),
            },
        );
        new_actor.properties.insert(
            "DamagePerSec".into(),
            ut99_format::TaggedProperty {
                name: "DamagePerSec".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Int(damage),
            },
        );
        if !damage_type.is_empty() {
            new_actor.properties.insert(
                "DamageType".into(),
                ut99_format::TaggedProperty {
                    name: "DamageType".into(),
                    array_index: 0,
                    value: ut99_format::TaggedValue::Name(damage_type.to_string()),
                },
            );
        }

        unr.actors[actor_idx] = new_actor.clone();

        self.history.push(crate::history::Action::UpdateZoneProperties {
            actor_tag: actor_tag.to_string(),
            old_actor,
            new_actor,
            description: format!("Update properties on zone '{}'", actor_tag),
        });

        self.map_is_dirty = true;
        self.log(format!("🌍 Updated zone '{}': Gravity=({:.0},{:.0},{:.0}), Friction={:.2}, Water={}", actor_tag, gravity.x, gravity.y, gravity.z, friction, is_water));
        Ok(())
    }

    /// Focus camera on active SkyZoneInfo anchor
    pub fn focus_camera_on_sky_zone(&mut self) {
        if let Some((_, pos, tag)) = self.find_active_sky_zone() {
            self.camera.position = pos - self.camera.forward() * 300.0;
            self.log(format!("Camera focused on SkyZone '{}' at ({:.0}, {:.0}, {:.0})", tag, pos.x, pos.y, pos.z));
        } else {
            self.log("No SkyZoneInfo actor found in level.");
        }
    }

    /// Focus camera on level bounding center
    pub fn focus_camera_on_level_center(&mut self) {
        if !self.world_brushes.is_empty() {
            let mut min = glam::Vec3::splat(f32::MAX);
            let mut max = glam::Vec3::splat(f32::MIN);
            for b in &self.world_brushes {
                for p in &b.polys {
                    for v in &p.vertices {
                        let wv = *v + b.location;
                        min = min.min(wv);
                        max = max.max(wv);
                    }
                }
            }
            let center = (min + max) * 0.5;
            self.camera.position = center - self.camera.forward() * 800.0;
            self.log(format!("Camera focused on level center at ({:.0}, {:.0}, {:.0})", center.x, center.y, center.z));
        }
    }


    pub fn get_zone_audio_reports(&self) -> Vec<ZoneAudioReport> {
        let mut reports = Vec::new();

        if let Some(ref level) = self.loaded_unr {
            for (idx, actor) in level.actors.iter().enumerate() {
                let is_zone = actor.class_name.contains("ZoneInfo")
                    || actor.class_name.contains("WaterZone")
                    || actor.class_name.contains("SkyZoneInfo");
                let is_ambient = actor.class_name.contains("AmbientSound")
                    || actor.properties.contains_key("AmbientSound");

                if is_zone {
                    let mut preset = if actor.class_name.contains("WaterZone") {
                        EaxEnvironment::Underwater
                    } else {
                        EaxEnvironment::Generic
                    };

                    if let Some(prop) = actor.properties.get("ZoneEffect") {
                        if let TaggedValue::Str(s) = &prop.value {
                            if let Some(p) = parse_eax_preset(s) {
                                preset = p;
                            }
                        }
                    } else if let Some(prop) = actor.properties.get("ZoneReverb") {
                        if let TaggedValue::Str(s) = &prop.value {
                            if let Some(p) = parse_eax_preset(s) {
                                preset = p;
                            }
                        }
                    }

                    let acoustics = get_eax_preset(preset);

                    let sound_radius = if let Some(prop) = actor.properties.get("SoundRadius") {
                        match &prop.value {
                            TaggedValue::Byte(b) => (*b as f32) * 40.0,
                            TaggedValue::Int(i) => (*i as f32) * 40.0,
                            TaggedValue::Float(f) => *f * 40.0,
                            _ => acoustics.environment_size,
                        }
                    } else {
                        acoustics.environment_size
                    };

                    let sound_name = actor.properties.get("AmbientSound").and_then(|p| {
                        if let TaggedValue::Str(s) = &p.value {
                            Some(s.clone())
                        } else {
                            None
                        }
                    });

                    reports.push(ZoneAudioReport {
                        zone_name: if actor.name.is_empty() { format!("Zone_{}", idx) } else { actor.name.clone() },
                        actor_index: idx,
                        location: actor.location,
                        is_reverb: true,
                        preset,
                        reverb_time: acoustics.reverb_decay_time,
                        sound_radius,
                        sound_name,
                    });
                } else if is_ambient {
                    let sound_radius = if let Some(prop) = actor.properties.get("SoundRadius") {
                        match &prop.value {
                            TaggedValue::Byte(b) => (*b as f32) * 40.0,
                            TaggedValue::Int(i) => (*i as f32) * 40.0,
                            TaggedValue::Float(f) => *f * 40.0,
                            _ => 320.0,
                        }
                    } else {
                        320.0
                    };

                    let sound_name = actor.properties.get("AmbientSound").and_then(|p| {
                        if let TaggedValue::Str(s) = &p.value {
                            Some(s.clone())
                        } else {
                            None
                        }
                    });

                    reports.push(ZoneAudioReport {
                        zone_name: if actor.name.is_empty() { format!("AmbientSound_{}", idx) } else { actor.name.clone() },
                        actor_index: idx,
                        location: actor.location,
                        is_reverb: false,
                        preset: EaxEnvironment::Generic,
                        reverb_time: 0.0,
                        sound_radius,
                        sound_name,
                    });
                }
            }
        }

        if reports.is_empty() {
            let acoustics = get_eax_preset(EaxEnvironment::Generic);
            reports.push(ZoneAudioReport {
                zone_name: "WorldAcoustics".to_string(),
                actor_index: 0,
                location: Vec3::ZERO,
                is_reverb: true,
                preset: EaxEnvironment::Generic,
                reverb_time: acoustics.reverb_decay_time,
                sound_radius: acoustics.environment_size,
                sound_name: None,
            });
        }

        reports
    }

    pub fn apply_eax_preset_to_selected_zone(&mut self, preset: EaxEnvironment) -> bool {
        let mut applied_actor_name = None;
        if let Some(sel_idx) = self.selected_actor_index {
            if let Some(ref mut level) = self.loaded_unr {
                if let Some(actor) = level.actors.get_mut(sel_idx) {
                    actor.properties.insert(
                        "ZoneEffect".to_string(),
                        TaggedProperty {
                            name: "ZoneEffect".to_string(),
                            array_index: 0,
                            value: TaggedValue::Str(preset.name().to_string()),
                        },
                    );
                    actor.properties.insert(
                        "bReverbZone".to_string(),
                        TaggedProperty {
                            name: "bReverbZone".to_string(),
                            array_index: 0,
                            value: TaggedValue::Bool(true),
                        },
                    );
                    applied_actor_name = Some(actor.name.clone());
                }
            }
        }

        if let Some(name) = applied_actor_name {
            self.map_is_dirty = true;
            self.log(format!("Applied EAX Reverb preset '{}' to actor '{}'", preset.name(), name));
            true
        } else {
            false
        }
    }

    // ==========================================
    // Distance Fog Simulation
    // ==========================================
    pub fn sync_fog_from_selected_zone(&mut self) -> bool {
        let mut target_actor = None;
        if let Some(sel_idx) = self.selected_actor_index {
            if let Some(ref level) = self.loaded_unr {
                if let Some(actor) = level.actors.get(sel_idx) {
                    if actor.class_name.contains("ZoneInfo") {
                        target_actor = Some(actor.clone());
                    }
                }
            }
        }

        if target_actor.is_none() {
            if let Some(ref level) = self.loaded_unr {
                for actor in &level.actors {
                    if actor.class_name.contains("ZoneInfo") {
                        target_actor = Some(actor.clone());
                        break;
                    }
                }
            }
        }

        if let Some(actor) = target_actor {
            let mut fog_distance = 2500.0;
            if let Some(prop) = actor.properties.get("FogDistance") {
                match &prop.value {
                    TaggedValue::Float(f) => fog_distance = *f,
                    TaggedValue::Int(i) => fog_distance = *i as f32,
                    _ => {}
                }
            }

            self.fog_end = fog_distance.max(100.0);
            self.fog_start = (fog_distance * 0.15).min(self.fog_end - 50.0).max(10.0);
            self.fog_density = 1.0;
            self.distance_fog_enabled = true;
            self.log(format!("Synchronized distance fog from ZoneInfo '{}' (End: {:.0} uu)", actor.name, self.fog_end));
            return true;
        }

        false
    }

    // ==========================================
    // Sheet & Curtain Builder
    // ==========================================

    pub fn get_warpzone_pairs(&self) -> Vec<WarpZonePair> {
        let mut pairs = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            let mut warpzones = Vec::new();
            for (idx, actor) in level.actors.iter().enumerate() {
                if actor.class_name.contains("WarpZoneInfo") {
                    let mut other_side = String::new();
                    let mut zone_tag = String::new();

                    if let Some(prop) = actor.properties.get("OtherSide") {
                        if let TaggedValue::Str(s) = &prop.value {
                            other_side = s.clone();
                        }
                    }
                    if let Some(prop) = actor.properties.get("ZoneTag") {
                        if let TaggedValue::Str(s) = &prop.value {
                            zone_tag = s.clone();
                        }
                    } else if let Some(prop) = actor.properties.get("Tag") {
                        if let TaggedValue::Str(s) = &prop.value {
                            zone_tag = s.clone();
                        }
                    }

                    warpzones.push((idx, actor.name.clone(), zone_tag, other_side, actor.location));
                }
            }

            for &(idx_src, ref name_src, ref tag_src, ref other_side, loc_src) in &warpzones {
                let mut matched_target = None;
                if !other_side.is_empty() {
                    for &(idx_tgt, ref name_tgt, ref tag_tgt, _, loc_tgt) in &warpzones {
                        if idx_src != idx_tgt && (tag_tgt.eq_ignore_ascii_case(other_side) || name_tgt.eq_ignore_ascii_case(other_side)) {
                            matched_target = Some((name_tgt.clone(), tag_tgt.clone(), loc_tgt));
                            break;
                        }
                    }
                }

                if let Some((name_tgt, tag_tgt, loc_tgt)) = matched_target {
                    pairs.push(WarpZonePair {
                        source_name: name_src.clone(),
                        source_tag: tag_src.clone(),
                        source_location: loc_src,
                        target_name: name_tgt,
                        target_tag: tag_tgt,
                        target_location: loc_tgt,
                        distance: loc_src.distance(loc_tgt),
                        is_paired: true,
                    });
                } else {
                    pairs.push(WarpZonePair {
                        source_name: name_src.clone(),
                        source_tag: tag_src.clone(),
                        source_location: loc_src,
                        target_name: if other_side.is_empty() { "UNCONFIGURED_DESTINATION".to_string() } else { other_side.clone() },
                        target_tag: String::new(),
                        target_location: loc_src + Vec3::new(0.0, 0.0, 200.0),
                        distance: 0.0,
                        is_paired: false,
                    });
                }
            }
        }
        pairs
    }

    // ==========================================
    // WarpZone & Non-Euclidean Spatial Portal Visualizer & Topology Inspector
    // ==========================================

    /// Collect comprehensive non-euclidean topology, pairing status and spatial metrics
    pub fn collect_warpzone_topology(&self) -> Vec<WarpZoneTopologyEntry> {
        let mut entries = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            // 1. Gather all WarpZoneInfo actors
            struct RawWarp {
                actor_idx: usize,
                name: String,
                tag: String,
                this_tag: String,
                other_side: String,
                location: Vec3,
                rotation: (i32, i32, i32),
                yaw_deg: f32,
                pitch_deg: f32,
                roll_deg: f32,
                forward: Vec3,
            }

            let mut raws = Vec::new();
            for (idx, actor) in level.actors.iter().enumerate() {
                if actor.class_name.contains("WarpZoneInfo") {
                    let mut other_side = String::new();
                    let mut this_tag = String::new();

                    if let Some(prop) = actor.properties.get("OtherSideURL") {
                        if let TaggedValue::Str(s) = &prop.value {
                            other_side = s.clone();
                        }
                    } else if let Some(prop) = actor.properties.get("OtherSide") {
                        if let TaggedValue::Str(s) = &prop.value {
                            other_side = s.clone();
                        }
                    }

                    if let Some(prop) = actor.properties.get("ThisTag") {
                        if let TaggedValue::Str(s) = &prop.value {
                            this_tag = s.clone();
                        }
                    } else if let Some(prop) = actor.properties.get("ZoneTag") {
                        if let TaggedValue::Str(s) = &prop.value {
                            this_tag = s.clone();
                        }
                    }
                    if this_tag.is_empty() {
                        this_tag = actor.tag.clone();
                    }
                    if this_tag.is_empty() {
                        this_tag = actor.name.clone();
                    }

                    let p_deg = ((actor.rotation.0 as f32 / 65536.0) * 360.0).rem_euclid(360.0);
                    let y_deg = ((actor.rotation.1 as f32 / 65536.0) * 360.0).rem_euclid(360.0);
                    let r_deg = ((actor.rotation.2 as f32 / 65536.0) * 360.0).rem_euclid(360.0);

                    // Forward vector in UE1 coordinates (Yaw around Z, Pitch up/down)
                    let y_rad = y_deg.to_radians();
                    let p_rad = p_deg.to_radians();
                    let cos_p = p_rad.cos();
                    let forward = Vec3::new(y_rad.cos() * cos_p, y_rad.sin() * cos_p, p_rad.sin()).normalize_or_zero();

                    raws.push(RawWarp {
                        actor_idx: idx,
                        name: actor.name.clone(),
                        tag: actor.tag.clone(),
                        this_tag,
                        other_side,
                        location: actor.location,
                        rotation: actor.rotation,
                        yaw_deg: y_deg,
                        pitch_deg: p_deg,
                        roll_deg: r_deg,
                        forward,
                    });
                }
            }

            // 2. Resolve topological connectivity & metrics
            for (i, src) in raws.iter().enumerate() {
                let mut target_idx_opt = None;
                if !src.other_side.is_empty() {
                    for (j, candidate) in raws.iter().enumerate() {
                        if i != j && (candidate.this_tag.eq_ignore_ascii_case(&src.other_side)
                            || candidate.tag.eq_ignore_ascii_case(&src.other_side)
                            || candidate.name.eq_ignore_ascii_case(&src.other_side))
                        {
                            target_idx_opt = Some(j);
                            break;
                        }
                    }
                }

                // Check adjacent PF_PORTAL geometry
                let mut min_portal_dist = f32::MAX;
                for brush in &self.world_brushes {
                    for poly in &brush.polys {
                        if (poly.flags & ut99_bsp::PF_PORTAL) != 0 && !poly.vertices.is_empty() {
                            let center: Vec3 = poly.vertices.iter().sum::<Vec3>() / poly.vertices.len() as f32;
                            let d = src.location.distance(center);
                            if d < min_portal_dist {
                                min_portal_dist = d;
                            }
                        }
                    }
                }
                let has_adjacent_portal = min_portal_dist <= 384.0;

                if let Some(t_idx) = target_idx_opt {
                    let tgt = &raws[t_idx];

                    // Check reciprocity: does tgt point back to src?
                    let points_back_to_src = !tgt.other_side.is_empty()
                        && (tgt.other_side.eq_ignore_ascii_case(&src.this_tag)
                            || tgt.other_side.eq_ignore_ascii_case(&src.tag)
                            || tgt.other_side.eq_ignore_ascii_case(&src.name));

                    let status = if points_back_to_src {
                        WarpConnectionStatus::MatchedBiDirectional
                    } else {
                        // Check if it forms a closed loop chain (e.g. A -> B -> C -> A)
                        let mut curr = t_idx;
                        let mut is_loop = false;
                        for _ in 0..16 {
                            let next_candidate = raws[curr].other_side.as_str();
                            if next_candidate.is_empty() { break; }
                            if let Some(next_idx) = raws.iter().position(|r| {
                                r.this_tag.eq_ignore_ascii_case(next_candidate)
                                    || r.tag.eq_ignore_ascii_case(next_candidate)
                                    || r.name.eq_ignore_ascii_case(next_candidate)
                            }) {
                                if next_idx == i {
                                    is_loop = true;
                                    break;
                                }
                                curr = next_idx;
                            } else {
                                break;
                            }
                        }
                        if is_loop {
                            WarpConnectionStatus::LoopChain
                        } else {
                            WarpConnectionStatus::Unidirectional
                        }
                    };

                    let delta_trans = tgt.location - src.location;
                    let dist = delta_trans.length();
                    let d_yaw = (tgt.yaw_deg - src.yaw_deg).rem_euclid(360.0);
                    let d_pitch = (tgt.pitch_deg - src.pitch_deg).rem_euclid(360.0);
                    let d_roll = (tgt.roll_deg - src.roll_deg).rem_euclid(360.0);
                    let is_seamless = (d_yaw - 180.0).abs() <= 2.5 || (d_yaw - 180.0).abs() >= 357.5;

                    entries.push(WarpZoneTopologyEntry {
                        actor_index: src.actor_idx,
                        name: src.name.clone(),
                        tag: src.tag.clone(),
                        this_tag: src.this_tag.clone(),
                        other_side_url: src.other_side.clone(),
                        location: src.location,
                        rotation: src.rotation,
                        yaw_deg: src.yaw_deg,
                        pitch_deg: src.pitch_deg,
                        roll_deg: src.roll_deg,
                        forward_vector: src.forward,

                        status,
                        target_index: Some(tgt.actor_idx),
                        target_name: Some(tgt.name.clone()),
                        target_tag: Some(tgt.this_tag.clone()),
                        target_location: Some(tgt.location),
                        target_rotation: Some(tgt.rotation),
                        target_yaw_deg: Some(tgt.yaw_deg),
                        target_forward_vector: Some(tgt.forward),

                        delta_translation: Some(delta_trans),
                        distance_uu: Some(dist),
                        delta_yaw_deg: Some(d_yaw),
                        delta_pitch_deg: Some(d_pitch),
                        delta_roll_deg: Some(d_roll),
                        is_seamless_mirror: is_seamless,

                        has_adjacent_portal_surface: has_adjacent_portal,
                        nearest_portal_surface_distance: if min_portal_dist < f32::MAX { min_portal_dist } else { 0.0 },
                    });
                } else {
                    entries.push(WarpZoneTopologyEntry {
                        actor_index: src.actor_idx,
                        name: src.name.clone(),
                        tag: src.tag.clone(),
                        this_tag: src.this_tag.clone(),
                        other_side_url: src.other_side.clone(),
                        location: src.location,
                        rotation: src.rotation,
                        yaw_deg: src.yaw_deg,
                        pitch_deg: src.pitch_deg,
                        roll_deg: src.roll_deg,
                        forward_vector: src.forward,

                        status: WarpConnectionStatus::OrphanBroken,
                        target_index: None,
                        target_name: None,
                        target_tag: None,
                        target_location: None,
                        target_rotation: None,
                        target_yaw_deg: None,
                        target_forward_vector: None,

                        delta_translation: None,
                        distance_uu: None,
                        delta_yaw_deg: None,
                        delta_pitch_deg: None,
                        delta_roll_deg: None,
                        is_seamless_mirror: false,

                        has_adjacent_portal_surface: has_adjacent_portal,
                        nearest_portal_surface_distance: if min_portal_dist < f32::MAX { min_portal_dist } else { 0.0 },
                    });
                }
            }
        }
        entries
    }

    /// Create a synchronized, opposing pair of WarpZoneInfo actors with bidirectional links and 180° opposing yaw
    pub fn create_coordinated_warp_pair(&mut self, base_pos: Vec3, separation: Vec3) -> (usize, usize) {
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let pair_id = unr.actors.len();
        let tag_a = format!("Warp_{}A", pair_id);
        let tag_b = format!("Warp_{}B", pair_id);

        let mut actor_a = ut99_format::ActorRecord {
            export_index: unr.actors.len(),
            name: format!("WarpZoneInfo_{}A", pair_id),
            class_name: "Engine.WarpZoneInfo".to_string(),
            flags: 0,
            location: base_pos,
            rotation: (0, 0, 0), // Yaw 0°
            prepivot: Vec3::ZERO,
            main_scale: Vec3::ONE,
            post_scale: Vec3::ONE,
            light_brightness: 0,
            light_radius: 0,
            light_hue: 0,
            light_saturation: 0,
            light_type: 0,
            light_effect: 0,
            tag: tag_a.clone(),
            event: String::new(),
            group: "WarpZones".to_string(),
            brush_ref: 0,
            csg_oper: 0,
            poly_flags: 0,
            object_paths: std::collections::HashMap::new(),
            raw_property_meta: std::collections::HashMap::new(),
            properties: std::collections::HashMap::new(),
        };
        actor_a.properties.insert("ThisTag".to_string(), ut99_format::TaggedProperty {
            name: "ThisTag".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tag_a.clone()),
        });
        actor_a.properties.insert("OtherSideURL".to_string(), ut99_format::TaggedProperty {
            name: "OtherSideURL".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tag_b.clone()),
        });

        let mut actor_b = ut99_format::ActorRecord {
            export_index: unr.actors.len() + 1,
            name: format!("WarpZoneInfo_{}B", pair_id),
            class_name: "Engine.WarpZoneInfo".to_string(),
            flags: 0,
            location: base_pos + separation,
            rotation: (0, 32768, 0), // Yaw 180° for opposing seamless mirror
            prepivot: Vec3::ZERO,
            main_scale: Vec3::ONE,
            post_scale: Vec3::ONE,
            light_brightness: 0,
            light_radius: 0,
            light_hue: 0,
            light_saturation: 0,
            light_type: 0,
            light_effect: 0,
            tag: tag_b.clone(),
            event: String::new(),
            group: "WarpZones".to_string(),
            brush_ref: 0,
            csg_oper: 0,
            poly_flags: 0,
            object_paths: std::collections::HashMap::new(),
            raw_property_meta: std::collections::HashMap::new(),
            properties: std::collections::HashMap::new(),
        };
        actor_b.properties.insert("ThisTag".to_string(), ut99_format::TaggedProperty {
            name: "ThisTag".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tag_b.clone()),
        });
        actor_b.properties.insert("OtherSideURL".to_string(), ut99_format::TaggedProperty {
            name: "OtherSideURL".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tag_a.clone()),
        });

        let idx_a = unr.actors.len();
        unr.actors.push(actor_a.clone());
        self.history.push(crate::history::Action::AddActor { actor: actor_a.clone() });

        let idx_b = unr.actors.len();
        unr.actors.push(actor_b.clone());
        self.history.push(crate::history::Action::AddActor { actor: actor_b.clone() });

        self.warp_zone_settings.selected_warp_index = Some(idx_a);
        self.map_is_dirty = true;
        self.log(format!("🌀 Created coordinated WarpZone pair: '{}' <-> '{}' (Distance: {:.0} uu)", actor_a.name, actor_b.name, separation.length()));
        (idx_a, idx_b)
    }

    /// Link two WarpZones bidirectionally with atomic Undo/Redo
    pub fn auto_pair_warp_zones(&mut self, src_idx: usize, tgt_idx: usize) -> Result<(), String> {
        let unr = self.loaded_unr.as_mut().ok_or("No level loaded")?;
        if src_idx >= unr.actors.len() || tgt_idx >= unr.actors.len() || src_idx == tgt_idx {
            return Err("Invalid WarpZone actor indices".to_string());
        }

        let old_src = unr.actors[src_idx].clone();
        let old_tgt = unr.actors[tgt_idx].clone();

        let src_tag = if !old_src.tag.is_empty() { old_src.tag.clone() } else { old_src.name.clone() };
        let tgt_tag = if !old_tgt.tag.is_empty() { old_tgt.tag.clone() } else { old_tgt.name.clone() };

        let mut new_src = old_src.clone();
        new_src.properties.insert("ThisTag".to_string(), ut99_format::TaggedProperty {
            name: "ThisTag".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(src_tag.clone()),
        });
        new_src.properties.insert("OtherSideURL".to_string(), ut99_format::TaggedProperty {
            name: "OtherSideURL".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tgt_tag.clone()),
        });

        let mut new_tgt = old_tgt.clone();
        new_tgt.properties.insert("ThisTag".to_string(), ut99_format::TaggedProperty {
            name: "ThisTag".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(tgt_tag.clone()),
        });
        new_tgt.properties.insert("OtherSideURL".to_string(), ut99_format::TaggedProperty {
            name: "OtherSideURL".to_string(),
            array_index: 0,
            value: ut99_format::TaggedValue::Str(src_tag.clone()),
        });

        unr.actors[src_idx] = new_src.clone();
        unr.actors[tgt_idx] = new_tgt.clone();

        self.history.push(crate::history::Action::UpdateWarpZonePair {
            source_tag: src_tag.clone(),
            target_tag: tgt_tag.clone(),
            old_source: old_src,
            new_source: new_src,
            old_target: Some(old_tgt),
            new_target: Some(new_tgt),
            description: format!("Pair WarpZones '{}' <-> '{}'", src_tag, tgt_tag),
        });

        self.map_is_dirty = true;
        self.log(format!("🌀 Linked WarpZones: '{}' <-> '{}'", src_tag, tgt_tag));
        Ok(())
    }

    /// Invert portal exit orientation (+180° / 32768 rotator units) with Undo/Redo
    pub fn invert_warp_orientation(&mut self, actor_idx: usize) -> Result<(), String> {
        let unr = self.loaded_unr.as_mut().ok_or("No level loaded")?;
        if actor_idx >= unr.actors.len() {
            return Err("Invalid actor index".to_string());
        }

        let old_actor = unr.actors[actor_idx].clone();
        let mut new_actor = old_actor.clone();
        new_actor.rotation.1 = (new_actor.rotation.1 + 32768).rem_euclid(65536);

        let tag = if !old_actor.tag.is_empty() { old_actor.tag.clone() } else { old_actor.name.clone() };

        unr.actors[actor_idx] = new_actor.clone();

        self.history.push(crate::history::Action::UpdateWarpZonePair {
            source_tag: tag.clone(),
            target_tag: String::new(),
            old_source: old_actor,
            new_source: new_actor,
            old_target: None,
            new_target: None,
            description: format!("Invert Yaw on WarpZone '{}'", tag),
        });

        self.map_is_dirty = true;
        self.log(format!("🔄 Inverted orientation (+180° Yaw) on WarpZone '{}'", tag));
        Ok(())
    }

    /// Align WarpZone orientation with the normal of the nearest PF_PORTAL surface
    pub fn align_warp_to_nearest_portal_surface(&mut self, actor_idx: usize) -> Result<(), String> {
        let unr = self.loaded_unr.as_mut().ok_or("No level loaded")?;
        if actor_idx >= unr.actors.len() {
            return Err("Invalid actor index".to_string());
        }

        let actor_pos = unr.actors[actor_idx].location;
        let mut nearest_normal: Option<Vec3> = None;
        let mut min_dist = f32::MAX;

        for brush in &self.world_brushes {
            for poly in &brush.polys {
                if (poly.flags & ut99_bsp::PF_PORTAL) != 0 && poly.vertices.len() >= 3 {
                    let center: Vec3 = poly.vertices.iter().sum::<Vec3>() / poly.vertices.len() as f32;
                    let d = actor_pos.distance(center);
                    if d < min_dist && d <= 512.0 {
                        min_dist = d;
                        let v0 = poly.vertices[0];
                        let v1 = poly.vertices[1];
                        let v2 = poly.vertices[2];
                        let norm = (v1 - v0).cross(v2 - v0).normalize_or_zero();
                        if norm.length_squared() > 0.5 {
                            nearest_normal = Some(norm);
                        }
                    }
                }
            }
        }

        let norm = nearest_normal.ok_or("No adjacent PF_PORTAL surface found within 512 uu")?;
        let yaw_rad = norm.y.atan2(norm.x);
        let yaw_rot = ((yaw_rad / std::f32::consts::TAU) * 65536.0) as i32;

        let old_actor = unr.actors[actor_idx].clone();
        let mut new_actor = old_actor.clone();
        new_actor.rotation.1 = yaw_rot.rem_euclid(65536);

        let tag = if !old_actor.tag.is_empty() { old_actor.tag.clone() } else { old_actor.name.clone() };
        unr.actors[actor_idx] = new_actor.clone();

        self.history.push(crate::history::Action::UpdateWarpZonePair {
            source_tag: tag.clone(),
            target_tag: String::new(),
            old_source: old_actor,
            new_source: new_actor,
            old_target: None,
            new_target: None,
            description: format!("Align WarpZone '{}' to Portal Surface", tag),
        });

        self.map_is_dirty = true;
        self.log(format!("📐 Aligned WarpZone '{}' to portal surface normal ({:.2}, {:.2}, {:.2})", tag, norm.x, norm.y, norm.z));
        Ok(())
    }

    /// Seamlessly teleport editor viewport camera through the portal to verify continuous flow
    pub fn teleport_camera_through_warp_zone(&mut self, source_actor_idx: usize) -> Result<String, String> {
        let topology = self.collect_warpzone_topology();
        let entry = topology.iter().find(|e| e.actor_index == source_actor_idx)
            .ok_or_else(|| format!("WarpZone at index {} not found in topology", source_actor_idx))?;

        let target_pos = entry.target_location
            .ok_or_else(|| format!("WarpZone '{}' is unlinked or has no valid target", entry.name))?;
        let target_yaw_deg = entry.target_yaw_deg.unwrap_or(0.0);
        let target_fwd = entry.target_forward_vector.unwrap_or(Vec3::X);

        // Vector from portal anchor to camera
        let v_cam = self.camera.position - entry.location;
        let delta_yaw_rad = (target_yaw_deg - entry.yaw_deg + 180.0).to_radians();

        let new_pos = if v_cam.length() < 48.0 {
            // Camera was right on top of portal, place slightly in front of destination
            target_pos + target_fwd * 160.0
        } else {
            let cos_y = delta_yaw_rad.cos();
            let sin_y = delta_yaw_rad.sin();
            let rot_cam = Vec3::new(
                v_cam.x * cos_y - v_cam.y * sin_y,
                v_cam.x * sin_y + v_cam.y * cos_y,
                v_cam.z,
            );
            target_pos + rot_cam
        };

        self.camera.position = new_pos;
        self.camera.yaw = (self.camera.yaw + delta_yaw_rad).rem_euclid(std::f32::consts::TAU);

        let target_name = entry.target_name.as_deref().unwrap_or("Target");
        let msg = format!("👁 Teleported camera through WarpZone '{}' -> '{}' (Position: {:.0}, {:.0}, {:.0})", entry.name, target_name, new_pos.x, new_pos.y, new_pos.z);
        self.log(msg.clone());
        Ok(msg)
    }

    /// Focus camera directly on a WarpZone portal anchor
    pub fn focus_camera_on_warp_zone(&mut self, actor_idx: usize) {
        if let Some(ref level) = self.loaded_unr {
            if let Some(actor) = level.actors.get(actor_idx) {
                self.camera.position = actor.location - self.camera.forward() * 300.0;
                self.log(format!("Camera focused on WarpZone '{}' at ({:.0}, {:.0}, {:.0})", actor.name, actor.location.x, actor.location.y, actor.location.z));
            }
        }
    }

    // ==========================================
    // Anti-Portal Occluder Analyzer
    // ==========================================
    pub fn analyze_anti_portals(&self) -> AntiPortalReport {
        let mut report = AntiPortalReport::default();
        let mut total_polys = 0;
        let mut anti_polys = 0;

        for (idx, brush) in self.world_brushes.iter().enumerate() {
            let mut is_ap_brush = false;
            for poly in &brush.polys {
                total_polys += 1;
                if (poly.flags & ut99_bsp::PF_ANTI_PORTAL) != 0 {
                    anti_polys += 1;
                    is_ap_brush = true;
                }
            }
            if is_ap_brush {
                report.anti_portal_brushes.push(idx);
            }
        }

        report.total_world_polys = total_polys;
        report.total_anti_portal_polys = anti_polys;
        report.total_anti_portals = report.anti_portal_brushes.len();

        if total_polys > 0 && anti_polys > 0 {
            let ratio = (anti_polys as f32 / total_polys as f32) * 15.0;
            report.estimated_culling_efficiency = (ratio * 100.0).clamp(15.0, 75.0);
        }

        report
    }

    pub fn make_selected_brush_anti_portal(&mut self) -> bool {
        if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                for poly in &mut brush.polys {
                    poly.flags |= ut99_bsp::PF_ANTI_PORTAL | ut99_bsp::PF_NOT_SOLID;
                }
                let new_polys = brush.polys.clone();
                self.history.push(Action::ModifyBrushGeometry {
                    brush_index: b_idx,
                    old_polys,
                    new_polys,
                    description: "Flag as Anti-Portal".to_string(),
                });
                self.map_is_dirty = true;
                self.log(format!("Flagged brush {} as Anti-Portal (PF_ANTI_PORTAL | PF_NOT_SOLID)", b_idx));
                return true;
            }
        }
        false
    }

    // ==========================================
    // Tournament Item Balance Simulator
    // ==========================================
    pub fn analyze_map_gameplay_balance(&self) -> ItemBalanceReport {
        let mut actor_tuples: Vec<(String, String, Vec3)> = Vec::new();
        if let Some(ref level) = self.loaded_unr {
            for actor in &level.actors {
                actor_tuples.push((actor.name.clone(), actor.class_name.clone(), actor.location));
            }
        }
        analyze_item_balance(&actor_tuples, &self.nav_graph)
    }

    // ==========================================
    // Zone Boundary Leak Detector
    // ==========================================
    pub fn analyze_zone_seals(&self) -> Vec<ZoneSealReport> {
        let mut reports = Vec::new();
        let config = ZoneLeakConfig::default();
        let brush_bounds: Vec<(Vec3, Vec3)> = self.world_brushes.iter().map(|b| b.world_bounds()).collect();

        let mut portal_polys = Vec::new();
        for brush in &self.world_brushes {
            for poly in &brush.polys {
                if (poly.flags & ut99_bsp::PF_PORTAL) != 0 {
                    portal_polys.push(poly.clone());
                }
            }
        }

        if let Some(ref level) = self.loaded_unr {
            for actor in &level.actors {
                if actor.class_name.contains("Zone") {
                    let report = test_zone_watertightness_with_bounds(
                        &actor.name,
                        &actor.class_name,
                        actor.location,
                        &brush_bounds,
                        &portal_polys,
                        &config,
                    );
                    reports.push(report);
                }
            }
        }

        if reports.is_empty() {
            let report = test_zone_watertightness_with_bounds(
                "WorldZone",
                "ZoneInfo",
                Vec3::ZERO,
                &brush_bounds,
                &portal_polys,
                &config,
            );
            reports.push(report);
        }

        reports
    }

    // ==========================================
    // Mover 8-Keyframe Trajectories
    // ==========================================

    pub fn compute_3d_zone_flood_fill_report(&self) -> ut99_bsp::ZoneFloodFillResult {
        let mut zone_actors = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                if a.class_name.contains("ZoneInfo")
                    || a.class_name.contains("WaterZone")
                    || a.class_name.contains("LavaZone")
                    || a.class_name.contains("VacuumZone")
                {
                    zone_actors.push((a.name.clone(), a.location));
                }
            }
        }
        for a in &self.map.actors {
            if a.class.contains("ZoneInfo") || a.class.contains("WaterZone") {
                zone_actors.push((a.name.clone(), a.get_location()));
            }
        }
        ut99_bsp::compute_bsp_zone_flood_fill(&self.world_brushes, &zone_actors)
    }

    /// Generates a canonical UT99 climbable ladder construct (`REQ-G05` — Definitive UnrealEd 2.0 Guide):
    /// - A thin vertical `Non-Solid` + `PF_PORTAL | PF_INVISIBLE` enclosure brush;
    /// - A `WaterZone` actor inside with `bWaterZone = true`, `ViewFog = (0,0,0)` (no underwater tint),
    ///   `FluidFriction = 0.3`, and upward `ZoneVelocity`;
    /// - Bottom and top `LiftExit` + central `LiftCenter` navigation markers so bots can climb it,
    /// all recorded atomically in `UndoHistory`.

    pub fn compute_zone_pvs(&mut self) -> ZonePvsMatrix {
        // Collect zone portal polygons from world brushes
        let mut portals = Vec::new();
        let mut max_zone_id = 0usize;

        for brush in &self.world_brushes {
            let world_polys = brush.to_world_polys();
            for poly in world_polys {
                if (poly.flags & ut99_bsp::poly::PF_PORTAL) != 0 && poly.vertices.len() >= 3 {
                    let center = poly.vertices.iter().copied().sum::<Vec3>() / poly.vertices.len() as f32;
                    let normal = poly.plane.normal;
                    // Determine connected zones or default zone indices
                    let zone_a = max_zone_id;
                    let zone_b = max_zone_id + 1;
                    max_zone_id += 1;

                    portals.push(ZonePortalEdge {
                        portal_index: portals.len(),
                        zone_a,
                        zone_b,
                        portal_center: center,
                        portal_normal: normal,
                        portal_area: 256.0 * 256.0,
                    });
                }
            }
        }

        let zone_count = (max_zone_id + 1).max(2);
        let matrix = build_zone_pvs_graph(zone_count, &portals);
        self.log(format!(
            "Zone PVS graph compiled: {} zones, {} portals, occlusion efficiency {:.1}%.",
            matrix.zone_count,
            portals.len(),
            matrix.occlusion_efficiency_pct()
        ));
        self.zone_pvs_matrix = Some(matrix.clone());
        matrix
    }

    // ==========================================
    // Soft Penumbra Shadow Profiler
    // ==========================================

    pub fn add_sky_zone_info_at_builder(&mut self) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let mut actor = ut99_format::ActorRecord::default();
            actor.name = "SkyZoneInfo0".to_string();
            actor.class_name = "SkyZoneInfo".to_string();
            actor.location = loc;
            unr.actors.push(actor);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.log(format!("Added SkyZoneInfo at ({:.0}, {:.0}, {:.0}). Surfaces with PF_FakeBackdrop will project this skybox.", loc.x, loc.y, loc.z));
        }
    }

    pub fn add_zone_info_at_builder(&mut self, zone_name: &str) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.actors.iter().filter(|a| a.class_name.contains("ZoneInfo")).count();
            let mut actor = ut99_format::ActorRecord::default();
            actor.name = format!("ZoneInfo{}", count);
            actor.class_name = "ZoneInfo".to_string();
            actor.location = loc;
            actor.group = zone_name.to_string();
            unr.actors.push(actor);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.log(format!("Added ZoneInfo ('{}') at ({:.0}, {:.0}, {:.0}).", zone_name, loc.x, loc.y, loc.z));
        }
    }

    pub fn add_water_zone_at_builder(&mut self) {
        let loc = self.builder_brush.location;
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let count = unr.actors.iter().filter(|a| a.class_name.contains("WaterZone")).count();
            let mut actor = ut99_format::ActorRecord::default();
            actor.name = format!("WaterZone{}", count);
            actor.class_name = "WaterZone".to_string();
            actor.location = loc;
            unr.actors.push(actor);
            self.selected_actor_index = Some(unr.actors.len() - 1);
            self.log(format!("Added WaterZone at ({:.0}, {:.0}, {:.0}) with buoyancy and fluid friction.", loc.x, loc.y, loc.z));
        }
    }

    // ==========================================

}


impl EditorState {
    pub fn get_skybox_links(&self) -> Vec<SkyboxLink> {
        let mut sky_zones: Vec<(String, Vec3)> = Vec::new();
        if let Some(ref unr) = self.loaded_unr {
            for actor in &unr.actors {
                if actor.class_name.contains("SkyZoneInfo") {
                    sky_zones.push((actor.name.clone(), actor.location));
                }
            }
        }

        let mut links = Vec::new();
        for (brush_idx, brush) in self.world_brushes.iter().enumerate() {
            for poly in &brush.polys {
                if (poly.flags & ut99_bsp::PF_FAKE_BACKDROP) != 0 && poly.vertices.len() >= 3 {
                    let world_verts: Vec<Vec3> = poly.vertices.iter().map(|&v| v + brush.location).collect();
                    let mut centroid = Vec3::ZERO;
                    for &wv in &world_verts {
                        centroid += wv;
                    }
                    centroid /= world_verts.len() as f32;

                    if let Some(sz) = sky_zones.first() {
                        links.push(SkyboxLink {
                            skyzone_name: sz.0.clone(),
                            skyzone_location: sz.1,
                            backdrop_centroid: centroid,
                            backdrop_vertices: world_verts,
                            brush_index: brush_idx,
                        });
                    } else {
                        links.push(SkyboxLink {
                            skyzone_name: "Unassigned SkyZone".to_string(),
                            skyzone_location: centroid + Vec3::new(0.0, 0.0, 400.0),
                            backdrop_centroid: centroid,
                            backdrop_vertices: world_verts,
                            brush_index: brush_idx,
                        });
                    }
                }
            }
        }

        links
    }


    pub fn is_brush_fake_backdrop(&self, brush_idx: usize) -> bool {
        if let Some(brush) = self.world_brushes.get(brush_idx) {
            brush.polys.iter().any(|p| (p.flags & ut99_bsp::PF_FAKE_BACKDROP) != 0)
        } else {
            false
        }
    }

}
