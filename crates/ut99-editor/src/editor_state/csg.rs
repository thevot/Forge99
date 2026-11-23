use super::*;
use glam::Vec3;

impl EditorState {
    pub fn set_builder_cube(&mut self, size: Vec3) {
        self.builder_shape = BuilderShape::Cube;
        self.builder_cube_size = size;
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_cube("ActiveBuilder", CsgOperation::Active, size);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Cube ({:.0} x {:.0} x {:.0})", size.x, size.y, size.z));
    }

    pub fn set_builder_cylinder(&mut self, height: f32, radius: f32, sides: usize) {
        self.builder_shape = BuilderShape::Cylinder;
        self.builder_cylinder_params = (height, radius, sides);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_cylinder("ActiveBuilder", CsgOperation::Active, height, radius, sides);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Cylinder (H:{:.0}, R:{:.0}, Sides:{})", height, radius, sides));
    }

    pub fn set_builder_cone(&mut self, height: f32, radius: f32, sides: usize) {
        self.builder_shape = BuilderShape::Cone;
        self.builder_cone_params = (height, radius, sides);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_cone("ActiveBuilder", CsgOperation::Active, height, radius, sides);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Cone (H:{:.0}, R:{:.0}, Sides:{})", height, radius, sides));
    }

    pub fn set_builder_sheet(&mut self, width: f32, height: f32) {
        self.builder_shape = BuilderShape::Sheet;
        self.builder_sheet_params = (width, height);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_sheet("ActiveBuilder", CsgOperation::Active, width, height);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Sheet ({:.0} x {:.0})", width, height));
    }

    pub fn set_builder_stairs(&mut self, steps: usize, height: f32, width: f32, length: f32) {
        self.builder_shape = BuilderShape::Stairs;
        self.builder_stairs_params = (steps, height, width, length);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_staircase("ActiveBuilder", CsgOperation::Active, steps, height, width, length);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Linear Stairs (Steps:{}, StepH:{:.0}, StepW:{:.0}, StepL:{:.0})", steps, height, width, length));
    }

    pub fn set_builder_sphere(&mut self, radius: f32, rings: usize, segments: usize) {
        self.builder_shape = BuilderShape::Sphere;
        self.builder_sphere_params = (radius, rings, segments);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_sphere("ActiveBuilder", CsgOperation::Active, radius, rings, segments);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Sphere (R:{:.0}, Rings:{}, Segments:{})", radius, rings, segments));
    }

    pub fn set_builder_curved_stairs(
        &mut self,
        steps: usize,
        height: f32,
        inner_radius: f32,
        width: f32,
        angle: f32,
        counter_clockwise: bool,
    ) {
        self.builder_shape = BuilderShape::CurvedStairs;
        self.builder_curved_stairs_params = (steps, height, inner_radius, width, angle, counter_clockwise);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_curved_staircase(
            "ActiveBuilder",
            CsgOperation::Active,
            steps,
            height,
            inner_radius,
            width,
            angle,
            counter_clockwise,
        );
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!(
            "Builder Brush: Curved Stairs (Steps:{}, StepH:{:.0}, InnerR:{:.0}, StepW:{:.0}, Angle:{:.0}°, CCW:{})",
            steps, height, inner_radius, width, angle, counter_clockwise
        ));
    }

    pub fn set_builder_spiral_stairs(
        &mut self,
        steps: usize,
        height: f32,
        inner_radius: f32,
        width: f32,
        total_angle: f32,
        counter_clockwise: bool,
    ) {
        self.builder_shape = BuilderShape::SpiralStairs;
        self.builder_spiral_stairs_params = (steps, height, inner_radius, width, total_angle, counter_clockwise);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_spiral_staircase(
            "ActiveBuilder",
            CsgOperation::Active,
            steps,
            height,
            inner_radius,
            width,
            total_angle,
            counter_clockwise,
        );
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!(
            "Builder Brush: Spiral Stairs (Steps:{}, StepH:{:.0}, InnerR:{:.0}, StepW:{:.0}, TotalAngle:{:.0}°, CCW:{})",
            steps, height, inner_radius, width, total_angle, counter_clockwise
        ));
    }

    pub fn set_builder_tetrahedron(&mut self, radius: f32) {
        self.builder_shape = BuilderShape::Tetrahedron;
        self.builder_tetrahedron_params = radius;
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_tetrahedron("ActiveBuilder", CsgOperation::Active, radius);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Tetrahedron (Radius:{:.0})", radius));
    }

    pub fn set_builder_torus(
        &mut self,
        ring_radius: f32,
        tube_radius: f32,
        ring_sides: usize,
        tube_sides: usize,
    ) {
        self.builder_shape = BuilderShape::Torus;
        self.builder_torus_params = (ring_radius, tube_radius, ring_sides, tube_sides);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_torus(
            "ActiveBuilder",
            CsgOperation::Active,
            ring_radius,
            tube_radius,
            ring_sides,
            tube_sides,
        );
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!(
            "Builder Brush: Torus (RingR:{:.0}, TubeR:{:.0}, RingSides:{}, TubeSides:{})",
            ring_radius, tube_radius, ring_sides, tube_sides
        ));
    }

    pub fn set_builder_volumetric(&mut self, width: f32, height: f32, sheets: usize) {
        self.builder_shape = BuilderShape::Volumetric;
        self.builder_volumetric_params = (width, height, sheets);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_volumetric("ActiveBuilder", CsgOperation::Active, width, height, sheets);
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!("Builder Brush: Volumetric (W:{:.0}, H:{:.0}, Sheets:{})", width, height, sheets));
    }

    pub fn set_builder_terrain(
        &mut self,
        width_steps: usize,
        length_steps: usize,
        step_size: f32,
        height_amplitude: f32,
        seed: u32,
    ) {
        self.builder_shape = BuilderShape::Terrain;
        self.builder_terrain_params = (width_steps, length_steps, step_size, height_amplitude, seed);
        let loc = self.builder_brush.location;
        let rot = self.builder_brush.rotation;
        self.builder_brush = build_terrain(
            "ActiveBuilder",
            CsgOperation::Active,
            width_steps,
            length_steps,
            step_size,
            height_amplitude,
            seed,
        );
        self.builder_brush.location = loc;
        self.builder_brush.rotation = rot;
        self.log(format!(
            "Builder Brush: Terrain Grid ({}x{}, StepSize:{:.0}, Amplitude:{:.0}, Seed:{})",
            width_steps, length_steps, step_size, height_amplitude, seed
        ));
    }

    pub fn reset_builder_brush(&mut self) {
        let old_brush = self.builder_brush.clone();
        self.builder_brush.location = Vec3::ZERO;
        self.builder_brush.rotation = Vec3::ZERO;
        if old_brush.location != Vec3::ZERO || old_brush.rotation != Vec3::ZERO {
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "Reset Builder Brush to Origin".to_string(),
            });
        }
        self.log("Builder brush reset to origin (0, 0, 0).");
    }

    pub fn csg_add(&mut self) {
        let mut new_brush = self.builder_brush.clone();
        new_brush.csg_oper = CsgOperation::Add;
        new_brush.name = format!("Brush_Add_{}", self.world_brushes.len());
        if let Some(ref tex) = self.active_texture {
            for p in &mut new_brush.polys {
                p.texture = tex.clone();
            }
        }
        let diag = self.audit_builder_brush_csg_safety();
        if !diag.is_convex {
            self.log(format!("⚠️ Pre-CSG Warning: Builder brush is non-convex ({} reflex violations). May cause BSP holes/HOM errors.", diag.reflex_vertex_count));
            self.toast_warning("⚠️ Placed non-convex brush: check for BSP cuts or use Intersect/Deintersect");
        }
        self.history.push(Action::AddBrush { brush: new_brush.clone() });
        self.world_brushes.push(new_brush);
        self.log(format!("CSG ADD: Placed new solid additive brush at {:?}", self.builder_brush.location));
        if diag.is_convex {
            self.toast_success("CSG Add: Placed solid additive brush");
        }
    }

    pub fn csg_subtract(&mut self) {
        let mut new_brush = self.builder_brush.clone();
        new_brush.csg_oper = CsgOperation::Subtract;
        new_brush.name = format!("Brush_Sub_{}", self.world_brushes.len());
        if let Some(ref tex) = self.active_texture {
            for p in &mut new_brush.polys {
                p.texture = tex.clone();
            }
        }
        let diag = self.audit_builder_brush_csg_safety();
        if !diag.is_convex {
            self.log(format!("⚠️ Pre-CSG Warning: Builder brush is non-convex ({} reflex violations). May cause BSP holes/HOM errors.", diag.reflex_vertex_count));
            self.toast_warning("⚠️ Subtracted non-convex brush: check for BSP cuts or use Intersect/Deintersect");
        }
        self.history.push(Action::AddBrush { brush: new_brush.clone() });
        self.world_brushes.push(new_brush);
        self.log(format!("CSG SUBTRACT: Carved new hollow space at {:?}", self.builder_brush.location));
        if diag.is_convex {
            self.toast_success("CSG Subtract: Carved hollow space");
        }
    }

    pub fn shading_for_quadrant(&self, idx: usize) -> crate::camera::ViewportShading {
        if idx < 3 {
            self.viewport_shadings[idx]
        } else if idx == 3 {
            if self.viewport_shading != crate::camera::ViewportShading::Wireframe {
                self.viewport_shading
            } else {
                self.viewport_shadings[3]
            }
        } else {
            self.viewport_shading
        }
    }

    pub fn set_quadrant_shading(&mut self, idx: usize, shading: crate::camera::ViewportShading) {
        if idx < 4 {
            self.viewport_shadings[idx] = shading;
        }
        self.viewport_shading = shading;
    }

    pub fn csg_intersect(&mut self) {
        let (builder_min, builder_max) = self.builder_brush.world_bounds();
        let mut overlapping_brushes = 0;
        let mut candidate_polys = Vec::new();
        for brush in &self.world_brushes {
            if brush.intersects_bounds(builder_min, builder_max, 16.0) {
                candidate_polys.extend(brush.to_world_polys());
                overlapping_brushes += 1;
            }
        }

        if overlapping_brushes == 0 {
            self.log("CSG INTERSECT: No overlapping world geometry found within bounding bounds.");
            return;
        }

        let mut inside_polys =
            ut99_bsp::csg_boolean_with_world(&self.builder_brush, &self.world_brushes, true);

        // Fallback if builder only overlapped an empty Subtract brush with no solid volume inside
        if inside_polys.is_empty() && !candidate_polys.is_empty() {
            if let Some(world_bsp) = BspNode::new(candidate_polys.clone()) {
                let builder_world_polys = self.builder_brush.to_world_polys();
                inside_polys = world_bsp.clip_polygons_inside(&builder_world_polys);
                if let Some(builder_bsp) = BspNode::new(builder_world_polys.clone()) {
                    for cap in builder_bsp.clip_polygons_inside(&candidate_polys) {
                        let is_coplanar = builder_world_polys.iter().any(|bp| {
                            let dot = bp.plane.normal.dot(cap.plane.normal);
                            dot.abs() > 0.999
                                && (bp.plane.dist - cap.plane.dist * dot.signum()).abs() < 0.1
                        });
                        if !is_coplanar {
                            inside_polys.push(cap);
                        }
                    }
                }
            }
        }

        if !inside_polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let local_polys: Vec<_> = inside_polys
                .into_iter()
                .filter_map(|mut p| {
                    for v in &mut p.vertices {
                        *v -= loc;
                    }
                    p.base -= loc;
                    if p.vertices.len() >= 3 {
                        if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                            p.plane = np;
                        }
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect();

            let count = local_polys.len();
            self.builder_brush.polys = local_polys;
            self.builder_brush.rotation = Vec3::ZERO;
            self.builder_brush.scale = Vec3::ONE;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "CSG Intersect".to_string(),
            });
            self.log(format!(
                "CSG INTERSECT: Trimmed builder brush to {count} polygons across {overlapping_brushes} overlapping brushes."
            ));
        } else {
            self.log("CSG INTERSECT: No overlapping geometry found.");
        }
    }

    pub fn csg_deintersect(&mut self) {
        let (builder_min, builder_max) = self.builder_brush.world_bounds();
        let mut overlapping_brushes = 0;
        let mut candidate_polys = Vec::new();
        for brush in &self.world_brushes {
            if brush.intersects_bounds(builder_min, builder_max, 16.0) {
                candidate_polys.extend(brush.to_world_polys());
                overlapping_brushes += 1;
            }
        }

        if overlapping_brushes == 0 {
            self.log("CSG DEINTERSECT: No overlapping world geometry found within bounding bounds.");
            return;
        }

        let mut outside_polys =
            ut99_bsp::csg_boolean_with_world(&self.builder_brush, &self.world_brushes, false);

        if outside_polys.is_empty() && !candidate_polys.is_empty() {
            if let Some(world_bsp) = BspNode::new(candidate_polys.clone()) {
                let builder_world_polys = self.builder_brush.to_world_polys();
                outside_polys = world_bsp.clip_polygons(&builder_world_polys);
                if let Some(builder_bsp) = BspNode::new(builder_world_polys.clone()) {
                    for cap in builder_bsp.clip_polygons_inside(&candidate_polys) {
                        let is_coplanar = builder_world_polys.iter().any(|bp| {
                            let dot = bp.plane.normal.dot(cap.plane.normal);
                            dot.abs() > 0.999
                                && (bp.plane.dist - cap.plane.dist * dot.signum()).abs() < 0.1
                        });
                        if !is_coplanar {
                            outside_polys.push(cap.inverted());
                        }
                    }
                }
            }
        }

        if !outside_polys.is_empty() {
            let old_brush = self.builder_brush.clone();
            let loc = self.builder_brush.location;
            let local_polys: Vec<_> = outside_polys
                .into_iter()
                .filter_map(|mut p| {
                    for v in &mut p.vertices {
                        *v -= loc;
                    }
                    p.base -= loc;
                    if p.vertices.len() >= 3 {
                        if let Some(np) = Plane::from_points(p.vertices[0], p.vertices[1], p.vertices[2]) {
                            p.plane = np;
                        }
                        Some(p)
                    } else {
                        None
                    }
                })
                .collect();

            let count = local_polys.len();
            self.builder_brush.polys = local_polys;
            self.builder_brush.rotation = Vec3::ZERO;
            self.builder_brush.scale = Vec3::ONE;
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush: self.builder_brush.clone(),
                description: "CSG Deintersect".to_string(),
            });
            self.log(format!(
                "CSG DEINTERSECT: Carved builder brush to {count} polygons across {overlapping_brushes} overlapping brushes."
            ));
        } else {
            self.log("CSG DEINTERSECT: Builder brush is entirely within solid world; no cuts produced.");
        }
    }

    /// Copies any entries in `self.brush_groups` onto `brush.group` before reordering.
    pub fn sync_brushes_from_brush_groups(&mut self) {
        for (idx, grp) in &self.brush_groups {
            if let Some(b) = self.world_brushes.get_mut(*idx) {
                if !grp.is_empty() {
                    b.group = Some(grp.clone());
                }
            }
        }
    }

    /// Rebuilds `self.brush_groups` from each `Brush::group` after brush order changes.
    pub fn sync_brush_groups_from_brushes(&mut self) {
        self.brush_groups.clear();
        for (idx, b) in self.world_brushes.iter().enumerate() {
            if let Some(ref grp) = b.group {
                if !grp.is_empty() {
                    self.brush_groups.insert(idx, grp.clone());
                }
            }
        }
        self.geometry_revision = self.geometry_revision.wrapping_add(1);
    }

    pub fn push_shape2d_undo(&mut self) {
        self.shape2d_undo_stack.push(self.shape2d.clone());
        if self.shape2d_undo_stack.len() > 64 {
            self.shape2d_undo_stack.remove(0);
        }
        self.shape2d_redo_stack.clear();
    }

    pub fn undo_shape2d(&mut self) -> bool {
        if let Some(prev) = self.shape2d_undo_stack.pop() {
            let cur = std::mem::replace(&mut self.shape2d, prev);
            self.shape2d_redo_stack.push(cur);
            self.shape_editor_selected_segment = 0;
            true
        } else {
            false
        }
    }

    pub fn redo_shape2d(&mut self) -> bool {
        if let Some(next) = self.shape2d_redo_stack.pop() {
            let cur = std::mem::replace(&mut self.shape2d, next);
            self.shape2d_undo_stack.push(cur);
            self.shape_editor_selected_segment = 0;
            true
        } else {
            false
        }
    }

    pub fn send_selected_brush_to_first(&mut self) {
        if let Some(idx) = self.selected_brush_index {
            if idx > 0 && idx < self.world_brushes.len() {
                self.sync_brushes_from_brush_groups();
                let old_brushes = self.world_brushes.to_vec();
                let old_selection = self.selected_brush_index;
                let b = self.world_brushes.remove(idx);
                self.world_brushes.insert(0, b);
                self.selected_brush_index = Some(0);
                self.sync_brush_groups_from_brushes();
                self.history.push(Action::ReorderBrushes {
                    old_brushes,
                    new_brushes: self.world_brushes.to_vec(),
                    old_selection,
                    new_selection: Some(0),
                    description: "Send Brush to First".to_string(),
                });
                self.log("Brush Order: Moved selected brush to FIRST (evaluated earliest).");
            }
        }
    }

    pub fn send_selected_brush_to_last(&mut self) {
        if let Some(idx) = self.selected_brush_index {
            if idx < self.world_brushes.len().saturating_sub(1) {
                self.sync_brushes_from_brush_groups();
                let old_brushes = self.world_brushes.to_vec();
                let old_selection = self.selected_brush_index;
                let b = self.world_brushes.remove(idx);
                self.world_brushes.push(b);
                let new_idx = self.world_brushes.len() - 1;
                self.selected_brush_index = Some(new_idx);
                self.sync_brush_groups_from_brushes();
                self.history.push(Action::ReorderBrushes {
                    old_brushes,
                    new_brushes: self.world_brushes.to_vec(),
                    old_selection,
                    new_selection: Some(new_idx),
                    description: "Send Brush to Last".to_string(),
                });
                self.log("Brush Order: Moved selected brush to LAST (evaluated latest).");
            }
        }
    }

    pub fn move_selected_brush_up(&mut self) {
        if let Some(idx) = self.selected_brush_index {
            if idx > 0 && idx < self.world_brushes.len() {
                self.sync_brushes_from_brush_groups();
                let old_brushes = self.world_brushes.to_vec();
                let old_selection = self.selected_brush_index;
                self.world_brushes.swap(idx, idx - 1);
                self.selected_brush_index = Some(idx - 1);
                self.sync_brush_groups_from_brushes();
                self.history.push(Action::ReorderBrushes {
                    old_brushes,
                    new_brushes: self.world_brushes.to_vec(),
                    old_selection,
                    new_selection: Some(idx - 1),
                    description: format!("Move Brush Up (swap {} with {})", idx, idx - 1),
                });
                self.log(format!("Brush Order: Swapped brush {} with {}.", idx, idx - 1));
            }
        }
    }

    pub fn move_selected_brush_down(&mut self) {
        if let Some(idx) = self.selected_brush_index {
            if idx + 1 < self.world_brushes.len() {
                self.sync_brushes_from_brush_groups();
                let old_brushes = self.world_brushes.to_vec();
                let old_selection = self.selected_brush_index;
                self.world_brushes.swap(idx, idx + 1);
                self.selected_brush_index = Some(idx + 1);
                self.sync_brush_groups_from_brushes();
                self.history.push(Action::ReorderBrushes {
                    old_brushes,
                    new_brushes: self.world_brushes.to_vec(),
                    old_selection,
                    new_selection: Some(idx + 1),
                    description: format!("Move Brush Down (swap {} with {})", idx, idx + 1),
                });
                self.log(format!("Brush Order: Swapped brush {} with {}.", idx, idx + 1));
            }
        }
    }

}

/// plus `SkyboxBackdrop`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialBrushPreset {
    InvisibleCollisionHull,
    MaskedDecoration,
    ModulatedWindow,
    Portal,
    RegularBrush,
    SemisolidPillar,
    TransparentWindow,
    Water,
    ZonePortal,
    SkyboxBackdrop,
}

impl SpecialBrushPreset {
    pub const ALL: [SpecialBrushPreset; 10] = [
        SpecialBrushPreset::InvisibleCollisionHull,
        SpecialBrushPreset::MaskedDecoration,
        SpecialBrushPreset::ModulatedWindow,
        SpecialBrushPreset::Portal,
        SpecialBrushPreset::RegularBrush,
        SpecialBrushPreset::SemisolidPillar,
        SpecialBrushPreset::TransparentWindow,
        SpecialBrushPreset::Water,
        SpecialBrushPreset::ZonePortal,
        SpecialBrushPreset::SkyboxBackdrop,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            SpecialBrushPreset::InvisibleCollisionHull => "Invisible Collision Hull",
            SpecialBrushPreset::MaskedDecoration => "Masked Decoration",
            SpecialBrushPreset::ModulatedWindow => "Modulated Window",
            SpecialBrushPreset::Portal => "Portal",
            SpecialBrushPreset::RegularBrush => "Regular Brush",
            SpecialBrushPreset::SemisolidPillar => "Semisolid Pillar",
            SpecialBrushPreset::TransparentWindow => "Transparent Window",
            SpecialBrushPreset::Water => "Water",
            SpecialBrushPreset::ZonePortal => "Zone Portal",
            SpecialBrushPreset::SkyboxBackdrop => "Skybox Backdrop",
        }
    }

    pub fn poly_flags(&self) -> u32 {
        use ut99_bsp::{
            PF_FAKE_BACKDROP, PF_INVISIBLE, PF_MASKED, PF_MODULATED, PF_NOT_SOLID, PF_PORTAL,
            PF_SEMISOLID, PF_TRANSLUCENT, PF_TWO_SIDED, PF_UNLIT,
        };
        match self {
            SpecialBrushPreset::InvisibleCollisionHull => PF_INVISIBLE | PF_SEMISOLID,
            SpecialBrushPreset::MaskedDecoration => PF_MASKED | PF_TWO_SIDED | PF_NOT_SOLID,
            SpecialBrushPreset::ModulatedWindow => PF_MODULATED | PF_TWO_SIDED | PF_NOT_SOLID,
            SpecialBrushPreset::Portal => PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID,
            SpecialBrushPreset::RegularBrush => 0,
            SpecialBrushPreset::SemisolidPillar => PF_SEMISOLID,
            SpecialBrushPreset::TransparentWindow => PF_TRANSLUCENT | PF_TWO_SIDED | PF_NOT_SOLID,
            SpecialBrushPreset::Water => PF_PORTAL | PF_TRANSLUCENT | PF_TWO_SIDED | PF_NOT_SOLID,
            SpecialBrushPreset::ZonePortal => PF_PORTAL | PF_INVISIBLE | PF_NOT_SOLID,
            SpecialBrushPreset::SkyboxBackdrop => PF_FAKE_BACKDROP | PF_UNLIT,
        }
    }
}



impl EditorState {
    /// UnrealEd 2.0 `Polygons -> To Brush`:
    /// Copies the selected World Brush's (or Mover's) polygons and transform into the Red Builder Brush with Undo/Redo.
    pub fn polygons_to_brush(&mut self) -> bool {
        let src_opt = if let Some(b_idx) = self.selected_brush_index {
            self.world_brushes.get(b_idx).cloned()
        } else if let Some(m_idx) = self.selected_mover_index {
            self.movers.get(m_idx).map(|m| {
                let mut b = m.brush.clone();
                if b.name.is_empty() {
                    b.name = m.name.clone();
                }
                b
            })
        } else {
            self.toast_warning("Select a World Brush or Mover first for Polygons -> To Brush");
            return false;
        };
        let Some(src) = src_opt else {
            return false;
        };
        let old_brush = self.builder_brush.clone();
        self.builder_brush.location = src.location;
        self.builder_brush.rotation = src.rotation;
        self.builder_brush.scale = src.scale;
        self.builder_brush.polys = src.polys;
        let new_brush = self.builder_brush.clone();
        self.history.push(Action::ModifyBuilderBrush {
            old_brush,
            new_brush,
            description: format!("Polygons -> To Brush (from '{}')", src.name),
        });
        self.log(format!("Copied polygons from '{}' to Red Builder Brush.", src.name));
        self.toast_success("Polygons -> To Brush applied");
        true
    }


    /// UnrealEd 2.0 `Polygons -> From Brush`:
    /// Replaces the selected World Brush's (or Mover's) polygons with the Red Builder Brush's polygons with Undo/Redo.
    pub fn polygons_from_brush(&mut self) -> bool {
        let new_polys = self.builder_brush.polys.clone();
        if new_polys.is_empty() {
            return false;
        }
        if let Some(b_idx) = self.selected_brush_index {
            let Some(target) = self.world_brushes.get_mut(b_idx) else {
                return false;
            };
            let old_polys = target.polys.clone();
            let name = target.name.clone();
            target.polys = new_polys.clone();
            self.history.push(Action::ModifyBrushGeometry {
                brush_index: b_idx,
                old_polys,
                new_polys,
                description: format!("Polygons -> From Brush (into '{}')", name),
            });
            self.map_is_dirty = true;
            self.log(format!("Copied polygons from Red Builder Brush into '{}'.", name));
            self.toast_success("Polygons -> From Brush applied");
            true
        } else if let Some(m_idx) = self.selected_mover_index {
            let Some(mover) = self.movers.get_mut(m_idx) else {
                return false;
            };
            let old_mover = mover.clone();
            let name = mover.name.clone();
            mover.brush.polys = new_polys;
            let new_mover = mover.clone();
            self.history.push(Action::UpdateElevatorMoverConfig {
                mover_index: m_idx,
                old_mover,
                new_mover,
                description: format!("Polygons -> From Brush (into Mover '{}')", name),
            });
            self.map_is_dirty = true;
            self.log(format!("Copied polygons from Red Builder Brush into Mover '{}'.", name));
            self.toast_success("Polygons -> From Brush applied to Mover");
            true
        } else {
            self.toast_warning("Select a World Brush or Mover first for Polygons -> From Brush");
            false
        }
    }


    /// UnrealEd 2.0 `Polygons -> Merge`:
    /// Merges coplanar adjacent polygons sharing an edge on the selected World Brush, Mover, or Red Builder Brush with Undo/Redo.
    pub fn merge_selected_brush_polygons(&mut self) -> usize {
        if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                let merged = brush.merge_coplanar_polygons();
                if merged > 0 {
                    let new_polys = brush.polys.clone();
                    let name = brush.name.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Polygons -> Merge ({} merged on '{}')", merged, name),
                    });
                    self.map_is_dirty = true;
                    self.log(format!("Polygons -> Merge: merged {} coplanar polygon pair(s) on '{}'.", merged, name));
                }
                return merged;
            }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get_mut(m_idx) {
                let old_mover = mover.clone();
                let merged = mover.brush.merge_coplanar_polygons();
                if merged > 0 {
                    let name = mover.name.clone();
                    let new_mover = mover.clone();
                    self.history.push(Action::UpdateElevatorMoverConfig {
                        mover_index: m_idx,
                        old_mover,
                        new_mover,
                        description: format!("Polygons -> Merge ({} merged on Mover '{}')", merged, name),
                    });
                    self.map_is_dirty = true;
                    self.log(format!("Polygons -> Merge: merged {} coplanar polygon pair(s) on Mover '{}'.", merged, name));
                }
                return merged;
            }
        }
        let old_brush = self.builder_brush.clone();
        let merged = self.builder_brush.merge_coplanar_polygons();
        if merged > 0 {
            let new_brush = self.builder_brush.clone();
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush,
                description: format!("Polygons -> Merge ({} merged on Builder)", merged),
            });
            self.log(format!("Polygons -> Merge: merged {} coplanar polygon pair(s) on Red Builder Brush.", merged));
        }
        merged
    }


    /// UnrealEd 2.0 `Polygons -> Separate`:
    /// Fan-triangulates polygons with >3 vertices on the selected World Brush, Mover, or Red Builder Brush with Undo/Redo.
    pub fn separate_selected_brush_polygons(&mut self) -> usize {
        if let Some(b_idx) = self.selected_brush_index {
            if let Some(brush) = self.world_brushes.get_mut(b_idx) {
                let old_polys = brush.polys.clone();
                let added = brush.separate_polygons();
                if added > 0 {
                    let new_polys = brush.polys.clone();
                    let name = brush.name.clone();
                    self.history.push(Action::ModifyBrushGeometry {
                        brush_index: b_idx,
                        old_polys,
                        new_polys,
                        description: format!("Polygons -> Separate (+{} polys on '{}')", added, name),
                    });
                    self.map_is_dirty = true;
                    self.log(format!("Polygons -> Separate: created {} triangle polygon(s) on '{}'.", added, name));
                }
                return added;
            }
        } else if let Some(m_idx) = self.selected_mover_index {
            if let Some(mover) = self.movers.get_mut(m_idx) {
                let old_mover = mover.clone();
                let added = mover.brush.separate_polygons();
                if added > 0 {
                    let name = mover.name.clone();
                    let new_mover = mover.clone();
                    self.history.push(Action::UpdateElevatorMoverConfig {
                        mover_index: m_idx,
                        old_mover,
                        new_mover,
                        description: format!("Polygons -> Separate (+{} polys on Mover '{}')", added, name),
                    });
                    self.map_is_dirty = true;
                    self.log(format!("Polygons -> Separate: created {} triangle polygon(s) on Mover '{}'.", added, name));
                }
                return added;
            }
        }
        let old_brush = self.builder_brush.clone();
        let added = self.builder_brush.separate_polygons();
        if added > 0 {
            let new_brush = self.builder_brush.clone();
            self.history.push(Action::ModifyBuilderBrush {
                old_brush,
                new_brush,
                description: format!("Polygons -> Separate (+{} polys on Builder)", added),
            });
            self.log(format!("Polygons -> Separate: created {} triangle polygon(s) on Red Builder Brush.", added));
        }
        added
    }


    /// Captures a unified [`crate::document::LevelDocument`] SSOT snapshot of the current level (`Deliverable F.1`).
    pub fn to_level_document(&self) -> crate::document::LevelDocument {
        crate::document::LevelDocument::from_editor_state(self)
    }


    /// Restores/synchronizes the editor state from a unified [`crate::document::LevelDocument`] SSOT snapshot (`Deliverable F.1`).
    pub fn apply_level_document(&mut self, doc: &crate::document::LevelDocument) {
        doc.apply_to_editor_state(self);
    }


    pub fn add_special_brush(&mut self, flags: u32, is_subtractive: bool) {
        let sanitized_flags = if (flags & ut99_bsp::PF_NOT_SOLID) != 0 {
            flags & !ut99_bsp::PF_SEMISOLID
        } else {
            flags
        };
        let mut new_brush = self.builder_brush.clone();
        new_brush.csg_oper = if is_subtractive {
            CsgOperation::Subtract
        } else {
            CsgOperation::Add
        };
        new_brush.name = format!("SpecialBrush_{}", self.world_brushes.len());

        for p in &mut new_brush.polys {
            p.flags = sanitized_flags;
            if let Some(ref tex) = self.active_texture {
                p.texture = tex.clone();
            }
        }

        self.history.push(Action::AddBrush { brush: new_brush.clone() });
        self.world_brushes.push(new_brush);
        self.selected_brush_index = Some(self.world_brushes.len() - 1);
        self.log(format!("Add Special: Placed special brush with flags {:#010X} at {:?}", sanitized_flags, self.builder_brush.location));
    }


    /// Applies one of the 9 canonical UnrealEd 2.0 `Add Special Brush` Prefabs (`REQ-B05` — `add special.htm`)
    /// to `special_brush_flags` and `special_is_subtractive`.
    pub fn apply_special_brush_preset(&mut self, preset: SpecialBrushPreset) {
        self.special_brush_flags = preset.poly_flags();
        self.special_is_subtractive = false;
    }


    /// Reconstructs a 3D CSG `Brush` from a compiled `Model_World` `UModel` (`world_bsp`) when loading
    /// a brush-stripped `.unr` map where the author deleted the source CSG brushes (`BL-P0-05`).
    pub fn reconstruct_brush_from_compiled_bsp(
        level: &ut99_format::UnrLevel,
    ) -> Option<ut99_bsp::Brush> {
        let bsp = level.world_bsp.as_ref()?;
        if bsp.nodes.is_empty() || bsp.points.is_empty() || bsp.verts.is_empty() {
            return None;
        }

        let mut reconstructed =
            ut99_bsp::Brush::new("Reconstructed_Model_World", CsgOperation::Add);
        for node in &bsp.nodes {
            let num_verts = node.num_vertices as usize;
            let vert_start = node.i_vert_pool as isize;
            if num_verts < 3 || vert_start < 0 {
                continue;
            }
            let vert_start = vert_start as usize;
            if vert_start + num_verts > bsp.verts.len() {
                continue;
            }
            let mut poly_verts = Vec::with_capacity(num_verts);
            for v_idx in vert_start..(vert_start + num_verts) {
                let p_idx = bsp.verts[v_idx].p_vertex;
                if p_idx >= 0 && (p_idx as usize) < bsp.points.len() {
                    poly_verts.push(bsp.points[p_idx as usize]);
                }
            }
            if poly_verts.len() < 3 {
                continue;
            }
            if let Some(mut poly) = ut99_bsp::Polygon::new(poly_verts) {
                if node.i_surf >= 0 && (node.i_surf as usize) < bsp.surfs.len() {
                    let surf = &bsp.surfs[node.i_surf as usize];
                    poly.flags = surf.poly_flags;
                    if surf.p_base >= 0 && (surf.p_base as usize) < bsp.points.len() {
                        poly.base = bsp.points[surf.p_base as usize];
                    }
                    if surf.v_texture_u >= 0 && (surf.v_texture_u as usize) < bsp.vectors.len() {
                        poly.texture_u = bsp.vectors[surf.v_texture_u as usize];
                    }
                    if surf.v_texture_v >= 0 && (surf.v_texture_v as usize) < bsp.vectors.len() {
                        poly.texture_v = bsp.vectors[surf.v_texture_v as usize];
                    }
                    if surf.texture_ref != 0 {
                        let tex_str = level.package.resolve_full_object_path(surf.texture_ref);
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
                }
                reconstructed.polys.push(poly);
            }
        }

        if reconstructed.polys.is_empty() {
            None
        } else {
            Some(reconstructed)
        }
    }

}
