use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Interpolation smoothing mode used by Unreal Engine 1 movers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum MoverGlideType {
    #[default]
    Linear,     // Constant linear velocity between keyframes
    Smooth,     // Cosine / Hermite ease-in ease-out velocity profile
}

/// Keyframe pose definition containing 3D translation and Euler rotation (Pitch, Yaw, Roll)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct MoverKeyframe {
    pub position: Vec3,
    pub rotation: Vec3, // Pitch, Yaw, Roll in degrees
}

/// Representation of a multi-keyframe animated mover for route visualization and scrubbing
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MoverTrajectory {
    pub mover_name: String,
    pub mover_tag: String,
    pub num_keys: usize,
    pub keyframes: Vec<MoverKeyframe>,
    pub move_time_secs: f32,
    pub stay_open_time_secs: f32,
    pub glide_type: MoverGlideType,
    pub total_path_length_uu: f32,
    pub spline_points: Vec<Vec3>,
}

impl MoverTrajectory {
    pub fn new(
        mover_name: impl Into<String>,
        mover_tag: impl Into<String>,
        keyframes: Vec<MoverKeyframe>,
        move_time_secs: f32,
        stay_open_time_secs: f32,
        glide_type: MoverGlideType,
    ) -> Self {
        let num_keys = keyframes.len().clamp(1, 8);
        let mut total_dist = 0.0;
        for i in 0..num_keys.saturating_sub(1) {
            total_dist += keyframes[i].position.distance(keyframes[i + 1].position);
        }

        let mut traj = Self {
            mover_name: mover_name.into(),
            mover_tag: mover_tag.into(),
            num_keys,
            keyframes,
            move_time_secs: move_time_secs.max(0.1),
            stay_open_time_secs: stay_open_time_secs.max(0.0),
            glide_type,
            total_path_length_uu: total_dist,
            spline_points: Vec::new(),
        };

        traj.generate_spline_points(32);
        traj
    }

    /// Evaluates the mover's position and rotation at normalized progress t in [0.0, 1.0]
    pub fn evaluate_pose(&self, t: f32) -> (Vec3, Vec3) {
        if self.keyframes.is_empty() {
            return (Vec3::ZERO, Vec3::ZERO);
        }
        if self.keyframes.len() == 1 {
            return (self.keyframes[0].position, self.keyframes[0].rotation);
        }

        let clamped_t = t.clamp(0.0, 1.0);
        let segment_count = (self.num_keys - 1) as f32;
        let scaled_t = clamped_t * segment_count;
        let seg_idx = (scaled_t.floor() as usize).min(self.num_keys - 2);
        let local_t = (scaled_t - seg_idx as f32).clamp(0.0, 1.0);

        // Apply glide smoothing factor
        let eased_t = match self.glide_type {
            MoverGlideType::Linear => local_t,
            MoverGlideType::Smooth => {
                // Cosine easing: 0.5 * (1 - cos(pi * t))
                0.5 * (1.0 - (std::f32::consts::PI * local_t).cos())
            }
        };

        let k0 = &self.keyframes[seg_idx];
        let k1 = &self.keyframes[seg_idx + 1];

        let interp_pos = k0.position.lerp(k1.position, eased_t);
        let interp_rot = k0.rotation.lerp(k1.rotation, eased_t);

        (interp_pos, interp_rot)
    }

    /// Pre-calculates discrete spline points along the entire trajectory for viewport rendering
    fn generate_spline_points(&mut self, samples: usize) {
        self.spline_points.clear();
        let num_pts = samples.clamp(8, 128);
        for i in 0..=num_pts {
            let t = i as f32 / num_pts as f32;
            let (pos, _) = self.evaluate_pose(t);
            self.spline_points.push(pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mover_linear_trajectory() {
        let keys = vec![
            MoverKeyframe {
                position: Vec3::new(0.0, 0.0, 0.0),
                rotation: Vec3::ZERO,
            },
            MoverKeyframe {
                position: Vec3::new(0.0, 0.0, 256.0),
                rotation: Vec3::ZERO,
            },
        ];

        let traj = MoverTrajectory::new("Elevator_0", "Lift1", keys, 2.0, 1.0, MoverGlideType::Linear);
        assert_eq!(traj.num_keys, 2);
        assert_eq!(traj.total_path_length_uu, 256.0);

        // Test start, mid, end evaluation
        let (pos_start, _) = traj.evaluate_pose(0.0);
        let (pos_mid, _) = traj.evaluate_pose(0.5);
        let (pos_end, _) = traj.evaluate_pose(1.0);

        assert_eq!(pos_start, Vec3::new(0.0, 0.0, 0.0));
        assert_eq!(pos_mid, Vec3::new(0.0, 0.0, 128.0));
        assert_eq!(pos_end, Vec3::new(0.0, 0.0, 256.0));
        assert!(!traj.spline_points.is_empty());
    }

    #[test]
    fn test_mover_multi_key_smooth() {
        let keys = vec![
            MoverKeyframe { position: Vec3::new(0.0, 0.0, 0.0), rotation: Vec3::ZERO },
            MoverKeyframe { position: Vec3::new(100.0, 0.0, 100.0), rotation: Vec3::new(0.0, 45.0, 0.0) },
            MoverKeyframe { position: Vec3::new(200.0, 0.0, 0.0), rotation: Vec3::new(0.0, 90.0, 0.0) },
        ];

        let traj = MoverTrajectory::new("Crane", "CraneArm", keys, 4.0, 2.0, MoverGlideType::Smooth);
        assert_eq!(traj.num_keys, 3);
        assert!(traj.total_path_length_uu > 200.0);

        let (pos_half, rot_half) = traj.evaluate_pose(0.5);
        assert!((pos_half.x - 100.0).abs() < 1e-2);
        assert!((rot_half.y - 45.0).abs() < 1e-2);
    }
}
