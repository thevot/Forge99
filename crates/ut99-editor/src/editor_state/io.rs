use super::*;
use std::fs::File;
use std::path::{Path, PathBuf};
use glam::Vec3;

impl EditorState {
    pub fn load_t3d(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let map = parse_t3d(file)?;

        self.log(format!("Loaded T3D map: {:?} with {} actors", path, map.actors.len()));
        let stem = path.file_stem().map(|s| s.to_string_lossy().to_string());
        self.load_t3d_from_map(map, stem.as_deref());
        self.map_path = Some(path.to_path_buf());
        self.add_recent_map(path);
        self.map_is_dirty = false;
        self.build_ai_paths();
        Ok(())
    }

    pub fn load_t3d_from_map(&mut self, map: ut99_format::T3dMap, map_stem: Option<&str>) {
        const ROT_TO_DEG: f32 = 360.0 / 65536.0;
        let parse_t3d_vec3 = |s: &str| -> Option<Vec3> {
            let trimmed = s.trim().trim_start_matches('(').trim_end_matches(')');
            let mut x = 0.0f32;
            let mut y = 0.0f32;
            let mut z = 0.0f32;
            let mut found = false;
            for part in trimmed.split(',') {
                if let Some((k, v)) = part.split_once('=') {
                    let val: f32 = v.trim().parse().unwrap_or(0.0);
                    match k.trim() {
                        "X" | "x" => { x = val; found = true; }
                        "Y" | "y" => { y = val; found = true; }
                        "Z" | "z" => { z = val; found = true; }
                        _ => {}
                    }
                }
            }
            if found { Some(Vec3::new(x, y, z)) } else { None }
        };
        let parse_t3d_rotator = |s: &str| -> Option<(i32, i32, i32)> {
            let trimmed = s.trim().trim_start_matches('(').trim_end_matches(')');
            let mut pitch = 0i32;
            let mut yaw = 0i32;
            let mut roll = 0i32;
            let mut found = false;
            for part in trimmed.split(',') {
                if let Some((k, v)) = part.split_once('=') {
                    let val: i32 = v.trim().parse::<f32>().map(|f| f as i32).unwrap_or(0);
                    match k.trim() {
                        "Pitch" | "pitch" => { pitch = val; found = true; }
                        "Yaw" | "yaw" => { yaw = val; found = true; }
                        "Roll" | "roll" => { roll = val; found = true; }
                        _ => {}
                    }
                }
            }
            if found { Some((pitch, yaw, roll)) } else { None }
        };

        let mut loaded_brushes = Vec::new();
        let mut loaded_movers = Vec::new();
        let mut synth_level = ut99_format::UnrLevel::default();
        synth_level.map_name = map_stem.unwrap_or("Untitled").to_string();
        self.brush_groups.clear();

        for actor in &map.actors {
            let cls_lower = actor.class.to_ascii_lowercase();
            if cls_lower == "levelinfo" {
                if let Some(v) = actor.get_property("Title") {
                    self.level_title = v.trim_matches('"').to_string();
                }
                if let Some(v) = actor.get_property("Author") {
                    self.level_author = v.trim_matches('"').to_string();
                }
                if let Some(v) = actor.get_property("IdealPlayerCount") {
                    self.level_ideal_player_count = v.trim_matches('"').to_string();
                }
                if let Some(v) = actor.get_property("Song") {
                    self.level_song = v.trim_matches('"').to_string();
                }
                if let Some(v) = actor.get_property("DefaultGameType") {
                    let clean = v.trim_start_matches("Class'").trim_end_matches('\'').trim_matches('"');
                    self.level_game_type = clean.to_string();
                }
                if let Some(v) = actor.get_property("LevelEnterText") {
                    self.level_enter_text = v.trim_matches('"').to_string();
                }
                if let Some(v) = actor.get_property("AmbientBrightness").and_then(|s| s.parse().ok()) {
                    self.level_ambient_brightness = v;
                }
                if let Some(v) = actor.get_property("AmbientHue").and_then(|s| s.parse().ok()) {
                    self.level_ambient_hue = v;
                }
                if let Some(v) = actor.get_property("AmbientSaturation").and_then(|s| s.parse().ok()) {
                    self.level_ambient_saturation = v;
                }
            }

            if let Some(ref brush_data) = actor.brush {
                let oper = if let Some(op_str) = actor.get_property("CsgOper") {
                    if op_str.contains("CSG_Subtract") {
                        CsgOperation::Subtract
                    } else {
                        CsgOperation::Add
                    }
                } else {
                    CsgOperation::Add
                };

                let brush_name = if !actor.name.is_empty() {
                    actor.name.clone()
                } else {
                    brush_data.name.clone()
                };
                let mut b = Brush::new(brush_name.clone(), oper);
                b.location = actor.get_location();
                if let Some(rot_str) = actor.get_property("Rotation") {
                    if let Some((pitch, yaw, roll)) = parse_t3d_rotator(rot_str) {
                        b.rotation = Vec3::new(
                            pitch as f32 * ROT_TO_DEG,
                            yaw as f32 * ROT_TO_DEG,
                            roll as f32 * ROT_TO_DEG,
                        );
                    }
                }
                let actor_poly_flags: u32 = actor
                    .get_property("PolyFlags")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                for p in &brush_data.polys {
                    if let Some(mut poly) = ut99_bsp::Polygon::new(p.vertices.clone()) {
                        poly.flags = if p.flags != 0 { p.flags } else { actor_poly_flags };
                        if p.base != Vec3::ZERO {
                            poly.base = p.base;
                        }
                        if p.texture_u != Vec3::ZERO {
                            poly.texture_u = p.texture_u;
                        }
                        if p.texture_v != Vec3::ZERO {
                            poly.texture_v = p.texture_v;
                        }
                        poly.pan_u = p.pan_u;
                        poly.pan_v = p.pan_v;
                        if !p.texture.is_empty() && p.texture != "Default" {
                            poly.texture = p.texture.clone();
                        }
                        b.polys.push(poly);
                    }
                }

                if cls_lower.contains("mover") {
                    let mut mover = EditorMover::new(b.clone());
                    mover.name = brush_name;
                    let short_cls = actor.class.rsplit('.').next().unwrap_or(&actor.class).trim();
                    if !short_cls.is_empty() {
                        mover.mover_class = short_cls.to_string();
                    }
                    if let Some(st) = actor.get_property("InitialState") {
                        mover.initial_state = st.trim_matches('"').trim_matches('\'').to_string();
                    }
                    if let Some(t) = actor.get_property("StayOpenTime").and_then(|s| s.parse().ok()) {
                        mover.stay_open_time = t;
                    }
                    if let Some(t) = actor.get_property("MoveTime").and_then(|s| s.parse().ok()) {
                        mover.move_time = t;
                    }
                    if let Some(t) = actor.get_property("DelayTime").and_then(|s| s.parse().ok()) {
                        mover.delay_time = t;
                    }
                    if let Some(k) = actor.get_property("KeyNum").and_then(|s| s.parse::<usize>().ok()) {
                        mover.current_key = k.min(7);
                    }
                    if let Some(tag) = actor.get_property("Tag") {
                        mover.tag = tag.trim_matches('"').to_string();
                    }
                    if let Some(ev) = actor.get_property("Event") {
                        mover.event = ev.trim_matches('"').to_string();
                    }
                    if let Some(gt) = actor.get_property("MoverGlideType").or_else(|| actor.get_property("GlideType")) {
                        mover.glide_type = gt.trim_matches('"').to_string();
                    }
                    if let Some(et) = actor.get_property("MoverEncroachType").or_else(|| actor.get_property("EncroachType")) {
                        mover.encroach_type = et.trim_matches('"').to_string();
                    }
                    let strip_sound_ref = |s: &str| -> String {
                        let s = s.trim().trim_matches('"');
                        if let Some(rest) = s.strip_prefix("Sound'") {
                            rest.trim_end_matches('\'').to_string()
                        } else {
                            s.to_string()
                        }
                    };
                    if let Some(snd) = actor.get_property("OpeningSound") {
                        mover.opening_sound = strip_sound_ref(snd);
                    }
                    if let Some(snd) = actor.get_property("OpenedSound") {
                        mover.opened_sound = strip_sound_ref(snd);
                    }
                    if let Some(snd) = actor.get_property("ClosingSound") {
                        mover.closing_sound = strip_sound_ref(snd);
                    }
                    if let Some(snd) = actor.get_property("ClosedSound") {
                        mover.closed_sound = strip_sound_ref(snd);
                    }
                    if let Some(snd) = actor.get_property("MoveAmbientSound") {
                        mover.move_ambient_sound = strip_sound_ref(snd);
                    }
                    let base_loc = mover.brush.location;
                    let base_rot = mover.brush.rotation;
                    mover.keyframes = [None, None, None, None, None, None, None, None];
                    mover.keyframes[0] = Some(MoverKeyframe {
                        location: base_loc,
                        rotation: base_rot,
                    });
                    for k_idx in 1..8 {
                        let pos_key = format!("KeyPos({})", k_idx);
                        let rot_key = format!("KeyRot({})", k_idx);
                        let pos_opt = actor.get_property(&pos_key).and_then(parse_t3d_vec3);
                        let rot_opt = actor.get_property(&rot_key).and_then(parse_t3d_rotator);
                        if pos_opt.is_some() || rot_opt.is_some() {
                            let d_pos = pos_opt.unwrap_or(Vec3::ZERO);
                            let d_rot = rot_opt
                                .map(|(p, y, r)| Vec3::new(p as f32 * ROT_TO_DEG, y as f32 * ROT_TO_DEG, r as f32 * ROT_TO_DEG))
                                .unwrap_or(Vec3::ZERO);
                            mover.keyframes[k_idx] = Some(MoverKeyframe {
                                location: base_loc + d_pos,
                                rotation: base_rot + d_rot,
                            });
                        }
                    }
                    loaded_movers.push(mover);
                } else {
                    if let Some(grp) = actor.get_property("Group") {
                        let g = grp.trim_matches('"').to_string();
                        if !g.is_empty() {
                            b.group = Some(g.clone());
                            self.brush_groups.insert(loaded_brushes.len(), g);
                        }
                    }
                    loaded_brushes.push(b);
                }
            } else if cls_lower != "levelinfo" {
                let mut rec = ut99_format::ActorRecord::default();
                rec.name = actor.name.clone();
                rec.class_name = actor.class.clone();
                rec.location = actor.get_location();
                if let Some(rot_str) = actor.get_property("Rotation") {
                    if let Some(rot) = parse_t3d_rotator(rot_str) {
                        rec.rotation = rot;
                    }
                }
                if let Some(tag) = actor.get_property("Tag") {
                    rec.tag = tag.trim_matches('"').to_string();
                }
                if let Some(ev) = actor.get_property("Event") {
                    rec.event = ev.trim_matches('"').to_string();
                }
                if let Some(grp) = actor.get_property("Group") {
                    rec.group = grp.trim_matches('"').to_string();
                }
                if cls_lower.contains("light") {
                    rec.light_brightness = actor.get_property("LightBrightness").and_then(|s| s.parse().ok()).unwrap_or(64);
                    rec.light_radius = actor.get_property("LightRadius").and_then(|s| s.parse().ok()).unwrap_or(32);
                    rec.light_hue = actor.get_property("LightHue").and_then(|s| s.parse().ok()).unwrap_or(0);
                    rec.light_saturation = actor.get_property("LightSaturation").and_then(|s| s.parse().ok()).unwrap_or(255);
                    rec.light_type = actor.get_property("LightType").and_then(|s| s.parse().ok()).unwrap_or(1);
                    rec.light_effect = actor.get_property("LightEffect").and_then(|s| s.parse().ok()).unwrap_or(0);
                }
                for (k, v) in &actor.properties {
                    let k_low = k.to_ascii_lowercase();
                    if matches!(
                        k_low.as_str(),
                        "location" | "rotation" | "tag" | "event" | "group" | "lightbrightness" | "lightradius" | "lighthue" | "lightsaturation" | "lighttype" | "lighteffect" | "name"
                    ) {
                        continue;
                    }
                    let tv = if v.eq_ignore_ascii_case("true") {
                        TaggedValue::Bool(true)
                    } else if v.eq_ignore_ascii_case("false") {
                        TaggedValue::Bool(false)
                    } else if v.starts_with('"') && v.ends_with('"') {
                        TaggedValue::Str(v.trim_matches('"').to_string())
                    } else if let Some(vec_v) = parse_t3d_vec3(v) {
                        TaggedValue::Vector(vec_v)
                    } else if let Some((pitch, yaw, roll)) = parse_t3d_rotator(v) {
                        TaggedValue::Rotator { pitch, yaw, roll }
                    } else if let Ok(i) = v.parse::<i32>() {
                        TaggedValue::Int(i)
                    } else if let Ok(f) = v.parse::<f32>() {
                        TaggedValue::Float(f)
                    } else {
                        TaggedValue::Name(v.clone())
                    };
                    rec.properties.insert(
                        k.clone(),
                        TaggedProperty {
                            name: k.clone(),
                            array_index: 0,
                            value: tv,
                        },
                    );
                }

                synth_level.actors.push(rec);
            }
        }

        synth_level.sync_actor_views();
        if !loaded_brushes.is_empty() {
            self.world_brushes = loaded_brushes.into();
            self.log(format!("Imported {} world geometry brushes from map.", self.world_brushes.len()));
        }
        self.movers = loaded_movers.into();
        self.loaded_unr = Some(synth_level);
        self.map = map;
        self.map_instance_id = next_map_instance_id();
        self.cancel_stale_async_auto_save();
    }

    pub fn save_t3d(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("map.t3d").to_string();
        let start_time = std::time::Instant::now();
        let mut file = File::create(path)?;

        let exported_map = crate::compiler::CompilerBridge::export_t3d_map(self);
        self.map = exported_map;

        write_t3d(&mut file, &self.map)?;
        let duration = start_time.elapsed();
        let size = std::fs::metadata(path).map(|m| m.len()).ok();
        let size_str = size.map(|s| {
            if s >= 1024 * 1024 {
                format!("{:.2} MB", s as f64 / (1024.0 * 1024.0))
            } else {
                format!("{:.1} KB", s as f64 / 1024.0)
            }
        }).unwrap_or_else(|| "Exported".to_string());

        self.log(format!("Successfully exported authoritative T3D map to {:?} in {:.2}s", path, duration.as_secs_f32()));
        self.toast_success(format!("Exported T3D: {file_name} ({size_str})"));
        self.map_path = Some(path.to_path_buf());
        self.add_recent_map(path);
        self.map_is_dirty = false;

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::SaveT3d(path.to_path_buf()),
            filename: file_name,
            target_path: path.to_path_buf(),
            current_stage: 2,
            total_stages: 2,
            stage_message: format!("Exported in {:.2}s ({})", duration.as_secs_f32(), size_str),
            phase: SavePhase::Completed,
            started_at: start_time,
            completed_at: Some(std::time::Instant::now()),
            file_size_bytes: size,
            error_message: None,
        });

        Ok(())
    }

    /// UnrealEd 2.0 `File -> Import... -> Into Existing Map` (`MAP IMPORTADD FILE=...` / `REQ-J02`):
    /// Merges the World Brushes, Movers, and Actors of a `.t3d` file into the currently open map
    /// without clearing existing geometry or actors, recording a full Undo checkpoint.
    /// Returns `(added_brushes, added_movers, added_actors)`.
    pub fn import_t3d_into_existing_map(&mut self, path: impl AsRef<Path>) -> anyhow::Result<(usize, usize, usize)> {
        let path = path.as_ref();
        let mut imported = EditorState::default();
        imported.world_brushes.clear();
        imported.movers.clear();
        imported.load_t3d(path)?;

        let old_brushes = self.world_brushes.to_vec();
        let old_actors = self.loaded_unr.as_ref().map(|u| u.actors.to_vec());
        let old_movers = self.movers.to_vec();

        let base_brush_idx = self.world_brushes.len();
        let mut added_brushes = 0usize;
        for mut b in imported.world_brushes {
            if self.world_brushes.iter().any(|eb| eb.name.eq_ignore_ascii_case(&b.name)) {
                b.name = format!("{}_Merged{}", b.name, base_brush_idx + added_brushes);
            }
            self.world_brushes.push(b);
            added_brushes += 1;
        }
        for (idx, grp) in imported.brush_groups {
            self.brush_groups.insert(base_brush_idx + idx, grp);
        }

        let base_mover_idx = self.movers.len();
        let mut added_movers = 0usize;
        for mut m in imported.movers {
            if self.movers.iter().any(|em| em.name.eq_ignore_ascii_case(&m.name)) {
                m.name = format!("{}_Merged{}", m.name, base_mover_idx + added_movers);
            }
            self.movers.push(m);
            added_movers += 1;
        }

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        let mut added_actors = 0usize;
        if let (Some(ref mut dest_unr), Some(src_unr)) = (&mut self.loaded_unr, imported.loaded_unr) {
            let base_act_idx = dest_unr.actors.len();
            for mut act in src_unr.actors {
                let short_cls = act.class_name.rsplit('.').next().unwrap_or(&act.class_name);
                if short_cls.eq_ignore_ascii_case("InventorySpot") {
                    continue;
                }
                if dest_unr.actors.iter().any(|ea| ea.name.eq_ignore_ascii_case(&act.name)) {
                    act.name = format!("{}_Merged{}", act.name, base_act_idx + added_actors);
                }
                let cls_lower = act.class_name.to_ascii_lowercase();
                if cls_lower.contains("light") {
                    dest_unr.lights.push(act.clone());
                } else if cls_lower.contains("playerstart") {
                    dest_unr.player_starts.push(act.clone());
                } else if cls_lower.contains("pathnode") || cls_lower.contains("lift") || cls_lower.contains("jump") {
                    dest_unr.path_nodes.push(act.clone());
                } else {
                    dest_unr.other_actors.push(act.clone());
                }
                dest_unr.actors.push(act);
                added_actors += 1;
            }
        }

        let new_brushes = self.world_brushes.to_vec();
        let new_actors = self.loaded_unr.as_ref().map(|u| u.actors.to_vec());
        let new_movers = self.movers.to_vec();
        self.history.push(Action::ScaleEntireLevel {
            old_brushes,
            new_brushes,
            old_actors,
            new_actors,
            old_movers,
            new_movers,
            factor: Vec3::ONE,
        });
        self.map_is_dirty = true;
        self.log(format!(
            "Merged T3D '{}' into existing map: +{} brush(es), +{} mover(s), +{} actor(s).",
            path.display(),
            added_brushes,
            added_movers,
            added_actors
        ));
        Ok((added_brushes, added_movers, added_actors))
    }

    pub fn load_unr(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let path = path.as_ref();
        let level = ut99_format::UnrLevel::open(path)?;
        self.load_unr_level(level, path)
    }

    /// Applies an already parsed level using its original path for identity and assets.
    pub(super) fn load_unr_level(&mut self, level: ut99_format::UnrLevel, path: &Path) -> anyhow::Result<()> {
        self.log(format!(
            "Loaded UNR map '{}': {} actors ({} brushes, {} lights, {} player starts, {} path nodes, {} weapons/pickups)",
            level.map_name,
            level.actors.len(),
            level.brushes.len(),
            level.lights.len(),
            level.player_starts.len(),
            level.path_nodes.len(),
            level.weapons_and_pickups.len(),
        ));

        const ROT_TO_DEG: f32 = 360.0 / 65536.0;
        let mut loaded_brushes = Vec::new();
        let mut loaded_movers = Vec::new();
        self.brush_groups.clear();

        for unr_brush in &level.brushes {
            let oper = match unr_brush.actor.csg_oper {
                2 => CsgOperation::Subtract,
                _ => CsgOperation::Add,
            };

            let loc = unr_brush.actor.location;
            let mut b = Brush::new(&unr_brush.actor.name, oper);
            b.location = loc;
            for p in &unr_brush.world_polys {
                let local_verts: Vec<Vec3> = p.vertices.iter().map(|&v| v - loc).collect();
                if let Some(mut poly) = ut99_bsp::Polygon::new(local_verts) {
                    poly.flags = p.poly_flags;
                    poly.base = p.base - loc;
                    poly.texture_u = p.texture_u;
                    poly.texture_v = p.texture_v;
                    poly.pan_u = p.pan_u;
                    poly.pan_v = p.pan_v;
                    if p.texture_ref != 0 {
                        let tex_str = level.package.resolve_full_object_path(p.texture_ref);
                        if tex_str != "None"
                            && !tex_str.is_empty()
                            && !tex_str.starts_with("Export(")
                            && !tex_str.starts_with("Import(")
                        {
                            let clean_tex = if let Some((_, inner)) = tex_str.split_once('\'') {
                                inner.trim_end_matches('\'').to_string()
                            } else {
                                tex_str
                            };
                            poly.texture = clean_tex;
                        }
                    }
                    b.polys.push(poly);
                }
            }
            if !b.polys.is_empty() {
                if !unr_brush.actor.group.is_empty() {
                    b.group = Some(unr_brush.actor.group.clone());
                    self.brush_groups.insert(loaded_brushes.len(), unr_brush.actor.group.clone());
                }
                if unr_brush.actor.class_name.to_ascii_lowercase().contains("mover") {
                    let mut mover = EditorMover::new(b.clone());
                    mover.name = unr_brush.actor.name.clone();
                    let short_cls = unr_brush
                        .actor
                        .class_name
                        .rsplit('.')
                        .next()
                        .unwrap_or(&unr_brush.actor.class_name)
                        .trim();
                    if !short_cls.is_empty() {
                        mover.mover_class = short_cls.to_string();
                    }
                    mover.tag = unr_brush.actor.tag.clone();
                    mover.event = unr_brush.actor.event.clone();
                    if let Some(prop) = unr_brush.actor.properties.get("InitialState") {
                        match &prop.value {
                            TaggedValue::Name(s) | TaggedValue::Str(s) => mover.initial_state = s.clone(),
                            _ => {}
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("StayOpenTime") {
                        if let TaggedValue::Float(f) = prop.value {
                            mover.stay_open_time = f;
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("MoveTime") {
                        if let TaggedValue::Float(f) = prop.value {
                            mover.move_time = f;
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("DelayTime") {
                        if let TaggedValue::Float(f) = prop.value {
                            mover.delay_time = f;
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("KeyNum") {
                        if let TaggedValue::Byte(k) = prop.value {
                            mover.current_key = (k as usize).min(7);
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("MoverGlideType") {
                        if let TaggedValue::Name(ref s) = prop.value {
                            mover.glide_type = s.clone();
                        }
                    }
                    if let Some(prop) = unr_brush.actor.properties.get("MoverEncroachType") {
                        if let TaggedValue::Name(ref s) = prop.value {
                            mover.encroach_type = s.clone();
                        }
                    }
                    let strip_snd = |s: &str| -> String {
                        let s = s.trim();
                        if let Some(rest) = s.strip_prefix("Sound'") {
                            rest.trim_end_matches('\'').to_string()
                        } else {
                            s.to_string()
                        }
                    };
                    if let Some(snd) = unr_brush.actor.object_paths.get("OpeningSound") {
                        mover.opening_sound = strip_snd(snd);
                    }
                    if let Some(snd) = unr_brush.actor.object_paths.get("OpenedSound") {
                        mover.opened_sound = strip_snd(snd);
                    }
                    if let Some(snd) = unr_brush.actor.object_paths.get("ClosingSound") {
                        mover.closing_sound = strip_snd(snd);
                    }
                    if let Some(snd) = unr_brush.actor.object_paths.get("ClosedSound") {
                        mover.closed_sound = strip_snd(snd);
                    }
                    if let Some(snd) = unr_brush.actor.object_paths.get("MoveAmbientSound") {
                        mover.move_ambient_sound = strip_snd(snd);
                    }
                    let base_loc = mover.brush.location;
                    let base_rot = mover.brush.rotation;
                    mover.keyframes = [None, None, None, None, None, None, None, None];
                    mover.keyframes[0] = Some(MoverKeyframe {
                        location: base_loc,
                        rotation: base_rot,
                    });
                    for k_idx in 1..8 {
                        let pos_key = format!("KeyPos({})", k_idx);
                        let rot_key = format!("KeyRot({})", k_idx);
                        let pos_opt = unr_brush.actor.properties.get(&pos_key).and_then(|p| match p.value {
                            TaggedValue::Vector(v) => Some(v),
                            _ => None,
                        });
                        let rot_opt = unr_brush.actor.properties.get(&rot_key).and_then(|p| match p.value {
                            TaggedValue::Rotator { pitch, yaw, roll } => Some(Vec3::new(
                                pitch as f32 * ROT_TO_DEG,
                                yaw as f32 * ROT_TO_DEG,
                                roll as f32 * ROT_TO_DEG,
                            )),
                            _ => None,
                        });
                        if pos_opt.is_some() || rot_opt.is_some() {
                            mover.keyframes[k_idx] = Some(MoverKeyframe {
                                location: base_loc + pos_opt.unwrap_or(Vec3::ZERO),
                                rotation: base_rot + rot_opt.unwrap_or(Vec3::ZERO),
                            });
                        }
                    }
                    loaded_movers.push(mover);
                }
                loaded_brushes.push(b);
            }
        }

        if !loaded_brushes.is_empty() {
            self.world_brushes = loaded_brushes.into();
            self.log(format!(
                "Successfully populated 3D world with {} geometry brushes.",
                self.world_brushes.len()
            ));
        } else if let Some(reconstructed) = Self::reconstruct_brush_from_compiled_bsp(&level) {
            let poly_count = reconstructed.polys.len();
            self.world_brushes = vec![reconstructed].into();
            self.log(format!(
                "Brush-stripped .unr detected: reconstructed {} BSP polygons from compiled Model_World.",
                poly_count
            ));
        }
        self.movers = loaded_movers.into();

        // Center camera near the action
        if let Some(start) = level.player_starts.iter().find(|ps| ps.location != Vec3::ZERO).or_else(|| level.player_starts.first()) {
            self.camera.position = start.location + Vec3::new(0.0, -350.0, 180.0);
            self.camera.yaw = 90.0;
            self.camera.pitch = -15.0;
            self.log(format!(
                "Camera centered on PlayerStart '{}' at ({:.0}, {:.0}, {:.0})",
                start.name, start.location.x, start.location.y, start.location.z
            ));
        } else if let Some(brush) = self.world_brushes.first() {
            self.camera.position = brush.location + Vec3::new(0.0, -600.0, 300.0);
        }

        // Extract Level Properties from LevelInfo actor if present
        if let Some(level_info) = level.actors.iter().find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo")) {
            if let Some(prop) = level_info.properties.get("Title") {
                if let ut99_format::TaggedValue::Str(ref s) = prop.value {
                    self.level_title = s.clone();
                }
            }
            if let Some(prop) = level_info.properties.get("Author") {
                if let ut99_format::TaggedValue::Str(ref s) = prop.value {
                    self.level_author = s.clone();
                }
            }
            if let Some(prop) = level_info.properties.get("IdealPlayerCount") {
                if let ut99_format::TaggedValue::Str(ref s) = prop.value {
                    self.level_ideal_player_count = s.clone();
                }
            }
            if let Some(prop) = level_info.properties.get("Song") {
                match &prop.value {
                    ut99_format::TaggedValue::Name(s) | ut99_format::TaggedValue::Str(s) => {
                        self.level_song = s.clone();
                    }
                    _ => {}
                }
            }
            if let Some(prop) = level_info.properties.get("DefaultGameType") {
                match &prop.value {
                    ut99_format::TaggedValue::Name(s) | ut99_format::TaggedValue::Str(s) => {
                        self.level_game_type = s.clone();
                    }
                    _ => {}
                }
            }
            if let Some(prop) = level_info.properties.get("LevelEnterText") {
                if let ut99_format::TaggedValue::Str(ref s) = prop.value {
                    self.level_enter_text = s.clone();
                }
            }
            if let Some(prop) = level_info.properties.get("AmbientBrightness") {
                if let ut99_format::TaggedValue::Byte(b) = prop.value {
                    self.level_ambient_brightness = b;
                }
            }
            if let Some(prop) = level_info.properties.get("AmbientHue") {
                if let ut99_format::TaggedValue::Byte(b) = prop.value {
                    self.level_ambient_hue = b;
                }
            }
            if let Some(prop) = level_info.properties.get("AmbientSaturation") {
                if let ut99_format::TaggedValue::Byte(b) = prop.value {
                    self.level_ambient_saturation = b;
                }
            }
        }

        if !level.embedded_textures.is_empty() {
            for tex in &level.embedded_textures {
                self.texture_manager.upsert_texture("MyLevel", tex.clone());
            }
            self.texture_manager.pin_package("MyLevel");
            self.log(format!(
                "Loaded {} embedded MyLevel texture(s) from '{}'.",
                level.embedded_textures.len(),
                level.map_name
            ));
        }

        if !level.embedded_sounds.is_empty() {
            self.mylevel_sounds = level.embedded_sounds.clone();
            if !self.sound_packages.iter().any(|p| p.eq_ignore_ascii_case("MyLevel")) {
                self.sound_packages.insert(0, "MyLevel".to_string());
            }
            if self.selected_sound_package.eq_ignore_ascii_case("MyLevel") || self.loaded_sounds.is_empty() {
                self.selected_sound_package = "MyLevel".to_string();
                self.loaded_sounds = self.mylevel_sounds.clone();
            }
            self.log(format!(
                "Loaded {} embedded MyLevel sound(s) from '{}'.",
                level.embedded_sounds.len(),
                level.map_name
            ));
        }

        // Auto-load referenced external .utx packages if located adjacent to map or in UT99_DIR/Textures
        let mut referenced_pkgs = std::collections::HashSet::new();
        for b in &self.world_brushes {
            for p in &b.polys {
                if let Some((pkg_name, _)) = p.texture.split_once('.') {
                    if !pkg_name.is_empty() && !pkg_name.eq_ignore_ascii_case("MyLevel") && !pkg_name.eq_ignore_ascii_case("Default") {
                        referenced_pkgs.insert(pkg_name.to_string());
                    }
                }
            }
        }
        for pkg_name in referenced_pkgs {
            if self.texture_manager.get_package(&pkg_name).is_none() {
                let mut candidates = Vec::new();
                if let Some(map_dir) = path.parent() {
                    candidates.push(map_dir.join(format!("{pkg_name}.utx")));
                    if let Some(ut_root) = map_dir.parent() {
                        candidates.push(ut_root.join("Textures").join(format!("{pkg_name}.utx")));
                    }
                }
                if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
                    candidates.push(ut_dir.join("Textures").join(format!("{pkg_name}.utx")));
                }
                for cand in candidates {
                    if cand.exists() {
                        if let Ok(utx_pkg) = ut99_format::UtxPackage::open(&cand) {
                            self.texture_manager.insert_package(pkg_name.clone(), utx_pkg);
                            self.log(format!("Auto-loaded referenced texture package '{}'", pkg_name));
                            break;
                        }
                    }
                }
            }
        }

        // Restore serialized ReachSpecs into nav_graph if present
        if !level.reach_specs.is_empty() {
            self.nav_graph = NavGraph::new();
            let mut name_to_idx = std::collections::HashMap::new();
            for actor in &level.actors {
                if actor.class_name.contains("PathNode")
                    || actor.class_name.contains("PlayerStart")
                    || actor.class_name.contains("Inventory")
                    || actor.class_name.contains("Lift")
                    || actor.class_name.contains("Teleporter")
                {
                    let idx = self.nav_graph.add_node(&actor.name, actor.location, &actor.class_name);
                    name_to_idx.insert(actor.name.to_ascii_lowercase(), idx);
                }
            }
            for rs in &level.reach_specs {
                if let (Some(&from), Some(&to)) = (
                    name_to_idx.get(&rs.start_actor_name.to_ascii_lowercase()),
                    name_to_idx.get(&rs.end_actor_name.to_ascii_lowercase()),
                ) {
                    self.nav_graph.add_reach_spec(from, to, rs.reach_flags as u32);
                }
            }
        }

        self.loaded_unr = Some(level);
        self.map_path = Some(path.to_path_buf());
        self.add_recent_map(path);
        self.map_is_dirty = false;
        self.map_instance_id = next_map_instance_id();
        self.cancel_stale_async_auto_save();
        self.refresh_ai_path_links();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("map.unr");
        self.toast_success(format!("Loaded map: {file_name}"));
        Ok(())
    }

    pub fn prompt_or_open_unr_dialog(&mut self) {
        if self.map_is_dirty {
            self.pending_dirty_action = Some(PendingDirtyAction::OpenUnrDialog);
            self.show_unsaved_exit_modal = true;
        } else {
            self.execute_open_unr_dialog();
        }
    }

    pub fn prompt_or_open_t3d_dialog(&mut self) {
        if self.map_is_dirty {
            self.pending_dirty_action = Some(PendingDirtyAction::OpenT3dDialog);
            self.show_unsaved_exit_modal = true;
        } else {
            self.execute_open_t3d_dialog();
        }
    }

    pub fn prompt_or_open_map_path(&mut self, path: std::path::PathBuf) {
        if self.map_is_dirty {
            self.pending_dirty_action = Some(PendingDirtyAction::OpenMapPath(path));
            self.show_unsaved_exit_modal = true;
        } else {
            self.execute_open_map_path(&path);
        }
    }

    pub fn execute_open_unr_dialog(&mut self) {
        let default_dir = std::path::Path::new("UnrealTournament/Maps");
        let mut dialog = rfd::FileDialog::new().add_filter("Unreal Map (*.unr)", &["unr"]);
        if default_dir.exists() {
            dialog = dialog.set_directory(default_dir);
        }
        if let Some(path) = dialog.pick_file() {
            if let Err(e) = self.load_unr(&path) {
                self.log(format!("Error loading UNR map: {e}"));
            }
        }
    }

    pub fn execute_open_t3d_dialog(&mut self) {
        let dialog = rfd::FileDialog::new().add_filter("Unreal Text 3D (*.t3d)", &["t3d"]);
        if let Some(path) = dialog.pick_file() {
            if let Err(e) = self.load_t3d(&path) {
                self.log(format!("Error loading map: {e}"));
            }
        }
    }

    pub fn execute_open_map_path(&mut self, path: &std::path::Path) {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        if ext == "unr" {
            if let Err(e) = self.load_unr(path) {
                self.log(format!("Error loading UNR map: {e}"));
            }
        } else if ext == "t3d" {
            if let Err(e) = self.load_t3d(path) {
                self.log(format!("Error loading T3D map: {e}"));
            }
        }
    }

    /// Executes any action (Exit, New, Open, Close Tab) that was deferred pending save confirmation.
    pub fn execute_pending_dirty_action(&mut self) {
        self.show_unsaved_exit_modal = false;
        if let Some(act) = self.pending_dirty_action.take() {
            match act {
                PendingDirtyAction::ExitApp => {
                    self.confirm_exit_without_saving = true;
                }
                PendingDirtyAction::NewMap => {
                    self.new_map();
                }
                PendingDirtyAction::CloseDocumentTab(idx) => {
                    self.confirmed_close_tab_index = Some(idx);
                }
                PendingDirtyAction::OpenUnrDialog => {
                    self.execute_open_unr_dialog();
                }
                PendingDirtyAction::OpenT3dDialog => {
                    self.execute_open_t3d_dialog();
                }
                PendingDirtyAction::OpenMapPath(path) => {
                    self.execute_open_map_path(&path);
                }
            }
        }
    }

    /// Creates a fresh document. Returns false while a save or viewport transform is active.
    pub fn new_map(&mut self) -> bool {
        if self.pending_save_op.is_some() || self.save_worker_rx.is_some() {
            self.toast_warning("Finish the current save or export before creating a new map.");
            return false;
        }
        if self.active_gizmo_axis != GizmoAxis::None {
            self.toast_warning("Finish or cancel the viewport transform before creating a new map.");
            return false;
        }
        self.clear_document_for_new_map();
        self.world_brushes.clear();
        self.brush_groups.clear();
        self.hidden_actor_groups.clear();
        let mut initial_room = build_cube("Room0", CsgOperation::Subtract, Vec3::new(1024.0, 1024.0, 512.0));
        initial_room.location = Vec3::new(0.0, 0.0, 256.0);
        self.world_brushes.push(initial_room);

        // Reset Level Properties to defaults
        self.level_title = "Untitled Arena".to_string();
        self.level_author = if self.default_author.trim().is_empty() {
            "Unknown Mapper".to_string()
        } else {
            self.default_author.clone()
        };
        self.level_ideal_player_count = "4-8".to_string();
        self.level_song = String::new();
        self.level_song_section = 0;
        self.level_game_type = "Botpack.DeathMatchPlus".to_string();
        self.level_enter_text = String::new();
        self.level_ambient_brightness = 0;
        self.level_ambient_hue = 0;
        self.level_ambient_saturation = 255;

        let mut actors = Vec::new();
        let mut start = ut99_format::ActorRecord::default();
        start.name = "PlayerStart0".into();
        start.class_name = "PlayerStart".into();
        start.location = Vec3::new(0.0, 0.0, 48.0);
        actors.push(start.clone());

        let mut light = ut99_format::ActorRecord::default();
        light.name = "Light0".into();
        light.class_name = "Light".into();
        light.location = Vec3::new(0.0, 0.0, 384.0);
        light.light_brightness = 64;
        light.light_radius = 64;
        actors.push(light.clone());

        let level = ut99_format::UnrLevel {
            map_name: "Untitled.unr".into(),
            actors: actors.into(),
            brushes: Vec::new(),
            lights: vec![light],
            player_starts: vec![start],
            path_nodes: Vec::new(),
            weapons_and_pickups: Vec::new(),
            other_actors: Vec::new(),
            embedded_textures: Vec::new(),
            package: ut99_format::Package::default(),
            ..Default::default()
        };

        self.loaded_unr = Some(level);
        self.map_path = None;
        self.selected_brush_index = None;
        self.selected_actor_index = None;
        self.is_builder_selected = false;
        self.hidden_brushes.clear();
        self.hidden_actors.clear();
        self.hidden_movers.clear();
        self.hidden_static_meshes.clear();
        self.locked_brushes.clear();
        self.locked_actors.clear();
        self.locked_movers.clear();
        self.locked_static_meshes.clear();
        self.is_builder_locked = false;
        self.outliner_last_clicked_row = None;
        self.builder_brush.location = Vec3::ZERO;
        self.builder_brush.rotation = Vec3::ZERO;
        self.camera.position = Vec3::new(0.0, -350.0, 180.0);
        self.camera.yaw = 90.0f32.to_radians();
        self.camera.pitch = -15.0f32.to_radians();
        self.history = History::default();
        self.map_is_dirty = false;
        self.log("Created New Map from scratch: subtractive room (1024 x 1024 x 512) with PlayerStart and Light.");
        self.toast_info("Created new empty map.");
        self.refresh_ai_path_links();
        true
    }

    fn clear_document_for_new_map(&mut self) {
        self.map_instance_id = next_map_instance_id();
        self.cancel_stale_async_auto_save();
        // Invalidate interactions before removing any objects their indices refer to.
        self.deselect_all();
        self.selected_projector_index = None;
        self.selected_dispatcher_name = None;
        self.memorized_surfaces.clear();
        self.selection_locked = false;
        self.active_gizmo_axis = GizmoAxis::None;
        self.hovered_gizmo_axis = GizmoAxis::None;
        self.active_gizmo_delta = Vec3::ZERO;
        self.active_gizmo_angle_deg = 0.0;
        self.gizmo_drag_initial_brushes.clear();
        self.gizmo_drag_initial_actors.clear();
        self.numeric_edit_brush_loc_initial = None;
        self.numeric_edit_brush_rot_initial = None;
        self.numeric_edit_brush_scale_initial = None;
        self.numeric_edit_actor_loc_initial = None;
        self.uv_edit_initial_polys = None;
        self.active_drag_drop = None;
        self.drag_drop_preview_pos = None;
        self.drag_drop_preview_normal = None;
        self.active_marquee = None;
        self.clip_points.clear();
        self.clip_normal_flipped = false;
        self.freehand_points.clear();
        self.cursor_delta_text = None;
        self.laser_ruler_start = None;
        self.laser_ruler_end = None;
        self.context_menu_world_pos = None;
        self.show_viewport_context_menu = false;

        self.movers.clear();
        self.static_meshes.clear();
        self.projectors.clear();
        self.active_projector_decals.clear();
        self.antiportals.clear();
        self.particle_emitters.clear();
        self.particle_settings.selected_emitter_index = 0;
        self.map = T3dMap::default();
        self.mover_preview_time = 0.0;
        self.mover_preview_playing = false;
        self.cinematic_settings.active_track = ut99_bsp::CinematicCameraTrack::default();
        self.cinematic_settings.playhead_t = 0.0;
        self.cinematic_settings.is_playing = false;
        self.cinematic_settings.pilot_camera = false;
        self.cinematic_settings.selected_keyframe_idx = None;
        self.camera_bookmarks = std::array::from_fn(|_| None);
        self.background_image_path = None;

        // MyLevel belongs to this document; imported and external libraries remain available.
        self.texture_manager.clear_mylevel_assets();
        let is_mylevel = |name: &str| name.to_ascii_lowercase().starts_with("mylevel.");
        if self.active_texture.as_deref().is_some_and(is_mylevel) { self.active_texture = None; }
        for poly in &mut self.builder_brush.polys {
            if is_mylevel(&poly.texture) { poly.texture.clear(); }
        }
        if self.selected_texture_package.eq_ignore_ascii_case("MyLevel") {
            self.selected_texture_package = self.texture_manager.available_packages().first().cloned().unwrap_or_default();
            self.selected_texture_group.clear();
        }
        self.mylevel_sounds.clear();
        self.sound_packages.retain(|name| !name.eq_ignore_ascii_case("MyLevel"));
        self.loaded_sounds.retain(|sound| !sound.package.eq_ignore_ascii_case("MyLevel"));
        if self.selected_sound_package.eq_ignore_ascii_case("MyLevel") {
            self.selected_sound_package.clear();
            self.selected_sound_name = None;
        }

        self.nav_graph = NavGraph::new();
        self.bot_simulator = None;
        self.sim_start_node = 0;
        self.sim_target_node = 0;
        self.last_autopath_report = None;
        self.last_bsp_report = None;
        self.last_lighting_stats = None;
        self.map_audit_issues.clear();
        self.level_audit_snapshot = None;
        self.selected_issue_index = None;
        self.cached_map_manifest = None;
        self.packaging_map_name_edit.clear();
        self.build_progress = None;
        self.save_feedback = None;
        self.pending_dirty_action = None;
        self.show_unsaved_exit_modal = false;
        self.auto_save_timer = 0.0;
        self.geometry_revision = self.geometry_revision.wrapping_add(1);
        self.scene_bounds_cache.borrow_mut().reset();
        self.outliner_cache.reset();
    }

    pub fn save_unr(&mut self, path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let path = path.as_ref();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("map.unr").to_string();
        self.log(format!("Compiling and saving map to {:?} with automatic backup...", path));
        let start_time = std::time::Instant::now();
        match crate::compiler::CompilerBridge::save_unr_safe(self, path) {
            Ok(saved_path) => {
                let duration = start_time.elapsed();
                self.map_path = Some(saved_path.clone());
                self.add_recent_map(&saved_path);
                self.map_is_dirty = false;
                self.log(format!("Map successfully saved and built at {:?} in {:.2}s", saved_path, duration.as_secs_f32()));
                let size = std::fs::metadata(&saved_path).map(|m| m.len()).ok();
                let size_str = size.map(|s| {
                    if s >= 1024 * 1024 {
                        format!("{:.2} MB", s as f64 / (1024.0 * 1024.0))
                    } else {
                        format!("{:.1} KB", s as f64 / 1024.0)
                    }
                }).unwrap_or_else(|| "Saved".to_string());
                self.toast_success(format!("Saved map: {file_name} ({size_str})"));

                self.save_feedback = Some(SaveFeedbackState {
                    kind: SaveOperationKind::SaveUnr(saved_path.clone()),
                    filename: file_name,
                    target_path: saved_path.clone(),
                    current_stage: 4,
                    total_stages: 4,
                    stage_message: format!("Saved in {:.2}s ({})", duration.as_secs_f32(), size_str),
                    phase: SavePhase::Completed,
                    started_at: start_time,
                    completed_at: Some(std::time::Instant::now()),
                    file_size_bytes: size,
                    error_message: None,
                });

                Ok(saved_path)
            }
            Err(e) => {
                self.log(format!("Failed to save UNR map: {e}"));
                self.toast_error(format!("Save failed: {e}"));
                self.save_feedback = Some(SaveFeedbackState {
                    kind: SaveOperationKind::SaveUnr(path.to_path_buf()),
                    filename: file_name,
                    target_path: path.to_path_buf(),
                    current_stage: 2,
                    total_stages: 4,
                    stage_message: format!("Save failed: {e}"),
                    phase: SavePhase::Failed,
                    started_at: start_time,
                    completed_at: Some(std::time::Instant::now()),
                    file_size_bytes: None,
                    error_message: Some(e.to_string()),
                });
                Err(e)
            }
        }
    }

    /// The wizard's verified path requires a current static audit and validates
    /// the staged package before replacing the requested destination.
    pub fn save_wizard_unr_verified(&mut self, path: impl AsRef<Path>) -> anyhow::Result<crate::compiler::UnrSaveReport> {
        if self.level_audit_freshness() != LevelAuditFreshness::Current {
            anyhow::bail!("Level Doctor report is absent or out of date");
        }
        if self.map_audit_issues.iter().any(|issue| issue.severity == ut99_bsp::IssueSeverity::Error) {
            anyhow::bail!("Level Doctor reported blocking errors");
        }
        let audited_name = self.map_path.as_ref().and_then(|p| p.file_stem()).and_then(|s| s.to_str());
        let target_name = path.as_ref().file_stem().and_then(|s| s.to_str());
        if !matches!((audited_name, target_name), (Some(a), Some(b)) if a.eq_ignore_ascii_case(b)) {
            anyhow::bail!("The destination map name differs from the audited map name");
        }
        let report = crate::compiler::CompilerBridge::save_unr_verified(self, path.as_ref())?;
        self.map_path = Some(report.path.clone());
        self.add_recent_map(&report.path);
        self.map_is_dirty = false;
        // Mark the map as wizard-generated so it is never used as reference material.
        match ut99_format::unr::write_generator_manifest(
            &report.path,
            env!("CARGO_PKG_VERSION"),
            self.map_wizard_config.seed,
            self.map_wizard_config.game_mode.prefix(),
        ) {
            Ok(manifest) => self.log(format!("Generator manifest written to {manifest:?}")),
            Err(e) => self.log(format!("Could not write generator manifest: {e}")),
        }
        self.log(format!("Verified package saved to {:?} using {:?}; native reason: {:?}; engine warning: {:?}", report.path, report.backend, report.native_reason, report.engine_warning));
        Ok(report)
    }

    pub fn save_unr_native(&mut self, path: impl AsRef<Path>) -> anyhow::Result<PathBuf> {
        let path = path.as_ref();
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("map.unr").to_string();
        self.log(format!("Saving map to native 64-bit .UNR package (No UCC) at {:?}...", path));
        let start_time = std::time::Instant::now();
        match crate::compiler::CompilerBridge::save_unr_native(self, path) {
            Ok(saved_path) => {
                let duration = start_time.elapsed();
                self.map_path = Some(saved_path.clone());
                self.add_recent_map(&saved_path);
                self.map_is_dirty = false;
                self.log(format!("Native .UNR map successfully written to {:?} in {:.2}s", saved_path, duration.as_secs_f32()));
                let size = std::fs::metadata(&saved_path).map(|m| m.len()).ok();
                let size_str = size.map(|s| {
                    if s >= 1024 * 1024 {
                        format!("{:.2} MB", s as f64 / (1024.0 * 1024.0))
                    } else {
                        format!("{:.1} KB", s as f64 / 1024.0)
                    }
                }).unwrap_or_else(|| "Saved".to_string());
                self.toast_success(format!("Saved native map: {file_name} ({size_str})"));

                self.save_feedback = Some(SaveFeedbackState {
                    kind: SaveOperationKind::SaveUnrNative(saved_path.clone()),
                    filename: file_name,
                    target_path: saved_path.clone(),
                    current_stage: 2,
                    total_stages: 2,
                    stage_message: format!("Saved in {:.2}s ({})", duration.as_secs_f32(), size_str),
                    phase: SavePhase::Completed,
                    started_at: start_time,
                    completed_at: Some(std::time::Instant::now()),
                    file_size_bytes: size,
                    error_message: None,
                });

                Ok(saved_path)
            }
            Err(e) => {
                self.log(format!("Failed to save native UNR map: {e}"));
                self.toast_error(format!("Save failed: {e}"));
                self.save_feedback = Some(SaveFeedbackState {
                    kind: SaveOperationKind::SaveUnrNative(path.to_path_buf()),
                    filename: file_name,
                    target_path: path.to_path_buf(),
                    current_stage: 1,
                    total_stages: 2,
                    stage_message: format!("Save failed: {e}"),
                    phase: SavePhase::Failed,
                    started_at: start_time,
                    completed_at: Some(std::time::Instant::now()),
                    file_size_bytes: None,
                    error_message: Some(e.to_string()),
                });
                Err(e)
            }
        }
    }

    fn can_start_save_operation(&mut self) -> bool {
        if self.pending_save_op.is_some() || self.save_worker_rx.is_some() {
            self.toast_warning("A save or export is already in progress. Wait before starting another operation.");
            if self.pending_dirty_action.is_some() { self.show_unsaved_exit_modal = true; }
            return false;
        }
        true
    }

    /// Initiates a staged save with high-visibility UI progress feedback (Frame 0: overlay render, Frame 1: execution).
    pub fn trigger_save_unr(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() {
            return;
        }
        let path_buf = path.as_ref().to_path_buf();
        let filename = path_buf.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map.unr")
            .to_string();

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::SaveUnr(path_buf.clone()),
            filename,
            target_path: path_buf.clone(),
            current_stage: 1,
            total_stages: 4,
            stage_message: "Preparing and exporting T3D geometry...".to_string(),
            phase: SavePhase::PendingPreparation,
            started_at: std::time::Instant::now(),
            completed_at: None,
            file_size_bytes: None,
            error_message: None,
        });
        self.pending_save_op = Some(SaveOperationKind::SaveUnr(path_buf));
    }

    /// Initiates a staged native 64-bit .unr package save with UI feedback.
    pub fn trigger_save_unr_native(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() {
            return;
        }
        let path_buf = path.as_ref().to_path_buf();
        let filename = path_buf.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map.unr")
            .to_string();

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::SaveUnrNative(path_buf.clone()),
            filename,
            target_path: path_buf.clone(),
            current_stage: 1,
            total_stages: 2,
            stage_message: "Serializing native 64-bit .UNR tables and actors...".to_string(),
            phase: SavePhase::PendingPreparation,
            started_at: std::time::Instant::now(),
            completed_at: None,
            file_size_bytes: None,
            error_message: None,
        });
        self.pending_save_op = Some(SaveOperationKind::SaveUnrNative(path_buf));
    }

    /// Initiates a staged T3D document save with UI feedback.
    pub fn trigger_save_t3d(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() {
            return;
        }
        let path_buf = path.as_ref().to_path_buf();
        let filename = path_buf.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("map.t3d")
            .to_string();

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::SaveT3d(path_buf.clone()),
            filename,
            target_path: path_buf.clone(),
            current_stage: 1,
            total_stages: 2,
            stage_message: "Generating authoritative T3D text...".to_string(),
            phase: SavePhase::PendingPreparation,
            started_at: std::time::Instant::now(),
            completed_at: None,
            file_size_bytes: None,
            error_message: None,
        });
        self.pending_save_op = Some(SaveOperationKind::SaveT3d(path_buf));
    }

    /// Exports a T3D copy without changing the current document.
    pub fn trigger_export_t3d(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() { return; }
        self.trigger_save_t3d(path.as_ref());
        if let Some(feedback) = &mut self.save_feedback {
            feedback.kind = SaveOperationKind::ExportT3d(path.as_ref().to_path_buf());
        }
        self.pending_save_op = Some(SaveOperationKind::ExportT3d(path.as_ref().to_path_buf()));
    }

    /// Initiates a staged glTF 2.0 export with UI feedback.
    pub fn trigger_export_gltf(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() {
            return;
        }
        let path_buf = path.as_ref().to_path_buf();
        let filename = path_buf.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("scene.gltf")
            .to_string();

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::ExportGltf(path_buf.clone()),
            filename,
            target_path: path_buf.clone(),
            current_stage: 1,
            total_stages: 2,
            stage_message: "Generating glTF 2.0 scene graph...".to_string(),
            phase: SavePhase::PendingPreparation,
            started_at: std::time::Instant::now(),
            completed_at: None,
            file_size_bytes: None,
            error_message: None,
        });
        self.pending_save_op = Some(SaveOperationKind::ExportGltf(path_buf));
    }

    /// Initiates a staged Wavefront OBJ export with UI feedback.
    pub fn trigger_export_obj(&mut self, path: impl AsRef<Path>) {
        if !self.can_start_save_operation() {
            return;
        }
        let path_buf = path.as_ref().to_path_buf();
        let filename = path_buf.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("BuilderBrush.obj")
            .to_string();

        self.save_feedback = Some(SaveFeedbackState {
            kind: SaveOperationKind::ExportObj(path_buf.clone()),
            filename,
            target_path: path_buf.clone(),
            current_stage: 1,
            total_stages: 2,
            stage_message: "Generating Wavefront OBJ vertices and faces...".to_string(),
            phase: SavePhase::PendingPreparation,
            started_at: std::time::Instant::now(),
            completed_at: None,
            file_size_bytes: None,
            error_message: None,
        });
        self.pending_save_op = Some(SaveOperationKind::ExportObj(path_buf));
    }

    /// Progresses pending save operations across frames, ensuring the UI has presented the
    /// active progress spinner before executing heavy serialization.
    pub fn process_pending_save_operation(&mut self, ctx: &egui::Context) {
        // 1. Auto-dismiss completed or failed feedback after 4.5 seconds
        if let Some(ref feedback) = self.save_feedback {
            if feedback.phase == SavePhase::Completed || feedback.phase == SavePhase::Failed {
                if let Some(completed_time) = feedback.completed_at {
                    if completed_time.elapsed().as_secs_f32() > 4.5 {
                        self.save_feedback = None;
                    }
                }
            }
        }

        // 2. Handle active state machine transitions
        let current_phase = self.save_feedback.as_ref().map(|fb| fb.phase);
        match current_phase {
            Some(SavePhase::PendingPreparation) => {
                // Frame 0 rendered the modal. Now advance to Executing and trigger repaint for Frame 1!
                if let Some(ref mut fb) = self.save_feedback {
                    fb.phase = SavePhase::Executing;
                    fb.stage_message = match fb.kind {
                        SaveOperationKind::SaveUnr(_) => "Preparing and exporting T3D geometry...".to_string(),
                        SaveOperationKind::SaveUnrNative(_) => "Serializing native 64-bit tables and actors...".to_string(),
                        SaveOperationKind::SaveT3d(_) | SaveOperationKind::ExportT3d(_) => "Generating authoritative T3D text...".to_string(),
                        SaveOperationKind::ExportGltf(_) => "Generating glTF 2.0 scene graph...".to_string(),
                        SaveOperationKind::ExportObj(_) => "Generating 3D OBJ model...".to_string(),
                    };
                }
                ctx.request_repaint();
            }
            Some(SavePhase::Executing) => {
                if self.save_worker_rx.is_none() {
                    if let Some(op) = self.pending_save_op.take() {
                        let (tx, rx) = std::sync::mpsc::channel();
                        self.save_worker_rx = Some(rx);
                        let start_time = std::time::Instant::now();

                        match op {
                            SaveOperationKind::SaveUnr(p) => {
                                let ut_dir = crate::compiler::CompilerBridge::find_ut_dir();
                                let force_native = std::env::var("UT99_NATIVE_SAVE").map_or(false, |v| v == "1");
                                let has_embedded = crate::compiler::CompilerBridge::has_embedded_assets(self);
                                let snapshot = crate::compiler::UnrExportSnapshot::capture(self);

                                std::thread::spawn(move || {
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 1,
                                        total_stages: 4,
                                        stage_message: "Preparing and exporting T3D geometry...".to_string(),
                                    });

                                    if p.exists() {
                                        let bak_path = p.with_extension("unr.bak");
                                        let _ = std::fs::copy(&p, &bak_path);
                                    }

                                    let mut saved = false;
                                    let mut last_err = String::new();

                                    if !force_native && !has_embedded {
                                        if let Some(ref ut) = ut_dir {
                                            match crate::compiler::CompilerBridge::export_prepared_t3d_from_snapshot(&snapshot.t3d) {
                                                Ok((ref buf, ref t3d_text)) => {
                                                    match crate::compiler::CompilerBridge::build_unr_with_engine_from_t3d(
                                                        ut, &p, buf, t3d_text, Some(&tx),
                                                    ) {
                                                        Ok(_) => {
                                                            saved = true;
                                                        }
                                                        Err(e) => {
                                                            last_err = format!("Engine build failed ({e}); falling back to native Rust .UNR writer");
                                                            tracing::warn!("{last_err}");
                                                        }
                                                    }
                                                }
                                                Err(e) => {
                                                    last_err = format!("T3D export failed ({e}); falling back to native Rust .UNR writer");
                                                    tracing::warn!("{last_err}");
                                                }
                                            }
                                        }
                                    }

                                    if !saved {
                                        let _ = tx.send(SaveWorkerMessage::Stage {
                                            current_stage: 3,
                                            total_stages: 4,
                                            stage_message: "Writing with native 64-bit .UNR serializer...".to_string(),
                                        });
                                        match crate::compiler::CompilerBridge::build_unr_level_from_snapshot(&snapshot, &p) {
                                            Ok(level) => {
                                                if let Some(parent) = p.parent() {
                                                    let _ = std::fs::create_dir_all(parent);
                                                }
                                                match level.save(&p) {
                                                    Ok(_) => { saved = true; }
                                                    Err(e) => { last_err = format!("Native save error: {e}"); }
                                                }
                                            }
                                            Err(e) => {
                                                last_err = format!("Failed to build native level: {e}");
                                            }
                                        }
                                    }

                                    if saved {
                                        let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                                        let _ = tx.send(SaveWorkerMessage::Success {
                                            saved_path: p,
                                            file_size_bytes: size,
                                            duration: start_time.elapsed(),
                                        });
                                    } else {
                                        let _ = tx.send(SaveWorkerMessage::Error { message: last_err });
                                    }
                                });
                            }
                            SaveOperationKind::SaveUnrNative(p) => {
                                let snapshot = crate::compiler::UnrExportSnapshot::capture(self);
                                std::thread::spawn(move || {
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 1,
                                        total_stages: 2,
                                        stage_message: "Serializing native 64-bit .UNR...".to_string(),
                                    });
                                    if p.exists() {
                                        let bak_path = p.with_extension("unr.bak");
                                        let _ = std::fs::copy(&p, &bak_path);
                                    }
                                    match crate::compiler::CompilerBridge::build_unr_level_from_snapshot(&snapshot, &p) {
                                        Ok(level) => {
                                            if let Some(parent) = p.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            let _ = tx.send(SaveWorkerMessage::Stage {
                                                current_stage: 2,
                                                total_stages: 2,
                                                stage_message: "Writing .UNR package to disk...".to_string(),
                                            });
                                            match level.save(&p) {
                                                Ok(_) => {
                                                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
                                                    let _ = tx.send(SaveWorkerMessage::Success {
                                                        saved_path: p,
                                                        file_size_bytes: size,
                                                        duration: start_time.elapsed(),
                                                    });
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                        }
                                    }
                                });
                            }
                            SaveOperationKind::SaveT3d(p) | SaveOperationKind::ExportT3d(p) => {
                                let snapshot = crate::compiler::T3dExportSnapshot::capture(self);
                                std::thread::spawn(move || {
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 1,
                                        total_stages: 2,
                                        stage_message: "Generating authoritative T3D text...".to_string(),
                                    });
                                    match crate::compiler::CompilerBridge::export_prepared_t3d_from_snapshot(&snapshot) {
                                        Ok((buf, _)) => {
                                            let _ = tx.send(SaveWorkerMessage::Stage {
                                                current_stage: 2,
                                                total_stages: 2,
                                                stage_message: "Writing T3D file to disk...".to_string(),
                                            });
                                            if let Some(parent) = p.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }
                                            match std::fs::write(&p, &buf) {
                                                Ok(_) => {
                                                    let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(buf.len() as u64);
                                                    let _ = tx.send(SaveWorkerMessage::Success {
                                                        saved_path: p,
                                                        file_size_bytes: size,
                                                        duration: start_time.elapsed(),
                                                    });
                                                }
                                                Err(e) => {
                                                    let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                        }
                                    }
                                });
                            }
                            SaveOperationKind::ExportGltf(p) => {
                                let snapshot = crate::compiler::T3dExportSnapshot::capture(self);
                                std::thread::spawn(move || {
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 1,
                                        total_stages: 2,
                                        stage_message: "Generating glTF 2.0 scene graph...".to_string(),
                                    });
                                    let json = EditorState::export_map_to_gltf_from_snapshot(&snapshot);
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 2,
                                        total_stages: 2,
                                        stage_message: "Writing glTF file to disk...".to_string(),
                                    });
                                    if let Some(parent) = p.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    match std::fs::write(&p, json.as_bytes()) {
                                        Ok(_) => {
                                            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(json.len() as u64);
                                            let _ = tx.send(SaveWorkerMessage::Success {
                                                saved_path: p,
                                                file_size_bytes: size,
                                                duration: start_time.elapsed(),
                                            });
                                        }
                                        Err(e) => {
                                            let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                        }
                                    }
                                });
                            }
                            SaveOperationKind::ExportObj(p) => {
                                let obj = self.export_builder_brush_to_obj();
                                std::thread::spawn(move || {
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 1,
                                        total_stages: 2,
                                        stage_message: "Generating Wavefront OBJ vertices and faces...".to_string(),
                                    });
                                    let _ = tx.send(SaveWorkerMessage::Stage {
                                        current_stage: 2,
                                        total_stages: 2,
                                        stage_message: "Writing Wavefront OBJ file to disk...".to_string(),
                                    });
                                    if let Some(parent) = p.parent() {
                                        let _ = std::fs::create_dir_all(parent);
                                    }
                                    match std::fs::write(&p, obj.as_bytes()) {
                                        Ok(_) => {
                                            let size = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(obj.len() as u64);
                                            let _ = tx.send(SaveWorkerMessage::Success {
                                                saved_path: p,
                                                file_size_bytes: size,
                                                duration: start_time.elapsed(),
                                            });
                                        }
                                        Err(e) => {
                                            let _ = tx.send(SaveWorkerMessage::Error { message: e.to_string() });
                                        }
                                    }
                                });
                            }
                        }
                    }
                }

                // Poll messages from background worker
                let is_test_environment = cfg!(test)
                    || std::thread::current().name().map_or(false, |n| n.contains("test"))
                    || std::env::var("RUST_TEST_NOCAPTURE").is_ok();

                if is_test_environment {
                    let mut msgs = Vec::new();
                    if let Some(ref rx) = self.save_worker_rx {
                        while let Ok(msg) = rx.recv_timeout(std::time::Duration::from_secs(5)) {
                            let is_terminal = matches!(msg, SaveWorkerMessage::Success { .. } | SaveWorkerMessage::Error { .. });
                            msgs.push(msg);
                            if is_terminal {
                                break;
                            }
                        }
                    }
                    for msg in msgs {
                        self.handle_save_worker_message(msg);
                    }
                } else {
                    let mut messages = Vec::new();
                    if let Some(ref rx) = self.save_worker_rx {
                        while let Ok(msg) = rx.try_recv() {
                            messages.push(msg);
                        }
                    }
                    for msg in messages {
                        self.handle_save_worker_message(msg);
                    }
                    if self.save_worker_rx.is_some() {
                        ctx.request_repaint();
                    }
                }
            }
            Some(SavePhase::Completed) | Some(SavePhase::Failed) => {
                // Request repaint periodically until dismiss timeout expires
                ctx.request_repaint_after(std::time::Duration::from_millis(150));
            }
            None => {}
        }
    }

    /// Handles progress and completion events emitted by the save worker thread.
    pub fn handle_save_worker_message(&mut self, msg: SaveWorkerMessage) {
        match msg {
            SaveWorkerMessage::Stage { current_stage, total_stages, stage_message } => {
                if let Some(ref mut fb) = self.save_feedback {
                    fb.current_stage = current_stage;
                    fb.total_stages = total_stages;
                    fb.stage_message = stage_message;
                }
            }
            SaveWorkerMessage::Success { saved_path, file_size_bytes, duration } => {
                // An unclassified result is not evidence that the document was saved.
                let saves_document = self.save_feedback.as_ref().is_some_and(|fb| fb.kind.saves_document());
                if saves_document {
                    self.map_path = Some(saved_path.clone());
                    self.add_recent_map(&saved_path);
                    self.map_is_dirty = false;
                }
                let verb = if saves_document { "Saved" } else { "Exported" };
                let size_str = if file_size_bytes >= 1024 * 1024 {
                    format!("{:.2} MB", file_size_bytes as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} KB", file_size_bytes as f64 / 1024.0)
                };
                let file_name = saved_path.file_name().and_then(|n| n.to_str()).unwrap_or("output").to_string();
                self.log(format!("{verb} to {:?} in {:.2}s ({size_str})", saved_path, duration.as_secs_f32()));
                self.toast_success(format!("{verb}: {file_name} ({size_str})"));
                if let Some(ref mut fb) = self.save_feedback {
                    fb.phase = SavePhase::Completed;
                    fb.target_path = saved_path;
                    fb.completed_at = Some(std::time::Instant::now());
                    fb.file_size_bytes = Some(file_size_bytes);
                    fb.stage_message = format!("{verb} in {:.2}s ({size_str})", duration.as_secs_f32());
                }
                self.save_worker_rx = None;
                if self.pending_dirty_action.is_some() {
                    if saves_document { self.execute_pending_dirty_action(); }
                    else { self.show_unsaved_exit_modal = true; }
                }
            }
            SaveWorkerMessage::Error { message } => {
                let saves_document = self.save_feedback.as_ref().is_some_and(|fb| fb.kind.saves_document());
                let operation = if saves_document { "Save" } else { "Export" };
                let error = format!("{operation} failed: {message}");
                self.log(&error);
                self.toast_error(&error);
                if let Some(ref mut fb) = self.save_feedback {
                    fb.phase = SavePhase::Failed;
                    fb.completed_at = Some(std::time::Instant::now());
                    fb.error_message = Some(message);
                    fb.stage_message = error;
                }
                self.save_worker_rx = None;
                if saves_document { self.pending_dirty_action = None; }
                else if self.pending_dirty_action.is_some() { self.show_unsaved_exit_modal = true; }
            }
        }
    }

    /// Returns the canonical default directory for UT99 maps (`UnrealTournament/Maps`),
    /// prioritizing active installation paths found by CompilerBridge.
    pub fn default_maps_directory(&self) -> PathBuf {
        if let Some(ref dir) = self.default_export_dir {
            if !dir.as_os_str().is_empty() {
                let _ = std::fs::create_dir_all(dir);
                return dir.clone();
            }
        }
        if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
            let maps = ut_dir.join("Maps");
            if maps.exists() {
                return maps;
            }
        }
        if let Ok(cur) = std::env::current_dir() {
            let rel_maps = cur.join("UnrealTournament").join("Maps");
            if rel_maps.exists() {
                return rel_maps;
            }
            let cur_maps = cur.join("Maps");
            if cur_maps.exists() {
                return cur_maps;
            }
            return cur;
        }
        PathBuf::from("UnrealTournament/Maps")
    }

    /// Destination for wizard-generated maps. Deliberately separate from the UT `Maps` folder so
    /// generated files never mix with the stock maps used as reference material.
    pub fn wizard_output_directory(&self) -> PathBuf {
        let dir = match self.wizard_output_dir.as_ref().filter(|d| !d.as_os_str().is_empty()) {
            Some(dir) => dir.clone(),
            None => ["USERPROFILE", "HOME"]
                .iter()
                .find_map(|var| std::env::var_os(var).filter(|v| !v.is_empty()))
                .map(|home| PathBuf::from(home).join("Documents").join("Forge99").join("GeneratedMaps"))
                .unwrap_or_else(|| PathBuf::from("GeneratedMaps")),
        };
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    /// Copies a generated map (and its generator manifest) into the UT `Maps` folder for playtesting.
    /// An existing file is replaced only if it is itself a wizard-generated map.
    pub fn install_wizard_map_in_ut(&self, saved_path: &Path) -> Result<PathBuf, String> {
        let ut_dir = crate::compiler::CompilerBridge::find_ut_dir()
            .ok_or_else(|| "UT99 installation not found".to_string())?;
        let maps = ut_dir.join("Maps");
        let name = saved_path.file_name().ok_or_else(|| "invalid map path".to_string())?;
        let dest = maps.join(name);
        if dest == saved_path {
            return Ok(dest);
        }
        if dest.exists() && !ut99_format::unr::has_generator_manifest(&dest) {
            return Err(format!("{} already exists and is not a wizard-generated map; rename the map to install it", dest.display()));
        }
        std::fs::create_dir_all(&maps).map_err(|e| e.to_string())?;
        std::fs::copy(saved_path, &dest).map_err(|e| format!("copy failed: {e}"))?;
        let manifest = ut99_format::unr::manifest_path_for(saved_path);
        if manifest.exists() {
            let _ = std::fs::copy(&manifest, ut99_format::unr::manifest_path_for(&dest));
        }
        Ok(dest)
    }

    /// Reveals a target file or folder directly inside the OS File Explorer (Windows Explorer / Finder).
    pub fn reveal_in_file_explorer(&self, path: impl AsRef<Path>) {
        reveal_in_file_explorer(path.as_ref());
    }

    /// Exports a loaded texture package (e.g. `"MyLevel"` or any custom package) to a standalone binary `.utx` file.
    pub fn export_texture_package_utx(&mut self, pkg_name: &str, path: impl AsRef<Path>) -> anyhow::Result<usize> {
        let path = path.as_ref();
        let count = self.texture_manager.save_package_to_utx(pkg_name, path)?;
        self.log(format!("Exported texture package '{}' ({} textures) to {:?}", pkg_name, count, path));
        self.toast_success(format!("Saved .utx: {} ({} textures)", pkg_name, count));
        Ok(count)
    }

    /// Opens a standalone binary `.utx` file from disk and loads it into `TextureManager`.
    pub fn open_texture_package_utx(&mut self, path: impl AsRef<Path>) -> anyhow::Result<String> {
        let path = path.as_ref();
        let pkg_name = self.texture_manager.open_package_from_path(path)?;
        self.selected_texture_package = pkg_name.clone();
        self.log(format!("Opened .utx texture package '{}' from {:?}", pkg_name, path));
        self.toast_success(format!("Opened .utx: {}", pkg_name));
        Ok(pkg_name)
    }

}


impl EditorState {
    pub fn export_map_to_gltf(&self) -> String {
        let snapshot = crate::compiler::T3dExportSnapshot::capture(self);
        Self::export_map_to_gltf_from_snapshot(&snapshot)
    }

    pub fn export_map_to_gltf_from_snapshot(snap: &crate::compiler::T3dExportSnapshot) -> String {
        let movers = crate::editor_state::gameplay_analysis::compute_mover_trajectories_from_slice(&snap.movers);
        let mut actor_tuples = Vec::new();
        if let Some(ref actors) = snap.loaded_unr_actors {
            for a in actors.iter() {
                actor_tuples.push((a.name.clone(), a.class_name.clone(), a.location));
            }
        }
        export_map_to_gltf_json(&snap.map_name_stem, &snap.world_brushes, &movers, &actor_tuples)
    }


    pub fn export_map_to_gltf_file(&self, path: &Path) -> std::io::Result<()> {
        let json = self.export_map_to_gltf();
        std::fs::write(path, json)?;
        Ok(())
    }


    pub fn generate_map_manifest(&self) -> MapManifest {
        let map_name = self.map_path.as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "UntitledMap".to_string());

        let mut manifest = if let Some(ref level) = self.loaded_unr {
            scan_map_dependencies(level)
        } else {
            let mut m = MapManifest::default();
            m.stock_dependencies.push(PackageDependency {
                name: "Botpack".to_string(),
                package_type: "Code".to_string(),
                is_stock: true,
            });
            m.stock_dependencies.push(PackageDependency {
                name: "Core".to_string(),
                package_type: "System".to_string(),
                is_stock: true,
            });
            m.stock_dependencies.push(PackageDependency {
                name: "Engine".to_string(),
                package_type: "System".to_string(),
                is_stock: true,
            });
            m
        };

        manifest.map_name = map_name;
        if manifest.title.is_empty() || manifest.title == "UntitledMap" {
            manifest.title = if !self.level_title.is_empty() && self.level_title != "Untitled Arena" {
                self.level_title.clone()
            } else {
                manifest.map_name.clone()
            };
        }
        if manifest.author.is_empty() {
            manifest.author = self.level_author.clone();
        }
        if manifest.ideal_player_count.is_empty() {
            manifest.ideal_player_count = self.level_ideal_player_count.clone();
        }
        if manifest.song.is_empty() {
            manifest.song = self.level_song.clone();
        }

        // Scan world brushes for textures
        for brush in &self.world_brushes {
            for poly in &brush.polys {
                if !poly.texture.is_empty() {
                    if let Some(dot_idx) = poly.texture.find('.') {
                        let pkg = &poly.texture[..dot_idx];
                        let is_stock = STOCK_PACKAGES.contains(&pkg.to_lowercase().as_str());
                        let dep = PackageDependency {
                            name: pkg.to_string(),
                            package_type: "Texture".to_string(),
                            is_stock,
                        };
                        if is_stock {
                            if !manifest.stock_dependencies.contains(&dep) {
                                manifest.stock_dependencies.push(dep);
                            }
                        } else if !manifest.custom_dependencies.contains(&dep) {
                            manifest.custom_dependencies.push(dep);
                        }
                    }
                }
            }
        }

        // Scan movers for textures
        for mover in &self.movers {
            for poly in &mover.brush.polys {
                if !poly.texture.is_empty() {
                    if let Some(dot_idx) = poly.texture.find('.') {
                        let pkg = &poly.texture[..dot_idx];
                        let is_stock = STOCK_PACKAGES.contains(&pkg.to_lowercase().as_str());
                        let dep = PackageDependency {
                            name: pkg.to_string(),
                            package_type: "Texture".to_string(),
                            is_stock,
                        };
                        if is_stock {
                            if !manifest.stock_dependencies.contains(&dep) {
                                manifest.stock_dependencies.push(dep);
                            }
                        } else if !manifest.custom_dependencies.contains(&dep) {
                            manifest.custom_dependencies.push(dep);
                        }
                    }
                }
            }
        }

        // Scan music package if present
        if !self.level_song.is_empty() {
            let song_pkg = if let Some(dot_idx) = self.level_song.find('.') {
                &self.level_song[..dot_idx]
            } else {
                &self.level_song
            };
            let is_stock = STOCK_PACKAGES.contains(&song_pkg.to_lowercase().as_str());
            let dep = PackageDependency {
                name: song_pkg.to_string(),
                package_type: "Music".to_string(),
                is_stock,
            };
            if is_stock {
                if !manifest.stock_dependencies.contains(&dep) {
                    manifest.stock_dependencies.push(dep);
                }
            } else if !manifest.custom_dependencies.contains(&dep) {
                manifest.custom_dependencies.push(dep);
            }
        }

        // Total actors count
        let total_actors = if let Some(ref level) = self.loaded_unr {
            level.actors.len()
        } else {
            self.map.actors.len()
        };
        manifest.total_actors = total_actors;

        manifest.stock_dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        manifest.custom_dependencies.sort_by(|a, b| a.name.cmp(&b.name));
        manifest
    }


    pub fn refresh_map_manifest(&mut self) {
        let manifest = self.generate_map_manifest();
        self.cached_map_manifest = Some(manifest);
    }


    pub fn export_manifest_readme(&self, path: &std::path::Path) -> std::io::Result<()> {
        let manifest = self.cached_map_manifest.clone().unwrap_or_else(|| self.generate_map_manifest());
        let readme = manifest.generate_readme();
        std::fs::write(path, readme)
    }


    pub fn get_manifest_readme_text(&self) -> String {
        let manifest = self.cached_map_manifest.clone().unwrap_or_else(|| self.generate_map_manifest());
        manifest.generate_readme()
    }


    pub fn resolved_playtest_executable(&self) -> Result<PathBuf, String> {
        crate::playtest::resolve_executable(&self.ut99_executable_path)
    }

    fn prepare_playtest(&self) -> Result<crate::playtest::PreparedLaunch, String> {
        if self.pending_save_op.is_some() || self.save_worker_rx.is_some() {
            return Err("A save or export is already in progress; wait before starting a playtest.".into());
        }
        crate::playtest::PreparedLaunch::new(&self.ut99_executable_path, &self.ut99_launch_args)
    }

    /// Check configuration before a wizard generation/save operation.
    pub fn preflight_playtest(&mut self) -> Result<(), String> {
        let first_toast = self.next_toast_id;
        let result = self.prepare_playtest().map(|_| ());
        self.finish_playtest_result(result, first_toast)
    }

    pub fn play_map_in_ut99(&mut self) -> Result<(), String> {
        let first_toast = self.next_toast_id;
        let result = (|| {
            let launch = self.prepare_playtest()?;
            if self.map_path.as_ref().is_some_and(|path| !path.extension().and_then(|e| e.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("unr"))) {
                return Err("Save the current document as .unr before playtesting; the source file will not be overwritten.".into());
            }
            // Only a valid launch may update installation state or choose/create an output folder.
            self.sync_audio_installation();
            let target = match &self.map_path {
                Some(path) => crate::playtest::absolute_path(path)?,
                None => {
                    let directory = self.default_maps_directory();
                    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| e.to_string())?.as_nanos();
                    directory.join(format!("Forge99Playtest-{}-{nanos}.unr", std::process::id()))
                }
            };
            self.log("Building map for UT99 playtest...");
            let saved = self.save_unr(&target).map_err(|e| format!("Playtest build/save failed: {e}"))?;
            launch.spawn(&saved).map_err(|e| format!("Map saved to '{}', but playtest could not start: {e}", saved.display()))?;
            self.playtest_started(&saved);
            Ok(())
        })();
        self.finish_playtest_result(result, first_toast)
    }

    pub fn launch_saved_map_in_ut99(&mut self, saved_path: &Path) -> Result<(), String> {
        let first_toast = self.next_toast_id;
        let result = self.prepare_playtest().and_then(|launch| launch.spawn(saved_path));
        if result.is_ok() { self.playtest_started(saved_path); }
        self.finish_playtest_result(result, first_toast)
    }

    fn playtest_started(&mut self, saved_path: &Path) {
        let message = format!("UT99 process started for '{}'.", saved_path.display());
        self.log(&message);
        self.toast_success(message);
    }

    fn finish_playtest_result(&mut self, result: Result<(), String>, first_toast: u64) -> Result<(), String> {
        if let Err(message) = &result {
            self.log(message);
            // Save errors already have a toast. Add playtest context without duplicating it.
            if let Some(toast) = self.toasts.last_mut().filter(|toast| toast.id >= first_toast && matches!(toast.kind, ToastKind::Error)) {
                toast.message = message.clone();
            } else { self.toast_error(message); }
        }
        result
    }

}

/// Normalizes a filesystem path for Windows Explorer:
/// 1. Resolves relative paths against current working directory or canonical path if existing.
/// 2. Strips the Win32 verbatim namespaces (`\\?\` and `\\?\UNC\`) which Windows Explorer (`explorer.exe`) rejects.
/// 3. Normalizes forward slashes `/` to Windows backslashes `\`.
pub fn normalize_path_for_windows_explorer(path: &Path) -> (PathBuf, String) {
    let raw_str = path.to_string_lossy();
    let is_windows_absolute = raw_str.starts_with(r"\\?\")
        || raw_str.starts_with(r"\\")
        || (raw_str.len() >= 3
            && raw_str.as_bytes()[0].is_ascii_alphabetic()
            && raw_str.as_bytes()[1] == b':'
            && matches!(raw_str.as_bytes()[2], b'\\' | b'/'));
    let abs_path = if let Ok(canon) = path.canonicalize() {
        canon
    } else if path.is_absolute() || is_windows_absolute {
        path.to_path_buf()
    } else if let Ok(curr) = std::env::current_dir() {
        curr.join(path)
    } else {
        path.to_path_buf()
    };

    let p_str = abs_path.to_string_lossy().to_string();
    let stripped = if let Some(unc) = p_str.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else if let Some(regular) = p_str.strip_prefix(r"\\?\") {
        regular.to_string()
    } else {
        p_str
    };

    let normalized_str = stripped.replace('/', "\\");
    let normalized_buf = PathBuf::from(&normalized_str);
    (normalized_buf, normalized_str)
}

/// Opens the host OS file explorer (Windows Explorer, macOS Finder, or Linux file manager)
/// with the given file selected, or opening the directory.
pub fn reveal_in_file_explorer(path: &Path) {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let (abs_path, p_str) = normalize_path_for_windows_explorer(path);

        if abs_path.is_dir() {
            // Path is an existing directory: open the folder directly in Explorer without /select
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!("\"{p_str}\""));
            let _ = cmd.spawn();
        } else if abs_path.is_file() || abs_path.exists() {
            // Path is an existing file: open its parent folder and select the file
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!("/select,\"{p_str}\""));
            let _ = cmd.spawn();
        } else if let Some(parent) = abs_path.parent() {
            // Path doesn't exist yet: open its parent directory
            let parent_str = parent.to_string_lossy().replace('/', "\\");
            let mut cmd = std::process::Command::new("explorer");
            cmd.raw_arg(format!("\"{parent_str}\""));
            let _ = cmd.spawn();
        }
    }
    #[cfg(target_os = "macos")]
    {
        if path.is_file() {
            let _ = std::process::Command::new("open")
                .arg("-R")
                .arg(path)
                .spawn();
        } else {
            let target = if path.exists() { path } else { path.parent().unwrap_or(path) };
            let _ = std::process::Command::new("open")
                .arg(target)
                .spawn();
        }
    }
    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    {
        let target = if path.is_dir() {
            path
        } else if let Some(parent) = path.parent() {
            parent
        } else {
            path
        };
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
