use super::*;
use crate::history::Action;


/// Track only writes to document data. Builder tools and studio previews deliberately
/// use ordinary assignments in the exhaustive action handlers below.
fn replace_history_value<T: PartialEq>(target: &mut T, value: T, dirty: &mut bool) {
    *dirty |= *target != value;
    *target = value;
}

fn replace_history_tracked_vec<T: PartialEq + Clone>(
    target: &mut TrackedVec<T>,
    value: &[T],
    dirty: &mut bool,
) {
    *dirty |= target.as_slice() != value;
    target.replace(value.to_vec());
}

trait HistoryVecTarget<T> {
    fn history_len(&self) -> usize;
    fn history_with_mut<R>(&mut self, change: impl FnOnce(&mut Vec<T>) -> R) -> R;
}

impl<T> HistoryVecTarget<T> for Vec<T> {
    #[inline]
    fn history_len(&self) -> usize {
        self.len()
    }
    #[inline]
    fn history_with_mut<R>(&mut self, change: impl FnOnce(&mut Vec<T>) -> R) -> R {
        change(self)
    }
}

impl<T: Clone> HistoryVecTarget<T> for TrackedVec<T> {
    #[inline]
    fn history_len(&self) -> usize {
        self.len()
    }
    #[inline]
    fn history_with_mut<R>(&mut self, change: impl FnOnce(&mut Vec<T>) -> R) -> R {
        change(self.inner_mut())
    }
}

fn history_vec_change<T, Target: HistoryVecTarget<T>, R>(
    items: &mut Target,
    dirty: &mut bool,
    change: impl FnOnce(&mut Vec<T>) -> R,
) -> R {
    let previous_len = items.history_len();
    let result = items.history_with_mut(change);
    *dirty |= items.history_len() != previous_len;
    result
}

fn history_property_change<K: Eq + std::hash::Hash, V: PartialEq>(
    properties: &mut std::collections::HashMap<K, V>, dirty: &mut bool, key: K, value: V,
) -> Option<V> {
    *dirty |= properties.get(&key) != Some(&value);
    properties.insert(key, value)
}

impl EditorState {
    pub fn undo(&mut self) {
        if let Some(action) = self.history.undo_stack.pop() {
            let desc = action.description().to_string();
            self.apply_undo_action(&action);
            self.toast_info(format!("Undo: {desc}"));
            self.history.redo_stack.push(action);
        } else {
            self.log("Nothing to undo.");
            self.toast_warning("Nothing to undo");
        }
    }

    pub fn next_undo_description(&self) -> Option<&str> {
        self.history.next_undo_description()
    }

    pub fn next_redo_description(&self) -> Option<&str> {
        self.history.next_redo_description()
    }

    pub fn apply_undo_action(&mut self, action: &Action) {
        match action {
            Action::ClearAiPaths { old_graph, old_links, old_reach_specs, old_report } => {
                replace_history_value(&mut self.nav_graph, old_graph.clone(), &mut self.map_is_dirty);
                replace_history_tracked_vec(&mut self.ai_path_links, old_links, &mut self.map_is_dirty);
                if let (Some(level), Some(reach_specs)) = (&mut self.loaded_unr, old_reach_specs) {
                    replace_history_value(&mut level.reach_specs, reach_specs.clone(), &mut self.map_is_dirty);
                }
                replace_history_value(&mut self.last_autopath_report, old_report.clone(), &mut self.map_is_dirty);

                self.log("Undo: Clear AI Paths");
            }
            Action::Compound { actions, description } => {
                for sub in actions.iter().rev() {
                    self.apply_undo_action(sub);
                }
                self.log(format!("Undo: {}", description));
            }
            Action::MoveBrush { brush_index, old_location, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.location, *old_location, &mut self.map_is_dirty);
                        self.log(format!("Undo: Moved brush {} back to {:?}", brush_index, old_location));
                    }
                }
                Action::RotateBrush { brush_index, old_rotation, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.rotation, *old_rotation, &mut self.map_is_dirty);
                        self.log(format!("Undo: Rotated brush {} back to {:?}", brush_index, old_rotation));
                    }
                }
                Action::ScaleBrush { brush_index, old_scale, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.scale, *old_scale, &mut self.map_is_dirty);
                        self.log(format!("Undo: Scaled brush {} back to {:?}", brush_index, old_scale));
                    }
                }
                Action::ModifyBrushGeometry { brush_index, old_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.polys, old_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Undo: Reverted {} on brush {}", description, brush_index));
                    }
                }
                Action::ModifyBrushPivot { brush_index, old_location, old_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.location, *old_location, &mut self.map_is_dirty);
                        replace_history_value(&mut b.polys, old_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Undo: Reverted {} on brush {}", description, brush_index));
                    }
                }
                Action::MoveActor { actor_index, old_location, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut a.location, *old_location, &mut self.map_is_dirty);
                            self.log(format!("Undo: Moved actor {} back to {:?}", actor_index, old_location));
                        }
                    }
                }
                Action::RotateActor { actor_index, old_rotation, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut a.rotation, *old_rotation, &mut self.map_is_dirty);
                            self.log(format!("Undo: Rotated actor {} back to {:?}", actor_index, old_rotation));
                        }
                    }
                }
                Action::AddActor { .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.pop()) {
                            self.log(format!("Undo: Removed added actor '{}'", a.name));
                        }
                    }
                }
                Action::DeleteActor { index, actor } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let idx = (*index).min(unr.actors.len());
                        history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(idx, actor.clone()));
                        self.log(format!("Undo: Restored deleted actor '{}'", actor.name));
                    }
                }
                Action::AddBrush { .. } => {
                    if let Some(b) = history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.pop()) {
                        self.log(format!("Undo: Removed added brush '{}'", b.name));
                    }
                }
                Action::DeleteBrush { index, brush } => {
                    let idx = (*index).min(self.world_brushes.len());
                    history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(idx, brush.clone()));
                    self.log(format!("Undo: Restored deleted brush '{}'", brush.name));
                }
                Action::MoveMover { mover_index, old_location, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.brush.location, *old_location, &mut self.map_is_dirty);
                        let cur_key = m.current_key;
                        if let Some(ref mut kf) = m.keyframes[cur_key] {
                            replace_history_value(&mut kf.location, *old_location, &mut self.map_is_dirty);
                        }
                        self.log(format!("Undo: Moved mover {} back to {:?}", mover_index, old_location));
                    }
                }
                Action::RotateMover { mover_index, old_rotation, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.brush.rotation, *old_rotation, &mut self.map_is_dirty);
                        let cur_key = m.current_key;
                        if let Some(ref mut kf) = m.keyframes[cur_key] {
                            replace_history_value(&mut kf.rotation, *old_rotation, &mut self.map_is_dirty);
                        }
                        self.log(format!("Undo: Rotated mover {} back to {:?}", mover_index, old_rotation));
                    }
                }
                Action::TransformStaticMesh { mesh_index, old_rotation, old_scale, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut m.rotation, *old_rotation, &mut self.map_is_dirty);
                        replace_history_value(&mut m.scale, *old_scale, &mut self.map_is_dirty);
                        self.log(format!("Undo: Restored static mesh {} rotation/scale", mesh_index));
                    }
                }
                Action::AddMover { .. } => {
                    if let Some(m) = history_vec_change(&mut self.movers, &mut self.map_is_dirty, |items| items.pop()) {
                        self.log(format!("Undo: Removed added mover '{}'", m.name));
                    }
                }
                Action::DeleteMover { index, mover } => {
                    let idx = (*index).min(self.movers.len());
                    history_vec_change(&mut self.movers, &mut self.map_is_dirty, |items| items.insert(idx, mover.clone()));
                    self.log(format!("Undo: Restored deleted mover '{}'", mover.name));
                }
                Action::MoveStaticMesh { mesh_index, old_location, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut m.location, *old_location, &mut self.map_is_dirty);
                        self.log(format!("Undo: Moved static mesh {} back to {:?}", mesh_index, old_location));
                    }
                }
                Action::AddStaticMesh { .. } => {
                    if let Some(m) = history_vec_change(&mut self.static_meshes, &mut self.map_is_dirty, |items| items.pop()) {
                        self.log(format!("Undo: Removed added static mesh '{}'", m.name));
                    }
                }
                Action::DeleteStaticMesh { index, mesh } => {
                    let idx = (*index).min(self.static_meshes.len());
                    history_vec_change(&mut self.static_meshes, &mut self.map_is_dirty, |items| items.insert(idx, mesh.clone()));
                    self.log(format!("Undo: Restored deleted static mesh '{}'", mesh.name));
                }
                Action::SetSurfaceTexture { brush_index, poly_index, old_texture, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        if let Some(p) = b.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, old_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Undo: Restored texture on brush {} poly {} to '{}'", brush_index, poly_index, old_texture));
                        }
                    }
                }
                Action::BatchSetSurfaceTextures { modifications } => {
                    for (brush_idx, poly_idx, old_texture, _) in modifications {
                        if let Some(b) = self.world_brushes.get_mut(*brush_idx) {
                            if let Some(p) = b.polys.get_mut(*poly_idx) {
                                replace_history_value(&mut p.texture, old_texture.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }
                    self.log(format!("Undo: Restored previous textures on {} surfaces", modifications.len()));
                }
                Action::SetMoverSurfaceTexture { mover_index, poly_index, old_texture, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        if let Some(p) = m.brush.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, old_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Undo: Restored texture on mover {} poly {} to '{}'", mover_index, poly_index, old_texture));
                        }
                    }
                }
                Action::SetStaticMeshSurfaceTexture { mesh_index, poly_index, old_texture, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        if let Some(p) = m.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, old_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Undo: Restored texture on static mesh {} poly {} to '{}'", mesh_index, poly_index, old_texture));
                        }
                    }
                }
                Action::AddProjector { index, .. } => {
                    if *index < self.projectors.len() {
                        history_vec_change(&mut self.projectors, &mut self.map_is_dirty, |items| items.remove(*index));
                        self.selected_projector_index = None;
                        self.bake_projector_decals();
                        self.log(format!("Undo: Removed stamped decal projector #{}", index));
                    }
                }
                Action::DeleteProjector { index, projector } => {
                    let idx = (*index).min(self.projectors.len());
                    history_vec_change(&mut self.projectors, &mut self.map_is_dirty, |items| items.insert(idx, projector.clone()));
                    self.selected_projector_index = Some(idx);
                    self.bake_projector_decals();
                    self.log(format!("Undo: Restored deleted decal projector '{}'", projector.name));
                }
                Action::ModifyBuilderBrush { old_brush, description, .. } => {
                    self.builder_brush = old_brush.clone();
                    self.log(format!("Undo: Reverted {} on Builder Brush", description));
                }
                Action::ReorderBrushes { old_brushes, old_selection, description, .. } => {
                    replace_history_tracked_vec(&mut self.world_brushes, old_brushes, &mut self.map_is_dirty);
                    self.selected_brush_index = *old_selection;
                    self.sync_brush_groups_from_brushes();
                    self.log(format!("Undo: Reverted {} (restored original brush order)", description));
                }
                Action::TransformBrush { brush_index, old_rotation, old_scale, old_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.rotation, *old_rotation, &mut self.map_is_dirty);
                        replace_history_value(&mut b.scale, *old_scale, &mut self.map_is_dirty);
                        replace_history_value(&mut b.polys, old_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Undo: Reverted {} on brush {}", description, brush_index));
                    }
                }
                Action::ScaleEntireLevel { old_brushes, old_actors, old_movers, factor, .. } => {
                    replace_history_tracked_vec(&mut self.world_brushes, old_brushes, &mut self.map_is_dirty);
                    if let (Some(ref mut unr), Some(actors)) = (&mut self.loaded_unr, old_actors) {
                        replace_history_tracked_vec(&mut unr.actors, actors, &mut self.map_is_dirty);
                    }
                    replace_history_tracked_vec(&mut self.movers, old_movers, &mut self.map_is_dirty);
                    self.log(format!("Undo: Reverted Scale Entire Map ({:.2}, {:.2}, {:.2})", factor.x, factor.y, factor.z));
                }
                Action::PropagateActorProperties { class_name, old_actors, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        replace_history_tracked_vec(&mut unr.actors, old_actors, &mut self.map_is_dirty);
                    }
                    self.log(format!("Undo: Reverted {} on class '{}'", description, class_name));
                }
                Action::UpdateZoneProperties { actor_tag, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *actor_tag) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    self.log(format!("Undo: Reverted {} on zone '{}'", description, actor_tag));
                }
                Action::UpdateSoundEmitter { actor_tag, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *actor_tag || a.name == *actor_tag) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    self.log(format!("Undo: Reverted {} on sound emitter '{}'", description, actor_tag));
                }
                Action::UpdateWarpZonePair { source_tag, target_tag, old_source, old_target, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *source_tag || a.name == *source_tag) {
                            replace_history_value(&mut *a, old_source.clone(), &mut self.map_is_dirty);
                        }
                        if let Some(old_t) = old_target {
                            if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *target_tag || a.name == *target_tag) {
                                replace_history_value(&mut *a, old_t.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }
                    self.log(format!("Undo: Reverted {} on WarpZones '{}' / '{}'", description, source_tag, target_tag));
                }
                Action::BakeLighting { old_polys, description, .. } => {
                    for (b_idx, polys) in old_polys {
                        if let Some(brush) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut brush.polys, polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on {} brushes", description, old_polys.len()));
                }
                Action::UpdateZoneFog { actor_index, old_actor, old_viewport_fog, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    let (en, st, ed, den, col) = *old_viewport_fog;
                    self.distance_fog_enabled = en;
                    self.fog_start = st;
                    self.fog_end = ed;
                    self.fog_density = den;
                    self.fog_color = col;

                    self.log(format!("Undo: Reverted {} on actor index {}", description, actor_index));
                }
                Action::UpdateDispatcherSequence { actor_name, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.name == *actor_name) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on Dispatcher '{}'", description, actor_name));
                }
                Action::UpdateSkeletalMesh { mesh_index, old_mesh, description, .. } => {
                    if let Some(m) = self.skeletal_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut *m, old_mesh.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Undo: Reverted {} on SkeletalMesh #{}", description, mesh_index));
                }
                Action::UpdateParticleEmitter { emitter_index, old_emitter, description, .. } => {
                    if let Some(e) = self.particle_emitters.get_mut(*emitter_index) {
                        replace_history_value(&mut *e, old_emitter.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Undo: Reverted {} on ParticleEmitter #{}", description, emitter_index));
                }
                Action::UpdateLightEffects { actor_index, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on Light #{}", description, actor_index));
                }
                Action::BatchUpdateLightEffects { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, old_actor, _) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Undo: Reverted {} across {} light(s)", description, modifications.len()));
                }
                Action::AddAntiPortal { .. } => {
                    if let Some(ap) = history_vec_change(&mut self.antiportals, &mut self.map_is_dirty, |items| items.pop()) {

                        self.log(format!("Undo: Removed added AntiPortal '{}'", ap.name));
                    }
                }
                Action::UpdateAntiPortal { antiportal_index, old_antiportal, description, .. } => {
                    if let Some(ap) = self.antiportals.get_mut(*antiportal_index) {
                        replace_history_value(&mut *ap, old_antiportal.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Undo: Reverted {} on AntiPortal #{}", description, antiportal_index));
                }
                Action::UpdateMusicEvent { actor_index, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on MusicEvent #{}", description, actor_index));
                }
                Action::SetLevelMusic { old_song, old_section, description, .. } => {
                    replace_history_value(&mut self.level_song, old_song.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_song_section, *old_section, &mut self.map_is_dirty);
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(info) = unr.actors.iter_mut().find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo")) {
                            history_property_change(&mut info.properties, &mut self.map_is_dirty, "Song".to_string(), ut99_format::TaggedProperty {
                                name: "Song".to_string(),
                                array_index: 0,
                                value: ut99_format::TaggedValue::Str(old_song.clone()),
                            });
                            history_property_change(&mut info.properties, &mut self.map_is_dirty, "SongSection".to_string(), ut99_format::TaggedProperty {
                                name: "SongSection".to_string(),
                                array_index: 0,
                                value: ut99_format::TaggedValue::Byte(*old_section),
                            });
                        }
                    }

                    self.log(format!("Undo: Reverted {} to '{}' (Section {})", description, old_song, old_section));
                }
                Action::UpdateLightCorona { actor_index, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on Light #{}", description, actor_index));
                }
                Action::BatchUpdateLightCoronas { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, old_actor, _) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Undo: Reverted {} across {} light(s)", description, modifications.len()));
                }
                Action::UpdateZonePhysics { actor_index, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: Reverted {} on ZoneInfo #{}", description, actor_index));
                }
                Action::BatchUpdateZonePhysics { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, old_actor, _) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Undo: Reverted {} across {} zone(s)", description, modifications.len()));
                }
                Action::AddBallisticProbe { probe_index, name, .. } => {
                    if *probe_index < self.ballistics_studio_settings.probes.len() {
                        self.ballistics_studio_settings.probes.remove(*probe_index);
                    }
                    self.calculate_active_weapon_trajectory();
                    self.log(format!("Undo: Removed ballistic probe '{}'", name));
                }
                Action::DeleteBallisticProbe { probe_index, name, location, pitch, yaw, preset } => {
                    let probe = BallisticProbeData {
                        id: *probe_index,
                        name: name.clone(),
                        location: *location,
                        pitch_deg: *pitch,
                        yaw_deg: *yaw,
                        preset: *preset,
                    };
                    let idx = (*probe_index).min(self.ballistics_studio_settings.probes.len());
                    self.ballistics_studio_settings.probes.insert(idx, probe);
                    self.calculate_active_weapon_trajectory();
                    self.log(format!("Undo: Restored ballistic probe '{}'", name));
                }
                Action::AddTeleporterPair { actor_index_a, actor_index_b, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let max_idx = (*actor_index_a).max(*actor_index_b);
                        let min_idx = (*actor_index_a).min(*actor_index_b);
                        if max_idx < unr.actors.len() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(max_idx));
                        }
                        if min_idx < unr.actors.len() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(min_idx));
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTeleporterConfig { actor_index, old_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, old_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::AddTranslocatorProbe { probe_index, name, .. } => {
                    if *probe_index < self.translocator_studio_settings.probes.len() {
                        self.translocator_studio_settings.probes.remove(*probe_index);
                    }
                    self.calculate_active_translocator_simulation();
                    self.log(format!("Undo: Removed translocator probe '{}'", name));
                }
                Action::DeleteTranslocatorProbe { probe_index, name, location, pitch, yaw, preset } => {
                    let probe = TranslocatorProbeData {
                        id: *probe_index,
                        name: name.clone(),
                        location: *location,
                        pitch_deg: *pitch,
                        yaw_deg: *yaw,
                        preset: *preset,
                    };
                    let idx = (*probe_index).min(self.translocator_studio_settings.probes.len());
                    self.translocator_studio_settings.probes.insert(idx, probe);
                    self.calculate_active_translocator_simulation();
                    self.log(format!("Undo: Restored translocator probe '{}'", name));
                }
                Action::AddElevatorPathingSet { added_actor_indices, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = added_actor_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for &idx in &sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.collect_elevator_audit();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateElevatorMoverConfig { mover_index, old_mover, description, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut *m, old_mover.clone(), &mut self.map_is_dirty);
                    }

                    self.collect_elevator_audit();
                    self.log(format!("Undo: {}", description));
                }
                Action::AddWaterSheetBrush { brush_index, description, .. } => {
                    if *brush_index < self.world_brushes.len() {
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*brush_index));
                    } else if !self.world_brushes.is_empty() {
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.pop());
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateWaterSurfaceConfig { old_config, old_preset, description, .. } => {
                    self.water_surface_studio_settings.active_config = old_config.clone();
                    self.water_surface_studio_settings.selected_preset = *old_preset;
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateMoverSequenceConfig { mover_index, old_config, old_template, description, .. } => {
                    self.mover_studio_settings.active_config = old_config.clone();
                    self.mover_studio_settings.selected_template = *old_template;
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.name, old_config.mover_name.clone(), &mut self.map_is_dirty);
                        replace_history_value(&mut m.tag, old_config.mover_tag.clone(), &mut self.map_is_dirty);
                    }
                    self.audit_active_mover_hazards();
                    self.log(format!("Undo: {}", description));
                }
                Action::AddDoorTriggerActor { actor_index, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if *actor_index < unr.actors.len() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*actor_index));
                        } else if !unr.actors.is_empty() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.pop());
                        }
                    }

                    self.audit_active_mover_hazards();
                    self.log(format!("Undo: {}", description));
                }
                Action::AddDefensePointSet { added_actor_indices, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = added_actor_indices.clone();
                        sorted_indices.sort_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.scan_and_refresh_team_objectives();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTeamObjectiveSetup { old_mode, old_preset, description, .. } => {
                    self.team_objective_settings.active_game_mode = *old_mode;
                    self.team_objective_settings.selected_preset = *old_preset;
                    self.scan_and_refresh_team_objectives();
                    self.log(format!("Undo: {}", description));
                }
                Action::AddDestructibleBarrier { brush_index, added_actor_indices, description, .. } => {
                    if *brush_index < self.world_brushes.len() {
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*brush_index));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = added_actor_indices.clone();
                        sorted_indices.sort_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.audit_destructible_hazards();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateDestructibleConfig { old_config, old_archetype, description, .. } => {
                    self.destructible_studio_settings.active_config = old_config.clone();
                    self.destructible_studio_settings.selected_archetype = *old_archetype;
                    self.destructible_studio_settings.selected_pattern = old_config.fracture_pattern;
                    self.regenerate_destructible_shards();
                    self.audit_destructible_hazards();
                    self.log(format!("Undo: {}", description));
                }
                Action::AddJumpPadSetup { kicker_index, jump_spot_index, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut to_remove = Vec::new();
                        to_remove.push(*kicker_index);
                        if let Some(js_idx) = jump_spot_index {
                            to_remove.push(*js_idx);
                        }
                        to_remove.sort_by(|a, b| b.cmp(a));
                        for idx in to_remove {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_kicker_trajectory();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateKickerConfig { old_config, old_archetype, description, .. } => {
                    self.kicker_studio_settings.active_config = old_config.clone();
                    self.kicker_studio_settings.selected_archetype = *old_archetype;
                    self.recalculate_kicker_trajectory();
                    self.log(format!("Undo: {}", description));
                }
                Action::PurgeSliverPolygons { modifications, description } => {
                    for (b_idx, old_polys, _) in modifications {
                        if let Some(b) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut b.polys, old_polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.run_bsp_quality_audit();
                    self.log(format!("Undo: {}", description));
                }
                Action::BatchConvertSemiSolid { conversions, description } => {
                    for (b_idx, old_polys, _) in conversions {
                        if let Some(b) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut b.polys, old_polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.run_bsp_quality_audit();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateLoftConfig { old_config, old_keyframes, description, .. } => {
                    self.loft_studio_settings.active_config = old_config.clone();
                    self.loft_studio_settings.path_keyframes = old_keyframes.clone();
                    self.recalculate_loft_mesh();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitLoftedBrush { brush_index, is_builder_brush, old_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        if let Some(ref ob) = old_builder_brush {
                            self.builder_brush = ob.clone();
                        }
                    } else if let Some(idx) = brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateStairConfig { old_config, description, .. } => {
                    self.stairway_studio_settings.active_config = old_config.clone();
                    self.recalculate_stair_geometry();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitStairSetup { stair_brush_index, ramp_brush_index, is_builder_brush, old_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        if let Some(ref ob) = old_builder_brush {
                            self.builder_brush = ob.clone();
                        }
                    } else {
                        let mut to_remove = Vec::new();
                        if let Some(idx) = stair_brush_index {
                            to_remove.push(*idx);
                        }
                        if let Some(idx) = ramp_brush_index {
                            to_remove.push(*idx);
                        }
                        to_remove.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in to_remove {
                            if idx < self.world_brushes.len() {
                                history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateScatterConfig { old_config, description, .. } => {
                    self.scatter_studio_settings.active_config = old_config.clone();
                    self.recalculate_scatter_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitScatterBatch { added_actor_indices, description, .. } => {
                    let mut sorted = added_actor_indices.clone();
                    sorted.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in sorted {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }
                    self.selected_actor_index = None;

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateLightShaftConfig { old_config, description, .. } => {
                    self.light_shaft_settings.active_config = old_config.clone();
                    self.recalculate_light_shaft_geometry();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitLightShaftBrush { brush_index, is_builder_brush, old_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        if let Some(ref ob) = old_builder_brush {
                            self.builder_brush = ob.clone();
                        }
                    } else if let Some(idx) = brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTrackConfig { old_config, description, .. } => {
                    self.track_studio_settings.active_config = old_config.clone();
                    self.recalculate_track_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitTrackSetup { rails_brush_index, ties_brush_index, mover_actor_index, waypoint_actor_indices, description, .. } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = rails_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = ties_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = waypoint_actor_indices.clone();
                    if let Some(idx) = mover_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSurveillanceConfig { old_config, description, .. } => {
                    self.surveillance_settings.active_config = old_config.clone();
                    self.recalculate_surveillance_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSurveillanceSetup { monitor_brush_index, camera_actor_index, screen_actor_index, description, .. } => {
                    if let Some(idx) = monitor_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    let mut actor_indices = Vec::new();
                    if let Some(idx) = camera_actor_index {
                        actor_indices.push(*idx);
                    }
                    if let Some(idx) = screen_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateLaserConfig { old_config, description, .. } => {
                    self.laser_grid_settings.active_config = old_config.clone();
                    self.recalculate_laser_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitLaserGrid {
                    beam_brush_index,
                    diode_brush_index,
                    trigger_actor_indices,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = beam_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = diode_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }
                    let mut actor_indices = trigger_actor_indices.clone();
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateVentilatorConfig { old_config, description, .. } => {
                    self.ventilator_settings.active_config = old_config.clone();
                    self.recalculate_ventilator_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitVentilatorSetup {
                    fan_brush_index,
                    housing_brush_index,
                    mover_actor_index,
                    zone_actor_index,
                    trigger_actor_index,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = fan_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = housing_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = Vec::new();
                    if let Some(idx) = mover_actor_index {
                        actor_indices.push(*idx);
                    }
                    if let Some(idx) = zone_actor_index {
                        actor_indices.push(*idx);
                    }
                    if let Some(idx) = trigger_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateConveyorConfig { old_config, description, .. } => {
                    self.conveyor_settings.active_config = old_config.clone();
                    self.recalculate_conveyor_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitConveyorSetup {
                    deck_brush_index,
                    rollers_brush_index,
                    frame_brush_index,
                    zone_actor_index,
                    waypoint_actor_indices,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = deck_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = rollers_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = frame_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = waypoint_actor_indices.clone();
                    if let Some(idx) = zone_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTurretConfig { old_config, description, .. } => {
                    self.turret_settings.active_config = old_config.clone();
                    self.recalculate_turret_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitTurretSetup {
                    mount_brush_index,
                    shield_brush_index,
                    turret_actor_index,
                    waypoint_actor_indices,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = mount_brush_index {
                        brush_indices.push(*idx);
                    }
                    if let Some(idx) = shield_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = waypoint_actor_indices.clone();
                    if let Some(idx) = turret_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSoundscapeConfig { old_config, description, .. } => {
                    self.soundscape_settings.active_config = old_config.clone();
                    self.recalculate_soundscape_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSoundscapeSetup {
                    zone_brush_index,
                    ambient_actor_index,
                    zone_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = zone_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }

                    let mut actor_indices = Vec::new();
                    if let Some(idx) = ambient_actor_index {
                        actor_indices.push(*idx);
                    }
                    if let Some(idx) = zone_actor_index {
                        actor_indices.push(*idx);
                    }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateHazardChamberConfig { old_config, description, .. } => {
                    self.hazard_chamber_settings.active_config = old_config.clone();
                    self.recalculate_hazard_chamber_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitHazardChamberSetup {
                    chamber_brush_index,
                    door_l_brush_index,
                    door_r_brush_index,
                    window_brush_index,
                    console_brush_index,
                    zone_actor_index,
                    trigger_actor_index,
                    waypoint_actor_indices,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = chamber_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = door_l_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = door_r_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = window_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = console_brush_index { brush_indices.push(*idx); }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = waypoint_actor_indices.clone();
                    if let Some(idx) = zone_actor_index { actor_indices.push(*idx); }
                    if let Some(idx) = trigger_actor_index { actor_indices.push(*idx); }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdatePipeConduitConfig { old_config, description, .. } => {
                    self.pipe_conduit_settings.active_config = old_config.clone();
                    self.recalculate_pipe_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitPipeConduitSetup {
                    straight_brush_index,
                    elbow_brush_index,
                    flange_brush_indices,
                    collar_brush_index,
                    valve_body_brush_index,
                    valve_wheel_brush_index,
                    valve_mover_actor_index,
                    hazard_trigger_actor_index,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = straight_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = elbow_brush_index { brush_indices.push(*idx); }
                    brush_indices.extend(flange_brush_indices.iter().copied());
                    if let Some(idx) = collar_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = valve_body_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = valve_wheel_brush_index { brush_indices.push(*idx); }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = Vec::new();
                    if let Some(idx) = valve_mover_actor_index { actor_indices.push(*idx); }
                    if let Some(idx) = hazard_trigger_actor_index { actor_indices.push(*idx); }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateHologramConfig { old_config, description, .. } => {
                    self.hologram_settings.active_config = old_config.clone();
                    self.recalculate_hologram_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitHologramSetup {
                    display_brush_index,
                    pedestal_brush_index,
                    projection_cone_brush_index,
                    rotator_mover_actor_index,
                    projector_light_actor_index,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = display_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = pedestal_brush_index { brush_indices.push(*idx); }
                    if let Some(idx) = projection_cone_brush_index { brush_indices.push(*idx); }
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }

                    let mut actor_indices = Vec::new();
                    if let Some(idx) = rotator_mover_actor_index { actor_indices.push(*idx); }
                    if let Some(idx) = projector_light_actor_index { actor_indices.push(*idx); }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateWeatherConfig { old_config, description, .. } => {
                    self.weather_settings.active_config = old_config.clone();
                    self.recalculate_weather_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitWeatherSetup {
                    volume_brush_index,
                    zone_emitter_actor_index,
                    lightning_light_actor_index,
                    thunder_audio_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = volume_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }

                    let mut actor_indices = Vec::new();
                    if let Some(idx) = zone_emitter_actor_index { actor_indices.push(*idx); }
                    if let Some(idx) = lightning_light_actor_index { actor_indices.push(*idx); }
                    if let Some(idx) = thunder_audio_actor_index { actor_indices.push(*idx); }
                    actor_indices.sort_unstable_by(|a, b| b.cmp(a));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for idx in actor_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateArenaRulesConfig { old_config, description, .. } => {
                    self.arena_rules_settings.active_config = old_config.clone();
                    self.recalculate_arena_rules_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitArenaRulesSetup {
                    level_info_actor_index,
                    old_level_info,
                    old_level_game_type,
                    mutator_actor_indices,
                    description,
                    ..
                } => {
                    replace_history_value(&mut self.level_game_type, old_level_game_type.clone(), &mut self.map_is_dirty);
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (level_info_actor_index, old_level_info) {
                            if *idx < unr.actors.len() {
                                unr.actors[*idx] = act.clone();
                            }
                        }
                        let mut sorted_indices = mutator_actor_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_arena_rules_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateArmorStackingConfig { old_config, description, .. } => {
                    self.armor_stacking_settings.active_config = old_config.clone();
                    self.recalculate_armor_stacking_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitArmorStackingSetup {
                    pedestal_brush_index,
                    pickup_actor_indices,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = pickup_actor_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_armor_stacking_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateBotJumpRoutesConfig { old_config, description, .. } => {
                    self.bot_jump_routes_settings.active_config = old_config.clone();
                    self.recalculate_jump_route_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitBotJumpRouteSetup {
                    jump_plate_brush_index,
                    actor_indices,
                    description,
                    ..
                } => {
                    if let Some(idx) = jump_plate_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = actor_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_jump_route_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateWeaponCircuitConfig { old_config, description, .. } => {
                    self.weapon_circuit_settings.active_config = old_config.clone();
                    self.recalculate_weapon_circuit_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitWeaponCircuitSetup {
                    altar_brush_index,
                    actor_indices,
                    description,
                    ..
                } => {
                    if let Some(idx) = altar_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut sorted_indices = actor_indices.clone();
                        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
                        for idx in sorted_indices {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_weapon_circuit_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdatePowerupControlConfig { old_config, description, .. } => {
                    self.powerup_control_settings.active_config = old_config.clone();
                    self.recalculate_powerup_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitPowerupSetup {
                    altar_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = altar_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_powerup_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSpawnDistributionConfig { old_config, description, .. } => {
                    self.spawn_distribution_settings.active_config = old_config.clone();
                    self.recalculate_spawn_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSpawnPointSetup {
                    pad_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = pad_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_spawn_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateDominationConfig { old_config, description, .. } => {
                    self.domination_settings.active_config = old_config.clone();
                    self.recalculate_domination_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitDominationPointSetup {
                    pedestal_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_domination_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateAssaultConfig { old_config, description, .. } => {
                    self.assault_settings.active_config = old_config.clone();
                    self.recalculate_assault_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitAssaultObjectiveSetup {
                    door_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = door_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_assault_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateRadarConfig { old_config, description, .. } => {
                    self.radar_settings.active_config = old_config.clone();
                    self.recalculate_radar_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitRadarStationSetup {
                    mast_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = mast_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_radar_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateCtfBaseConfig { old_config, description, .. } => {
                    self.ctf_base_settings.active_config = old_config.clone();
                    self.recalculate_ctf_base_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitCtfBaseSetup {
                    pedestal_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_ctf_base_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTeamCannonConfig { old_config, description, .. } => {
                    self.team_cannon_settings.active_config = old_config.clone();
                    self.recalculate_team_cannon_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitTeamCannonSetup {
                    mount_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = mount_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_team_cannon_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateBioTrapConfig { old_config, description, .. } => {
                    self.bio_trap_settings.active_config = old_config.clone();
                    self.recalculate_bio_trap_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitBioTrapSetup {
                    grate_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = grate_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_bio_trap_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTriggerNetworkConfig { old_config, description, .. } => {
                    self.trigger_network_settings.active_config = old_config.clone();
                    self.recalculate_trigger_network_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitTriggerNetworkSetup {
                    plate_brush_index,
                    actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = plate_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_trigger_network_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateElevatorConfig { old_config, description, .. } => {
                    self.elevator_settings.active_config = old_config.clone();
                    self.recalculate_elevator_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitElevatorSetup {
                    platform_brush_index,
                    lift_center_actor_index,
                    lift_exit_actor_indices,
                    description,
                    ..
                } => {
                    if let Some(idx) = platform_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        let mut to_remove = Vec::new();
                        if let Some(c_idx) = lift_center_actor_index {
                            to_remove.push(*c_idx);
                        }
                        for e_idx in lift_exit_actor_indices {
                            to_remove.push(*e_idx);
                        }
                        to_remove.sort_unstable();
                        for idx in to_remove.into_iter().rev() {
                            if idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(idx));
                            }
                        }
                    }

                    self.recalculate_elevator_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateZoneVelocityConfig { old_config, description, .. } => {
                    self.zone_velocity_settings.active_config = old_config.clone();
                    self.recalculate_zone_velocity_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitZoneVelocitySetup {
                    portal_brush_index,
                    zone_info_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = portal_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = zone_info_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_zone_velocity_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateHazardBarrelConfig { old_config, description, .. } => {
                    self.hazard_barrel_settings.active_config = old_config.clone();
                    self.recalculate_hazard_barrel_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitHazardBarrelSetup {
                    barrel_brush_index,
                    barrel_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = barrel_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = barrel_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_hazard_barrel_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSpecialLitConfig { old_config, description, .. } => {
                    self.special_lit_settings.active_config = old_config.clone();
                    self.recalculate_special_lit_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSpecialLitSetup {
                    fixture_brush_index,
                    light_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = fixture_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = light_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_special_lit_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSniperVantageConfig { old_config, description, .. } => {
                    self.sniper_vantage_settings.active_config = old_config.clone();
                    self.recalculate_sniper_vantage_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSniperVantageSetup {
                    parapet_brush_index,
                    sniper_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = parapet_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = sniper_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_sniper_vantage_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSecretDoorConfig { old_config, description, .. } => {
                    self.secret_door_settings.active_config = old_config.clone();
                    self.recalculate_secret_door_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSecretDoorSetup {
                    door_brush_index,
                    trigger_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = door_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = trigger_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_secret_door_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateTeleporterArchConfig { old_config, description, .. } => {
                    self.teleporter_arch_settings.active_config = old_config.clone();
                    self.recalculate_teleporter_arch_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitTeleporterArchSetup {
                    arch_brush_index,
                    teleporter_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = arch_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = teleporter_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_teleporter_arch_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateFluidBasinConfig { old_config, description, .. } => {
                    self.fluid_basin_settings.active_config = old_config.clone();
                    self.recalculate_fluid_basin_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitFluidBasinSetup {
                    basin_brush_index,
                    zone_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = basin_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = zone_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_fluid_basin_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateAlarmBeaconConfig { old_config, description, .. } => {
                    self.alarm_beacon_settings.active_config = old_config.clone();
                    self.recalculate_alarm_beacon_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitAlarmBeaconSetup {
                    beacon_brush_index,
                    light_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = beacon_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = light_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_alarm_beacon_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSupplyLockerConfig { old_config, description, .. } => {
                    self.supply_locker_settings.active_config = old_config.clone();
                    self.recalculate_supply_locker_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSupplyLockerSetup {
                    locker_brush_index,
                    weapon_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = locker_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = weapon_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_supply_locker_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateSkylightConfig { old_config, description, .. } => {
                    self.skylight_settings.active_config = old_config.clone();
                    self.recalculate_skylight_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitSkylightSetup {
                    skylight_brush_index,
                    sunlight_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = skylight_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = sunlight_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_skylight_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateVentDuctConfig { old_config, description, .. } => {
                    self.vent_duct_settings.active_config = old_config.clone();
                    self.recalculate_vent_duct_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitVentDuctSetup {
                    duct_brush_index,
                    grate_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = duct_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = grate_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_vent_duct_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdatePistonConfig { old_config, description, .. } => {
                    self.piston_settings.active_config = old_config.clone();
                    self.recalculate_piston_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitPistonSetup {
                    piston_brush_index,
                    mover_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = piston_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = mover_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_piston_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateJumpBootsConfig { old_config, description, .. } => {
                    self.jump_boots_settings.active_config = old_config.clone();
                    self.recalculate_jump_boots_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitJumpBootsSetup {
                    cradle_brush_index,
                    boots_actor_index,
                    description,
                    ..
                } => {
                    if let Some(idx) = cradle_brush_index {
                        if *idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*idx));
                        }
                    }
                    if let Some(idx) = boots_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_jump_boots_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateForceFieldConfig { old_config, description, .. } => {
                    self.force_field_settings.active_config = old_config.clone();
                    self.recalculate_force_field_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitForceFieldSetup {
                    field_brush_index,
                    pylon_brush_indices,
                    generator_actor_index,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = field_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.extend(pylon_brush_indices.iter().copied());
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }
                    if let Some(idx) = generator_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_force_field_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::UpdateReactorCoreConfig { old_config, description, .. } => {
                    self.reactor_core_settings.active_config = old_config.clone();
                    self.recalculate_reactor_core_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::CommitReactorCoreSetup {
                    core_brush_index,
                    ring_brush_indices,
                    reactor_actor_index,
                    description,
                    ..
                } => {
                    let mut brush_indices = Vec::new();
                    if let Some(idx) = core_brush_index {
                        brush_indices.push(*idx);
                    }
                    brush_indices.extend(ring_brush_indices.iter().copied());
                    brush_indices.sort_unstable_by(|a, b| b.cmp(a));
                    for idx in brush_indices {
                        if idx < self.world_brushes.len() {
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(idx));
                        }
                    }
                    if let Some(idx) = reactor_actor_index {
                        if let Some(ref mut unr) = self.loaded_unr {
                            if *idx < unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*idx));
                            }
                        }
                    }

                    self.recalculate_reactor_core_preview();
                    self.log(format!("Undo: {}", description));
                }
                Action::ReplaceEntireLevel {
                    old_map_path,
                    old_launch_args,
                    old_brushes,
                    old_movers,
                    old_unr,
                    old_metadata,
                    description,
                    ..
                } => {
                    replace_history_value(&mut self.map_path, old_map_path.clone(), &mut self.map_is_dirty);
                    self.ut99_launch_args = old_launch_args.clone();
                    replace_history_tracked_vec(&mut self.world_brushes, old_brushes, &mut self.map_is_dirty);
                    replace_history_tracked_vec(&mut self.movers, old_movers, &mut self.map_is_dirty);
                    self.map_is_dirty = true;
                    self.loaded_unr = old_unr.clone();
                    self.map_instance_id = next_map_instance_id();
                    replace_history_value(&mut self.level_title, old_metadata.0.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_author, old_metadata.1.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ideal_player_count, old_metadata.2.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_song, old_metadata.3.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_game_type, old_metadata.4.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_enter_text, old_metadata.5.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_brightness, old_metadata.6, &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_hue, old_metadata.7, &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_saturation, old_metadata.8, &mut self.map_is_dirty);
                    self.selected_brush_index = None;
                    self.selected_actor_index = None;
                    self.selected_mover_index = None;
                    self.clear_multi_selection();
                    self.sync_brush_groups_from_brushes();
                    self.refresh_ai_path_links();

                    self.log(format!("Undo: {}", description));
                }
            }
        if let Some(ref mut unr) = self.loaded_unr {
            unr.sync_actor_views();
        }
        }

    pub fn redo(&mut self) {
        if let Some(action) = self.history.redo_stack.pop() {
            let desc = action.description().to_string();
            self.apply_redo_action(&action);
            self.toast_info(format!("Redo: {desc}"));
            self.history.undo_stack.push(action);
        } else {
            self.log("Nothing to redo.");
            self.toast_warning("Nothing to redo");
        }
    }

    pub fn apply_redo_action(&mut self, action: &Action) {
        match action {
            Action::ClearAiPaths { .. } => {
                self.map_is_dirty |= !self.nav_graph.reach_specs.is_empty()
                    || !self.nav_graph.adjacency.is_empty() || !self.ai_path_links.is_empty()
                    || self.last_autopath_report.is_some()
                    || self.loaded_unr.as_ref().is_some_and(|level| !level.reach_specs.is_empty());
                self.clear_ai_path_links_without_history();

                self.log("Redo: Clear AI Paths");
            }
            Action::Compound { actions, description } => {
                for sub in actions.iter() {
                    self.apply_redo_action(sub);
                }
                self.log(format!("Redo: {}", description));
            }
            Action::MoveBrush { brush_index, new_location, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.location, *new_location, &mut self.map_is_dirty);
                        self.log(format!("Redo: Moved brush {} to {:?}", brush_index, new_location));
                    }
                }
                Action::RotateBrush { brush_index, new_rotation, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.rotation, *new_rotation, &mut self.map_is_dirty);
                        self.log(format!("Redo: Rotated brush {} to {:?}", brush_index, new_rotation));
                    }
                }
                Action::ScaleBrush { brush_index, new_scale, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.scale, *new_scale, &mut self.map_is_dirty);
                        self.log(format!("Redo: Scaled brush {} to {:?}", brush_index, new_scale));
                    }
                }
                Action::ModifyBrushGeometry { brush_index, new_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.polys, new_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Redo: Applied {} on brush {}", description, brush_index));
                    }
                }
                Action::ModifyBrushPivot { brush_index, new_location, new_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.location, *new_location, &mut self.map_is_dirty);
                        replace_history_value(&mut b.polys, new_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Redo: Applied {} on brush {}", description, brush_index));
                    }
                }
                Action::MoveActor { actor_index, new_location, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut a.location, *new_location, &mut self.map_is_dirty);
                            self.log(format!("Redo: Moved actor {} to {:?}", actor_index, new_location));
                        }
                    }
                }
                Action::RotateActor { actor_index, new_rotation, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut a.rotation, *new_rotation, &mut self.map_is_dirty);
                            self.log(format!("Redo: Rotated actor {} to {:?}", actor_index, new_rotation));
                        }
                    }
                }
                Action::AddActor { actor } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.push(actor.clone()));
                        self.log(format!("Redo: Restored added actor '{}'", actor.name));
                    }
                }
                Action::DeleteActor { index, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if *index < unr.actors.len() {
                            let a = history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.remove(*index));
                            self.log(format!("Redo: Deleted actor '{}'", a.name));
                        }
                    }
                }
                Action::AddBrush { brush } => {
                    history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.push(brush.clone()));
                    self.log(format!("Redo: Restored added brush '{}'", brush.name));
                }
                Action::DeleteBrush { index, .. } => {
                    if *index < self.world_brushes.len() {
                        let b = history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.remove(*index));
                        self.log(format!("Redo: Deleted brush '{}'", b.name));
                    }
                }
                Action::MoveMover { mover_index, new_location, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.brush.location, *new_location, &mut self.map_is_dirty);
                        let cur_key = m.current_key;
                        if let Some(ref mut kf) = m.keyframes[cur_key] {
                            replace_history_value(&mut kf.location, *new_location, &mut self.map_is_dirty);
                        }
                        self.log(format!("Redo: Moved mover {} to {:?}", mover_index, new_location));
                    }
                }
                Action::RotateMover { mover_index, new_rotation, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.brush.rotation, *new_rotation, &mut self.map_is_dirty);
                        let cur_key = m.current_key;
                        if let Some(ref mut kf) = m.keyframes[cur_key] {
                            replace_history_value(&mut kf.rotation, *new_rotation, &mut self.map_is_dirty);
                        }
                        self.log(format!("Redo: Rotated mover {} to {:?}", mover_index, new_rotation));
                    }
                }
                Action::TransformStaticMesh { mesh_index, new_rotation, new_scale, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut m.rotation, *new_rotation, &mut self.map_is_dirty);
                        replace_history_value(&mut m.scale, *new_scale, &mut self.map_is_dirty);
                        self.log(format!("Redo: Applied static mesh {} rotation/scale", mesh_index));
                    }
                }
                Action::AddMover { mover } => {
                    history_vec_change(&mut self.movers, &mut self.map_is_dirty, |items| items.push(mover.clone()));
                    self.log(format!("Redo: Restored added mover '{}'", mover.name));
                }
                Action::DeleteMover { index, .. } => {
                    if *index < self.movers.len() {
                        let m = history_vec_change(&mut self.movers, &mut self.map_is_dirty, |items| items.remove(*index));
                        self.log(format!("Redo: Deleted mover '{}'", m.name));
                    }
                }
                Action::MoveStaticMesh { mesh_index, new_location, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut m.location, *new_location, &mut self.map_is_dirty);
                        self.log(format!("Redo: Moved static mesh {} to {:?}", mesh_index, new_location));
                    }
                }
                Action::AddStaticMesh { mesh } => {
                    history_vec_change(&mut self.static_meshes, &mut self.map_is_dirty, |items| items.push(mesh.clone()));
                    self.log(format!("Redo: Restored added static mesh '{}'", mesh.name));
                }
                Action::DeleteStaticMesh { index, .. } => {
                    if *index < self.static_meshes.len() {
                        let m = history_vec_change(&mut self.static_meshes, &mut self.map_is_dirty, |items| items.remove(*index));
                        self.log(format!("Redo: Deleted static mesh '{}'", m.name));
                    }
                }
                Action::SetSurfaceTexture { brush_index, poly_index, new_texture, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        if let Some(p) = b.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, new_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Redo: Set texture on brush {} poly {} to '{}'", brush_index, poly_index, new_texture));
                        }
                    }
                }
                Action::BatchSetSurfaceTextures { modifications } => {
                    for (brush_idx, poly_idx, _, new_texture) in modifications {
                        if let Some(b) = self.world_brushes.get_mut(*brush_idx) {
                            if let Some(p) = b.polys.get_mut(*poly_idx) {
                                replace_history_value(&mut p.texture, new_texture.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }
                    self.log(format!("Redo: Re-applied active texture on {} surfaces", modifications.len()));
                }
                Action::SetMoverSurfaceTexture { mover_index, poly_index, new_texture, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        if let Some(p) = m.brush.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, new_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Redo: Set texture on mover {} poly {} to '{}'", mover_index, poly_index, new_texture));
                        }
                    }
                }
                Action::SetStaticMeshSurfaceTexture { mesh_index, poly_index, new_texture, .. } => {
                    if let Some(m) = self.static_meshes.get_mut(*mesh_index) {
                        if let Some(p) = m.polys.get_mut(*poly_index) {
                            replace_history_value(&mut p.texture, new_texture.clone(), &mut self.map_is_dirty);
                            self.log(format!("Redo: Set texture on static mesh {} poly {} to '{}'", mesh_index, poly_index, new_texture));
                        }
                    }
                }
                Action::AddProjector { index, projector } => {
                    let idx = (*index).min(self.projectors.len());
                    history_vec_change(&mut self.projectors, &mut self.map_is_dirty, |items| items.insert(idx, projector.clone()));
                    self.selected_projector_index = Some(idx);
                    self.bake_projector_decals();
                    self.log(format!("Redo: Re-applied decal projector '{}'", projector.name));
                }
                Action::DeleteProjector { index, .. } => {
                    if *index < self.projectors.len() {
                        history_vec_change(&mut self.projectors, &mut self.map_is_dirty, |items| items.remove(*index));
                        self.selected_projector_index = None;
                        self.bake_projector_decals();
                        self.log(format!("Redo: Deleted decal projector #{}", index));
                    }
                }
                Action::ModifyBuilderBrush { new_brush, description, .. } => {
                    self.builder_brush = new_brush.clone();
                    self.log(format!("Redo: Applied {} on Builder Brush", description));
                }
                Action::ReorderBrushes { new_brushes, new_selection, description, .. } => {
                    replace_history_tracked_vec(&mut self.world_brushes, new_brushes, &mut self.map_is_dirty);
                    self.selected_brush_index = *new_selection;
                    self.sync_brush_groups_from_brushes();
                    self.log(format!("Redo: Re-applied {}", description));
                }
                Action::TransformBrush { brush_index, new_rotation, new_scale, new_polys, description, .. } => {
                    if let Some(b) = self.world_brushes.get_mut(*brush_index) {
                        replace_history_value(&mut b.rotation, *new_rotation, &mut self.map_is_dirty);
                        replace_history_value(&mut b.scale, *new_scale, &mut self.map_is_dirty);
                        replace_history_value(&mut b.polys, new_polys.clone(), &mut self.map_is_dirty);
                        self.log(format!("Redo: Re-applied {} on brush {}", description, brush_index));
                    }
                }
                Action::ScaleEntireLevel { new_brushes, new_actors, new_movers, factor, .. } => {
                    replace_history_tracked_vec(&mut self.world_brushes, new_brushes, &mut self.map_is_dirty);
                    if let (Some(ref mut unr), Some(actors)) = (&mut self.loaded_unr, new_actors) {
                        replace_history_tracked_vec(&mut unr.actors, actors, &mut self.map_is_dirty);
                    }
                    replace_history_tracked_vec(&mut self.movers, new_movers, &mut self.map_is_dirty);
                    self.log(format!("Redo: Re-applied Scale Entire Map ({:.2}, {:.2}, {:.2})", factor.x, factor.y, factor.z));
                }
                Action::PropagateActorProperties { class_name, new_actors, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        replace_history_tracked_vec(&mut unr.actors, new_actors, &mut self.map_is_dirty);
                    }
                    self.log(format!("Redo: Re-applied {} on class '{}'", description, class_name));
                }
                Action::UpdateZoneProperties { actor_tag, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *actor_tag) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    self.log(format!("Redo: Re-applied {} on zone '{}'", description, actor_tag));
                }
                Action::UpdateSoundEmitter { actor_tag, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *actor_tag || a.name == *actor_tag) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    self.log(format!("Redo: Re-applied {} on sound emitter '{}'", description, actor_tag));
                }
                Action::UpdateWarpZonePair { source_tag, target_tag, new_source, new_target, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *source_tag || a.name == *source_tag) {
                            replace_history_value(&mut *a, new_source.clone(), &mut self.map_is_dirty);
                        }
                        if let Some(new_t) = new_target {
                            if let Some(a) = unr.actors.iter_mut().find(|a| a.tag == *target_tag || a.name == *target_tag) {
                                replace_history_value(&mut *a, new_t.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }
                    self.log(format!("Redo: Re-applied {} on WarpZones '{}' / '{}'", description, source_tag, target_tag));
                }
                Action::BakeLighting { new_polys, description, .. } => {
                    for (b_idx, polys) in new_polys {
                        if let Some(brush) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut brush.polys, polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on {} brushes", description, new_polys.len()));
                }
                Action::UpdateZoneFog { actor_index, new_actor, new_viewport_fog, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }
                    let (en, st, ed, den, col) = *new_viewport_fog;
                    self.distance_fog_enabled = en;
                    self.fog_start = st;
                    self.fog_end = ed;
                    self.fog_density = den;
                    self.fog_color = col;

                    self.log(format!("Redo: Re-applied {} on actor index {}", description, actor_index));
                }
                Action::UpdateDispatcherSequence { actor_name, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.iter_mut().find(|a| a.name == *actor_name) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on Dispatcher '{}'", description, actor_name));
                }
                Action::UpdateSkeletalMesh { mesh_index, new_mesh, description, .. } => {
                    if let Some(m) = self.skeletal_meshes.get_mut(*mesh_index) {
                        replace_history_value(&mut *m, new_mesh.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Redo: Re-applied {} on SkeletalMesh #{}", description, mesh_index));
                }
                Action::UpdateParticleEmitter { emitter_index, new_emitter, description, .. } => {
                    if let Some(e) = self.particle_emitters.get_mut(*emitter_index) {
                        replace_history_value(&mut *e, new_emitter.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Redo: Re-applied {} on ParticleEmitter #{}", description, emitter_index));
                }
                Action::UpdateLightEffects { actor_index, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on Light #{}", description, actor_index));
                }
                Action::BatchUpdateLightEffects { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, _, new_actor) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Redo: Re-applied {} across {} light(s)", description, modifications.len()));
                }
                Action::AddAntiPortal { antiportal } => {
                    history_vec_change(&mut self.antiportals, &mut self.map_is_dirty, |items| items.push(antiportal.clone()));

                    self.log(format!("Redo: Restored added AntiPortal '{}'", antiportal.name));
                }
                Action::UpdateAntiPortal { antiportal_index, new_antiportal, description, .. } => {
                    if let Some(ap) = self.antiportals.get_mut(*antiportal_index) {
                        replace_history_value(&mut *ap, new_antiportal.clone(), &mut self.map_is_dirty);
                    }

                    self.log(format!("Redo: Re-applied {} on AntiPortal #{}", description, antiportal_index));
                }
                Action::UpdateMusicEvent { actor_index, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on MusicEvent #{}", description, actor_index));
                }
                Action::SetLevelMusic { new_song, new_section, description, .. } => {
                    replace_history_value(&mut self.level_song, new_song.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_song_section, *new_section, &mut self.map_is_dirty);
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(info) = unr.actors.iter_mut().find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo")) {
                            history_property_change(&mut info.properties, &mut self.map_is_dirty, "Song".to_string(), ut99_format::TaggedProperty {
                                name: "Song".to_string(),
                                array_index: 0,
                                value: ut99_format::TaggedValue::Str(new_song.clone()),
                            });
                            history_property_change(&mut info.properties, &mut self.map_is_dirty, "SongSection".to_string(), ut99_format::TaggedProperty {
                                name: "SongSection".to_string(),
                                array_index: 0,
                                value: ut99_format::TaggedValue::Byte(*new_section),
                            });
                        }
                    }

                    self.log(format!("Redo: Re-applied {} to '{}' (Section {})", description, new_song, new_section));
                }
                Action::UpdateLightCorona { actor_index, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on Light #{}", description, actor_index));
                }
                Action::BatchUpdateLightCoronas { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, _, new_actor) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Redo: Re-applied {} across {} light(s)", description, modifications.len()));
                }
                Action::UpdateZonePhysics { actor_index, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: Re-applied {} on ZoneInfo #{}", description, actor_index));
                }
                Action::BatchUpdateZonePhysics { modifications, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, _, new_actor) in modifications {
                            if let Some(a) = unr.actors.get_mut(*idx) {
                                replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                            }
                        }
                    }

                    self.log(format!("Redo: Re-applied {} across {} zone(s)", description, modifications.len()));
                }
                Action::AddBallisticProbe { probe_index, name, location, pitch, yaw, preset } => {
                    let probe = BallisticProbeData {
                        id: *probe_index,
                        name: name.clone(),
                        location: *location,
                        pitch_deg: *pitch,
                        yaw_deg: *yaw,
                        preset: *preset,
                    };
                    let idx = (*probe_index).min(self.ballistics_studio_settings.probes.len());
                    self.ballistics_studio_settings.probes.insert(idx, probe);
                    self.calculate_active_weapon_trajectory();
                    self.log(format!("Redo: Re-added ballistic probe '{}'", name));
                }
                Action::DeleteBallisticProbe { probe_index, name, .. } => {
                    if *probe_index < self.ballistics_studio_settings.probes.len() {
                        self.ballistics_studio_settings.probes.remove(*probe_index);
                    }
                    self.calculate_active_weapon_trajectory();
                    self.log(format!("Redo: Removed ballistic probe '{}'", name));
                }
                Action::AddTeleporterPair { actor_index_a, actor_index_b, actor_a, actor_b, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let min_idx = (*actor_index_a).min(*actor_index_b);
                        let max_idx = (*actor_index_a).max(*actor_index_b);
                        if min_idx <= unr.actors.len() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(min_idx, actor_a.clone()));
                        }
                        if max_idx <= unr.actors.len() {
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(max_idx, actor_b.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTeleporterConfig { actor_index, new_actor, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let Some(a) = unr.actors.get_mut(*actor_index) {
                            replace_history_value(&mut *a, new_actor.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::AddTranslocatorProbe { probe_index, name, location, pitch, yaw, preset } => {
                    let probe = TranslocatorProbeData {
                        id: *probe_index,
                        name: name.clone(),
                        location: *location,
                        pitch_deg: *pitch,
                        yaw_deg: *yaw,
                        preset: *preset,
                    };
                    let idx = (*probe_index).min(self.translocator_studio_settings.probes.len());
                    self.translocator_studio_settings.probes.insert(idx, probe);
                    self.calculate_active_translocator_simulation();
                    self.log(format!("Redo: Re-added translocator probe '{}'", name));
                }
                Action::DeleteTranslocatorProbe { probe_index, name, .. } => {
                    if *probe_index < self.translocator_studio_settings.probes.len() {
                        self.translocator_studio_settings.probes.remove(*probe_index);
                    }
                    self.calculate_active_translocator_simulation();
                    self.log(format!("Redo: Removed translocator probe '{}'", name));
                }
                Action::AddElevatorPathingSet { added_actor_indices, added_actors, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (&idx, actor) in added_actor_indices.iter().zip(added_actors.iter()) {
                            if idx <= unr.actors.len() {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(idx, actor.clone()));
                            } else {
                                history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.push(actor.clone()));
                            }
                        }
                    }

                    self.collect_elevator_audit();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateElevatorMoverConfig { mover_index, new_mover, description, .. } => {
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut *m, new_mover.clone(), &mut self.map_is_dirty);
                    }

                    self.collect_elevator_audit();
                    self.log(format!("Redo: {}", description));
                }
                Action::AddWaterSheetBrush { brush_index, brush, description } => {
                    let idx = (*brush_index).min(self.world_brushes.len());
                    history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(idx, brush.clone()));

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateWaterSurfaceConfig { new_config, new_preset, description, .. } => {
                    self.water_surface_studio_settings.active_config = new_config.clone();
                    self.water_surface_studio_settings.selected_preset = *new_preset;
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateMoverSequenceConfig { mover_index, new_config, new_template, description, .. } => {
                    self.mover_studio_settings.active_config = new_config.clone();
                    self.mover_studio_settings.selected_template = *new_template;
                    if let Some(m) = self.movers.get_mut(*mover_index) {
                        replace_history_value(&mut m.name, new_config.mover_name.clone(), &mut self.map_is_dirty);
                        replace_history_value(&mut m.tag, new_config.mover_tag.clone(), &mut self.map_is_dirty);
                    }
                    self.audit_active_mover_hazards();
                    self.log(format!("Redo: {}", description));
                }
                Action::AddDoorTriggerActor { actor_index, actor, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let idx = (*actor_index).min(unr.actors.len());
                        history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(idx, actor.clone()));
                    }

                    self.audit_active_mover_hazards();
                    self.log(format!("Redo: {}", description));
                }
                Action::AddDefensePointSet { added_actor_indices, added_actors, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, actor) in added_actor_indices.iter().zip(added_actors.iter()) {
                            let insert_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(insert_idx, actor.clone()));
                        }
                    }

                    self.scan_and_refresh_team_objectives();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTeamObjectiveSetup { new_mode, new_preset, description, .. } => {
                    self.team_objective_settings.active_game_mode = *new_mode;
                    self.team_objective_settings.selected_preset = *new_preset;
                    self.scan_and_refresh_team_objectives();
                    self.log(format!("Redo: {}", description));
                }
                Action::AddDestructibleBarrier { brush_index, brush, added_actor_indices, added_actors, description } => {
                    let b_idx = (*brush_index).min(self.world_brushes.len());
                    history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(b_idx, brush.clone()));
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, actor) in added_actor_indices.iter().zip(added_actors.iter()) {
                            let insert_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(insert_idx, actor.clone()));
                        }
                    }

                    self.audit_destructible_hazards();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateDestructibleConfig { new_config, new_archetype, description, .. } => {
                    self.destructible_studio_settings.active_config = new_config.clone();
                    self.destructible_studio_settings.selected_archetype = *new_archetype;
                    self.destructible_studio_settings.selected_pattern = new_config.fracture_pattern;
                    self.regenerate_destructible_shards();
                    self.audit_destructible_hazards();
                    self.log(format!("Redo: {}", description));
                }
                Action::AddJumpPadSetup { kicker_index, jump_spot_index, kicker_actor, jump_spot_actor, description } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        let k_idx = (*kicker_index).min(unr.actors.len());
                        history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(k_idx, kicker_actor.clone()));
                        if let (Some(js_idx), Some(js_actor)) = (jump_spot_index, jump_spot_actor) {
                            let ins_idx = (*js_idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, js_actor.clone()));
                        }
                    }

                    self.recalculate_kicker_trajectory();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateKickerConfig { new_config, new_archetype, description, .. } => {
                    self.kicker_studio_settings.active_config = new_config.clone();
                    self.kicker_studio_settings.selected_archetype = *new_archetype;
                    self.recalculate_kicker_trajectory();
                    self.log(format!("Redo: {}", description));
                }
                Action::PurgeSliverPolygons { modifications, description } => {
                    for (b_idx, _, new_polys) in modifications {
                        if let Some(b) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut b.polys, new_polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.run_bsp_quality_audit();
                    self.log(format!("Redo: {}", description));
                }
                Action::BatchConvertSemiSolid { conversions, description } => {
                    for (b_idx, _, new_polys) in conversions {
                        if let Some(b) = self.world_brushes.get_mut(*b_idx) {
                            replace_history_value(&mut b.polys, new_polys.clone(), &mut self.map_is_dirty);
                        }
                    }

                    self.run_bsp_quality_audit();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateLoftConfig { new_config, new_keyframes, description, .. } => {
                    self.loft_studio_settings.active_config = new_config.clone();
                    self.loft_studio_settings.path_keyframes = new_keyframes.clone();
                    self.recalculate_loft_mesh();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitLoftedBrush { brush_index, brush, is_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        self.builder_brush = brush.clone();
                    } else if let Some(idx) = brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, brush.clone()));
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateStairConfig { new_config, description, .. } => {
                    self.stairway_studio_settings.active_config = new_config.clone();
                    self.recalculate_stair_geometry();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitStairSetup { stair_brush_index, stair_brush, ramp_brush_index, ramp_brush, is_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        self.builder_brush = stair_brush.clone();
                    } else {
                        if let Some(idx) = stair_brush_index {
                            let ins_idx = (*idx).min(self.world_brushes.len());
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, stair_brush.clone()));
                        }
                        if let (Some(idx), Some(ramp)) = (ramp_brush_index, ramp_brush) {
                            let ins_idx = (*idx).min(self.world_brushes.len());
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, ramp.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateScatterConfig { new_config, description, .. } => {
                    self.scatter_studio_settings.active_config = new_config.clone();
                    self.recalculate_scatter_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitScatterBatch { added_actor_indices, actors, description, .. } => {
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, actor) in added_actor_indices.iter().zip(actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, actor.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateLightShaftConfig { new_config, description, .. } => {
                    self.light_shaft_settings.active_config = new_config.clone();
                    self.recalculate_light_shaft_geometry();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitLightShaftBrush { brush_index, brush, is_builder_brush, description, .. } => {
                    if *is_builder_brush {
                        self.builder_brush = brush.clone();
                    } else if let Some(idx) = brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, brush.clone()));
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTrackConfig { new_config, description, .. } => {
                    self.track_studio_settings.active_config = new_config.clone();
                    self.recalculate_track_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitTrackSetup {
                    rails_brush_index,
                    ties_brush_index,
                    rails_brush,
                    ties_brush,
                    mover_actor_index,
                    mover_actor,
                    waypoint_actor_indices,
                    waypoint_actors,
                    description,
                } => {
                    if let Some(idx) = rails_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, rails_brush.clone()));
                    }
                    if let Some(idx) = ties_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, ties_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (mover_actor_index, mover_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        for (idx, act) in waypoint_actor_indices.iter().zip(waypoint_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSurveillanceConfig { new_config, description, .. } => {
                    self.surveillance_settings.active_config = new_config.clone();
                    self.recalculate_surveillance_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSurveillanceSetup {
                    monitor_brush_index,
                    monitor_brush,
                    camera_actor_index,
                    camera_actor,
                    screen_actor_index,
                    screen_actor,
                    description,
                } => {
                    if let Some(idx) = monitor_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, monitor_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (camera_actor_index, camera_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (screen_actor_index, screen_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateLaserConfig { new_config, description, .. } => {
                    self.laser_grid_settings.active_config = new_config.clone();
                    self.recalculate_laser_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitLaserGrid {
                    beam_brush_index,
                    diode_brush_index,
                    beam_brush,
                    diode_brush,
                    trigger_actor_indices,
                    trigger_actors,
                    description,
                } => {
                    if let Some(idx) = beam_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, beam_brush.clone()));
                    }
                    if let Some(idx) = diode_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, diode_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, act) in trigger_actor_indices.iter().zip(trigger_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateVentilatorConfig { new_config, description, .. } => {
                    self.ventilator_settings.active_config = new_config.clone();
                    self.recalculate_ventilator_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitVentilatorSetup {
                    fan_brush_index,
                    housing_brush_index,
                    fan_brush,
                    housing_brush,
                    mover_actor_index,
                    mover_actor,
                    zone_actor_index,
                    zone_actor,
                    trigger_actor_index,
                    trigger_actor,
                    description,
                } => {
                    if let Some(idx) = fan_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, fan_brush.clone()));
                    }
                    if let Some(idx) = housing_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, housing_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (mover_actor_index, mover_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (zone_actor_index, zone_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (trigger_actor_index, trigger_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateConveyorConfig { new_config, description, .. } => {
                    self.conveyor_settings.active_config = new_config.clone();
                    self.recalculate_conveyor_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitConveyorSetup {
                    deck_brush_index,
                    rollers_brush_index,
                    frame_brush_index,
                    deck_brush,
                    rollers_brush,
                    frame_brush,
                    zone_actor_index,
                    zone_actor,
                    waypoint_actor_indices,
                    waypoint_actors,
                    description,
                } => {
                    if let Some(idx) = deck_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, deck_brush.clone()));
                    }
                    if let Some(idx) = rollers_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, rollers_brush.clone()));
                    }
                    if let Some(idx) = frame_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, frame_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (zone_actor_index, zone_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        for (idx, act) in waypoint_actor_indices.iter().zip(waypoint_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTurretConfig { new_config, description, .. } => {
                    self.turret_settings.active_config = new_config.clone();
                    self.recalculate_turret_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitTurretSetup {
                    mount_brush_index,
                    shield_brush_index,
                    mount_brush,
                    shield_brush,
                    turret_actor_index,
                    turret_actor,
                    waypoint_actor_indices,
                    waypoint_actors,
                    description,
                } => {
                    if let Some(idx) = mount_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, mount_brush.clone()));
                    }
                    if let Some(idx) = shield_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, shield_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (turret_actor_index, turret_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        for (idx, act) in waypoint_actor_indices.iter().zip(waypoint_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSoundscapeConfig { new_config, description, .. } => {
                    self.soundscape_settings.active_config = new_config.clone();
                    self.recalculate_soundscape_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSoundscapeSetup {
                    zone_brush_index,
                    zone_brush,
                    ambient_actor_index,
                    ambient_actor,
                    zone_actor_index,
                    zone_actor,
                    description,
                } => {
                    if let Some(idx) = zone_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, zone_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (ambient_actor_index, ambient_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (zone_actor_index, zone_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateHazardChamberConfig { new_config, description, .. } => {
                    self.hazard_chamber_settings.active_config = new_config.clone();
                    self.recalculate_hazard_chamber_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitHazardChamberSetup {
                    chamber_brush_index,
                    door_l_brush_index,
                    door_r_brush_index,
                    window_brush_index,
                    console_brush_index,
                    chamber_brush,
                    door_l_brush,
                    door_r_brush,
                    window_brush,
                    console_brush,
                    zone_actor_index,
                    zone_actor,
                    trigger_actor_index,
                    trigger_actor,
                    waypoint_actor_indices,
                    waypoint_actors,
                    description,
                } => {
                    if let Some(idx) = chamber_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, chamber_brush.clone()));
                    }
                    if let Some(idx) = door_l_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, door_l_brush.clone()));
                    }
                    if let Some(idx) = door_r_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, door_r_brush.clone()));
                    }
                    if let (Some(idx), Some(win_b)) = (window_brush_index, window_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, win_b.clone()));
                    }
                    if let Some(idx) = console_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, console_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (zone_actor_index, zone_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (trigger_actor_index, trigger_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        for (idx, act) in waypoint_actor_indices.iter().zip(waypoint_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdatePipeConduitConfig { new_config, description, .. } => {
                    self.pipe_conduit_settings.active_config = new_config.clone();
                    self.recalculate_pipe_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitPipeConduitSetup {
                    straight_brush_index,
                    straight_brush,
                    elbow_brush_index,
                    elbow_brush,
                    flange_brush_indices,
                    flange_brushes,
                    collar_brush_index,
                    collar_brush,
                    valve_body_brush_index,
                    valve_body_brush,
                    valve_wheel_brush_index,
                    valve_wheel_brush,
                    valve_mover_actor_index,
                    valve_mover_actor,
                    hazard_trigger_actor_index,
                    hazard_trigger_actor,
                    description,
                } => {
                    if let Some(idx) = straight_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, straight_brush.clone()));
                    }
                    if let (Some(idx), Some(b)) = (elbow_brush_index, elbow_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    for (idx, b) in flange_brush_indices.iter().zip(flange_brushes.iter()) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let (Some(idx), Some(b)) = (collar_brush_index, collar_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let (Some(idx), Some(b)) = (valve_body_brush_index, valve_body_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let (Some(idx), Some(b)) = (valve_wheel_brush_index, valve_wheel_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (valve_mover_actor_index, valve_mover_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (hazard_trigger_actor_index, hazard_trigger_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateHologramConfig { new_config, description, .. } => {
                    self.hologram_settings.active_config = new_config.clone();
                    self.recalculate_hologram_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitHologramSetup {
                    display_brush_index,
                    display_brush,
                    pedestal_brush_index,
                    pedestal_brush,
                    projection_cone_brush_index,
                    projection_cone_brush,
                    rotator_mover_actor_index,
                    rotator_mover_actor,
                    projector_light_actor_index,
                    projector_light_actor,
                    description,
                } => {
                    if let Some(idx) = display_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, display_brush.clone()));
                    }
                    if let (Some(idx), Some(b)) = (pedestal_brush_index, pedestal_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let (Some(idx), Some(b)) = (projection_cone_brush_index, projection_cone_brush) {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, b.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (rotator_mover_actor_index, rotator_mover_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (projector_light_actor_index, projector_light_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateWeatherConfig { new_config, description, .. } => {
                    self.weather_settings.active_config = new_config.clone();
                    self.recalculate_weather_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitWeatherSetup {
                    volume_brush_index,
                    volume_brush,
                    zone_emitter_actor_index,
                    zone_emitter_actor,
                    lightning_light_actor_index,
                    lightning_light_actor,
                    thunder_audio_actor_index,
                    thunder_audio_actor,
                    description,
                } => {
                    if let Some(idx) = volume_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, volume_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (zone_emitter_actor_index, zone_emitter_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (lightning_light_actor_index, lightning_light_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        if let (Some(idx), Some(act)) = (thunder_audio_actor_index, thunder_audio_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateArenaRulesConfig { new_config, description, .. } => {
                    self.arena_rules_settings.active_config = new_config.clone();
                    self.recalculate_arena_rules_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitArenaRulesSetup {
                    level_info_actor_index,
                    new_level_info,
                    new_level_game_type,
                    mutator_actor_indices,
                    mutator_actors,
                    description,
                    ..
                } => {
                    replace_history_value(&mut self.level_game_type, new_level_game_type.clone(), &mut self.map_is_dirty);
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (level_info_actor_index, new_level_info) {
                            if *idx < unr.actors.len() {
                                unr.actors[*idx] = act.clone();
                            }
                        }
                        for (idx, act) in mutator_actor_indices.iter().zip(mutator_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_arena_rules_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateArmorStackingConfig { new_config, description, .. } => {
                    self.armor_stacking_settings.active_config = new_config.clone();
                    self.recalculate_armor_stacking_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitArmorStackingSetup {
                    pedestal_brush_index,
                    pedestal_brush,
                    pickup_actor_indices,
                    pickup_actors,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, pedestal_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, act) in pickup_actor_indices.iter().zip(pickup_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_armor_stacking_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateBotJumpRoutesConfig { new_config, description, .. } => {
                    self.bot_jump_routes_settings.active_config = new_config.clone();
                    self.recalculate_jump_route_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitBotJumpRouteSetup {
                    jump_plate_brush_index,
                    jump_plate_brush,
                    actor_indices,
                    actors,
                    description,
                    ..
                } => {
                    if let Some(idx) = jump_plate_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, jump_plate_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, act) in actor_indices.iter().zip(actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_jump_route_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateWeaponCircuitConfig { new_config, description, .. } => {
                    self.weapon_circuit_settings.active_config = new_config.clone();
                    self.recalculate_weapon_circuit_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitWeaponCircuitSetup {
                    altar_brush_index,
                    altar_brush,
                    actor_indices,
                    actors,
                    description,
                    ..
                } => {
                    if let Some(idx) = altar_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, altar_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        for (idx, act) in actor_indices.iter().zip(actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_weapon_circuit_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdatePowerupControlConfig { new_config, description, .. } => {
                    self.powerup_control_settings.active_config = new_config.clone();
                    self.recalculate_powerup_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitPowerupSetup {
                    altar_brush_index,
                    altar_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = altar_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, altar_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_powerup_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSpawnDistributionConfig { new_config, description, .. } => {
                    self.spawn_distribution_settings.active_config = new_config.clone();
                    self.recalculate_spawn_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSpawnPointSetup {
                    pad_brush_index,
                    pad_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = pad_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, pad_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_spawn_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateDominationConfig { new_config, description, .. } => {
                    self.domination_settings.active_config = new_config.clone();
                    self.recalculate_domination_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitDominationPointSetup {
                    pedestal_brush_index,
                    pedestal_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, pedestal_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_domination_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateAssaultConfig { new_config, description, .. } => {
                    self.assault_settings.active_config = new_config.clone();
                    self.recalculate_assault_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitAssaultObjectiveSetup {
                    door_brush_index,
                    door_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = door_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, door_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_assault_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateRadarConfig { new_config, description, .. } => {
                    self.radar_settings.active_config = new_config.clone();
                    self.recalculate_radar_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitRadarStationSetup {
                    mast_brush_index,
                    mast_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = mast_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, mast_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_radar_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateCtfBaseConfig { new_config, description, .. } => {
                    self.ctf_base_settings.active_config = new_config.clone();
                    self.recalculate_ctf_base_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitCtfBaseSetup {
                    pedestal_brush_index,
                    pedestal_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = pedestal_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, pedestal_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_ctf_base_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTeamCannonConfig { new_config, description, .. } => {
                    self.team_cannon_settings.active_config = new_config.clone();
                    self.recalculate_team_cannon_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitTeamCannonSetup {
                    mount_brush_index,
                    mount_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = mount_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, mount_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_team_cannon_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateBioTrapConfig { new_config, description, .. } => {
                    self.bio_trap_settings.active_config = new_config.clone();
                    self.recalculate_bio_trap_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitBioTrapSetup {
                    grate_brush_index,
                    grate_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = grate_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, grate_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_bio_trap_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTriggerNetworkConfig { new_config, description, .. } => {
                    self.trigger_network_settings.active_config = new_config.clone();
                    self.recalculate_trigger_network_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitTriggerNetworkSetup {
                    plate_brush_index,
                    plate_brush,
                    actor_index,
                    actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = plate_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, plate_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (actor_index, actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_trigger_network_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateElevatorConfig { new_config, description, .. } => {
                    self.elevator_settings.active_config = new_config.clone();
                    self.recalculate_elevator_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitElevatorSetup {
                    platform_brush_index,
                    platform_brush,
                    lift_center_actor_index,
                    lift_center_actor,
                    lift_exit_actor_indices,
                    lift_exit_actors,
                    description,
                    ..
                } => {
                    if let Some(idx) = platform_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, platform_brush.clone()));
                    }
                    if let Some(ref mut unr) = self.loaded_unr {
                        if let (Some(idx), Some(act)) = (lift_center_actor_index, lift_center_actor) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                        for (idx, act) in lift_exit_actor_indices.iter().zip(lift_exit_actors.iter()) {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_elevator_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateZoneVelocityConfig { new_config, description, .. } => {
                    self.zone_velocity_settings.active_config = new_config.clone();
                    self.recalculate_zone_velocity_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitZoneVelocitySetup {
                    portal_brush_index,
                    portal_brush,
                    zone_info_actor_index,
                    zone_info_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = portal_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, portal_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (zone_info_actor_index, zone_info_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_zone_velocity_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateHazardBarrelConfig { new_config, description, .. } => {
                    self.hazard_barrel_settings.active_config = new_config.clone();
                    self.recalculate_hazard_barrel_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitHazardBarrelSetup {
                    barrel_brush_index,
                    barrel_brush,
                    barrel_actor_index,
                    barrel_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = barrel_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, barrel_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (barrel_actor_index, barrel_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_hazard_barrel_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSpecialLitConfig { new_config, description, .. } => {
                    self.special_lit_settings.active_config = new_config.clone();
                    self.recalculate_special_lit_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSpecialLitSetup {
                    fixture_brush_index,
                    fixture_brush,
                    light_actor_index,
                    light_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = fixture_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, fixture_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (light_actor_index, light_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_special_lit_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSniperVantageConfig { new_config, description, .. } => {
                    self.sniper_vantage_settings.active_config = new_config.clone();
                    self.recalculate_sniper_vantage_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSniperVantageSetup {
                    parapet_brush_index,
                    parapet_brush,
                    sniper_actor_index,
                    sniper_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = parapet_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, parapet_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (sniper_actor_index, sniper_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_sniper_vantage_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSecretDoorConfig { new_config, description, .. } => {
                    self.secret_door_settings.active_config = new_config.clone();
                    self.recalculate_secret_door_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSecretDoorSetup {
                    door_brush_index,
                    door_brush,
                    trigger_actor_index,
                    trigger_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = door_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, door_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (trigger_actor_index, trigger_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_secret_door_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateTeleporterArchConfig { new_config, description, .. } => {
                    self.teleporter_arch_settings.active_config = new_config.clone();
                    self.recalculate_teleporter_arch_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitTeleporterArchSetup {
                    arch_brush_index,
                    arch_brush,
                    teleporter_actor_index,
                    teleporter_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = arch_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, arch_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (teleporter_actor_index, teleporter_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_teleporter_arch_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateFluidBasinConfig { new_config, description, .. } => {
                    self.fluid_basin_settings.active_config = new_config.clone();
                    self.recalculate_fluid_basin_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitFluidBasinSetup {
                    basin_brush_index,
                    basin_brush,
                    zone_actor_index,
                    zone_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = basin_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, basin_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (zone_actor_index, zone_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_fluid_basin_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateAlarmBeaconConfig { new_config, description, .. } => {
                    self.alarm_beacon_settings.active_config = new_config.clone();
                    self.recalculate_alarm_beacon_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitAlarmBeaconSetup {
                    beacon_brush_index,
                    beacon_brush,
                    light_actor_index,
                    light_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = beacon_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, beacon_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (light_actor_index, light_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_alarm_beacon_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSupplyLockerConfig { new_config, description, .. } => {
                    self.supply_locker_settings.active_config = new_config.clone();
                    self.recalculate_supply_locker_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSupplyLockerSetup {
                    locker_brush_index,
                    locker_brush,
                    weapon_actor_index,
                    weapon_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = locker_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, locker_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (weapon_actor_index, weapon_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_supply_locker_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateSkylightConfig { new_config, description, .. } => {
                    self.skylight_settings.active_config = new_config.clone();
                    self.recalculate_skylight_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitSkylightSetup {
                    skylight_brush_index,
                    skylight_brush,
                    sunlight_actor_index,
                    sunlight_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = skylight_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, skylight_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (sunlight_actor_index, sunlight_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_skylight_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateVentDuctConfig { new_config, description, .. } => {
                    self.vent_duct_settings.active_config = new_config.clone();
                    self.recalculate_vent_duct_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitVentDuctSetup {
                    duct_brush_index,
                    duct_brush,
                    grate_actor_index,
                    grate_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = duct_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, duct_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (grate_actor_index, grate_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_vent_duct_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdatePistonConfig { new_config, description, .. } => {
                    self.piston_settings.active_config = new_config.clone();
                    self.recalculate_piston_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitPistonSetup {
                    piston_brush_index,
                    piston_brush,
                    mover_actor_index,
                    mover_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = piston_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, piston_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (mover_actor_index, mover_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_piston_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateJumpBootsConfig { new_config, description, .. } => {
                    self.jump_boots_settings.active_config = new_config.clone();
                    self.recalculate_jump_boots_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitJumpBootsSetup {
                    cradle_brush_index,
                    cradle_brush,
                    boots_actor_index,
                    boots_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = cradle_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, cradle_brush.clone()));
                    }
                    if let (Some(idx), Some(act)) = (boots_actor_index, boots_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_jump_boots_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateForceFieldConfig { new_config, description, .. } => {
                    self.force_field_settings.active_config = new_config.clone();
                    self.recalculate_force_field_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitForceFieldSetup {
                    field_brush_index,
                    field_brush,
                    pylon_brush_indices,
                    pylon_brushes,
                    generator_actor_index,
                    generator_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = field_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, field_brush.clone()));
                    }
                    for (i, p_idx) in pylon_brush_indices.iter().enumerate() {
                        if i < pylon_brushes.len() {
                            let ins_idx = (*p_idx).min(self.world_brushes.len());
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, pylon_brushes[i].clone()));
                        }
                    }
                    if let (Some(idx), Some(act)) = (generator_actor_index, generator_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_force_field_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::UpdateReactorCoreConfig { new_config, description, .. } => {
                    self.reactor_core_settings.active_config = new_config.clone();
                    self.recalculate_reactor_core_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::CommitReactorCoreSetup {
                    core_brush_index,
                    core_brush,
                    ring_brush_indices,
                    ring_brushes,
                    reactor_actor_index,
                    reactor_actor,
                    description,
                    ..
                } => {
                    if let Some(idx) = core_brush_index {
                        let ins_idx = (*idx).min(self.world_brushes.len());
                        history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, core_brush.clone()));
                    }
                    for (i, r_idx) in ring_brush_indices.iter().enumerate() {
                        if i < ring_brushes.len() {
                            let ins_idx = (*r_idx).min(self.world_brushes.len());
                            history_vec_change(&mut self.world_brushes, &mut self.map_is_dirty, |items| items.insert(ins_idx, ring_brushes[i].clone()));
                        }
                    }
                    if let (Some(idx), Some(act)) = (reactor_actor_index, reactor_actor) {
                        if let Some(ref mut unr) = self.loaded_unr {
                            let ins_idx = (*idx).min(unr.actors.len());
                            history_vec_change(&mut unr.actors, &mut self.map_is_dirty, |items| items.insert(ins_idx, act.clone()));
                        }
                    }

                    self.recalculate_reactor_core_preview();
                    self.log(format!("Redo: {}", description));
                }
                Action::ReplaceEntireLevel {
                    new_map_path,
                    new_launch_args,
                    new_brushes,
                    new_movers,
                    new_unr,
                    new_metadata,
                    description,
                    ..
                } => {
                    replace_history_value(&mut self.map_path, new_map_path.clone(), &mut self.map_is_dirty);
                    self.ut99_launch_args = new_launch_args.clone();
                    replace_history_tracked_vec(&mut self.world_brushes, new_brushes, &mut self.map_is_dirty);
                    replace_history_tracked_vec(&mut self.movers, new_movers, &mut self.map_is_dirty);
                    self.map_is_dirty = true;
                    self.loaded_unr = new_unr.clone();
                    self.map_instance_id = next_map_instance_id();
                    replace_history_value(&mut self.level_title, new_metadata.0.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_author, new_metadata.1.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ideal_player_count, new_metadata.2.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_song, new_metadata.3.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_game_type, new_metadata.4.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_enter_text, new_metadata.5.clone(), &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_brightness, new_metadata.6, &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_hue, new_metadata.7, &mut self.map_is_dirty);
                    replace_history_value(&mut self.level_ambient_saturation, new_metadata.8, &mut self.map_is_dirty);
                    self.selected_brush_index = None;
                    self.selected_actor_index = None;
                    self.selected_mover_index = None;
                    self.clear_multi_selection();
                    self.sync_brush_groups_from_brushes();
                    self.refresh_ai_path_links();

                    self.log(format!("Redo: {}", description));
                }
            }
        if let Some(ref mut unr) = self.loaded_unr {
            unr.sync_actor_views();
        }
        }
}
