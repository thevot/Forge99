use super::*;

impl EditorState {
    pub fn get_skeletal_meshes(&self) -> &[SkeletalMesh] {
        &self.skeletal_meshes
    }

    pub fn get_skeletal_meshes_mut(&mut self) -> &mut [SkeletalMesh] {
        &mut self.skeletal_meshes
    }

    /// Selects and activates an animation sequence on the specified skeletal mesh
    pub fn select_skeletal_animation(&mut self, mesh_idx: usize, anim_name: &str) -> bool {
        if let Some(mesh) = self.skeletal_meshes.get_mut(mesh_idx) {
            if mesh.animations.iter().any(|a| a.name == anim_name) {
                let mesh_name = mesh.name.clone();
                mesh.apply_pose_at_time(anim_name, 0.0);
                self.skeletal_settings.selected_mesh_index = mesh_idx;
                self.skeletal_settings.active_animation = Some(anim_name.to_string());
                self.skeletal_settings.timeline_time = 0.0;
                self.log(format!("Selected animation '{anim_name}' on SkeletalMesh '{mesh_name}'"));
                return true;
            }
        }
        false
    }

    /// Advances the interactive character animation clock and evaluates joint skinning
    pub fn update_skeletal_animation_timeline(&mut self, dt: f32) {
        let mesh_idx = self.skeletal_settings.selected_mesh_index;
        let is_playing = self.skeletal_settings.is_playing;
        let speed = self.skeletal_settings.play_speed;
        let is_loop = self.skeletal_settings.loop_preview;

        if !is_playing {
            // Even if paused, ensure pose is evaluated at current timeline_time
            if let Some(mesh) = self.skeletal_meshes.get_mut(mesh_idx) {
                if let Some(ref anim_name) = self.skeletal_settings.active_animation {
                    mesh.apply_pose_at_time(anim_name, self.skeletal_settings.timeline_time);
                }
            }
            return;
        }

        if let Some(mesh) = self.skeletal_meshes.get_mut(mesh_idx) {
            let active_name = self.skeletal_settings.active_animation.clone()
                .unwrap_or_else(|| "Run".to_string());

            let duration = mesh.animations.iter().find(|a| a.name == active_name)
                .map(|a| a.duration_secs)
                .unwrap_or(1.0)
                .max(0.1);

            let new_time = self.skeletal_settings.timeline_time + dt * speed;
            if new_time >= duration {
                if is_loop {
                    self.skeletal_settings.timeline_time = new_time % duration;
                } else {
                    self.skeletal_settings.timeline_time = duration;
                    self.skeletal_settings.is_playing = false;
                }
            } else {
                self.skeletal_settings.timeline_time = new_time;
            }

            mesh.apply_pose_at_time(&active_name, self.skeletal_settings.timeline_time);
        }
    }

    /// Attaches or detaches a weapon/accessory mesh to a skeletal bone socket with atomic undo/redo
    pub fn set_skeletal_socket_attachment(
        &mut self,
        mesh_idx: usize,
        socket_name: &str,
        attached_mesh: Option<String>,
    ) -> bool {
        let old_mesh = match self.skeletal_meshes.get(mesh_idx) {
            Some(m) => m.clone(),
            None => return false,
        };

        let mut updated = false;
        let mut new_mesh = old_mesh.clone();
        if let Some(sock) = new_mesh.sockets.iter_mut().find(|s| s.name == socket_name) {
            sock.attached_mesh_name = attached_mesh.clone();
            updated = true;
        }

        if updated {
            self.skeletal_meshes[mesh_idx] = new_mesh.clone();
            let desc = format!("Set attachment '{:?}' on socket '{}'", attached_mesh, socket_name);
            self.history.push(Action::UpdateSkeletalMesh {
                mesh_index: mesh_idx,
                old_mesh,
                new_mesh,
                description: desc.clone(),
            });
            self.map_is_dirty = true;
            self.log(desc);
            true
        } else {
            false
        }
    }

    /// Places a SkeletalMeshActor into the level at the builder brush pivot
    pub fn place_skeletal_mesh_at_pivot(&mut self, mesh_idx: usize) -> usize {
        let spawn_pos = self.builder_brush.location;
        let count = self.loaded_unr.as_ref().map(|l| l.actors.len()).unwrap_or(0);
        let mesh_name = self.skeletal_meshes.get(mesh_idx).map(|m| m.name.clone())
            .unwrap_or_else(|| "UT99_TournamentPlayer_Humanoid".to_string());

        let mut actor = ut99_format::ActorRecord::default();
        actor.class_name = "SkeletalMeshActor".to_string();
        actor.name = format!("SkeletalPlayer_{}", count);
        actor.tag = format!("BotRig_{}", count);
        actor.location = spawn_pos;

        actor.properties.insert(
            "Mesh".to_string(),
            TaggedProperty {
                name: "Mesh".to_string(),
                array_index: 0,
                value: TaggedValue::Str(mesh_name.clone()),
            },
        );

        if let Some(ref anim_name) = self.skeletal_settings.active_animation {
            actor.properties.insert(
                "AnimSequence".to_string(),
                TaggedProperty {
                    name: "AnimSequence".to_string(),
                    array_index: 0,
                    value: TaggedValue::Str(anim_name.clone()),
                },
            );
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
        self.map_is_dirty = true;
        self.log(format!("Spawned SkeletalMeshActor '{}' at ({:.1}, {:.1}, {:.1})", actor.name, spawn_pos.x, spawn_pos.y, spawn_pos.z));
        new_idx
    }

    /// Resets skeletal bones to canonical T-pose bind posture with atomic undo/redo
    pub fn reset_skeletal_mesh_to_bind_pose(&mut self, mesh_idx: usize) -> bool {
        let old_mesh = match self.skeletal_meshes.get(mesh_idx) {
            Some(m) => m.clone(),
            None => return false,
        };

        let mut new_mesh = ut99_bsp::SkeletalMesh::create_canonical_ut99_humanoid();
        new_mesh.name = old_mesh.name.clone();
        new_mesh.sockets = old_mesh.sockets.clone();
        new_mesh.animations = old_mesh.animations.clone();

        self.skeletal_meshes[mesh_idx] = new_mesh.clone();
        self.skeletal_settings.timeline_time = 0.0;
        self.skeletal_settings.is_playing = false;

        self.history.push(Action::UpdateSkeletalMesh {
            mesh_index: mesh_idx,
            old_mesh,
            new_mesh,
            description: "Reset skeleton to bind pose (T-Pose)".to_string(),
        });

        self.map_is_dirty = true;
        self.log("Reset skeletal bones to canonical T-pose bind posture");
        true
    }

    // ==========================================
    // Dynamic Particle FX & Volumetric Emitter Simulator
    // ==========================================

}
