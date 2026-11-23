use super::*;

impl EditorState {
    pub fn apply_cinematic_preset(&mut self, preset: ut99_bsp::CinematicPreset) {
        self.cinematic_settings.active_track = preset.create_track();
        self.cinematic_settings.playhead_t = 0.0;
        self.log(format!("Applied Cinematic Preset: {}", preset.title()));
    }

    /// Captures the current active viewport camera transform as a new spline keyframe
    pub fn add_keyframe_at_camera_position(&mut self) {
        let node_idx = self.cinematic_settings.active_track.keyframes.len();
        let kf = ut99_bsp::SplineCameraKeyframe {
            position: self.camera.position,
            rotation: glam::Vec3::new(self.camera.pitch, self.camera.yaw, 0.0),
            fov: self.camera.fov_y.to_degrees(),
            speed: 350.0,
            pause_time: 0.0,
            label: format!("Keyframe_{node_idx}"),
        };
        self.cinematic_settings.active_track.keyframes.push(kf);
        self.map_is_dirty = true;
        self.log(format!("Captured camera position to Cinematic Keyframe #{node_idx}"));
    }

    /// Removes a keyframe from the active cinematic track
    pub fn remove_cinematic_keyframe(&mut self, index: usize) {
        if index < self.cinematic_settings.active_track.keyframes.len() {
            self.cinematic_settings.active_track.keyframes.remove(index);
            self.map_is_dirty = true;
            self.log(format!("Removed Cinematic Keyframe #{index}"));
        }
    }

    /// Advances cinematic playback along the spline and updates pilot camera if active
    pub fn update_cinematic_playback(&mut self, delta_sec: f32) {
        let duration = self.cinematic_settings.active_track.estimated_duration().max(0.1);
        if self.cinematic_settings.is_playing {
            let speed = self.cinematic_settings.playback_speed;
            let step = (delta_sec * speed) / duration;
            self.cinematic_settings.playhead_t += step;
            if self.cinematic_settings.playhead_t >= 1.0 {
                if self.cinematic_settings.active_track.is_looping {
                    self.cinematic_settings.playhead_t = self.cinematic_settings.playhead_t.fract();
                } else {
                    self.cinematic_settings.playhead_t = 1.0;
                    self.cinematic_settings.is_playing = false;
                }
            }
        }

        // Pilot Viewport Camera: Sync viewport camera with spline playhead
        if self.cinematic_settings.pilot_camera {
            let (pos, rot, fov) = self.cinematic_settings.active_track.sample_at(self.cinematic_settings.playhead_t);
            self.camera.position = pos;
            self.camera.pitch = rot.x;
            self.camera.yaw = rot.y;
            self.camera.fov_y = fov.to_radians();
        }
    }

    /// Exports spline keyframe nodes as permanent UT99 InterpolationPoint actors into the level
    pub fn export_cinematic_to_level_actors(&mut self) -> Vec<usize> {
        let mut created_indices = Vec::new();
        let track_name = self.cinematic_settings.active_track.name.clone();

        for (i, kf) in self.cinematic_settings.active_track.keyframes.iter().enumerate() {
            let actor_name = format!("{}_Node_{}", track_name, i);
            let mut record = ut99_format::ActorRecord::default();
            record.class_name = "InterpolationPoint".to_string();
            record.location = kf.position;
            record.name = actor_name;
            record.tag = track_name.clone();
            record.event = format!("{}_Event_{}", track_name, i);
            record.rotation = (
                (kf.rotation.x * 65536.0 / 360.0) as i32,
                (kf.rotation.y * 65536.0 / 360.0) as i32,
                (kf.rotation.z * 65536.0 / 360.0) as i32,
            );

            if let Some(ref mut level) = self.loaded_unr {
                let idx = level.actors.len();
                level.actors.push(record.clone());
                self.history.push(crate::history::Action::AddActor {
                    actor: record,
                });
                created_indices.push(idx);
            }
        }

        let count = created_indices.len();
        self.map_is_dirty = true;
        self.log(format!("Exported {} InterpolationPoint actors for track '{}'", count, track_name));
        created_indices
    }

    /// Computes the 8 line segments forming the camera frustum pyramid at the current playhead
    pub fn get_camera_frustum_lines_at_playhead(&self) -> Vec<(glam::Vec3, glam::Vec3)> {
        let (pos, rot, fov) = self.cinematic_settings.active_track.sample_at(self.cinematic_settings.playhead_t);
        let dist = 140.0;
        let pitch_rad = rot.x.to_radians();
        let yaw_rad = rot.y.to_radians();

        let forward = glam::Vec3::new(
            pitch_rad.cos() * yaw_rad.cos(),
            pitch_rad.cos() * yaw_rad.sin(),
            pitch_rad.sin(),
        ).normalize_or_zero();

        let right = forward.cross(glam::Vec3::Z).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();

        let half_h = (fov.to_radians() * 0.5).tan() * dist;
        let half_w = half_h * 1.33; // 4:3 aspect ratio

        let center_plane = pos + forward * dist;
        let p_top_left = center_plane - right * half_w + up * half_h;
        let p_top_right = center_plane + right * half_w + up * half_h;
        let p_bot_right = center_plane + right * half_w - up * half_h;
        let p_bot_left = center_plane - right * half_w - up * half_h;

        vec![
            // 4 rays from camera eye to rectangle corners
            (pos, p_top_left),
            (pos, p_top_right),
            (pos, p_bot_right),
            (pos, p_bot_left),
            // 4 edges of the far rectangle
            (p_top_left, p_top_right),
            (p_top_right, p_bot_right),
            (p_bot_right, p_bot_left),
            (p_bot_left, p_top_left),
        ]
    }


}
