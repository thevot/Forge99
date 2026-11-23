use super::*;
use glam::Vec3;

impl EditorState {
    pub fn update_particles(&mut self, dt: f32) {
        if !self.particle_settings.is_simulating {
            return;
        }
        let speed = self.particle_settings.sim_speed;
        for emitter in &mut self.particle_emitters {
            emitter.update(dt * speed);
        }
    }

    pub fn add_particle_emitter(&mut self, name: impl Into<String>, loc: Vec3, emitter_type: ParticleEmitterType) {
        let emitter = ParticleEmitter::new(name, loc, emitter_type);
        self.particle_emitters.push(emitter);
        self.map_is_dirty = true;
    }

    /// Detects the highest solid floor plane directly underneath a world position
    pub fn detect_nearest_floor_for_pos(&self, pos: Vec3) -> Option<f32> {
        let mut highest_floor = None;
        for brush in &self.world_brushes {
            for poly in &brush.polys {
                if poly.plane.normal.z > 0.3 {
                    // Check if bounds roughly contain horizontal point
                    let min_x = poly.vertices.iter().map(|v| v.x).fold(f32::INFINITY, f32::min) - 32.0;
                    let max_x = poly.vertices.iter().map(|v| v.x).fold(f32::NEG_INFINITY, f32::max) + 32.0;
                    let min_y = poly.vertices.iter().map(|v| v.y).fold(f32::INFINITY, f32::min) - 32.0;
                    let max_y = poly.vertices.iter().map(|v| v.y).fold(f32::NEG_INFINITY, f32::max) + 32.0;

                    if pos.x >= min_x && pos.x <= max_x && pos.y >= min_y && pos.y <= max_y {
                        // Estimate plane Z at pos.x, pos.y
                        if poly.plane.normal.z.abs() > 1e-4 {
                            let d = poly.plane.normal.dot(poly.vertices[0]);
                            let floor_z = (d - poly.plane.normal.x * pos.x - poly.plane.normal.y * pos.y) / poly.plane.normal.z;
                            if floor_z <= pos.z {
                                highest_floor = Some(highest_floor.map_or(floor_z, |h: f32| h.max(floor_z)));
                            }
                        }
                    }
                }
            }
        }
        highest_floor
    }

    /// Detects floor plane underneath specified emitter
    pub fn detect_nearest_floor_for_emitter(&self, emitter_idx: usize) -> Option<f32> {
        self.particle_emitters.get(emitter_idx)
            .and_then(|e| self.detect_nearest_floor_for_pos(e.location))
    }

    /// Applies a canonical particle preset to an existing emitter with atomic undo/redo
    pub fn apply_particle_preset(&mut self, emitter_idx: usize, preset: ParticlePresetTemplate) -> bool {
        let old_emitter = match self.particle_emitters.get(emitter_idx) {
            Some(e) => e.clone(),
            None => return false,
        };

        let mut new_emitter = preset.create_emitter(&old_emitter.name, old_emitter.location);
        new_emitter.floor_z = old_emitter.floor_z;

        self.particle_emitters[emitter_idx] = new_emitter.clone();
        let desc = format!("Applied Particle Preset '{}' on #{}", preset.label(), emitter_idx);
        self.history.push(Action::UpdateParticleEmitter {
            emitter_index: emitter_idx,
            old_emitter,
            new_emitter,
            description: desc.clone(),
        });

        self.map_is_dirty = true;
        self.log(desc);
        true
    }

    /// Places a new particle emitter at the builder brush pivot with auto floor raycast
    pub fn place_particle_emitter_at_pivot(&mut self, preset: Option<ParticlePresetTemplate>) -> usize {
        let spawn_pos = self.builder_brush.location;
        let p = preset.unwrap_or(ParticlePresetTemplate::TorchFireFlames);
        let count = self.particle_emitters.len();
        let name = format!("ParticleFX_{}_{}", p.label().split_whitespace().next().unwrap_or("Emitter"), count);

        let mut emitter = p.create_emitter(&name, spawn_pos);
        if self.particle_settings.auto_detect_floor {
            if let Some(floor_z) = self.detect_nearest_floor_for_pos(spawn_pos) {
                emitter.floor_z = Some(floor_z);
            }
        }

        self.particle_emitters.push(emitter.clone());
        let new_idx = self.particle_emitters.len() - 1;
        self.particle_settings.selected_emitter_index = new_idx;

        // Also add an ActorRecord in the UNR level for persistence
        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "ParticleEmitter".to_string();
        actor.name = name.clone();
        actor.tag = format!("FX_{}", count);
        actor.location = spawn_pos;
        actor.properties.insert(
            "EmitterType".to_string(),
            TaggedProperty {
                name: "EmitterType".to_string(),
                array_index: 0,
                value: TaggedValue::Byte(emitter.emitter_type as u8),
            },
        );

        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut level) = self.loaded_unr {
            level.actors.push(actor.clone());
        }

        self.history.push(Action::AddActor {
            actor: actor.clone(),
        });

        self.map_is_dirty = true;
        self.log(format!("Placed ParticleEmitter '{}' ({}) at ({:.1}, {:.1}, {:.1})", name, p.label(), spawn_pos.x, spawn_pos.y, spawn_pos.z));
        new_idx
    }

    /// Clears live particle instances for a specific emitter
    pub fn clear_emitter_particles(&mut self, emitter_idx: usize) {
        if let Some(e) = self.particle_emitters.get_mut(emitter_idx) {
            e.particles.clear();
            e.spawn_accumulator = 0.0;
        }
    }

    // ==========================================
    // GLTF 2.0 Full Map Scene Export
    // ==========================================

}
