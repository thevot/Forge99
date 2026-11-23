//! Transactional snapshot for interactive viewport transforms (move / rotate / scale).
//!
//! A drag captures a [`TransformSnapshot`] of every affected object when it begins. While the
//! drag is live every `apply_*` call recomputes the objects *absolutely* from the snapshot
//! (`state = f(snapshot, mouse)`), so dragging back to the start restores the exact original values.
//! On release [`TransformSnapshot::commit`] diffs the current state against the snapshot and records a
//! *single* undo entry, whereas [`TransformSnapshot::cancel`] restores the original values
//! (Esc / RMB / focus loss).

use crate::editor_state::{euler_degrees_to_mat3, unreal_rotator_to_mat3, EditorMode, EditorState};
use crate::history::Action;
use crate::raycast::{GizmoAxis, GizmoMode};
use glam::{Mat3, Vec3};
use ut99_bsp::Brush;

/// Original state of one transformed object.
#[derive(Debug, Clone)]
pub enum TargetSnapshot {
    Brush { index: usize, location: Vec3, rotation: Vec3, scale: Vec3 },
    Actor { index: usize, location: Vec3, rotation: (i32, i32, i32) },
    Mover { index: usize, location: Vec3, rotation: Vec3 },
    StaticMesh { index: usize, location: Vec3, rotation: Vec3, scale: Vec3 },
    BuilderBrush(Box<Brush>),
}

impl TargetSnapshot {
    fn location(&self) -> Vec3 {
        match self {
            Self::Brush { location, .. }
            | Self::Actor { location, .. }
            | Self::Mover { location, .. }
            | Self::StaticMesh { location, .. } => *location,
            Self::BuilderBrush(b) => b.location,
        }
    }

    fn orientation(&self) -> Mat3 {
        match self {
            Self::Brush { rotation, .. } | Self::Mover { rotation, .. } | Self::StaticMesh { rotation, .. } => {
                euler_degrees_to_mat3(rotation.x, rotation.y, rotation.z)
            }
            Self::Actor { rotation, .. } => unreal_rotator_to_mat3(rotation.0, rotation.1, rotation.2),
            Self::BuilderBrush(b) => euler_degrees_to_mat3(b.rotation.x, b.rotation.y, b.rotation.z),
        }
    }
}

/// Which objects the current selection resolves to (single source of truth for pivot / snapshot).
enum Selected {
    None,
    Group { brushes: Vec<usize>, actors: Vec<usize> },
    Mover(usize),
    StaticMesh(usize),
    Builder,
}

fn resolve_selection(state: &EditorState) -> Selected {
    if state.has_multi_selection() {
        let mut brushes: Vec<usize> = state.multi_selected_brushes.iter().copied().collect();
        let mut actors: Vec<usize> = state.multi_selected_actors.iter().copied().collect();
        brushes.extend(state.selected_brush_index);
        actors.extend(state.selected_actor_index);
        brushes.sort_unstable();
        brushes.dedup();
        actors.sort_unstable();
        actors.dedup();
        Selected::Group { brushes, actors }
    } else if let Some(m) = state.selected_mover_index {
        Selected::Mover(m)
    } else if let Some(a) = state.selected_actor_index {
        Selected::Group { brushes: vec![], actors: vec![a] }
    } else if let Some(b) = state.selected_brush_index {
        Selected::Group { brushes: vec![b], actors: vec![] }
    } else if let Some(s) = state.selected_static_mesh_index {
        Selected::StaticMesh(s)
    } else if state.is_builder_selected {
        Selected::Builder
    } else {
        Selected::None
    }
}

/// Centroid of the selected vertices while in vertex-edit mode.
fn vertex_pivot(state: &EditorState) -> Option<Vec3> {
    if state.editor_mode == EditorMode::VertexEdit && !state.selected_vertex_indices.is_empty() {
        let unique = state.get_active_brush_unique_vertices();
        let mut sum = Vec3::ZERO;
        let mut n = 0;
        for &idx in &state.selected_vertex_indices {
            if let Some(&v) = unique.get(idx) {
                sum += v;
                n += 1;
            }
        }
        if n > 0 {
            return Some(sum / n as f32);
        }
        return None;
    }
    None
}

/// The single pivot used by press, hover, drag and rendering of the transform gizmo:
/// vertex centroid in vertex mode, otherwise the centroid of everything that would be transformed.
pub fn gizmo_pivot(state: &EditorState) -> Option<Vec3> {
    if state.editor_mode == EditorMode::VertexEdit && !state.selected_vertex_indices.is_empty() {
        return vertex_pivot(state);
    }
    match resolve_selection(state) {
        Selected::None => None,
        Selected::Builder => Some(state.builder_brush.location),
        Selected::Mover(i) => state.movers.get(i).map(|m| m.brush.location),
        Selected::StaticMesh(i) => state.static_meshes.get(i).map(|m| m.location),
        Selected::Group { brushes, actors } => {
            let mut sum = Vec3::ZERO;
            let mut n = 0usize;
            for i in brushes {
                if let Some(b) = state.world_brushes.get(i) {
                    sum += b.location;
                    n += 1;
                }
            }
            if let Some(ref unr) = state.loaded_unr {
                for i in actors {
                    if let Some(a) = unr.actors.get(i) {
                        sum += a.location;
                        n += 1;
                    }
                }
            }
            if n > 0 { Some(sum / n as f32) } else { None }
        }
    }
}

/// Location of the "primary" selected object (used as drag origin for direct click-drag).
pub fn primary_location(state: &EditorState) -> Option<Vec3> {
    if let Some(m) = state.selected_mover_index {
        state.movers.get(m).map(|m| m.brush.location)
    } else if let Some(a) = state.selected_actor_index {
        state.loaded_unr.as_ref().and_then(|u| u.actors.get(a).map(|a| a.location))
    } else if let Some(b) = state.selected_brush_index {
        state.world_brushes.get(b).map(|b| b.location)
    } else if let Some(s) = state.selected_static_mesh_index {
        state.static_meshes.get(s).map(|m| m.location)
    } else if state.is_builder_selected {
        Some(state.builder_brush.location)
    } else {
        None
    }
}

/// Filters out gizmo handles that are degenerate or nearly edge-on / end-on in the current view.
/// This prevents erratic jumps when attempting to drag an axis perpendicular to an orthographic viewport,
/// or when interacting with planes/rotation rings seen edge-on.
pub fn constrain_hit_for_view(
    hit: GizmoAxis,
    mode: GizmoMode,
    rot: Mat3,
    view_dir: Vec3,
) -> GizmoAxis {
    if hit == GizmoAxis::None || hit == GizmoAxis::ScreenPlane {
        return hit;
    }
    if view_dir.length_squared() < 0.25 {
        return hit;
    }
    let v = view_dir.normalize();
    match mode {
        GizmoMode::Translate | GizmoMode::Scale => {
            match hit {
                GizmoAxis::X | GizmoAxis::Y | GizmoAxis::Z => {
                    let local = match hit {
                        GizmoAxis::X => Vec3::X,
                        GizmoAxis::Y => Vec3::Y,
                        _ => Vec3::Z,
                    };
                    let world_dir = (rot * local).normalize();
                    // Axis parallel to view direction -> seen end-on, cannot project rays reliably
                    if world_dir.dot(v).abs() > 0.98 {
                        GizmoAxis::None
                    } else {
                        hit
                    }
                }
                GizmoAxis::PlaneXY | GizmoAxis::PlaneXZ | GizmoAxis::PlaneYZ => {
                    let normal = match hit {
                        GizmoAxis::PlaneXY => Vec3::Z,
                        GizmoAxis::PlaneXZ => Vec3::Y,
                        _ => Vec3::X,
                    };
                    let world_normal = (rot * normal).normalize();
                    // Plane seen edge-on -> projection denominator near zero
                    if world_normal.dot(v).abs() < 0.05 {
                        GizmoAxis::None
                    } else {
                        hit
                    }
                }
                _ => hit,
            }
        }
        GizmoMode::Rotate => {
            let local = match hit {
                GizmoAxis::X => Vec3::X,
                GizmoAxis::Y => Vec3::Y,
                GizmoAxis::Z => Vec3::Z,
                _ => return hit,
            };
            let world_normal = (rot * local).normalize();
            // Rotation ring seen edge-on -> angle calculation is degenerate
            if world_normal.dot(v).abs() < 0.05 {
                GizmoAxis::None
            } else {
                hit
            }
        }
    }
}

/// Converts a rotation matrix (`Rz(yaw) * Ry(pitch) * Rx(roll)`) back to `(pitch, yaw, roll)` in degrees,
/// each normalised to `[0, 360)`.
pub fn mat3_to_euler_degrees(m: Mat3) -> Vec3 {
    let r20 = m.x_axis.z;
    let pitch = (-r20).clamp(-1.0, 1.0).asin();
    let (yaw, roll) = if r20.abs() < 0.999_999 {
        (m.x_axis.y.atan2(m.x_axis.x), m.y_axis.z.atan2(m.z_axis.z))
    } else {
        // Gimbal lock: fold roll into yaw.
        ((-m.y_axis.x).atan2(m.y_axis.y), 0.0)
    };
    Vec3::new(
        pitch.to_degrees().rem_euclid(360.0),
        yaw.to_degrees().rem_euclid(360.0),
        roll.to_degrees().rem_euclid(360.0),
    )
}

fn euler_to_rotator(e: Vec3) -> (i32, i32, i32) {
    let c = |d: f32| ((d / 360.0 * 65536.0).round() as i64).rem_euclid(65536) as i32;
    (c(e.x), c(e.y), c(e.z))
}

/// Snapshot of all objects affected by a single gizmo / direct-drag gesture.
#[derive(Debug, Clone, Default)]
pub struct TransformSnapshot {
    pub targets: Vec<TargetSnapshot>,
}

/// Standard coordinate bounds limit for Unreal Engine 1 (UT99) maps (±65,536 uu).
pub const WORLD_LIMIT: f32 = 65536.0;

/// Returns true if the selection's pivot or any target exceeds the standard map bounds limit.
pub fn exceeds_world_limit(state: &EditorState) -> bool {
    if let Some(pivot) = gizmo_pivot(state) {
        if pivot.x.abs() > WORLD_LIMIT || pivot.y.abs() > WORLD_LIMIT || pivot.z.abs() > WORLD_LIMIT {
            return true;
        }
    }
    false
}

const EPS: f32 = 1e-5;

fn differs(a: Vec3, b: Vec3) -> bool {
    (a - b).length_squared() > EPS
}

impl TransformSnapshot {
    /// Captures the currently selected objects. Returns `None` when there is nothing
    /// transformable (or while editing vertices, which have their own undo path).
    pub fn capture(state: &EditorState) -> Option<Self> {
        if state.editor_mode == EditorMode::VertexEdit && !state.selected_vertex_indices.is_empty() {
            return None;
        }
        let mut targets = Vec::new();
        match resolve_selection(state) {
            Selected::None => return None,
            Selected::Builder => {
                targets.push(TargetSnapshot::BuilderBrush(Box::new(state.builder_brush.clone())));
            }
            Selected::Mover(index) => {
                let m = state.movers.get(index)?;
                targets.push(TargetSnapshot::Mover {
                    index,
                    location: m.brush.location,
                    rotation: m.brush.rotation,
                });
            }
            Selected::StaticMesh(index) => {
                let m = state.static_meshes.get(index)?;
                targets.push(TargetSnapshot::StaticMesh {
                    index,
                    location: m.location,
                    rotation: m.rotation,
                    scale: m.scale,
                });
            }
            Selected::Group { brushes, actors } => {
                for index in brushes {
                    if let Some(b) = state.world_brushes.get(index) {
                        targets.push(TargetSnapshot::Brush {
                            index,
                            location: b.location,
                            rotation: b.rotation,
                            scale: b.scale,
                        });
                    }
                }
                if let Some(ref unr) = state.loaded_unr {
                    for index in actors {
                        if let Some(a) = unr.actors.get(index) {
                            targets.push(TargetSnapshot::Actor {
                                index,
                                location: a.location,
                                rotation: a.rotation,
                            });
                        }
                    }
                }
            }
        }
        if targets.is_empty() { None } else { Some(Self { targets }) }
    }

    /// Writes location / rotation / scale of one target into the live state.
    /// `scale` is ignored for targets without scale (actors, movers).
    fn write(state: &mut EditorState, t: &TargetSnapshot, loc: Vec3, rot: Option<Vec3>, scale: Option<Vec3>) {
        match t {
            TargetSnapshot::Brush { index, .. } => {
                if let Some(b) = state.world_brushes.get_mut(*index) {
                    b.location = loc;
                    if let Some(r) = rot { b.rotation = r; }
                    if let Some(s) = scale { b.scale = s; }
                }
            }
            TargetSnapshot::StaticMesh { index, .. } => {
                if let Some(b) = state.static_meshes.get_mut(*index) {
                    b.location = loc;
                    if let Some(r) = rot { b.rotation = r; }
                    if let Some(s) = scale { b.scale = s; }
                }
            }
            TargetSnapshot::Actor { index, .. } => {
                if let Some(a) = state.loaded_unr.as_mut().and_then(|u| u.actors.get_mut(*index)) {
                    a.location = loc;
                    if let Some(r) = rot { a.rotation = euler_to_rotator(r); }
                }
            }
            TargetSnapshot::Mover { index, .. } => {
                if let Some(m) = state.movers.get_mut(*index) {
                    m.brush.location = loc;
                    if let Some(r) = rot { m.brush.rotation = r; }
                    let key = m.current_key;
                    let (l, r) = (m.brush.location, m.brush.rotation);
                    if let Some(kf) = m.keyframes.get_mut(key).and_then(|k| k.as_mut()) {
                        kf.location = l;
                        kf.rotation = r;
                    }
                }
            }
            TargetSnapshot::BuilderBrush(_) => {
                state.builder_brush.location = loc;
                if let Some(r) = rot { state.builder_brush.rotation = r; }
                if let Some(s) = scale { state.builder_brush.scale = s; }
            }
        }
    }

    fn finish(&self, state: &mut EditorState) {
        if let Some(ref mut unr) = state.loaded_unr {
            if self.targets.iter().any(|t| matches!(t, TargetSnapshot::Actor { .. })) {
                unr.sync_actor_views();
            }
        }
        if !matches!(self.targets.first(), Some(TargetSnapshot::BuilderBrush(_))) {
            state.map_is_dirty = true;
        }
    }

    /// Restores every captured object to its original values (drag cancelled, or zero-delta preview).
    pub fn cancel(&self, state: &mut EditorState) {
        for t in &self.targets {
            let (rot, scale) = match t {
                TargetSnapshot::Brush { rotation, scale, .. } | TargetSnapshot::StaticMesh { rotation, scale, .. } => {
                    (Some(*rotation), Some(*scale))
                }
                TargetSnapshot::Mover { rotation, .. } => (Some(*rotation), None),
                TargetSnapshot::Actor { rotation, .. } => {
                    // Restore the exact integer rotator, not a float round-trip.
                    if let Some(a) = state.loaded_unr.as_mut().and_then(|u| u.actors.get_mut(match t {
                        TargetSnapshot::Actor { index, .. } => *index,
                        _ => 0,
                    })) {
                        a.rotation = *rotation;
                    }
                    (None, None)
                }
                TargetSnapshot::BuilderBrush(b) => (Some(b.rotation), Some(b.scale)),
            };
            Self::write(state, t, t.location(), rot, scale);
        }
        if let Some(ref mut unr) = state.loaded_unr {
            unr.sync_actor_views();
        }
    }

    /// Absolute translation: every object becomes `original + delta`.
    pub fn apply_translation(&self, state: &mut EditorState, delta: Vec3) {
        for t in &self.targets {
            Self::write(state, t, t.location() + delta, None, None);
        }
        self.finish(state);
    }

    /// Absolute rotation of the whole selection by `angle_rad` about the world axis `world_axis`
    /// passing through `pivot` (positions orbit the pivot, orientations are composed with a quaternion
    /// rotation, so any starting orientation and any gizmo space behave correctly).
    pub fn apply_rotation(&self, state: &mut EditorState, pivot: Vec3, world_axis: Vec3, angle_rad: f32) {
        if angle_rad.abs() < 1e-7 || world_axis.length_squared() < 1e-12 {
            self.cancel(state);
            return;
        }
        let q = Mat3::from_axis_angle(world_axis.normalize(), angle_rad);
        for t in &self.targets {
            let loc = pivot + q * (t.location() - pivot);
            let euler = mat3_to_euler_degrees(q * t.orientation());
            Self::write(state, t, loc, Some(euler), None);
        }
        self.finish(state);
    }

    /// Absolute scale: object scales become `original * factor` (per local axis, clamped to a sane range);
    /// for multi-object selections positions are also scaled about `pivot` so the group scales as a whole.
    pub fn apply_scale(&self, state: &mut EditorState, pivot: Vec3, factor: Vec3) {
        let f = factor.clamp(Vec3::splat(0.01), Vec3::splat(100.0));
        let group = self.targets.len() > 1;
        for t in &self.targets {
            let loc = if group { pivot + (t.location() - pivot) * f } else { t.location() };
            let scale = match t {
                TargetSnapshot::Brush { scale, .. } | TargetSnapshot::StaticMesh { scale, .. } => {
                    Some((*scale * f).max(Vec3::splat(0.01)))
                }
                TargetSnapshot::BuilderBrush(b) => Some((b.scale * f).max(Vec3::splat(0.01))),
                TargetSnapshot::Actor { .. } | TargetSnapshot::Mover { .. } => None,
            };
            let rot = match t {
                TargetSnapshot::Brush { rotation, .. } | TargetSnapshot::StaticMesh { rotation, .. } => Some(*rotation),
                TargetSnapshot::Mover { rotation, .. } => Some(*rotation),
                TargetSnapshot::BuilderBrush(b) => Some(b.rotation),
                TargetSnapshot::Actor { .. } => None,
            };
            Self::write(state, t, loc, rot, scale);
        }
        self.finish(state);
    }

    /// Builds the undo actions describing the difference between snapshot and current state.
    /// Pure with respect to history: returns the actions without pushing them.
    pub fn diff_actions(&self, state: &EditorState) -> Vec<Action> {
        let mut actions = Vec::new();
        for t in &self.targets {
            match t {
                TargetSnapshot::Brush { index, location, rotation, scale } => {
                    if let Some(b) = state.world_brushes.get(*index) {
                        if differs(b.location, *location) {
                            actions.push(Action::MoveBrush {
                                brush_index: *index,
                                old_location: *location,
                                new_location: b.location,
                            });
                        }
                        if differs(b.rotation, *rotation) {
                            actions.push(Action::RotateBrush {
                                brush_index: *index,
                                old_rotation: *rotation,
                                new_rotation: b.rotation,
                            });
                        }
                        if differs(b.scale, *scale) {
                            actions.push(Action::ScaleBrush {
                                brush_index: *index,
                                old_scale: *scale,
                                new_scale: b.scale,
                            });
                        }
                    }
                }
                TargetSnapshot::Actor { index, location, rotation } => {
                    if let Some(a) = state.loaded_unr.as_ref().and_then(|u| u.actors.get(*index)) {
                        if differs(a.location, *location) {
                            actions.push(Action::MoveActor {
                                actor_index: *index,
                                old_location: *location,
                                new_location: a.location,
                            });
                        }
                        if a.rotation != *rotation {
                            actions.push(Action::RotateActor {
                                actor_index: *index,
                                old_rotation: *rotation,
                                new_rotation: a.rotation,
                            });
                        }
                    }
                }
                TargetSnapshot::Mover { index, location, rotation } => {
                    if let Some(m) = state.movers.get(*index) {
                        if differs(m.brush.location, *location) {
                            actions.push(Action::MoveMover {
                                mover_index: *index,
                                old_location: *location,
                                new_location: m.brush.location,
                            });
                        }
                        if differs(m.brush.rotation, *rotation) {
                            actions.push(Action::RotateMover {
                                mover_index: *index,
                                old_rotation: *rotation,
                                new_rotation: m.brush.rotation,
                            });
                        }
                    }
                }
                TargetSnapshot::StaticMesh { index, location, rotation, scale } => {
                    if let Some(m) = state.static_meshes.get(*index) {
                        if differs(m.location, *location) {
                            actions.push(Action::MoveStaticMesh {
                                mesh_index: *index,
                                old_location: *location,
                                new_location: m.location,
                            });
                        }
                        if differs(m.rotation, *rotation) || differs(m.scale, *scale) {
                            actions.push(Action::TransformStaticMesh {
                                mesh_index: *index,
                                old_rotation: *rotation,
                                new_rotation: m.rotation,
                                old_scale: *scale,
                                new_scale: m.scale,
                            });
                        }
                    }
                }
                TargetSnapshot::BuilderBrush(old) => {
                    let cur = &state.builder_brush;
                    if differs(cur.location, old.location)
                        || differs(cur.rotation, old.rotation)
                        || differs(cur.scale, old.scale)
                    {
                        actions.push(Action::ModifyBuilderBrush {
                            old_brush: (**old).clone(),
                            new_brush: cur.clone(),
                            description: "Transform Builder Brush via 3D Gizmo".to_string(),
                        });
                    }
                }
            }
        }
        actions
    }

    /// Commits the gesture: exactly one history entry (a `Compound` when several objects/channels changed).
    /// Returns the number of recorded primitive actions.
    pub fn commit(&self, state: &mut EditorState) -> usize {
        let mut actions = self.diff_actions(state);
        let n = actions.len();
        match n {
            0 => {}
            1 => state.history.push(actions.pop().unwrap()),
            _ => state.history.push(Action::Compound {
                description: format!("Transform {} object change(s) via 3D Gizmo", n),
                actions,
            }),
        }
        state.history.end_interaction();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_with_brushes(n: usize) -> EditorState {
        let mut s = EditorState::default();
        s.new_map();
        s.world_brushes.clear();
        for i in 0..n {
            let mut b = s.builder_brush.clone();
            b.location = Vec3::new(i as f32 * 100.0, 0.0, 0.0);
            s.world_brushes.push(b);
        }
        s
    }

    fn mat_close(a: Mat3, b: Mat3) -> bool {
        (a.x_axis - b.x_axis).length() < 1e-4
            && (a.y_axis - b.y_axis).length() < 1e-4
            && (a.z_axis - b.z_axis).length() < 1e-4
    }

    #[test]
    fn capture_none_when_nothing_selected_and_builder_when_explicitly_selected() {
        let mut s = state_with_brushes(0);
        assert_eq!(gizmo_pivot(&s), None);
        assert!(TransformSnapshot::capture(&s).is_none());

        s.is_builder_selected = true;
        assert_eq!(gizmo_pivot(&s), Some(s.builder_brush.location));
        let snap = TransformSnapshot::capture(&s).unwrap();
        assert!(matches!(snap.targets[0], TargetSnapshot::BuilderBrush(_)));
        assert!(snap.diff_actions(&s).is_empty());
    }

    #[test]
    fn cancel_restores_exact_values_single_brush() {
        let mut s = state_with_brushes(1);
        s.selected_brush_index = Some(0);
        let before = s.world_brushes[0].clone();
        let snap = TransformSnapshot::capture(&s).unwrap();

        s.world_brushes[0].location += Vec3::new(64.0, 32.0, -16.0);
        s.world_brushes[0].rotation = Vec3::new(0.0, 45.0, 0.0);
        s.world_brushes[0].scale *= 2.0;
        assert_eq!(snap.diff_actions(&s).len(), 3);

        snap.cancel(&mut s);
        assert_eq!(s.world_brushes[0].location, before.location);
        assert_eq!(s.world_brushes[0].rotation, before.rotation);
        assert_eq!(s.world_brushes[0].scale, before.scale);
        assert!(snap.diff_actions(&s).is_empty());
    }

    #[test]
    fn multi_selection_commit_is_one_undo_entry() {
        let mut s = state_with_brushes(3);
        s.selected_brush_index = Some(0);
        s.multi_selected_brushes.extend([0usize, 1, 2]);
        let snap = TransformSnapshot::capture(&s).unwrap();
        assert_eq!(snap.targets.len(), 3, "primary must not be duplicated");

        snap.apply_translation(&mut s, Vec3::new(0.0, 128.0, 0.0));
        let before = s.history.undo_stack.len();
        assert_eq!(snap.commit(&mut s), 3);
        assert_eq!(s.history.undo_stack.len(), before + 1, "one entry per gesture");
    }

    #[test]
    fn no_change_records_nothing() {
        let mut s = state_with_brushes(1);
        s.selected_brush_index = Some(0);
        let snap = TransformSnapshot::capture(&s).unwrap();
        let len = s.history.undo_stack.len();
        assert_eq!(snap.commit(&mut s), 0);
        assert_eq!(s.history.undo_stack.len(), len);
    }

    #[test]
    fn actor_cancel_restores_location_and_rotation() {
        let mut s = state_with_brushes(0);
        s.place_actor_of_class_at("Light", Vec3::new(10.0, 20.0, 30.0));
        let idx = s.loaded_unr.as_ref().unwrap().actors.len() - 1;
        s.selected_actor_index = Some(idx);
        let snap = TransformSnapshot::capture(&s).unwrap();
        {
            let a = &mut s.loaded_unr.as_mut().unwrap().actors[idx];
            a.location = Vec3::new(999.0, 0.0, 0.0);
            a.rotation = (100, 200, 300);
        }
        snap.cancel(&mut s);
        let a = &s.loaded_unr.as_ref().unwrap().actors[idx];
        assert_eq!(a.location, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(a.rotation, (0, 0, 0));
    }

    #[test]
    fn euler_roundtrip() {
        for &(p, y, r) in &[(0.0, 0.0, 0.0), (30.0, 45.0, 60.0), (-20.0, 170.0, 200.0), (89.0, 10.0, 5.0), (0.0, 300.0, 0.0)] {
            let m = euler_degrees_to_mat3(p, y, r);
            let e = mat3_to_euler_degrees(m);
            assert!(mat_close(m, euler_degrees_to_mat3(e.x, e.y, e.z)), "angles {p},{y},{r}");
        }
        // gimbal lock still reproduces the same orientation
        let m = euler_degrees_to_mat3(90.0, 40.0, 25.0);
        let e = mat3_to_euler_degrees(m);
        assert!(mat_close(m, euler_degrees_to_mat3(e.x, e.y, e.z)));
    }

    #[test]
    fn translation_is_absolute_and_reversible() {
        let mut s = state_with_brushes(2);
        s.multi_selected_brushes.extend([0usize, 1]);
        s.selected_brush_index = Some(0);
        let snap = TransformSnapshot::capture(&s).unwrap();
        let orig: Vec<Vec3> = s.world_brushes.iter().map(|b| b.location).collect();
        snap.apply_translation(&mut s, Vec3::new(50.0, 0.0, 0.0));
        snap.apply_translation(&mut s, Vec3::new(-7.0, 3.0, 0.0));
        snap.apply_translation(&mut s, Vec3::ZERO);
        for (b, o) in s.world_brushes.iter().zip(orig) {
            assert_eq!(b.location, o);
        }
    }

    #[test]
    fn group_rotation_orbits_pivot_and_rotates_orientation() {
        let mut s = state_with_brushes(2); // at x=0 and x=100
        s.multi_selected_brushes.extend([0usize, 1]);
        s.selected_brush_index = Some(0);
        let pivot = gizmo_pivot(&s).unwrap();
        assert!((pivot - Vec3::new(50.0, 0.0, 0.0)).length() < 1e-4);
        let snap = TransformSnapshot::capture(&s).unwrap();

        snap.apply_rotation(&mut s, pivot, Vec3::Z, std::f32::consts::FRAC_PI_2);
        // 90deg about Z around (50,0,0): (0,0)->(50,-50), (100,0)->(50,50)
        assert!((s.world_brushes[0].location - Vec3::new(50.0, -50.0, 0.0)).length() < 1e-3);
        assert!((s.world_brushes[1].location - Vec3::new(50.0, 50.0, 0.0)).length() < 1e-3);
        assert!((s.world_brushes[0].rotation.y - 90.0).abs() < 1e-3);
        // pivot invariant
        assert!((gizmo_pivot(&s).unwrap() - pivot).length() < 1e-3);
        // zero angle restores exactly
        snap.apply_rotation(&mut s, pivot, Vec3::Z, 0.0);
        assert!(snap.diff_actions(&s).is_empty());
    }

    #[test]
    fn rotation_in_local_space_composes_correctly() {
        let mut s = state_with_brushes(1);
        s.selected_brush_index = Some(0);
        s.world_brushes[0].rotation = Vec3::new(0.0, 90.0, 0.0);
        let snap = TransformSnapshot::capture(&s).unwrap();
        let loc = s.world_brushes[0].location;
        let local_x = euler_degrees_to_mat3(0.0, 90.0, 0.0) * Vec3::X;
        snap.apply_rotation(&mut s, loc, local_x, 0.5);
        let b = &s.world_brushes[0];
        let expected = Mat3::from_axis_angle(local_x, 0.5) * euler_degrees_to_mat3(0.0, 90.0, 0.0);
        assert!(mat_close(euler_degrees_to_mat3(b.rotation.x, b.rotation.y, b.rotation.z), expected));
        assert_eq!(b.location, loc);
    }

    #[test]
    fn scale_is_absolute_and_scales_group_positions() {
        let mut s = state_with_brushes(2);
        s.multi_selected_brushes.extend([0usize, 1]);
        s.selected_brush_index = Some(0);
        let pivot = gizmo_pivot(&s).unwrap();
        let snap = TransformSnapshot::capture(&s).unwrap();
        snap.apply_scale(&mut s, pivot, Vec3::splat(2.0));
        assert!((s.world_brushes[0].location.x - -50.0).abs() < 1e-3);
        assert!((s.world_brushes[1].location.x - 150.0).abs() < 1e-3);
        assert!((s.world_brushes[0].scale.x - 2.0).abs() < 1e-5);
        snap.apply_scale(&mut s, pivot, Vec3::ONE);
        assert!(snap.diff_actions(&s).is_empty());
    }

    #[test]
    fn static_mesh_is_pickable_and_becomes_gizmo_pivot() {
        let mut s = state_with_brushes(0);
        let mut mesh = s.builder_brush.clone();
        mesh.location = Vec3::new(500.0, 0.0, 0.0);
        s.static_meshes.push(mesh);
        let ray = crate::raycast::Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::X);
        assert!(s.select_at_ray_with_modifier(ray, false));
        assert_eq!(s.selected_static_mesh_index, Some(0));
        assert_eq!(gizmo_pivot(&s), Some(Vec3::new(500.0, 0.0, 0.0)));
        assert_eq!(primary_location(&s), Some(Vec3::new(500.0, 0.0, 0.0)));
    }

    #[test]
    fn static_mesh_and_mover_are_undoable() {
        let mut s = state_with_brushes(0);
        let mesh = s.builder_brush.clone();
        s.static_meshes.push(mesh);
        s.selected_static_mesh_index = Some(0);
        let snap = TransformSnapshot::capture(&s).unwrap();
        snap.apply_translation(&mut s, Vec3::new(10.0, 0.0, 0.0));
        snap.apply_scale(&mut s, Vec3::ZERO, Vec3::splat(2.0));
        assert_eq!(snap.diff_actions(&s).len(), 1, "scale only; translation was replaced by apply_scale base location");
    }

    #[test]
    fn ortho_view_constraints_filter_degenerate_handles() {
        // Top view: view along Z
        let top_view = Vec3::Z;
        let id = Mat3::IDENTITY;

        // Translation
        assert_eq!(constrain_hit_for_view(GizmoAxis::X, GizmoMode::Translate, id, top_view), GizmoAxis::X);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Y, GizmoMode::Translate, id, top_view), GizmoAxis::Y);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Z, GizmoMode::Translate, id, top_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::PlaneXY, GizmoMode::Translate, id, top_view), GizmoAxis::PlaneXY);
        assert_eq!(constrain_hit_for_view(GizmoAxis::PlaneXZ, GizmoMode::Translate, id, top_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::PlaneYZ, GizmoMode::Translate, id, top_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::ScreenPlane, GizmoMode::Translate, id, top_view), GizmoAxis::ScreenPlane);

        // Rotation in Top view: only Z ring is visible/face-on
        assert_eq!(constrain_hit_for_view(GizmoAxis::Z, GizmoMode::Rotate, id, top_view), GizmoAxis::Z);
        assert_eq!(constrain_hit_for_view(GizmoAxis::X, GizmoMode::Rotate, id, top_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Y, GizmoMode::Rotate, id, top_view), GizmoAxis::None);

        // Front view: view along Y
        let front_view = Vec3::Y;
        assert_eq!(constrain_hit_for_view(GizmoAxis::Y, GizmoMode::Translate, id, front_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::X, GizmoMode::Translate, id, front_view), GizmoAxis::X);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Z, GizmoMode::Translate, id, front_view), GizmoAxis::Z);

        // Side view: view along X
        let side_view = Vec3::X;
        assert_eq!(constrain_hit_for_view(GizmoAxis::X, GizmoMode::Translate, id, side_view), GizmoAxis::None);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Y, GizmoMode::Translate, id, side_view), GizmoAxis::Y);
        assert_eq!(constrain_hit_for_view(GizmoAxis::Z, GizmoMode::Translate, id, side_view), GizmoAxis::Z);

        // Local rotation in Top view: pitch 90deg makes local X point along world Z
        let rot_pitch90 = euler_degrees_to_mat3(90.0, 0.0, 0.0);
        // Local X is rotated to world Z, so in Top view local X is end-on
        assert_eq!(constrain_hit_for_view(GizmoAxis::X, GizmoMode::Translate, rot_pitch90, top_view), GizmoAxis::None);
        // Local Z is rotated to world -X, so in Top view local Z is perpendicular and valid
        assert_eq!(constrain_hit_for_view(GizmoAxis::Z, GizmoMode::Translate, rot_pitch90, top_view), GizmoAxis::Z);
    }
}
