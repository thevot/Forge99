//! Scene-keyed lazy CPU cache for expensive 3D editor overlay analyses.

use crate::editor_state::{
    EditorState, SkyboxLink, SoundEmitterProfile, TrajectoryArc, WarpZoneTopologyEntry,
    ZoneAudioReport,
};
use ut99_bsp::{
    DispatcherChainLink, DispatcherSequence, JumpPadTrajectoryArc, MoverTrajectory, MusicEventData,
    TeleportNetworkGraph, ZoneFluidPhysics, ZoneSealReport,
};

/// Zero-allocation case-insensitive ASCII substring search.
/// `needle_lower` must already be ASCII lowercase.
#[inline]
pub(crate) fn contains_ascii_icase(haystack: &str, needle_lower: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle_lower.as_bytes();
    if n.is_empty() {
        return true;
    }
    if n.len() > h.len() {
        return false;
    }
    h.windows(n.len())
        .any(|w| w.iter().zip(n).all(|(&a, &b)| a.to_ascii_lowercase() == b))
}

#[derive(Clone, Copy)]
struct Fnv64(u64);

impl Fnv64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    #[inline]
    fn new() -> Self {
        Self(Self::OFFSET)
    }

    #[inline]
    fn mix_u64(&mut self, v: u64) {
        self.0 ^= v;
        self.0 = self.0.wrapping_mul(Self::PRIME);
    }

    #[inline]
    fn finish(self) -> u64 {
        self.0
    }
}

/// Computes an O(1), allocation-free scene signature covering all state read by overlay analyses.
pub(super) fn compute_overlay_scene_signature(state: &EditorState, _world_sig: u64) -> u64 {
    let mut h = Fnv64::new();
    h.mix_u64(state.map_instance_id);
    h.mix_u64(state.geometry_revision);
    h.mix_u64(state.world_brushes.rev());
    h.mix_u64(state.movers.rev());
    h.mix_u64(state.static_meshes.rev());
    h.mix_u64(state.loaded_unr.as_ref().map_or(0, |l| l.actors.rev()));
    h.mix_u64(state.map.actors.rev());
    h.finish()
}

/// Lazily computes and retains scene-derived overlay analyses until the scene signature changes.
#[derive(Default)]
pub(super) struct OverlaySceneCache {
    sig: u64,
    initialized: bool,
    zone_seals: Option<Vec<ZoneSealReport>>,
    trajectories: Option<Vec<TrajectoryArc>>,
    event_tag_links: Option<Vec<(glam::Vec3, glam::Vec3, String)>>,
    skybox_links: Option<Vec<SkyboxLink>>,
    zone_audio_reports: Option<Vec<ZoneAudioReport>>,
    warpzone_topology: Option<Vec<WarpZoneTopologyEntry>>,
    mover_trajectories: Option<Vec<MoverTrajectory>>,
    dispatchers: Option<(Vec<DispatcherSequence>, Vec<DispatcherChainLink>)>,
    jumppad_arcs: Option<Vec<JumpPadTrajectoryArc>>,
    music_events: Option<Vec<MusicEventData>>,
    zone_physics: Option<Vec<(usize, String, ZoneFluidPhysics)>>,
    teleporter_network: Option<TeleportNetworkGraph>,
    sound_emitters: Option<Vec<SoundEmitterProfile>>,
    acoustic_brush_aabbs: Option<Vec<(usize, glam::Vec3, glam::Vec3)>>,
    acoustic_ray_cache: Option<([u32; 3], Vec<(glam::Vec3, bool, bool)>)>,
}

#[inline]
fn listener_pos_bits(pos: glam::Vec3) -> [u32; 3] {
    [
        if pos.x == 0.0 { 0.0f32 } else { pos.x }.to_bits(),
        if pos.y == 0.0 { 0.0f32 } else { pos.y }.to_bits(),
        if pos.z == 0.0 { 0.0f32 } else { pos.z }.to_bits(),
    ]
}

/// NaN-safe ray-segment vs AABB slab overlap test for `[t_min, t_max]`.
#[inline]
pub fn ray_intersects_aabb_segment(
    origin: glam::Vec3,
    dir_norm: glam::Vec3,
    t_min: f32,
    t_max: f32,
    min: glam::Vec3,
    max: glam::Vec3,
) -> bool {
    let mut t_enter = t_min;
    let mut t_exit = t_max;
    for axis in 0..3 {
        let o = origin[axis];
        let d = dir_norm[axis];
        let bmin = min[axis];
        let bmax = max[axis];
        if d.abs() < 1e-8 {
            if o < bmin || o > bmax {
                return false;
            }
        } else {
            let inv = 1.0 / d;
            let mut t0 = (bmin - o) * inv;
            let mut t1 = (bmax - o) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_enter = t_enter.max(t0);
            t_exit = t_exit.min(t1);
            if t_enter > t_exit {
                return false;
            }
        }
    }
    t_enter <= t_exit
}

fn build_acoustic_brush_aabbs(state: &EditorState) -> Vec<(usize, glam::Vec3, glam::Vec3)> {
    const EXCLUDE_FLAGS: u32 =
        ut99_bsp::PF_NOT_SOLID | ut99_bsp::PF_PORTAL | ut99_bsp::PF_INVISIBLE;
    let pad = glam::Vec3::splat(1e-3);
    let mut aabbs = Vec::new();
    for (b_idx, brush) in state.world_brushes.iter().enumerate() {
        let mut min = glam::Vec3::splat(f32::MAX);
        let mut max = glam::Vec3::splat(f32::MIN);
        let mut has_verts = false;
        for poly in &brush.polys {
            if (poly.flags & EXCLUDE_FLAGS) != 0 || poly.vertices.len() < 3 {
                continue;
            }
            for &v in &poly.vertices {
                let wp = v + brush.location;
                min = min.min(wp);
                max = max.max(wp);
                has_verts = true;
            }
        }
        if has_verts {
            aabbs.push((b_idx, min - pad, max + pad));
        }
    }
    aabbs
}

impl OverlaySceneCache {
    pub(super) fn sync(&mut self, sig: u64) {
        if !self.initialized || self.sig != sig {
            self.zone_seals = None;
            self.trajectories = None;
            self.event_tag_links = None;
            self.skybox_links = None;
            self.zone_audio_reports = None;
            self.warpzone_topology = None;
            self.mover_trajectories = None;
            self.dispatchers = None;
            self.jumppad_arcs = None;
            self.music_events = None;
            self.zone_physics = None;
            self.teleporter_network = None;
            self.sound_emitters = None;
            self.acoustic_brush_aabbs = None;
            self.acoustic_ray_cache = None;
            self.sig = sig;
            self.initialized = true;
        }
    }

    pub(super) fn zone_seals(&mut self, compute: impl FnOnce() -> Vec<ZoneSealReport>) -> &[ZoneSealReport] {
        self.zone_seals.get_or_insert_with(compute)
    }

    pub(super) fn trajectories(&mut self, compute: impl FnOnce() -> Vec<TrajectoryArc>) -> &[TrajectoryArc] {
        self.trajectories.get_or_insert_with(compute)
    }

    pub(super) fn event_tag_links(
        &mut self,
        compute: impl FnOnce() -> Vec<(glam::Vec3, glam::Vec3, String)>,
    ) -> &[(glam::Vec3, glam::Vec3, String)] {
        self.event_tag_links.get_or_insert_with(compute)
    }

    pub(super) fn skybox_links(&mut self, compute: impl FnOnce() -> Vec<SkyboxLink>) -> &[SkyboxLink] {
        self.skybox_links.get_or_insert_with(compute)
    }

    pub(super) fn zone_audio_reports(
        &mut self,
        compute: impl FnOnce() -> Vec<ZoneAudioReport>,
    ) -> &[ZoneAudioReport] {
        self.zone_audio_reports.get_or_insert_with(compute)
    }

    pub(super) fn warpzone_topology(
        &mut self,
        compute: impl FnOnce() -> Vec<WarpZoneTopologyEntry>,
    ) -> &[WarpZoneTopologyEntry] {
        self.warpzone_topology.get_or_insert_with(compute)
    }

    pub(super) fn mover_trajectories(
        &mut self,
        compute: impl FnOnce() -> Vec<MoverTrajectory>,
    ) -> &[MoverTrajectory] {
        self.mover_trajectories.get_or_insert_with(compute)
    }

    pub(super) fn dispatchers(
        &mut self,
        compute: impl FnOnce() -> (Vec<DispatcherSequence>, Vec<DispatcherChainLink>),
    ) -> (&[DispatcherSequence], &[DispatcherChainLink]) {
        let pair = self.dispatchers.get_or_insert_with(compute);
        (&pair.0, &pair.1)
    }

    pub(super) fn jumppad_arcs(
        &mut self,
        compute: impl FnOnce() -> Vec<JumpPadTrajectoryArc>,
    ) -> &[JumpPadTrajectoryArc] {
        self.jumppad_arcs.get_or_insert_with(compute)
    }

    pub(super) fn music_events(&mut self, compute: impl FnOnce() -> Vec<MusicEventData>) -> &[MusicEventData] {
        self.music_events.get_or_insert_with(compute)
    }

    pub(super) fn zone_physics(
        &mut self,
        compute: impl FnOnce() -> Vec<(usize, String, ZoneFluidPhysics)>,
    ) -> &[(usize, String, ZoneFluidPhysics)] {
        self.zone_physics.get_or_insert_with(compute)
    }

    pub(super) fn teleporter_network(
        &mut self,
        compute: impl FnOnce() -> TeleportNetworkGraph,
    ) -> &TeleportNetworkGraph {
        self.teleporter_network.get_or_insert_with(compute)
    }

    pub(super) fn sound_emitters(
        &mut self,
        compute: impl FnOnce() -> Vec<SoundEmitterProfile>,
    ) -> &[SoundEmitterProfile] {
        self.sound_emitters.get_or_insert_with(compute)
    }

    /// Evaluates occlusion state `(emitter_location, is_in_range, is_occluded)` per sound emitter
    /// with brush AABB broadphase + early exit, caching results per `listener_pos` until scene changes.
    pub(super) fn acoustic_ray_states(
        &mut self,
        state: &EditorState,
        listener_pos: glam::Vec3,
    ) -> &[(glam::Vec3, bool, bool)] {
        let key = listener_pos_bits(listener_pos);
        if self
            .acoustic_ray_cache
            .as_ref()
            .is_some_and(|(cached_key, _)| *cached_key == key)
        {
            return &self.acoustic_ray_cache.as_ref().expect("checked").1;
        }

        if self.sound_emitters.is_none() {
            self.sound_emitters = Some(state.collect_sound_emitters());
        }
        let emitters = self.sound_emitters.as_ref().expect("populated");

        if emitters.is_empty() {
            self.acoustic_ray_cache = Some((key, Vec::new()));
            return &self.acoustic_ray_cache.as_ref().expect("stored").1;
        }

        if self.acoustic_brush_aabbs.is_none() {
            self.acoustic_brush_aabbs = Some(build_acoustic_brush_aabbs(state));
        }
        let aabbs = self.acoustic_brush_aabbs.as_ref().expect("populated");

        const EXCLUDE_FLAGS: u32 =
            ut99_bsp::PF_NOT_SOLID | ut99_bsp::PF_PORTAL | ut99_bsp::PF_INVISIBLE;
        let pad = glam::Vec3::splat(1e-3);
        let mut results = Vec::with_capacity(emitters.len());

        for emitter in emitters {
            let delta = listener_pos - emitter.location;
            let distance = delta.length();
            let is_in_range =
                distance <= emitter.sound_radius_units && emitter.sound_radius_units > 0.0;
            if !is_in_range || distance <= 1.0 {
                results.push((emitter.location, is_in_range, false));
                continue;
            }
            let max_t = distance - 4.0;
            if max_t <= 4.0 {
                results.push((emitter.location, true, false));
                continue;
            }
            let ray = crate::raycast::Ray::new(emitter.location, delta);
            let seg_min = emitter.location.min(listener_pos) - pad;
            let seg_max = emitter.location.max(listener_pos) + pad;
            let mut occluded = false;

            'brush_loop: for &(b_idx, b_min, b_max) in aabbs {
                if b_max.x < seg_min.x
                    || b_min.x > seg_max.x
                    || b_max.y < seg_min.y
                    || b_min.y > seg_max.y
                    || b_max.z < seg_min.z
                    || b_min.z > seg_max.z
                {
                    continue;
                }
                if !ray_intersects_aabb_segment(ray.origin, ray.direction, 4.0, max_t, b_min, b_max)
                {
                    continue;
                }
                let brush = &state.world_brushes[b_idx];
                for poly in &brush.polys {
                    if (poly.flags & EXCLUDE_FLAGS) != 0 {
                        continue;
                    }
                    let n = poly.vertices.len();
                    if n < 3 {
                        continue;
                    }
                    let v0 = poly.vertices[0] + brush.location;
                    for i in 1..n - 1 {
                        let v1 = poly.vertices[i] + brush.location;
                        let v2 = poly.vertices[i + 1] + brush.location;
                        if let Some(t) = ray.intersect_triangle(v0, v1, v2) {
                            if t > 4.0 && t < max_t {
                                occluded = true;
                                break 'brush_loop;
                            }
                        }
                    }
                }
            }
            results.push((emitter.location, true, occluded));
        }

        self.acoustic_ray_cache = Some((key, results));
        &self.acoustic_ray_cache.as_ref().expect("stored").1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_state::EditorMover;
    use glam::Vec3;
    use ut99_bsp::{build_cube, CsgOperation};
    use ut99_format::{ActorRecord, TaggedProperty, TaggedValue};

    #[test]
    fn contains_ascii_icase_handles_mixed_case_and_edges() {
        assert!(contains_ascii_icase("Engine.PlayerStart", "playerstart"));
        assert!(contains_ascii_icase("Botpack.UT_Eightball", "ut_eightball"));
        assert!(contains_ascii_icase("TriggerLight", "light"));
        assert!(contains_ascii_icase("Anything", ""));
        assert!(!contains_ascii_icase("Brush", "playerstart"));
        assert!(!contains_ascii_icase("StaticMesh", "trigger"));
    }

    #[test]
    fn signature_is_deterministic_and_invalidates_on_scene_edits() {
        let mut state = EditorState::default();
        state.new_map();
        let mut disp = ActorRecord::default();
        disp.name = "Dispatcher0".to_string();
        disp.class_name = "Engine.Dispatcher".to_string();
        disp.location = Vec3::new(64.0, 0.0, 32.0);
        disp.properties.insert(
            "OutDelays[0]".to_string(),
            TaggedProperty {
                name: "OutDelays[0]".to_string(),
                array_index: 0,
                value: TaggedValue::Float(1.5),
            },
        );
        let disp_idx = state
            .loaded_unr
            .as_mut()
            .map(|lvl| {
                let idx = lvl.actors.len();
                lvl.actors.push(disp);
                idx
            })
            .expect("new_map initializes loaded_unr");
        let mut mover = EditorMover::new(build_cube("M0", CsgOperation::Add, Vec3::splat(64.0)));
        mover.keyframes[0] = Some(crate::editor_state::MoverKeyframe {
            location: Vec3::ZERO,
            rotation: Vec3::ZERO,
        });
        state.movers.push(mover);

        let base = compute_overlay_scene_signature(&state, 0);
        assert_eq!(base, compute_overlay_scene_signature(&state, 0));

        // 1. Brush edit
        state.world_brushes[0].location.x += 16.0;
        let s_brush = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_brush, base);

        // 2. Actor location & tag/event edit
        state.loaded_unr.as_mut().unwrap().actors[disp_idx].location.z += 32.0;
        let s_act_loc = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_act_loc, s_brush);

        state.loaded_unr.as_mut().unwrap().actors[disp_idx].tag = "DoorOpen".to_string();
        let s_act_tag = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_act_tag, s_act_loc);

        // 3. Dispatcher property edit
        state.loaded_unr.as_mut().unwrap().actors[disp_idx]
            .properties
            .get_mut("OutDelays[0]")
            .unwrap()
            .value = TaggedValue::Float(3.0);
        let s_disp_prop = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_disp_prop, s_act_tag);

        // 4. Mover keyframe edit
        state.movers[0].keyframes[1] = Some(crate::editor_state::MoverKeyframe {
            location: Vec3::new(0.0, 0.0, 256.0),
            rotation: Vec3::ZERO,
        });
        let s_mover = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_mover, s_disp_prop);

        // 5. Reset via new_map
        state.new_map();
        let s_new = compute_overlay_scene_signature(&state, 0);
        assert_ne!(s_new, s_mover);
        assert_ne!(s_new, base);
    }

    #[test]
    fn overlay_scene_cache_sync_reuses_and_invalidates_cleanly() {
        let mut cache = OverlaySceneCache::default();
        let mut calls = 0usize;

        cache.sync(101);
        let r1 = cache.zone_seals(|| {
            calls += 1;
            Vec::new()
        });
        assert!(r1.is_empty());
        assert_eq!(calls, 1);

        // Same signature hits cached result
        cache.sync(101);
        let _ = cache.zone_seals(|| {
            calls += 1;
            Vec::new()
        });
        assert_eq!(calls, 1);

        // Signature change invalidates lazy entry
        cache.sync(202);
        let _ = cache.zone_seals(|| {
            calls += 1;
            Vec::new()
        });
        assert_eq!(calls, 2);
    }

    #[test]
    fn trajectories_with_carved_match_uncached_on_kicker_room() {
        let mut state = EditorState::default();
        state.new_map();
        state.world_brushes.clear();
        state.world_brushes.push(build_cube(
            "Room",
            CsgOperation::Subtract,
            Vec3::new(2048.0, 2048.0, 512.0),
        ));

        let mut kicker = ActorRecord::default();
        kicker.name = "Kicker0".to_string();
        kicker.class_name = "Botpack.Kicker".to_string();
        kicker.location = Vec3::new(0.0, 0.0, -240.0);
        kicker.properties.insert(
            "KickVelocity".to_string(),
            TaggedProperty {
                name: "KickVelocity".to_string(),
                array_index: 0,
                value: TaggedValue::Vector(Vec3::new(320.0, 0.0, 620.0)),
            },
        );
        if let Some(ref mut lvl) = state.loaded_unr {
            lvl.actors.push(kicker);
        }

        let carved = ut99_bsp::evaluate_csg_brushes_indexed(&state.world_brushes);
        let expected = state.compute_trajectories();
        let actual = state.compute_trajectories_with_carved(Some(&carved));
        assert_eq!(expected.len(), 1);
        assert_eq!(actual.len(), expected.len());
        assert_eq!(actual[0].points.len(), expected[0].points.len());
        for (a, e) in actual[0].points.iter().zip(&expected[0].points) {
            assert!(a.distance(*e) < 1e-4, "point mismatch: {a:?} vs {e:?}");
        }
        assert_eq!(actual[0].landing_point, expected[0].landing_point);
    }

    #[test]
    fn read_only_and_camera_orbit_keep_tracked_revs_and_signature_constant() {
        let mut state = EditorState::default();
        state.new_map();
        let mover = EditorMover::new(build_cube("M0", CsgOperation::Add, Vec3::splat(64.0)));
        state.movers.push(mover);

        let rev_brushes = state.world_brushes.rev();
        let rev_movers = state.movers.rev();
        let rev_actors = state.loaded_unr.as_ref().unwrap().actors.rev();
        let map_id = state.map_instance_id;
        let sig0 = compute_overlay_scene_signature(&state, 0);

        state.camera.yaw += 0.75;
        state.camera.pitch -= 0.12;
        state.camera.position += Vec3::new(128.0, -64.0, 32.0);
        let _ = state.world_brushes.len();
        let _ = state.world_brushes.iter().count();
        let _ = state.movers.get(0);
        let _ = state.loaded_unr.as_ref().unwrap().actors.as_slice();

        assert_eq!(state.world_brushes.rev(), rev_brushes);
        assert_eq!(state.movers.rev(), rev_movers);
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors.rev(), rev_actors);
        assert_eq!(state.map_instance_id, map_id);
        assert_eq!(compute_overlay_scene_signature(&state, 0), sig0);

        assert!(state.new_map());
        assert_ne!(state.map_instance_id, map_id);
        assert_ne!(compute_overlay_scene_signature(&state, 0), sig0);
    }

    #[test]
    fn acoustic_ray_states_match_legacy_and_cache_cleanly() {
        let mut state = EditorState::default();
        state.new_map();
        state.world_brushes.clear();

        let mut wall = build_cube("Wall", CsgOperation::Add, Vec3::new(32.0, 512.0, 512.0));
        wall.location = Vec3::new(200.0, 0.0, 0.0);
        state.world_brushes.push(wall);

        let mut portal_wall =
            build_cube("PortalWall", CsgOperation::Add, Vec3::new(32.0, 512.0, 512.0));
        portal_wall.location = Vec3::new(300.0, 0.0, 0.0);
        for p in &mut portal_wall.polys {
            p.flags |= ut99_bsp::PF_PORTAL;
        }
        state.world_brushes.push(portal_wall);

        let add_emitter = |state: &mut EditorState, name: &str, loc: Vec3, radius_byte: u8| {
            let mut a = ActorRecord::default();
            a.name = name.to_string();
            a.tag = name.to_string();
            a.class_name = "Engine.AmbientSound".to_string();
            a.location = loc;
            a.properties.insert(
                "SoundRadius".to_string(),
                TaggedProperty {
                    name: "SoundRadius".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(radius_byte),
                },
            );
            state.loaded_unr.as_mut().unwrap().actors.push(a);
        };
        // SoundRadius 32 -> 1280 UU; SoundRadius 10 -> 400 UU
        add_emitter(&mut state, "EmBlocked", Vec3::new(0.0, 0.0, 0.0), 32);
        add_emitter(&mut state, "EmClear", Vec3::new(230.0, 0.0, 0.0), 32);
        add_emitter(&mut state, "EmFar", Vec3::new(0.0, 2000.0, 0.0), 10);

        let mut cache = OverlaySceneCache::default();
        let listener = Vec3::new(400.0, 0.0, 0.0);
        let sig = compute_overlay_scene_signature(&state, 0);
        cache.sync(sig);

        let cached_rays = cache.acoustic_ray_states(&state, listener).to_vec();
        let first_ptr = cache.acoustic_ray_states(&state, listener).as_ptr();
        let second_ptr = cache.acoustic_ray_states(&state, listener).as_ptr();
        assert_eq!(first_ptr, second_ptr);
        assert_eq!(cached_rays.len(), 3);
        let legacy_0 = state
            .compute_acoustic_occlusion_for_emitter("EmBlocked", listener)
            .unwrap();
        let legacy_1 = state
            .compute_acoustic_occlusion_for_emitter("EmClear", listener)
            .unwrap();
        let legacy_2 = state
            .compute_acoustic_occlusion_for_emitter("EmFar", listener)
            .unwrap();
        assert_eq!(
            cached_rays[0],
            (Vec3::new(0.0, 0.0, 0.0), legacy_0.is_in_range, legacy_0.walls_occluding_count > 0)
        );
        assert!(cached_rays[0].1 && cached_rays[0].2);
        assert_eq!(
            cached_rays[1],
            (Vec3::new(230.0, 0.0, 0.0), legacy_1.is_in_range, legacy_1.walls_occluding_count > 0)
        );
        assert!(cached_rays[1].1 && !cached_rays[1].2);
        assert!(!legacy_2.is_in_range);
        assert!(!cached_rays[2].1);

        // Moving listener changes cache entry and preserves occlusion parity.
        let shifted_listener = Vec3::new(420.0, 32.0, 0.0);
        let shifted_ptr = cache.acoustic_ray_states(&state, shifted_listener).as_ptr();
        assert_ne!(shifted_ptr, first_ptr);

        // Moving wall clear of EmBlocked invalidates cache via signature change.
        state.world_brushes[0].location.y = 2000.0;
        let sig2 = compute_overlay_scene_signature(&state, 0);
        assert_ne!(sig2, sig);
        cache.sync(sig2);
        let unblocked = cache.acoustic_ray_states(&state, listener);
        assert!(unblocked[0].1 && !unblocked[0].2);
    }
}
