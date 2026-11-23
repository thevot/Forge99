use super::*;
use std::path::Path;

impl EditorState {
    pub fn stop_audio_preview(&mut self) {
        let preview_wav = std::env::temp_dir().join("forge99_sound_preview.wav");
        let _ = std::fs::remove_file(&preview_wav);
        self.log("Audio Engine: Stopped active sound/music preview.");
    }

    /// Deletes a sound by name from the active sound package (and `MyLevel` if applicable) (`BL-P1-06`).
    pub fn delete_sound_from_package(&mut self, sound_name: &str) -> bool {
        if let Some(imports) = self.audio_catalog.imports.get_mut(&self.selected_sound_package.to_ascii_lowercase()) {
            imports.retain(|s| !s.name.eq_ignore_ascii_case(sound_name));
        }
        let before_loaded = self.loaded_sounds.len();
        self.loaded_sounds.retain(|s| !s.name.eq_ignore_ascii_case(sound_name));
        let removed_loaded = self.loaded_sounds.len() < before_loaded;

        let before_mylevel = self.mylevel_sounds.len();
        self.mylevel_sounds.retain(|s| !s.name.eq_ignore_ascii_case(sound_name));
        let removed_mylevel = self.mylevel_sounds.len() < before_mylevel;

        if let Some(ref mut unr) = self.loaded_unr {
            unr.embedded_sounds.retain(|s| !s.name.eq_ignore_ascii_case(sound_name));
        }
        if self.selected_sound_name.as_deref().map_or(false, |n| n.eq_ignore_ascii_case(sound_name)) {
            self.selected_sound_name = self.loaded_sounds.first().map(|s| s.name.clone());
        }
        if removed_loaded || removed_mylevel {
            self.log(format!(
                "Sound Browser: Deleted sound '{}' from package '{}'.",
                sound_name, self.selected_sound_package
            ));
            true
        } else {
            false
        }
    }

    /// Places an `AmbientSound` actor at the Red Builder Brush with the specified `.uax` sound reference (`BG-11` / `H.1`).
    pub fn place_ambient_sound_at_builder(&mut self, full_sound_name: &str) {
        let loc = self.builder_brush.location;
        self.place_actor_of_class_at("AmbientSound", loc);
        if let Some(a_idx) = self.selected_actor_index {
            if let Some(ref mut unr) = self.loaded_unr {
                if let Some(actor) = unr.actors.get_mut(a_idx) {
                    let formatted = if full_sound_name.starts_with("Sound'") {
                        full_sound_name.to_string()
                    } else {
                        format!("Sound'{}'", full_sound_name)
                    };
                    actor.properties.insert(
                        "AmbientSound".to_string(),
                        ut99_format::TaggedProperty {
                            name: "AmbientSound".to_string(),
                            array_index: 0,
                            value: ut99_format::TaggedValue::Str(formatted),
                        },
                    );
                    actor.properties.entry("SoundRadius".to_string()).or_insert_with(|| ut99_format::TaggedProperty {
                        name: "SoundRadius".to_string(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Byte(64),
                    });
                    actor.properties.entry("SoundVolume".to_string()).or_insert_with(|| ut99_format::TaggedProperty {
                        name: "SoundVolume".to_string(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Byte(190),
                    });
                    actor.properties.entry("SoundPitch".to_string()).or_insert_with(|| ut99_format::TaggedProperty {
                        name: "SoundPitch".to_string(),
                        array_index: 0,
                        value: ut99_format::TaggedValue::Byte(64),
                    });
                }
            }
        }
        self.log(format!(
            "Placed AmbientSound with '{}' at ({:.0}, {:.0}, {:.0}).",
            full_sound_name, loc.x, loc.y, loc.z
        ));
    }


    pub fn set_level_song(&mut self, song_pkg: &str) {
        let clean = song_pkg.trim().trim_end_matches(".umx").to_string();
        let section = self.level_song_section;
        self.apply_level_music_track(&clean, section);
    }

    // --- UnrealScript Class Source Viewer & DefaultProperties Studio ---

    // --- 3D Acoustic & Ambient Sound Studio (Attenuation & Occlusion Profiler) ---

    /// Collect all sound emitter profiles (AmbientSound, weapons, movers with sound, etc.)
    pub fn collect_sound_emitters(&self) -> Vec<SoundEmitterProfile> {
        let mut emitters = Vec::new();

        let check_actor = |a: &ut99_format::ActorRecord, emitters: &mut Vec<SoundEmitterProfile>| {
            let has_ambient = a.class_name.eq_ignore_ascii_case("AmbientSound")
                || a.properties.contains_key("AmbientSound")
                || a.properties.contains_key("SoundRadius")
                || a.properties.contains_key("SoundVolume");

            if has_ambient {
                let sound_name = a.properties.get("AmbientSound").map(|p| match &p.value {
                    ut99_format::TaggedValue::Name(s) | ut99_format::TaggedValue::Str(s) => s.clone(),
                    _ => String::new(),
                }).unwrap_or_else(|| {
                    if a.class_name.eq_ignore_ascii_case("AmbientSound") {
                        "AmbAncient.Loop.Hum".to_string()
                    } else {
                        "DefaultSound".to_string()
                    }
                });

                let sound_radius_byte = a.properties.get("SoundRadius").map(|p| match p.value {
                    ut99_format::TaggedValue::Byte(b) => b,
                    ut99_format::TaggedValue::Int(i) => i.clamp(0, 255) as u8,
                    _ => 32,
                }).unwrap_or(if a.class_name.eq_ignore_ascii_case("AmbientSound") { 32 } else { 16 });

                let sound_volume_byte = a.properties.get("SoundVolume").map(|p| match p.value {
                    ut99_format::TaggedValue::Byte(b) => b,
                    ut99_format::TaggedValue::Int(i) => i.clamp(0, 255) as u8,
                    _ => 128,
                }).unwrap_or(128);

                let sound_pitch_byte = a.properties.get("SoundPitch").map(|p| match p.value {
                    ut99_format::TaggedValue::Byte(b) => b,
                    ut99_format::TaggedValue::Int(i) => i.clamp(0, 255) as u8,
                    _ => 64,
                }).unwrap_or(64);

                let sound_radius_units = sound_radius_byte as f32 * 40.0;
                let sound_volume_pct = sound_volume_byte as f32 / 255.0;
                let sound_pitch_ratio = sound_pitch_byte as f32 / 64.0;

                emitters.push(SoundEmitterProfile {
                    actor_tag: if !a.tag.is_empty() { a.tag.clone() } else { a.name.clone() },
                    actor_name: a.name.clone(),
                    class_name: a.class_name.clone(),
                    location: a.location,
                    sound_name,
                    sound_radius_units,
                    sound_radius_byte,
                    sound_volume_byte,
                    sound_pitch_byte,
                    sound_volume_pct,
                    sound_pitch_ratio,
                    is_ambient: a.class_name.eq_ignore_ascii_case("AmbientSound") || a.properties.contains_key("AmbientSound"),
                });
            }
        };

        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                check_actor(a, &mut emitters);
            }
        }

        emitters
    }

    /// Compute acoustic line-of-sight occlusion and attenuation between an emitter and listener
    pub fn compute_acoustic_occlusion_for_emitter(&self, emitter_tag: &str, listener_pos: glam::Vec3) -> Option<AcousticOcclusionResult> {
        let emitters = self.collect_sound_emitters();
        let emitter = emitters.iter().find(|e| e.actor_tag == emitter_tag || e.actor_name == emitter_tag)?;

        let delta = listener_pos - emitter.location;
        let distance = delta.length();
        let is_in_range = distance <= emitter.sound_radius_units && emitter.sound_radius_units > 0.0;

        // Direct distance attenuation (linear volume ratio)
        let linear_ratio = if is_in_range {
            (1.0 - distance / emitter.sound_radius_units).clamp(0.0, 1.0) * emitter.sound_volume_pct
        } else {
            0.0
        };

        let direct_volume_db = if linear_ratio > 1e-4 {
            20.0 * linear_ratio.log10()
        } else {
            -96.0 // digital silence threshold
        };

        // Raycast line of sight between emitter and listener across world brushes
        let mut walls_occluding_count = 0;
        if distance > 1.0 {
            let ray = crate::raycast::Ray::new(emitter.location, delta);
            let max_t = distance - 4.0;
            if max_t > 4.0 {
                let pad = glam::Vec3::splat(1e-3);
                let seg_min = emitter.location.min(listener_pos) - pad;
                let seg_max = emitter.location.max(listener_pos) + pad;
                for brush in &self.world_brushes {
                    let mut b_min = glam::Vec3::splat(f32::MAX);
                    let mut b_max = glam::Vec3::splat(f32::MIN);
                    let mut has_verts = false;
                    for poly in &brush.polys {
                        if (poly.flags
                            & (ut99_bsp::PF_NOT_SOLID
                                | ut99_bsp::PF_PORTAL
                                | ut99_bsp::PF_INVISIBLE))
                            != 0
                            || poly.vertices.len() < 3
                        {
                            continue;
                        }
                        for &v in &poly.vertices {
                            let wp = v + brush.location;
                            b_min = b_min.min(wp);
                            b_max = b_max.max(wp);
                            has_verts = true;
                        }
                    }
                    if !has_verts {
                        continue;
                    }
                    let b_min = b_min - pad;
                    let b_max = b_max + pad;
                    if b_max.x < seg_min.x
                        || b_min.x > seg_max.x
                        || b_max.y < seg_min.y
                        || b_min.y > seg_max.y
                        || b_max.z < seg_min.z
                        || b_min.z > seg_max.z
                    {
                        continue;
                    }
                    if !crate::renderer::ray_intersects_aabb_segment(
                        ray.origin,
                        ray.direction,
                        4.0,
                        max_t,
                        b_min,
                        b_max,
                    ) {
                        continue;
                    }

                    let mut hit_brush = false;
                    for poly in &brush.polys {
                        if (poly.flags
                            & (ut99_bsp::PF_NOT_SOLID
                                | ut99_bsp::PF_PORTAL
                                | ut99_bsp::PF_INVISIBLE))
                            != 0
                        {
                            continue;
                        }
                        let n = poly.vertices.len();
                        if n < 3 {
                            continue;
                        }
                        let v0 = poly.vertices[0] + brush.location;
                        for i in 1..n - 1 {
                            let v1 = poly.vertices[i] + brush.location;
                            let v2 = poly.vertices[i + 1] + brush.location;
                            if let Some(t) = ray.intersect_triangle(v0, v1, v2) {
                                if t > 4.0 && t < max_t {
                                    hit_brush = true;
                                    break;
                                }
                            }
                        }
                        if hit_brush {
                            break;
                        }
                    }
                    if hit_brush {
                        walls_occluding_count += 1;
                    }
                }
            }
        }

        // Each wall adds 6 dB of absorption attenuation (standard UE1 audio occlusion)
        let occlusion_attenuation_db = walls_occluding_count as f32 * 6.0;
        let final_effective_volume_db = if direct_volume_db <= -96.0 {
            -96.0
        } else {
            (direct_volume_db - occlusion_attenuation_db).max(-96.0)
        };

        // High frequency cutoff: 22000Hz unobstructed, -4500Hz per occluding barrier down to 600Hz
        let low_pass_cutoff_hz = (22000.0 - walls_occluding_count as f32 * 4500.0).max(600.0);

        Some(AcousticOcclusionResult {
            emitter_tag: emitter.actor_tag.clone(),
            emitter_location: emitter.location,
            distance,
            is_in_range,
            direct_volume_db,
            walls_occluding_count,
            occlusion_attenuation_db,
            final_effective_volume_db,
            low_pass_cutoff_hz,
        })
    }

    /// Compute acoustic occlusions for all emitters relative to a listener position
    pub fn compute_all_acoustic_occlusions(&self, listener_pos: glam::Vec3) -> Vec<AcousticOcclusionResult> {
        let emitters = self.collect_sound_emitters();
        emitters
            .iter()
            .filter_map(|e| self.compute_acoustic_occlusion_for_emitter(&e.actor_tag, listener_pos))
            .collect()
    }

    /// Place a new AmbientSound actor at the builder brush pivot with Undo/Redo
    pub fn place_ambient_sound_at_pivot(&mut self, sound_name: &str, radius_byte: u8, volume_byte: u8) -> Result<String, String> {
        let spawn_pos = self.builder_brush.location;

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }

        let unr = self.loaded_unr.as_mut().unwrap();
        let amb_count = unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case("AmbientSound")).count();
        let name = format!("AmbientSound{}", amb_count);
        let tag = format!("AmbSound_{}", amb_count);

        let mut actor = ut99_format::ActorRecord::default();
        actor.name = name.clone();
        actor.class_name = "AmbientSound".to_string();
        actor.tag = tag;
        actor.location = spawn_pos;

        actor.properties.insert(
            "AmbientSound".into(),
            ut99_format::TaggedProperty {
                name: "AmbientSound".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Name(sound_name.to_string()),
            },
        );
        actor.properties.insert(
            "SoundRadius".into(),
            ut99_format::TaggedProperty {
                name: "SoundRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(radius_byte),
            },
        );
        actor.properties.insert(
            "SoundVolume".into(),
            ut99_format::TaggedProperty {
                name: "SoundVolume".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(volume_byte),
            },
        );
        actor.properties.insert(
            "SoundPitch".into(),
            ut99_format::TaggedProperty {
                name: "SoundPitch".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(64),
            },
        );

        unr.actors.push(actor.clone());
        self.history.push(crate::history::Action::AddActor { actor });
        self.map_is_dirty = true;
        self.log(format!("🔊 Placed AmbientSound '{}' ({}) at ({:.0}, {:.0}, {:.0})", name, sound_name, spawn_pos.x, spawn_pos.y, spawn_pos.z));
        Ok(name)
    }

    /// Update acoustic and sound emitter properties with Undo/Redo
    pub fn update_sound_emitter_properties(
        &mut self,
        actor_tag: &str,
        sound_name: &str,
        radius_byte: u8,
        volume_byte: u8,
        pitch_byte: u8,
    ) -> Result<(), String> {
        let unr = self.loaded_unr.as_mut().ok_or_else(|| "No level loaded".to_string())?;

        let actor_idx = unr.actors.iter().position(|a| a.tag == actor_tag || a.name == actor_tag)
            .ok_or_else(|| format!("Sound emitter actor '{}' not found", actor_tag))?;

        let old_actor = unr.actors[actor_idx].clone();
        let mut new_actor = old_actor.clone();

        new_actor.properties.insert(
            "AmbientSound".into(),
            ut99_format::TaggedProperty {
                name: "AmbientSound".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Name(sound_name.to_string()),
            },
        );
        new_actor.properties.insert(
            "SoundRadius".into(),
            ut99_format::TaggedProperty {
                name: "SoundRadius".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(radius_byte),
            },
        );
        new_actor.properties.insert(
            "SoundVolume".into(),
            ut99_format::TaggedProperty {
                name: "SoundVolume".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(volume_byte),
            },
        );
        new_actor.properties.insert(
            "SoundPitch".into(),
            ut99_format::TaggedProperty {
                name: "SoundPitch".into(),
                array_index: 0,
                value: ut99_format::TaggedValue::Byte(pitch_byte),
            },
        );

        unr.actors[actor_idx] = new_actor.clone();

        self.history.push(crate::history::Action::UpdateSoundEmitter {
            actor_tag: actor_tag.to_string(),
            old_actor,
            new_actor,
            description: format!("Update sound properties on '{}'", actor_tag),
        });

        self.map_is_dirty = true;
        self.log(format!("🔊 Updated sound emitter '{}': Sound='{}', Radius={} ({:.0} uu), Volume={}", actor_tag, sound_name, radius_byte, radius_byte as f32 * 40.0, volume_byte));
        Ok(())
    }

    /// Focus camera on a sound emitter actor
    pub fn focus_camera_on_sound_emitter(&mut self, actor_tag: &str) {
        let emitters = self.collect_sound_emitters();
        if let Some(e) = emitters.iter().find(|e| e.actor_tag == actor_tag || e.actor_name == actor_tag) {
            self.camera.position = e.location - self.camera.forward() * 250.0;
            self.log(format!("Camera focused on sound emitter '{}' at ({:.0}, {:.0}, {:.0})", e.actor_tag, e.location.x, e.location.y, e.location.z));
        }
    }

    pub(crate) fn audio_error(&mut self, message: impl Into<String>) {
        let message = message.into();
        self.log(&message);
        self.toast_error(message);
    }

    /// Update after a confirmed path edit; repeated calls for the same root do not rescan.
    pub fn sync_audio_installation(&mut self) {
        let _ = crate::compiler::CompilerBridge::sync_from_configured_path(&self.ut99_executable_path);
        let root = crate::compiler::CompilerBridge::find_ut_dir();
        if !self.audio_catalog.initialized || self.audio_catalog.root != root {
            self.refresh_audio_from_root(root);
        }
    }

    pub fn refresh_audio_resources(&mut self) {
        self.refresh_audio_from_root(crate::compiler::CompilerBridge::find_ut_dir());
    }

    pub(crate) fn refresh_audio_from_root(&mut self, root: Option<std::path::PathBuf>) {
        for error in self.audio_catalog.rescan(root) { self.audio_error(error); }
        self.sound_packages = self.audio_catalog.sounds.values().filter_map(|p| p.file_stem()?.to_str().map(str::to_string)).collect();
        for sounds in self.audio_catalog.imports.values() {
            if let Some(sound) = sounds.first() { self.sound_packages.push(sound.package.clone()); }
        }
        if !self.mylevel_sounds.is_empty() { self.sound_packages.push("MyLevel".into()); }
        self.sound_packages.sort_by_key(|s| s.to_ascii_lowercase());
        self.sound_packages.dedup_by(|a,b| a.eq_ignore_ascii_case(b));
        self.music_packages = self.audio_catalog.music.values().filter_map(|p| p.file_stem()?.to_str().map(str::to_string)).collect();
        let sound = self.sound_packages.iter().find(|s| s.eq_ignore_ascii_case(&self.selected_sound_package))
            .or_else(|| self.sound_packages.first()).cloned();
        self.loaded_sounds.clear();
        if let Some(name) = sound { self.load_sound_package(&name); }
        else { self.selected_sound_package.clear(); self.selected_sound_name = None; }
        let music = self.music_packages.iter().find(|s| s.eq_ignore_ascii_case(&self.selected_music_package))
            .or_else(|| self.music_packages.first()).cloned();
        self.loaded_music = None;
        if let Some(name) = music { self.load_music_package(&name); } else { self.selected_music_package.clear(); }
    }

    pub fn scan_sound_packages(&mut self) { self.refresh_audio_resources(); }
    pub fn scan_music_packages(&mut self) { self.refresh_audio_resources(); }

    pub fn load_sound_package(&mut self, pkg_name: &str) {
        self.selected_sound_package = pkg_name.to_string();
        let previous_name = self.selected_sound_name.take();
        self.loaded_sounds.clear();
        if pkg_name.eq_ignore_ascii_case("MyLevel") {
            self.loaded_sounds = self.mylevel_sounds.clone();
        } else {
            let imports = self.audio_catalog.imports.get(&pkg_name.to_ascii_lowercase()).cloned().unwrap_or_default();
            let result = self.audio_catalog.sound_path(pkg_name).and_then(|path| crate::audio_catalog::read_sounds(path, pkg_name));
            match result {
                Ok(sounds) => self.loaded_sounds = sounds,
                Err(error) if imports.is_empty() || self.audio_catalog.sounds.contains_key(&pkg_name.to_ascii_lowercase()) => self.audio_error(format!("Sound package '{pkg_name}' failed: {error:#}")),
                Err(_) => {}
            }
            for imported in imports {
                if let Some(existing) = self.loaded_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(&imported.name)) { *existing = imported; }
                else { self.loaded_sounds.push(imported); }
            }
        }
        self.selected_sound_name = previous_name.filter(|name| self.loaded_sounds.iter().any(|s| s.name.eq_ignore_ascii_case(name)))
            .or_else(|| self.loaded_sounds.first().map(|s| s.name.clone()));
    }

    pub fn load_music_package(&mut self, pkg_name: &str) {
        self.selected_music_package = pkg_name.to_string();
        self.loaded_music = None;
        match self.audio_catalog.music_path(pkg_name).and_then(|path| crate::audio_catalog::read_music(path, pkg_name)) {
            Ok((music, _)) => self.loaded_music = Some(music),
            Err(error) => self.audio_error(format!("Music package '{pkg_name}' failed: {error:#}")),
        }
    }

    pub fn music_module_bytes(&self, pkg_name: &str) -> anyhow::Result<Vec<u8>> {
        let path = self.audio_catalog.music_path(pkg_name)?;
        Ok(crate::audio_catalog::read_music(path, pkg_name)?.1)
    }

    /// Imports raw PCM `.wav` bytes into a `SoundResource` inside `package` (e.g. `"MyLevel"` or a custom `.uax` package).
    pub fn import_wav_sound(
        &mut self,
        name: &str,
        package: &str,
        wav_bytes: Vec<u8>,
    ) -> Option<ut99_format::package::SoundResource> {
        let sound = ut99_format::package::parse_sound_export(name, package, &wav_bytes)?;
        if !self.selected_sound_package.eq_ignore_ascii_case(package) {
            if package.eq_ignore_ascii_case("MyLevel") || self.audio_catalog.sounds.contains_key(&package.to_ascii_lowercase())
                || self.audio_catalog.imports.contains_key(&package.to_ascii_lowercase()) {
                self.load_sound_package(package);
            } else { self.loaded_sounds.clear(); }
        }
        if !package.eq_ignore_ascii_case("MyLevel") {
            let imports = self.audio_catalog.imports.entry(package.to_ascii_lowercase()).or_default();
            if let Some(existing) = imports.iter_mut().find(|s| s.name.eq_ignore_ascii_case(name)) { *existing = sound.clone(); }
            else { imports.push(sound.clone()); }
        }
        if !self.sound_packages.iter().any(|p| p.eq_ignore_ascii_case(package)) {
            self.sound_packages.insert(0, package.to_string());
        }
        if package.eq_ignore_ascii_case("MyLevel") {
            if let Some(existing) = self.mylevel_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(name)) {
                *existing = sound.clone();
            } else {
                self.mylevel_sounds.push(sound.clone());
            }
            if let Some(ref mut unr) = self.loaded_unr {
                if let Some(existing) = unr.embedded_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(name)) {
                    *existing = sound.clone();
                } else {
                    unr.embedded_sounds.push(sound.clone());
                }
            }
        }
        self.selected_sound_package = package.to_string();
        if let Some(existing) = self.loaded_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(name)) {
            *existing = sound.clone();
        } else {
            self.loaded_sounds.push(sound.clone());
        }
        self.selected_sound_name = Some(sound.name.clone());
        self.log(format!(
            "Sound Browser: Imported WAV '{}' ({} Hz, {} ch, {} bit) into package '{}'.",
            sound.name, sound.sample_rate, sound.channels, sound.bits_per_sample, package
        ));
        Some(sound)
    }

    /// Embeds a loaded sound by name into the `MyLevel` pseudo-package so it is saved inside the `.unr` map.
    pub fn embed_sound_in_mylevel(&mut self, sound_name: &str) -> bool {
        let Some(mut sound) = self
            .loaded_sounds
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(sound_name))
            .cloned()
        else {
            return false;
        };
        sound.package = "MyLevel".to_string();
        if !self.sound_packages.iter().any(|p| p.eq_ignore_ascii_case("MyLevel")) {
            self.sound_packages.insert(0, "MyLevel".to_string());
        }
        if let Some(existing) = self.mylevel_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(&sound.name)) {
            *existing = sound.clone();
        } else {
            self.mylevel_sounds.push(sound.clone());
        }
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            if let Some(existing) = unr.embedded_sounds.iter_mut().find(|s| s.name.eq_ignore_ascii_case(&sound.name)) {
                *existing = sound.clone();
            } else {
                unr.embedded_sounds.push(sound.clone());
            }
        }
        self.log(format!("Embedded sound '{}' into MyLevel package.", sound.name));
        true
    }

    /// Exports all currently loaded sounds into a standalone binary `.uax` package on disk.
    pub fn export_sound_package_uax(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let pkg = ut99_format::package::build_uax_package(&self.loaded_sounds)?;
        let bytes = pkg.to_bytes()?;
        std::fs::write(path.as_ref(), bytes)?;
        self.log(format!(
            "Exported {} sound(s) to UAX package '{}'.",
            self.loaded_sounds.len(),
            path.as_ref().display()
        ));
        Ok(())
    }

    /// Exports tracker music bytes into a standalone binary `.umx` package on disk.
    pub fn export_music_package_umx(
        &mut self,
        path: impl AsRef<Path>,
        tracker_bytes: &[u8],
    ) -> anyhow::Result<()> {
        let Some(ref music) = self.loaded_music else {
            anyhow::bail!("No MusicResource loaded to export as .umx");
        };
        let pkg = ut99_format::package::build_umx_package(&music.name, &music.format, tracker_bytes)?;
        let bytes = pkg.to_bytes()?;
        std::fs::write(path.as_ref(), bytes)?;
        self.log(format!(
            "Exported music '{}' ({}) to UMX package '{}'.",
            music.name,
            music.format,
            path.as_ref().display()
        ));
        Ok(())
    }

    /// Embeds or updates `MyLevel.Screenshot` (the map selection preview thumbnail in UT99) (`BL-P2-04`).
    pub fn set_mylevel_screenshot_rgba8(
        &mut self,
        width: u32,
        height: u32,
        rgba_pixels: &[u8],
    ) {
        let tex = ut99_format::UtxTexture {
            name: "Screenshot".to_string(),
            group: String::new(),
            width,
            height,
            format: 0,
            masked: false,
            rgba_pixels: rgba_pixels.to_vec(),
        };
        self.texture_manager.upsert_texture("MyLevel", tex.clone());
        self.texture_manager.pin_package("MyLevel");
        if let Some(ref mut unr) = self.loaded_unr {
            unr.embedded_textures
                .retain(|t| !t.name.eq_ignore_ascii_case("Screenshot"));
            unr.embedded_textures.push(tex);
        }
        self.log(format!(
            "Embedded map preview thumbnail 'MyLevel.Screenshot' ({}x{}).",
            width, height
        ));
    }

    /// Garbage-collects unreferenced `MyLevel` textures and sounds (`BL-P2-04` / `TEXTURE CULL`).
    pub fn prune_mylevel_assets(&mut self) -> (usize, usize) {
        let mut referenced = std::collections::HashSet::new();
        referenced.insert("screenshot".to_string());

        for b in &self.world_brushes {
            for p in &b.polys {
                let short = p.texture.rsplit('.').next().unwrap_or(&p.texture).to_ascii_lowercase();
                if !short.is_empty() {
                    referenced.insert(short);
                }
            }
        }
        for m in &self.movers {
            for p in &m.brush.polys {
                let short = p.texture.rsplit('.').next().unwrap_or(&p.texture).to_ascii_lowercase();
                if !short.is_empty() {
                    referenced.insert(short);
                }
            }
            for snd in [&m.opening_sound, &m.closed_sound] {
                let clean = snd.split('\'').nth(1).unwrap_or(snd);
                let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                if !short.is_empty() {
                    referenced.insert(short);
                }
            }
        }
        for sm in &self.static_meshes {
            for p in &sm.polys {
                let short = p.texture.rsplit('.').next().unwrap_or(&p.texture).to_ascii_lowercase();
                if !short.is_empty() {
                    referenced.insert(short);
                }
            }
        }
        if let Some(ref unr) = self.loaded_unr {
            for a in &unr.actors {
                for path in a.object_paths.values() {
                    let clean = path.split('\'').nth(1).unwrap_or(path);
                    let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                    if !short.is_empty() {
                        referenced.insert(short);
                    }
                }
                for prop in a.properties.values() {
                    if let ut99_format::TaggedValue::Str(s) | ut99_format::TaggedValue::Name(s) = &prop.value {
                        let clean = s.split('\'').nth(1).unwrap_or(s);
                        let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                        if !short.is_empty() {
                            referenced.insert(short);
                        }
                    }
                }
            }
        }

        let mut pruned_tex = 0usize;
        if let Some(mut mylevel_pkg) = self.texture_manager.get_package("MyLevel").cloned() {
            let before = mylevel_pkg.textures.len();
            mylevel_pkg
                .textures
                .retain(|t| referenced.contains(&t.name.to_ascii_lowercase()));
            pruned_tex = before.saturating_sub(mylevel_pkg.textures.len());
            self.texture_manager
                .insert_package("MyLevel".to_string(), mylevel_pkg);
        }

        let before_snd = self.mylevel_sounds.len();
        self.mylevel_sounds
            .retain(|s| referenced.contains(&s.name.to_ascii_lowercase()));
        let pruned_snd = before_snd.saturating_sub(self.mylevel_sounds.len());

        if let Some(ref mut unr) = self.loaded_unr {
            unr.embedded_textures
                .retain(|t| referenced.contains(&t.name.to_ascii_lowercase()));
            unr.embedded_sounds
                .retain(|s| referenced.contains(&s.name.to_ascii_lowercase()));
        }

        self.log(format!(
            "MyLevel GC (TEXTURE CULL): pruned {} unused texture(s) and {} unused sound(s).",
            pruned_tex, pruned_snd
        ));
        (pruned_tex, pruned_snd)
    }

    // --- Fase 6: Static Meshes & OBJ Interop ---

    pub fn play_sound_preview(&mut self, full_sound_name: &str, wav_bytes: &[u8]) -> Option<std::path::PathBuf> {
        if wav_bytes.len() < 44 {
            self.log(format!("Cannot play sound '{}': invalid WAV buffer ({} bytes).", full_sound_name, wav_bytes.len()));
            return None;
        }
        let preview_path = std::env::temp_dir().join("forge99_sound_preview.wav");
        if std::fs::write(&preview_path, wav_bytes).is_ok() {
            #[cfg(target_os = "windows")]
            {
                let path_str = preview_path.to_string_lossy().replace('\'', "''");
                std::thread::spawn(move || {
                    let _ = std::process::Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-NonInteractive",
                            "-WindowStyle",
                            "Hidden",
                            "-Command",
                            &format!("(New-Object System.Media.SoundPlayer '{}').PlaySync()", path_str),
                        ])
                        .status();
                });
            }
            self.log(format!("Playing sound '{}' ({} bytes WAV)...", full_sound_name, wav_bytes.len()));
            Some(preview_path)
        } else {
            self.log(format!("Failed to write temporary WAV preview for '{}'.", full_sound_name));
            None
        }
    }

}
