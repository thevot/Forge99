use glam::{Mat4, Quat, Vec3};

/// Represents a single joint / bone in a skeletal hierarchy
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletalBone {
    pub name: String,
    pub parent_index: Option<usize>,
    pub local_pos: Vec3,
    pub local_rot: Quat,
    pub world_transform: Mat4,
}

/// An attachment socket on a specific bone (e.g. WeaponPoint_R, BackHolster, HeadGear)
#[derive(Debug, Clone, PartialEq)]
pub struct SkeletalSocket {
    pub name: String,
    pub bone_name: String,
    pub relative_pos: Vec3,
    pub relative_rot: Quat,
    pub attached_mesh_name: Option<String>,
}

/// A locational damage hitbox bound to a specific skeleton bone (from Gildor's SkelEdit CMeshHitBox)
#[derive(Debug, Clone, PartialEq)]
pub struct MeshHitBox {
    pub name: String,
    pub bone_name: String,
    pub offset: Vec3,
    pub extents: Vec3, // Half-extents
    pub damage_multiplier: f32, // e.g. 2.5 for headshot, 1.0 for body, 0.7 for limbs
    pub is_critical_headshot: bool,
}

/// Raycast hit detection result against skeletal hitboxes
#[derive(Debug, Clone, PartialEq)]
pub struct HitBoxHitResult {
    pub hitbox_name: String,
    pub bone_name: String,
    pub hit_point: Vec3,
    pub distance: f32,
    pub damage_multiplier: f32,
    pub is_critical_headshot: bool,
}

/// A bone pose transformation for a single frame of animation
#[derive(Debug, Clone, PartialEq)]
pub struct BoneTransform {
    pub bone_name: String,
    pub local_pos: Vec3,
    pub local_rot: Quat,
}

/// A keyframe in an animation track
#[derive(Debug, Clone, PartialEq)]
pub struct AnimKeyframe {
    pub time_secs: f32,
    pub bone_transforms: Vec<BoneTransform>,
}

/// A complete skeletal animation sequence (e.g. Run, Jump, Fire, Death)
#[derive(Debug, Clone, PartialEq)]
pub struct AnimSequence {
    pub name: String,
    pub duration_secs: f32,
    pub frame_rate: f32,
    pub loop_anim: bool,
    pub keyframes: Vec<AnimKeyframe>,
}

/// Canonical UT99 animation sequence profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CanonicalAnimType {
    Idle,
    Run,
    Walk,
    Jump,
    Fire,
    Death,
    Taunt,
}

impl CanonicalAnimType {
    pub fn label(&self) -> &'static str {
        match self {
            CanonicalAnimType::Idle => "Idle (Breathing Stance)",
            CanonicalAnimType::Run => "Run (Full Biped Sprint)",
            CanonicalAnimType::Walk => "Walk (Tactical Patrol)",
            CanonicalAnimType::Jump => "Jump (Airborne Extension)",
            CanonicalAnimType::Fire => "Fire (Weapon Recoil Stance)",
            CanonicalAnimType::Death => "Death (Backward Collapse)",
            CanonicalAnimType::Taunt => "Taunt (Victory Chest Strike)",
        }
    }

    pub fn duration(&self) -> f32 {
        match self {
            CanonicalAnimType::Idle => 2.0,
            CanonicalAnimType::Run => 0.8,
            CanonicalAnimType::Walk => 1.2,
            CanonicalAnimType::Jump => 1.0,
            CanonicalAnimType::Fire => 0.4,
            CanonicalAnimType::Death => 1.6,
            CanonicalAnimType::Taunt => 1.5,
        }
    }

    pub fn all() -> &'static [CanonicalAnimType] {
        &[
            CanonicalAnimType::Idle,
            CanonicalAnimType::Run,
            CanonicalAnimType::Walk,
            CanonicalAnimType::Jump,
            CanonicalAnimType::Fire,
            CanonicalAnimType::Death,
            CanonicalAnimType::Taunt,
        ]
    }
}

/// Represents an animated or bind-pose skeletal mesh with bone weighting and sockets
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SkeletalMesh {
    pub name: String,
    pub bones: Vec<SkeletalBone>,
    pub base_vertices: Vec<Vec3>,
    pub bone_indices: Vec<usize>,  // Primary bone index per vertex
    pub bone_weights: Vec<f32>,    // Influence weight per vertex (0.0 .. 1.0)
    pub triangles: Vec<[usize; 3]>,// Triangle vertex index tuples
    pub sockets: Vec<SkeletalSocket>,
    pub hitboxes: Vec<MeshHitBox>,
    pub animations: Vec<AnimSequence>,
    pub active_animation: Option<String>,
    pub current_time: f32,
}

impl SkeletalMesh {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Adds a bone to the hierarchy, returning its new bone index
    pub fn add_bone(
        &mut self,
        name: impl Into<String>,
        parent_index: Option<usize>,
        local_pos: Vec3,
        local_rot: Quat,
    ) -> usize {
        let idx = self.bones.len();
        self.bones.push(SkeletalBone {
            name: name.into(),
            parent_index,
            local_pos,
            local_rot,
            world_transform: Mat4::IDENTITY,
        });
        idx
    }

    /// Adds an attachment socket to the skeletal mesh
    pub fn add_socket(&mut self, socket: SkeletalSocket) {
        self.sockets.push(socket);
    }

    /// Evaluates world transform of an attachment socket
    pub fn get_socket_world_transform(&self, socket_name: &str) -> Option<Mat4> {
        let socket = self.sockets.iter().find(|s| s.name == socket_name)?;
        let bone = self.bones.iter().find(|b| b.name == socket.bone_name)?;
        let socket_local = Mat4::from_rotation_translation(socket.relative_rot, socket.relative_pos);
        Some(bone.world_transform * socket_local)
    }

    /// Adds a locational damage hitbox to the skeletal mesh
    pub fn add_hitbox(&mut self, hitbox: MeshHitBox) {
        self.hitboxes.push(hitbox);
    }

    /// Gets world-space center and extents of a hitbox
    pub fn get_hitbox_world_bounds(&self, hitbox_name: &str) -> Option<(Vec3, Vec3)> {
        let hb = self.hitboxes.iter().find(|h| h.name == hitbox_name)?;
        let bone = self.bones.iter().find(|b| b.name == hb.bone_name)?;
        let center = bone.world_transform.transform_point3(hb.offset);
        Some((center - hb.extents, center + hb.extents))
    }

    /// Tests a ray against all locational damage hitboxes, returning the closest hit
    pub fn test_hitbox_ray(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<HitBoxHitResult> {
        let dir = ray_dir.normalize_or_zero();
        if dir.length_squared() < 1e-4 {
            return None;
        }

        let mut closest_hit: Option<HitBoxHitResult> = None;
        let mut min_t = f32::MAX;

        for hb in &self.hitboxes {
            let bone = match self.bones.iter().find(|b| b.name == hb.bone_name) {
                Some(b) => b,
                None => continue,
            };
            let center = bone.world_transform.transform_point3(hb.offset);
            let b_min = center - hb.extents;
            let b_max = center + hb.extents;

            // Slab test for AABB
            let mut tmin = (b_min.x - ray_origin.x) / dir.x;
            let mut tmax = (b_max.x - ray_origin.x) / dir.x;
            if tmin > tmax { std::mem::swap(&mut tmin, &mut tmax); }

            let mut tymin = (b_min.y - ray_origin.y) / dir.y;
            let mut tymax = (b_max.y - ray_origin.y) / dir.y;
            if tymin > tymax { std::mem::swap(&mut tymin, &mut tymax); }

            if (tmin > tymax) || (tymin > tmax) { continue; }
            if tymin > tmin { tmin = tymin; }
            if tymax < tmax { tmax = tymax; }

            let mut tzmin = (b_min.z - ray_origin.z) / dir.z;
            let mut tzmax = (b_max.z - ray_origin.z) / dir.z;
            if tzmin > tzmax { std::mem::swap(&mut tzmin, &mut tzmax); }

            if (tmin > tzmax) || (tzmin > tmax) { continue; }
            if tzmin > tmin { tmin = tzmin; }

            if tmin >= 0.0 && tmin < min_t {
                min_t = tmin;
                closest_hit = Some(HitBoxHitResult {
                    hitbox_name: hb.name.clone(),
                    bone_name: hb.bone_name.clone(),
                    hit_point: ray_origin + dir * tmin,
                    distance: tmin,
                    damage_multiplier: hb.damage_multiplier,
                    is_critical_headshot: hb.is_critical_headshot,
                });
            }
        }

        closest_hit
    }

    /// Exports skeletal mesh into Epic Games ActorX .psk binary format
    pub fn to_actorx_psk(&self) -> Vec<u8> {
        use crate::actorx::{ActorXPskMesh, PskBone, PskFace, PskMaterial, PskPoint, PskWedge, PskWeight};

        let mut psk = ActorXPskMesh::default();
        for &v in &self.base_vertices {
            psk.points.push(PskPoint { point: v });
        }

        for (idx, bone) in self.bones.iter().enumerate() {
            let parent_idx = bone.parent_index.map(|p| p as i32).unwrap_or(-1);
            psk.bones.push(PskBone {
                name: bone.name.clone(),
                flags: 0,
                num_children: self.bones.iter().filter(|b| b.parent_index == Some(idx)).count() as u32,
                parent_index: parent_idx,
                orientation: bone.local_rot,
                position: bone.local_pos,
                length: 10.0,
                size: Vec3::ONE,
            });
        }

        for (i, &b_idx) in self.bone_indices.iter().enumerate() {
            let w = self.bone_weights.get(i).copied().unwrap_or(1.0);
            psk.weights.push(PskWeight {
                weight: w,
                point_index: i as u32,
                bone_index: b_idx as u32,
            });
        }

        // Triangles to wedges and faces
        psk.materials.push(PskMaterial {
            name: "DefaultMaterial".to_string(),
            texture_index: 0,
            poly_flags: 0,
        });

        for tri in &self.triangles {
            let w0 = psk.wedges.len() as u16;
            psk.wedges.push(PskWedge { point_index: tri[0] as u16, u: 0.0, v: 0.0, material_index: 0 });
            let w1 = psk.wedges.len() as u16;
            psk.wedges.push(PskWedge { point_index: tri[1] as u16, u: 1.0, v: 0.0, material_index: 0 });
            let w2 = psk.wedges.len() as u16;
            psk.wedges.push(PskWedge { point_index: tri[2] as u16, u: 0.5, v: 1.0, material_index: 0 });

            psk.faces.push(PskFace {
                wedge_indices: [w0, w1, w2],
                mat_index: 0,
                aux_mat_index: 0,
                smoothing_groups: 1,
            });
        }

        psk.to_bytes()
    }

    /// Computes Forward Kinematics (FK) to update bone world transforms from root down to leaves
    pub fn compute_forward_kinematics(&mut self) {
        let n = self.bones.len();
        for i in 0..n {
            let local_mat = Mat4::from_rotation_translation(self.bones[i].local_rot, self.bones[i].local_pos);
            let world_mat = if let Some(parent_idx) = self.bones[i].parent_index {
                self.bones[parent_idx].world_transform * local_mat
            } else {
                local_mat
            };
            self.bones[i].world_transform = world_mat;
        }
    }

    /// Transforms mesh base vertices using Linear Blend Skinning (LBS)
    pub fn compute_skinned_vertices(&self) -> Vec<Vec3> {
        let mut skinned = Vec::with_capacity(self.base_vertices.len());
        for (i, &base_v) in self.base_vertices.iter().enumerate() {
            if i < self.bone_indices.len() && self.bone_indices[i] < self.bones.len() {
                let bone_idx = self.bone_indices[i];
                let weight = if i < self.bone_weights.len() { self.bone_weights[i] } else { 1.0 };
                let transformed_v = self.bones[bone_idx].world_transform.transform_point3(base_v);
                skinned.push(base_v.lerp(transformed_v, weight));
            } else {
                skinned.push(base_v);
            }
        }
        skinned
    }

    /// Evaluates and applies an animation pose at a specific time in seconds
    pub fn apply_pose_at_time(&mut self, anim_name: &str, time_secs: f32) -> bool {
        let anim = match self.animations.iter().find(|a| a.name == anim_name) {
            Some(a) => a.clone(),
            None => return false,
        };

        let t = if anim.loop_anim && anim.duration_secs > 0.0 {
            time_secs % anim.duration_secs
        } else {
            time_secs.clamp(0.0, anim.duration_secs)
        };

        if anim.keyframes.is_empty() {
            return false;
        }

        // Find surrounding keyframes
        let mut prev_kf = &anim.keyframes[0];
        let mut next_kf = &anim.keyframes[0];

        for kf in &anim.keyframes {
            if kf.time_secs <= t {
                prev_kf = kf;
            }
            if kf.time_secs >= t {
                next_kf = kf;
                break;
            }
        }

        let span = (next_kf.time_secs - prev_kf.time_secs).max(0.0001);
        let factor = ((t - prev_kf.time_secs) / span).clamp(0.0, 1.0);

        for bt_prev in &prev_kf.bone_transforms {
            if let Some(bone) = self.bones.iter_mut().find(|b| b.name == bt_prev.bone_name) {
                if let Some(bt_next) = next_kf.bone_transforms.iter().find(|b| b.bone_name == bt_prev.bone_name) {
                    bone.local_pos = bt_prev.local_pos.lerp(bt_next.local_pos, factor);
                    bone.local_rot = bt_prev.local_rot.slerp(bt_next.local_rot, factor);
                } else {
                    bone.local_pos = bt_prev.local_pos;
                    bone.local_rot = bt_prev.local_rot;
                }
            }
        }

        self.compute_forward_kinematics();
        self.active_animation = Some(anim_name.to_string());
        self.current_time = t;
        true
    }

    /// Creates a canonical animation sequence for humanoid actors
    pub fn create_canonical_animation(anim_type: CanonicalAnimType) -> AnimSequence {
        let name = match anim_type {
            CanonicalAnimType::Idle => "Idle",
            CanonicalAnimType::Run => "Run",
            CanonicalAnimType::Walk => "Walk",
            CanonicalAnimType::Jump => "Jump",
            CanonicalAnimType::Fire => "Fire",
            CanonicalAnimType::Death => "Death",
            CanonicalAnimType::Taunt => "Taunt",
        };

        let duration = anim_type.duration();
        let num_frames = 9;
        let mut keyframes = Vec::new();

        for f in 0..num_frames {
            let t = (f as f32 / (num_frames - 1) as f32) * duration;
            let phase = (f as f32 / (num_frames - 1) as f32) * std::f32::consts::TAU;
            let mut transforms = Vec::new();

            match anim_type {
                CanonicalAnimType::Idle => {
                    // Slight breathing chest elevation and root bobbing
                    let breath = (phase * 2.0).sin() * 0.8;
                    transforms.push(BoneTransform {
                        bone_name: "Chest".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, 16.0 + breath),
                        local_rot: Quat::from_rotation_x(breath * 0.02),
                    });
                }
                CanonicalAnimType::Run => {
                    // Alternating legs and arms swing
                    let swing = phase.sin();
                    let cos_swing = phase.cos();
                    transforms.push(BoneTransform {
                        bone_name: "Root".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, 40.0 + (phase * 2.0).sin().abs() * 3.0),
                        local_rot: Quat::from_rotation_y(swing * 0.05),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "L_Hip".to_string(),
                        local_pos: Vec3::new(-10.0, 0.0, -4.0),
                        local_rot: Quat::from_rotation_x(swing * 0.65),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "R_Hip".to_string(),
                        local_pos: Vec3::new(10.0, 0.0, -4.0),
                        local_rot: Quat::from_rotation_x(-swing * 0.65),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "L_Shoulder".to_string(),
                        local_pos: Vec3::new(-16.0, 0.0, 6.0),
                        local_rot: Quat::from_rotation_z(-cos_swing * 0.60),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "R_Shoulder".to_string(),
                        local_pos: Vec3::new(16.0, 0.0, 6.0),
                        local_rot: Quat::from_rotation_z(cos_swing * 0.60),
                    });
                }
                CanonicalAnimType::Walk => {
                    let swing = phase.sin();
                    transforms.push(BoneTransform {
                        bone_name: "L_Hip".to_string(),
                        local_pos: Vec3::new(-10.0, 0.0, -4.0),
                        local_rot: Quat::from_rotation_x(swing * 0.35),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "R_Hip".to_string(),
                        local_pos: Vec3::new(10.0, 0.0, -4.0),
                        local_rot: Quat::from_rotation_x(-swing * 0.35),
                    });
                }
                CanonicalAnimType::Jump => {
                    let jump_curve = (phase * 0.5).sin();
                    transforms.push(BoneTransform {
                        bone_name: "Root".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, 40.0 + jump_curve * 28.0),
                        local_rot: Quat::from_rotation_x(-jump_curve * 0.15),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "L_Knee".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, -18.0),
                        local_rot: Quat::from_rotation_x(jump_curve * 0.4),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "R_Knee".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, -18.0),
                        local_rot: Quat::from_rotation_x(jump_curve * 0.4),
                    });
                }
                CanonicalAnimType::Fire => {
                    let recoil = (1.0 - (f as f32 / (num_frames - 1) as f32)).powi(2) * 0.35;
                    transforms.push(BoneTransform {
                        bone_name: "R_Shoulder".to_string(),
                        local_pos: Vec3::new(16.0, -recoil * 4.0, 6.0),
                        local_rot: Quat::from_rotation_x(0.8 - recoil),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "Chest".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, 16.0),
                        local_rot: Quat::from_rotation_y(recoil * 0.1),
                    });
                }
                CanonicalAnimType::Death => {
                    let fall = f as f32 / (num_frames - 1) as f32;
                    transforms.push(BoneTransform {
                        bone_name: "Root".to_string(),
                        local_pos: Vec3::new(0.0, -fall * 18.0, 40.0 - fall * 36.0),
                        local_rot: Quat::from_rotation_x(-fall * 1.4),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "Head".to_string(),
                        local_pos: Vec3::new(0.0, 0.0, 12.0),
                        local_rot: Quat::from_rotation_x(-fall * 0.5),
                    });
                }
                CanonicalAnimType::Taunt => {
                    let raise = (phase).sin().abs();
                    transforms.push(BoneTransform {
                        bone_name: "R_Shoulder".to_string(),
                        local_pos: Vec3::new(16.0, 0.0, 6.0),
                        local_rot: Quat::from_rotation_z(raise * 1.2),
                    });
                    transforms.push(BoneTransform {
                        bone_name: "L_Shoulder".to_string(),
                        local_pos: Vec3::new(-16.0, 0.0, 6.0),
                        local_rot: Quat::from_rotation_z(-raise * 1.2),
                    });
                }
            }

            keyframes.push(AnimKeyframe {
                time_secs: t,
                bone_transforms: transforms,
            });
        }

        AnimSequence {
            name: name.to_string(),
            duration_secs: duration,
            frame_rate: 30.0,
            loop_anim: anim_type != CanonicalAnimType::Death,
            keyframes,
        }
    }

    /// Creates a canonical UT99 Tournament Humanoid bot/player skeleton with standard sockets and animations
    pub fn create_canonical_ut99_humanoid() -> Self {
        let mut mesh = Self::new("UT99_TournamentPlayer_Humanoid");

        // 1. Pelvis / Root (Z = 40.0)
        let root = mesh.add_bone("Root", None, Vec3::new(0.0, 0.0, 40.0), Quat::IDENTITY);
        // Spine & Chest
        let spine = mesh.add_bone("Spine", Some(root), Vec3::new(0.0, 0.0, 16.0), Quat::IDENTITY);
        let chest = mesh.add_bone("Chest", Some(spine), Vec3::new(0.0, 0.0, 16.0), Quat::IDENTITY);
        let neck = mesh.add_bone("Neck", Some(chest), Vec3::new(0.0, 0.0, 8.0), Quat::IDENTITY);
        let _head = mesh.add_bone("Head", Some(neck), Vec3::new(0.0, 0.0, 12.0), Quat::IDENTITY);

        // Left Arm
        let l_shoulder = mesh.add_bone("L_Shoulder", Some(chest), Vec3::new(-16.0, 0.0, 6.0), Quat::IDENTITY);
        let l_elbow = mesh.add_bone("L_Elbow", Some(l_shoulder), Vec3::new(-16.0, 0.0, 0.0), Quat::IDENTITY);
        let _l_hand = mesh.add_bone("L_Hand", Some(l_elbow), Vec3::new(-14.0, 0.0, 0.0), Quat::IDENTITY);

        // Right Arm (Weapon Hand)
        let r_shoulder = mesh.add_bone("R_Shoulder", Some(chest), Vec3::new(16.0, 0.0, 6.0), Quat::IDENTITY);
        let r_elbow = mesh.add_bone("R_Elbow", Some(r_shoulder), Vec3::new(16.0, 0.0, 0.0), Quat::IDENTITY);
        let _r_hand = mesh.add_bone("R_Hand", Some(r_elbow), Vec3::new(14.0, 0.0, 0.0), Quat::IDENTITY);

        // Left Leg
        let l_hip = mesh.add_bone("L_Hip", Some(root), Vec3::new(-10.0, 0.0, -4.0), Quat::IDENTITY);
        let l_knee = mesh.add_bone("L_Knee", Some(l_hip), Vec3::new(0.0, 0.0, -18.0), Quat::IDENTITY);
        let _l_foot = mesh.add_bone("L_Foot", Some(l_knee), Vec3::new(0.0, 4.0, -18.0), Quat::IDENTITY);

        // Right Leg
        let r_hip = mesh.add_bone("R_Hip", Some(root), Vec3::new(10.0, 0.0, -4.0), Quat::IDENTITY);
        let r_knee = mesh.add_bone("R_Knee", Some(r_hip), Vec3::new(0.0, 0.0, -18.0), Quat::IDENTITY);
        let _r_foot = mesh.add_bone("R_Foot", Some(r_knee), Vec3::new(0.0, 4.0, -18.0), Quat::IDENTITY);

        // Sockets
        mesh.add_socket(SkeletalSocket {
            name: "WeaponPoint_R".to_string(),
            bone_name: "R_Hand".to_string(),
            relative_pos: Vec3::new(0.0, 6.0, 0.0),
            relative_rot: Quat::IDENTITY,
            attached_mesh_name: Some("ShockRifle".to_string()),
        });
        mesh.add_socket(SkeletalSocket {
            name: "BackHolster".to_string(),
            bone_name: "Chest".to_string(),
            relative_pos: Vec3::new(0.0, -8.0, 4.0),
            relative_rot: Quat::from_rotation_y(std::f32::consts::PI),
            attached_mesh_name: Some("FlakCannon".to_string()),
        });
        mesh.add_socket(SkeletalSocket {
            name: "HeadGear".to_string(),
            bone_name: "Head".to_string(),
            relative_pos: Vec3::new(0.0, 0.0, 6.0),
            relative_rot: Quat::IDENTITY,
            attached_mesh_name: None,
        });

        // Canonical Locational Damage HitBoxes (Gildor SkelEdit CMeshHitBox)
        mesh.add_hitbox(MeshHitBox {
            name: "Head".to_string(),
            bone_name: "Head".to_string(),
            offset: Vec3::new(0.0, 0.0, 2.0),
            extents: Vec3::new(8.0, 8.0, 9.0),
            damage_multiplier: 2.5,
            is_critical_headshot: true,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "Torso".to_string(),
            bone_name: "Chest".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(14.0, 10.0, 14.0),
            damage_multiplier: 1.0,
            is_critical_headshot: false,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "Pelvis".to_string(),
            bone_name: "Root".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(12.0, 10.0, 10.0),
            damage_multiplier: 1.0,
            is_critical_headshot: false,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "LeftArm".to_string(),
            bone_name: "L_Elbow".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(6.0, 6.0, 12.0),
            damage_multiplier: 0.7,
            is_critical_headshot: false,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "RightArm".to_string(),
            bone_name: "R_Elbow".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(6.0, 6.0, 12.0),
            damage_multiplier: 0.7,
            is_critical_headshot: false,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "LeftLeg".to_string(),
            bone_name: "L_Knee".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(7.0, 7.0, 16.0),
            damage_multiplier: 0.8,
            is_critical_headshot: false,
        });
        mesh.add_hitbox(MeshHitBox {
            name: "RightLeg".to_string(),
            bone_name: "R_Knee".to_string(),
            offset: Vec3::new(0.0, 0.0, 0.0),
            extents: Vec3::new(7.0, 7.0, 16.0),
            damage_multiplier: 0.8,
            is_critical_headshot: false,
        });

        // Add canonical animations
        for &anim_type in CanonicalAnimType::all() {
            mesh.animations.push(Self::create_canonical_animation(anim_type));
        }

        mesh.compute_forward_kinematics();
        mesh
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanoid_skeleton_and_hitbox_raycast() {
        let humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
        assert_eq!(humanoid.hitboxes.len(), 7);

        // Ray aimed directly at Head (Z=94)
        let ray_orig = Vec3::new(0.0, 100.0, 94.0);
        let ray_dir = Vec3::new(0.0, -1.0, 0.0);
        let hit = humanoid.test_hitbox_ray(ray_orig, ray_dir).expect("Headshot ray should hit Head hitbox");
        assert_eq!(hit.hitbox_name, "Head");
        assert!(hit.is_critical_headshot);
        assert!((hit.damage_multiplier - 2.5).abs() < 1e-3);

        // Test ActorX PSK export
        let psk_bytes = humanoid.to_actorx_psk();
        assert!(!psk_bytes.is_empty());
        assert_eq!(&psk_bytes[0..8], b"ACTRHEAD");
    }

    #[test]
    fn test_humanoid_skeleton_construction() {
        let mut humanoid = SkeletalMesh::create_canonical_ut99_humanoid();
        assert_eq!(humanoid.bones.len(), 17);

        // Root at (0, 0, 40)
        let root_pos = humanoid.bones[0].world_transform.transform_point3(Vec3::ZERO);
        assert!((root_pos.z - 40.0).abs() < 1e-3);

        // Head position accumulates root + spine + chest + neck + head = 40 + 16 + 16 + 8 + 12 = 92
        let head_idx = humanoid.bones.iter().position(|b| b.name == "Head").unwrap();
        let head_pos = humanoid.bones[head_idx].world_transform.transform_point3(Vec3::ZERO);
        assert!((head_pos.z - 92.0).abs() < 1e-2);

        // Add sample vertex attached to head
        humanoid.base_vertices.push(Vec3::new(0.0, 0.0, 92.0));
        humanoid.bone_indices.push(head_idx);
        humanoid.bone_weights.push(1.0);

        let skinned = humanoid.compute_skinned_vertices();
        assert_eq!(skinned.len(), 1);
    }
}
