use super::*;
use super::MoverKeyframe;
use glam::Vec3;

impl EditorState {
    // ==========================================
    // Mover & Keyframe Methods
    // ==========================================
    pub fn add_mover_from_builder(&mut self) {
        self.add_mover_from_builder_with_class("Mover");
    }

    /// Creates a new Mover from the Red Builder Brush using any of the 8 canonical UT99 Mover classes
    /// (`Mover`, `AssertMover`, `AttachMover`, `ElevatorMover`, `GradualMover`, `LoopMover`, `MixMover`, `RotatingMover`).
    pub fn add_mover_from_builder_with_class(&mut self, mover_class: &str) {
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        let cls = if mover_class.trim().is_empty() {
            "Mover".to_string()
        } else {
            mover_class.trim().to_string()
        };

        let mut mover_brush = self.builder_brush.clone();
        // Deliverable I.5: Auto-Transform Permanently of non-identity Scale before creating Mover
        // Prevents UT99 collision & texture skewing when the Mover rotates across keyframes.
        if mover_brush.scale != Vec3::ONE {
            mover_brush.rotation = Vec3::ZERO;
            mover_brush.transform_permanently();
            mover_brush.rotation = rot;
        }
        let count = self.movers.len();
        let name = format!("{}{}", cls, count);
        mover_brush.name = name.clone();
        mover_brush.csg_oper = CsgOperation::Active;

        let mut keyframes: [Option<MoverKeyframe>; 8] = Default::default();
        keyframes[0] = Some(MoverKeyframe { location: loc, rotation: rot });
        // Key 1 defaults to 128 units higher
        keyframes[1] = Some(MoverKeyframe { location: loc + Vec3::new(0.0, 0.0, 128.0), rotation: rot });

        let default_state = match cls.as_str() {
            "LoopMover" => "ConstantLoop",
            "ElevatorMover" | "GradualMover" | "AssertMover" => "TriggerControl",
            _ => "BumpOpenTimed",
        };

        let mover = EditorMover {
            name: name.clone(),
            mover_class: cls.clone(),
            brush: mover_brush,
            current_key: 0,
            keyframes,
            initial_state: default_state.to_string(),
            stay_open_time: 4.0,
            move_time: 1.0,
            delay_time: 0.0,
            glide_type: "MV_Glide".to_string(),
            encroach_type: "ME_ReturnWhenEncroach".to_string(),
            tag: format!("MoverTag{}", count),
            event: String::new(),
            opening_sound: String::new(),
            opened_sound: String::new(),
            closing_sound: String::new(),
            closed_sound: String::new(),
            move_ambient_sound: String::new(),
        };

        self.history.push(Action::AddMover {
            mover: mover.clone(),
        });
        self.movers.push(mover);
        self.selected_mover_index = Some(self.movers.len() - 1);
        self.selected_brush_index = None;
        self.selected_actor_index = None;

        // Also sync to loaded_unr if present
        if self.loaded_unr.is_none() {
            self.loaded_unr = Some(ut99_format::UnrLevel::default());
        }
        if let Some(ref mut unr) = self.loaded_unr {
            let mut actor = ut99_format::ActorRecord::default();
            actor.name = name.clone();
            actor.class_name = cls.clone();
            actor.location = loc;
            actor.tag = format!("MoverTag{}", count);
            unr.actors.push(actor);
        }

        self.log(format!(
            "Added {} '{}' (InitialState={}) with Key 0 at ({:.0}, {:.0}, {:.0}) and Key 1 at ({:.0}, {:.0}, {:.0}).",
            cls,
            name,
            default_state,
            loc.x,
            loc.y,
            loc.z,
            loc.x,
            loc.y,
            loc.z + 128.0
        ));
    }

    pub fn set_mover_key(&mut self, mover_idx: usize, key: usize) -> bool {
        if key >= 8 { return false; }
        let mut log_msg = None;
        if let Some(mover) = self.movers.get_mut(mover_idx) {
            mover.current_key = key;
            if let Some(ref kf) = mover.keyframes[key] {
                mover.brush.location = kf.location;
                mover.brush.rotation = kf.rotation;
                log_msg = Some(format!("Mover '{}': Switched to Key {} at ({:.0}, {:.0}, {:.0}).", mover.name, key, kf.location.x, kf.location.y, kf.location.z));
            } else if let Some(kf0) = mover.keyframes[0].clone() {
                mover.keyframes[key] = Some(kf0.clone());
                mover.brush.location = kf0.location;
                mover.brush.rotation = kf0.rotation;
                log_msg = Some(format!("Mover '{}': Initialized Key {} from Key 0.", mover.name, key));
            }
        } else {
            return false;
        }
        if let Some(msg) = log_msg {
            self.log(msg);
        }
        true
    }

    pub fn set_selected_mover_key(&mut self, key: usize) -> bool {
        if let Some(m_idx) = self.selected_mover_index {
            self.set_mover_key(m_idx, key)
        } else {
            false
        }
    }

    pub fn cycle_selected_mover_key(&mut self, forward: bool) -> Option<usize> {
        if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get(m_idx) {
                let cur = mover.current_key;
                let next = if forward {
                    (cur + 1) % 8
                } else {
                    if cur == 0 { 7 } else { cur - 1 }
                };
                self.set_mover_key(m_idx, next);
                return Some(next);
            }
        }
        None
    }

    pub fn duplicate_selected_mover_key(&mut self, target_key: usize) -> bool {
        if target_key >= 8 { return false; }
        let mut log_msg = None;
        if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get_mut(m_idx) {
                let cur = mover.current_key;
                if let Some(kf) = mover.keyframes[cur].clone() {
                    mover.keyframes[target_key] = Some(kf);
                    log_msg = Some(format!("Mover '{}': Copied Key {} to Key {}.", mover.name, cur, target_key));
                }
            }
        }
        if let Some(msg) = log_msg {
            self.log(msg);
            true
        } else {
            false
        }
    }

    pub fn save_selected_mover_current_key_pos(&mut self, loc: Vec3, rot: Vec3) {
        let mut log_msg = None;
        if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get_mut(m_idx) {
                let k = mover.current_key;
                mover.keyframes[k] = Some(MoverKeyframe { location: loc, rotation: rot });
                mover.brush.location = loc;
                mover.brush.rotation = rot;
                log_msg = Some(format!("Mover '{}': Saved Key {} at ({:.0}, {:.0}, {:.0}).", mover.name, k, loc.x, loc.y, loc.z));
            }
        }
        if let Some(msg) = log_msg {
            self.log(msg);
        }
    }

    // ==========================================
    // Add Special Brush Method
    // ==========================================

}
