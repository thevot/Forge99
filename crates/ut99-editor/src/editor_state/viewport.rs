use super::*;
use glam::Vec3;

impl EditorState {
    /// Configures the 2D Viewport reference background image and display mode (`View -> Background Image`).
    pub fn set_background_image(&mut self, path: Option<String>, mode: BackgroundDisplayMode) {
        self.background_image_path = path.clone();
        self.background_display_mode = mode;
        match path {
            Some(p) => self.log(format!(
                "Viewport Background Image set to '{}' (Mode: {:?}).",
                p, mode
            )),
            None => self.log("Viewport Background Image cleared."),
        }
    }


    pub fn set_camera_speed_preset(&mut self, preset: u8) {
        let p = preset.clamp(1, 8);
        self.camera_speed_preset = p;
        self.camera.speed = match p {
            1 => 150.0,
            2 => 300.0,
            3 => 450.0,
            4 => 600.0,
            5 => 900.0,
            6 => 1200.0,
            7 => 1600.0,
            8 => 2000.0,
            _ => 600.0,
        };
        self.log(format!("Camera flight speed: Preset {p} ({:.0} uu/s)", self.camera.speed));
    }


    pub fn cycle_camera_speed(&mut self) {
        let next = if self.camera_speed_preset >= 8 { 1 } else { self.camera_speed_preset + 1 };
        self.set_camera_speed_preset(next);
    }


    /// Returns the ViewportType corresponding to physical screen coordinates (x, y).
    /// Returns None if the coordinates lie outside the active 3D viewport canvas.
    /// In Single layout, returns the active viewport type.
    /// In FourViewports layout (2x2 Quad), resolves:
    /// - Top-left: Top (XY)
    /// - Top-right: Side (YZ)
    /// - Bottom-left: Front (XZ)
    /// - Bottom-right: Perspective (3D)
    pub fn viewport_type_at(&self, x: f32, y: f32) -> Option<ViewportType> {
        let [vx, vy, vw, vh] = self.viewport_rect;
        if x < vx || x > (vx + vw) || y < vy || y > (vy + vh) {
            return None;
        }

        if self.viewport_layout == ViewportLayout::FourViewports {
            let hw = vw * 0.5;
            let hh = vh * 0.5;
            if x < vx + hw && y < vy + hh {
                Some(ViewportType::Top)
            } else if x >= vx + hw && y < vy + hh {
                Some(ViewportType::Side)
            } else if x < vx + hw && y >= vy + hh {
                Some(ViewportType::Front)
            } else {
                Some(ViewportType::Perspective)
            }
        } else {
            Some(self.active_viewport_type)
        }
    }


    /// Automatically synchronizes the active viewport type to the quadrant under the cursor
    /// when in FourViewports mode. Returns Some(ViewportType) if within the viewport rect.
    pub fn sync_hovered_viewport(&mut self, x: f32, y: f32) -> Option<ViewportType> {
        if let Some(vt) = self.viewport_type_at(x, y) {
            if self.viewport_layout == ViewportLayout::FourViewports && self.active_viewport_type != vt {
                self.active_viewport_type = vt;
            }
            Some(vt)
        } else {
            None
        }
    }


    /// Returns the screen rectangle [x, y, w, h] corresponding to the given viewport type.
    /// In Single layout, returns self.viewport_rect.
    /// In FourViewports (2x2 Quad), returns the respective quadrant rect.
    pub fn quadrant_rect_for_type(&self, vp_type: ViewportType) -> [f32; 4] {
        let [vx, vy, vw, vh] = self.viewport_rect;
        if self.viewport_layout == ViewportLayout::FourViewports {
            let hw = vw * 0.5;
            let hh = vh * 0.5;
            match vp_type {
                ViewportType::Top => [vx, vy, hw, hh],
                ViewportType::Side => [vx + hw, vy, vw - hw, hh],
                ViewportType::Front => [vx, vy + hh, hw, vh - hh],
                ViewportType::Perspective => [vx + hw, vy + hh, vw - hw, vh - hh],
            }
        } else {
            self.viewport_rect
        }
    }


    /// Teleports the 3D camera to frame the currently selected object or multi-selection AABB in viewport
    pub fn focus_camera_on_selection(&mut self) {
        if self.has_multi_selection() {
            let mut min_pt = Vec3::splat(f32::MAX);
            let mut max_pt = Vec3::splat(f32::MIN);
            let mut found = false;

            for &b_idx in &self.multi_selected_brushes {
                if let Some(b) = self.world_brushes.get(b_idx) {
                    min_pt = min_pt.min(b.location);
                    max_pt = max_pt.max(b.location);
                    found = true;
                    for poly in &b.polys {
                        for &v in &poly.vertices {
                            let wp = b.location + v * b.scale;
                            min_pt = min_pt.min(wp);
                            max_pt = max_pt.max(wp);
                        }
                    }
                }
            }
            if let Some(ref unr) = self.loaded_unr {
                for &a_idx in &self.multi_selected_actors {
                    if let Some(a) = unr.actors.get(a_idx) {
                        min_pt = min_pt.min(a.location - Vec3::splat(32.0));
                        max_pt = max_pt.max(a.location + Vec3::splat(32.0));
                        found = true;
                    }
                }
            }

            if found {
                let center = (min_pt + max_pt) * 0.5;
                let radius = ((max_pt - min_pt).length() * 0.5).max(200.0);
                let dist = (radius * 1.75).max(400.0);
                self.camera.position = center - self.camera.forward() * dist;
                self.log(format!(
                    "Camera framed multi-selection at ({:.0}, {:.0}, {:.0}) [dist {:.0} UU]",
                    center.x, center.y, center.z, dist
                ));
                return;
            }
        }

        let (target_pos, target_dist) = if let Some(m_idx) = self.selected_mover_index {
            (self.movers.get(m_idx).map(|m| m.brush.location), 400.0)
        } else if let Some(sel_idx) = self.selected_actor_index {
            (
                self.loaded_unr
                    .as_ref()
                    .and_then(|u| u.actors.get(sel_idx).map(|a| a.location)),
                400.0,
            )
        } else if let Some(b_idx) = self.selected_brush_index {
            if let Some(b) = self.world_brushes.get(b_idx) {
                let mut max_r = 0.0_f32;
                for p in &b.polys {
                    for &v in &p.vertices {
                        max_r = max_r.max((v * b.scale).length());
                    }
                }
                let dist = (max_r * 1.75).max(400.0);
                (Some(b.location), dist)
            } else {
                (None, 400.0)
            }
        } else if let Some(sm_idx) = self.selected_static_mesh_index {
            (self.static_meshes.get(sm_idx).map(|m| m.location), 400.0)
        } else {
            (Some(self.builder_brush.location), 400.0)
        };

        if let Some(pos) = target_pos {
            let offset = -self.camera.forward() * target_dist;
            self.camera.position = pos + offset;
            self.log(format!("Camera framed selection at ({:.0}, {:.0}, {:.0})", pos.x, pos.y, pos.z));
        }
    }


    /// Deliverable I.6: Resets viewport layout and clamps UI scale / viewport bounds to safe visible coordinates.
    pub fn reset_and_clamp_window_layout(&mut self) {
        self.ui_scale = self.ui_scale.clamp(0.75, 2.0);
        self.viewport_layout = ViewportLayout::FourViewports;
        self.active_viewport_type = ViewportType::Perspective;
        self.inspector_panel_mode = InspectorPanelMode::Split;
        self.log("Window Layout: Clamped floating window bounds and reset 2x2 Quad Viewport layout.");
    }


    pub fn set_camera_bookmark(&mut self, slot: usize, name: Option<String>) {
        let slot_idx = slot.min(9);
        let mut bm = crate::camera::CameraBookmark::new(slot_idx + 1, &self.camera);
        if let Some(n) = name {
            bm.name = n;
        }
        let pos = bm.position;
        let name_str = bm.name.clone();
        self.camera_bookmarks[slot_idx] = Some(bm);
        self.log(format!("Saved Camera Bookmark #{} ('{}') at ({:.0}, {:.0}, {:.0})", slot_idx + 1, name_str, pos.x, pos.y, pos.z));
    }


    pub fn goto_camera_bookmark(&mut self, slot: usize) -> bool {
        let slot_idx = slot.min(9);
        if let Some(ref bm) = self.camera_bookmarks[slot_idx] {
            self.camera.position = bm.position;
            self.camera.yaw = bm.yaw;
            self.camera.pitch = bm.pitch;
            self.camera.ortho_zoom = bm.ortho_zoom;
            self.log(format!("Jumped to Camera Bookmark #{} ('{}')", slot_idx + 1, bm.name));
            true
        } else {
            self.log(format!("Camera Bookmark #{} is empty.", slot_idx + 1));
            false
        }
    }


    pub fn clear_camera_bookmark(&mut self, slot: usize) {
        let slot_idx = slot.min(9);
        self.camera_bookmarks[slot_idx] = None;
        self.log(format!("Cleared Camera Bookmark #{}", slot_idx + 1));
    }


    /// UnrealEd 2.0 `CAMERA ALIGN [NAME=...]` (`REQ-K07`):
    /// Optionally selects the named Actor/Mover/Brush first, then centers all 2D orthographic viewports
    /// (`Top`, `Front`, `Side`) and aims the 3D perspective camera directly at the target location.
    pub fn align_cameras_to_selected(&mut self, target_name: Option<&str>) -> Vec3 {
        if let Some(name_query) = target_name {
            let q = name_query.trim();
            if !q.is_empty() {
                let mut found = false;
                if let Some(ref unr) = self.loaded_unr {
                    if let Some((idx, _)) = unr.actors.iter().enumerate().find(|(_, a)| a.name.eq_ignore_ascii_case(q)) {
                        self.selected_actor_index = Some(idx);
                        self.selected_brush_index = None;
                        self.selected_mover_index = None;
                        found = true;
                    }
                }
                if !found {
                    if let Some((idx, _)) = self.movers.iter().enumerate().find(|(_, m)| m.name.eq_ignore_ascii_case(q)) {
                        self.selected_mover_index = Some(idx);
                        self.selected_actor_index = None;
                        self.selected_brush_index = None;
                        found = true;
                    }
                }
                if !found {
                    if let Some((idx, _)) = self.world_brushes.iter().enumerate().find(|(_, b)| b.name.eq_ignore_ascii_case(q)) {
                        self.selected_brush_index = Some(idx);
                        self.selected_actor_index = None;
                        self.selected_mover_index = None;
                    }
                }
            }
        }

        let target = if let Some(a_idx) = self.selected_actor_index {
            self.loaded_unr
                .as_ref()
                .and_then(|u| u.actors.get(a_idx))
                .map(|a| a.location)
                .unwrap_or(self.builder_brush.location)
        } else if let Some(m_idx) = self.selected_mover_index {
            self.movers
                .get(m_idx)
                .map(|m| m.brush.location)
                .unwrap_or(self.builder_brush.location)
        } else if let Some(b_idx) = self.selected_brush_index {
            self.world_brushes
                .get(b_idx)
                .map(|b| b.location)
                .unwrap_or(self.builder_brush.location)
        } else {
            self.builder_brush.location
        };

        let to_target = target - self.camera.position;
        if to_target.length_squared() > 1.0 {
            let dir = to_target.normalize();
            self.camera.yaw = dir.y.atan2(dir.x);
            self.camera.pitch = dir.z.clamp(-1.0, 1.0).asin();
        }
        self.focus_camera_on(target);
        target
    }

}
