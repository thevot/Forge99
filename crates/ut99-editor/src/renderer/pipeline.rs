use super::lightmap_atlas::{lightmap_content_key, LightmapAtlas, LightmapAtlasRect};
use super::shaders::{LINE_SHADER, SOLID_SHADER, VIEWPORT_BLIT_SHADER};
use super::vertex::{LineVertex, SolidVertex, Uniforms};
use crate::camera::Camera;
use glam::{Quat, Vec3};
use ut99_bsp::CsgOperation;
use wgpu::util::DeviceExt;

pub const MAX_TEXTURE_ARRAY_LAYERS: u32 = 256;
pub const MAX_DIFFUSE_TEXTURE_LAYERS: u32 = 256;
pub const TEXTURE_ARRAY_DIM: u32 = 256;
pub const TEXTURE_ARRAY_MIP_LEVELS: u32 = 9;

fn brush_model_matrix(brush: &ut99_bsp::Brush) -> glam::Mat4 {
    let rotation = glam::Mat4::from_euler(
        glam::EulerRot::YXZ,
        brush.rotation.y.to_radians(),
        brush.rotation.x.to_radians(),
        brush.rotation.z.to_radians(),
    );
    glam::Mat4::from_translation(brush.location) * rotation * glam::Mat4::from_scale(brush.scale)
}

fn srgb_u8_to_linear(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb_u8(lin: f32) -> u8 {
    let c = lin.clamp(0.0, 1.0);
    let s = if c <= 0.0031308 {
        c * 12.92
    } else {
        1.055 * c.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// 2x2 box downsample for RGBA8 sRGB textures, weighting RGB over opaque texels (>=128 alpha)
/// when any texel in the 2x2 block is opaque so masked cutouts avoid dark mip fringes.
fn downsample_rgba8_srgb_2x2(src: &[u8], src_w: u32, src_h: u32) -> (u32, u32, Vec<u8>) {
    let dst_w = (src_w / 2).max(1);
    let dst_h = (src_h / 2).max(1);
    let mut dst = Vec::with_capacity((dst_w * dst_h * 4) as usize);
    for dy in 0..dst_h {
        let sy0 = (dy * 2).min(src_h - 1);
        let sy1 = (dy * 2 + 1).min(src_h - 1);
        for dx in 0..dst_w {
            let sx0 = (dx * 2).min(src_w - 1);
            let sx1 = (dx * 2 + 1).min(src_w - 1);
            let coords = [(sx0, sy0), (sx1, sy0), (sx0, sy1), (sx1, sy1)];
            let mut a_sum = 0u32;
            let mut lin_opaque = [0.0f32; 3];
            let mut opaque_count = 0.0f32;
            let mut lin_all = [0.0f32; 3];
            for &(sx, sy) in &coords {
                let idx = ((sy * src_w + sx) * 4) as usize;
                let r = srgb_u8_to_linear(src[idx]);
                let g = srgb_u8_to_linear(src[idx + 1]);
                let b = srgb_u8_to_linear(src[idx + 2]);
                let a = src[idx + 3];
                a_sum += a as u32;
                lin_all[0] += r;
                lin_all[1] += g;
                lin_all[2] += b;
                if a >= 128 {
                    lin_opaque[0] += r;
                    lin_opaque[1] += g;
                    lin_opaque[2] += b;
                    opaque_count += 1.0;
                }
            }
            let (avg_r, avg_g, avg_b) = if opaque_count > 0.0 {
                (
                    lin_opaque[0] / opaque_count,
                    lin_opaque[1] / opaque_count,
                    lin_opaque[2] / opaque_count,
                )
            } else {
                (lin_all[0] * 0.25, lin_all[1] * 0.25, lin_all[2] * 0.25)
            };
            dst.push(linear_to_srgb_u8(avg_r));
            dst.push(linear_to_srgb_u8(avg_g));
            dst.push(linear_to_srgb_u8(avg_b));
            dst.push(((a_sum + 2) / 4).min(255) as u8);
        }
    }
    (dst_w, dst_h, dst)
}

fn upload_layer_rgba_with_mipmaps(
    queue: &wgpu::Queue,
    texture_array: &wgpu::Texture,
    layer: u32,
    base_rgba: &[u8],
) {
    let mip_count = texture_array.mip_level_count().max(1);
    let mut cur_w = TEXTURE_ARRAY_DIM;
    let mut cur_h = TEXTURE_ARRAY_DIM;
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: texture_array,
            mip_level: 0,
            origin: wgpu::Origin3d {
                x: 0,
                y: 0,
                z: layer,
            },
            aspect: wgpu::TextureAspect::All,
        },
        base_rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(cur_w * 4),
            rows_per_image: Some(cur_h),
        },
        wgpu::Extent3d {
            width: cur_w,
            height: cur_h,
            depth_or_array_layers: 1,
        },
    );
    if mip_count <= 1 {
        return;
    }
    let mut cur = base_rgba.to_vec();
    for mip in 1..mip_count {
        let (next_w, next_h, next_data) = downsample_rgba8_srgb_2x2(&cur, cur_w, cur_h);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: texture_array,
                mip_level: mip,
                origin: wgpu::Origin3d {
                    x: 0,
                    y: 0,
                    z: layer,
                },
                aspect: wgpu::TextureAspect::All,
            },
            &next_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(next_w * 4),
                rows_per_image: Some(next_h),
            },
            wgpu::Extent3d {
                width: next_w,
                height: next_h,
                depth_or_array_layers: 1,
            },
        );
        cur_w = next_w;
        cur_h = next_h;
        cur = next_data;
    }
}

fn retain_vertex_storage<T>(vertices: Vec<T>, max_retained_bytes: usize) -> Vec<T> {
    if vertices.capacity().saturating_mul(std::mem::size_of::<T>()) <= max_retained_bytes {
        vertices
    } else {
        Vec::new()
    }
}

#[derive(Clone, Copy)]
struct SceneHash(u64);

impl SceneHash {
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
    fn mix_u32(&mut self, v: u32) {
        self.mix_u64(v as u64);
    }

    #[inline]
    fn mix_bool(&mut self, v: bool) {
        self.mix_u64(u64::from(v));
    }

    #[inline]
    fn mix_usize(&mut self, v: usize) {
        self.mix_u64(v as u64);
    }

    #[inline]
    fn mix_opt_usize(&mut self, v: Option<usize>) {
        match v {
            Some(idx) => {
                self.mix_u64(1);
                self.mix_usize(idx);
            }
            None => self.mix_u64(0),
        }
    }

    #[inline]
    fn mix_vec3(&mut self, v: glam::Vec3) {
        self.mix_u32(v.x.to_bits());
        self.mix_u32(v.y.to_bits());
        self.mix_u32(v.z.to_bits());
    }

    #[inline]
    fn mix_str(&mut self, s: &str) {
        self.mix_usize(s.len());
        for &b in s.as_bytes() {
            self.0 ^= b as u64;
            self.0 = self.0.wrapping_mul(Self::PRIME);
        }
    }

    #[inline]
    fn mix_opt_str(&mut self, s: Option<&str>) {
        match s {
            Some(text) => {
                self.mix_u64(1);
                self.mix_str(text);
            }
            None => self.mix_u64(0),
        }
    }

    #[inline]
    fn finish(self) -> u64 {
        self.0
    }
}

#[inline]
fn shading_tag(shading: crate::camera::ViewportShading) -> u8 {
    match shading {
        crate::camera::ViewportShading::Wireframe => 0,
        crate::camera::ViewportShading::Unlit => 1,
        crate::camera::ViewportShading::Lit => 2,
        crate::camera::ViewportShading::ZonePortal => 3,
        crate::camera::ViewportShading::TextureUsage => 4,
        crate::camera::ViewportShading::BspCuts => 5,
        crate::camera::ViewportShading::DepthComplexity => 6,
    }
}

#[inline]
fn viewport_type_tag(vt: crate::camera::ViewportType) -> u8 {
    match vt {
        crate::camera::ViewportType::Top => 0,
        crate::camera::ViewportType::Front => 1,
        crate::camera::ViewportType::Side => 2,
        crate::camera::ViewportType::Perspective => 3,
    }
}

#[inline]
fn mix_usize_set(h: &mut SceneHash, set: &std::collections::HashSet<usize>) {
    h.mix_usize(set.len());
    let mut sum = 0u64;
    for &item in set {
        let mut eh = SceneHash::new();
        eh.mix_usize(item);
        sum = sum.wrapping_add(eh.finish());
    }
    h.mix_u64(sum);
}

#[inline]
fn mix_pair_set(h: &mut SceneHash, set: &std::collections::HashSet<(usize, usize)>) {
    h.mix_usize(set.len());
    let mut sum = 0u64;
    for &(a, b) in set {
        let mut eh = SceneHash::new();
        eh.mix_usize(a);
        eh.mix_usize(b);
        sum = sum.wrapping_add(eh.finish());
    }
    h.mix_u64(sum);
}

#[inline]
fn mix_str_set(h: &mut SceneHash, set: &std::collections::HashSet<String>) {
    h.mix_usize(set.len());
    let mut sum = 0u64;
    for item in set {
        let mut eh = SceneHash::new();
        eh.mix_str(item);
        sum = sum.wrapping_add(eh.finish());
    }
    h.mix_u64(sum);
}

#[inline]
fn mix_brush_groups_map(h: &mut SceneHash, map: &std::collections::HashMap<usize, String>) {
    h.mix_usize(map.len());
    let mut sum = 0u64;
    for (&idx, name) in map {
        let mut eh = SceneHash::new();
        eh.mix_usize(idx);
        eh.mix_str(name);
        sum = sum.wrapping_add(eh.finish());
    }
    h.mix_u64(sum);
}

fn compute_solid_visibility_hash(state: &crate::editor_state::EditorState) -> u64 {
    let mut h = SceneHash::new();
    mix_usize_set(&mut h, &state.hidden_brushes);
    mix_usize_set(&mut h, &state.hidden_movers);
    mix_usize_set(&mut h, &state.hidden_static_meshes);
    mix_str_set(&mut h, &state.hidden_actor_groups);
    mix_brush_groups_map(&mut h, &state.brush_groups);
    h.finish()
}

fn compute_line_visibility_selection_hash(state: &crate::editor_state::EditorState) -> u64 {
    let mut h = SceneHash::new();
    h.mix_u64(compute_solid_visibility_hash(state));
    mix_usize_set(&mut h, &state.hidden_actors);
    mix_usize_set(&mut h, &state.multi_selected_brushes);
    mix_pair_set(&mut h, &state.selected_surfaces);
    mix_usize_set(&mut h, &state.multi_selected_actors);
    h.mix_opt_usize(state.selected_brush_index);
    h.mix_opt_usize(state.selected_actor_index);
    h.mix_opt_usize(state.selected_mover_index);
    h.mix_opt_usize(state.selected_static_mesh_index);
    h.mix_opt_usize(state.selected_projector_index);
    h.finish()
}

fn compute_bg_hash(state: &crate::editor_state::EditorState) -> u64 {
    let mut h = SceneHash::new();
    h.mix_opt_str(state.background_image_path.as_deref());
    h.mix_u64(match state.background_display_mode {
        crate::editor_state::BackgroundDisplayMode::Center => 0,
        crate::editor_state::BackgroundDisplayMode::Tile => 1,
        crate::editor_state::BackgroundDisplayMode::Stretch => 2,
    });
    h.finish()
}

fn compute_builder_brush_hash(state: &crate::editor_state::EditorState) -> u64 {
    let mut h = SceneHash::new();
    h.mix_bool(state.show_builder_brush);
    h.mix_bool(state.is_builder_selected);
    if state.show_builder_brush {
        let b = &state.builder_brush;
        h.mix_u64(match b.csg_oper {
            ut99_bsp::CsgOperation::Add => 1,
            ut99_bsp::CsgOperation::Subtract => 2,
            _ => 3,
        });
        h.mix_vec3(b.location);
        h.mix_vec3(b.rotation);
        h.mix_vec3(b.scale);
        h.mix_usize(b.polys.len());
        for p in &b.polys {
            h.mix_usize(p.vertices.len());
            for &v in &p.vertices {
                h.mix_vec3(v);
            }
        }
    }
    h.finish()
}

fn compute_overlay_flags_hash(state: &crate::editor_state::EditorState) -> u64 {
    let mut h = SceneHash::new();
    h.mix_bool(state.show_ai_paths);
    h.mix_bool(state.show_event_links);
    h.mix_bool(state.show_trajectories);
    h.mix_bool(state.show_actor_radii);
    h.mix_bool(state.show_skybox_links);
    h.mix_bool(state.sky_zone_settings.highlight_sky_surfaces);
    h.mix_bool(state.show_nav_coverage);
    h.mix_bool(state.show_audio_zones);
    h.mix_bool(state.show_warpzones);
    h.mix_opt_usize(state.warp_zone_settings.selected_warp_index);
    h.mix_bool(state.warp_zone_settings.show_portal_normal_vectors);
    h.mix_bool(state.warp_zone_settings.show_connecting_helices);
    h.mix_bool(state.warp_zone_settings.show_portal_camera_frustum);
    h.mix_bool(state.show_anti_portals);
    h.mix_bool(state.show_zone_leaks_visual);
    h.mix_bool(state.show_mover_trajectories);
    h.mix_opt_str(state.dispatcher_settings.selected_dispatcher_name.as_deref());
    h.mix_u32(state.dispatcher_settings.timeline_time.to_bits());
    h.mix_opt_str(state.selected_dispatcher_name.as_deref());
    h.mix_bool(state.show_zone_pvs);
    h.mix_bool(state.show_bot_paths);
    h.mix_opt_usize(state.bot_nav_settings.selected_node_id);
    h.mix_bool(state.bot_nav_settings.show_tactical_cover);
    h.mix_bool(state.bot_nav_settings.show_jumppad_arcs);
    h.mix_u64(compute_bg_hash(state));
    h.finish()
}

fn compute_nav_graph_hash(state: &crate::editor_state::EditorState) -> u64 {
    if !(state.show_ai_paths || state.show_bot_paths || state.show_nav_coverage) {
        return 0;
    }
    let g = &state.nav_graph;
    if g.nodes.is_empty() && g.reach_specs.is_empty() {
        return 0;
    }
    let mut h = SceneHash::new();
    h.mix_usize(g.nodes.len());
    h.mix_usize(g.reach_specs.len());
    for node in &g.nodes {
        h.mix_vec3(node.location);
        h.mix_str(&node.node_type);
    }
    for spec in &g.reach_specs {
        h.mix_usize(spec.from);
        h.mix_usize(spec.to);
        h.mix_u32(spec.reach_flags);
    }
    h.finish()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct StaticSolidRanges {
    opaque_end: u32,
    twosided_end: u32,
    detail_end: u32,
    masked_end: u32,
    total: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticSolidKey {
    map_instance_id: u64,
    world_brushes_rev: u64,
    geometry_revision: u64,
    movers_rev: u64,
    static_meshes_rev: u64,
    decals_rev: u64,
    texture_resource_revision: u64,
    effective_shading: u8,
    any_solid_viewport: bool,
    render_lux_heatmap: bool,
    show_mover_polys: bool,
    selected_mover_index: Option<usize>,
    visibility_hash: u64,
    zone_flood_sig: u64,
    bg_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct StaticLineKey {
    map_instance_id: u64,
    grid_step_bits: u32,
    grid_extent_bits: u32,
    viewport_layout: u8,
    active_viewport_type: u8,
    effective_shading: u8,
    any_solid_viewport: bool,
    zone_flood_sig: u64,
    world_brushes_rev: u64,
    geometry_revision: u64,
    movers_rev: u64,
    static_meshes_rev: u64,
    projectors_rev: u64,
    decals_rev: u64,
    actors_rev: u64,
    map_actors_rev: u64,
    ai_path_links_rev: u64,
    visibility_selection_hash: u64,
    builder_brush_hash: u64,
    overlay_flags_hash: u64,
    nav_graph_hash: u64,
    traj_csg_sig: u64,
}

#[derive(Clone)]
struct CachedStudioLinesKey {
    map_instance_id: u64,
    world_brushes_rev: u64,
    geometry_revision: u64,
    actors_rev: u64,
    selected_actor_index: Option<usize>,
    builder_brush_location: [u32; 3],
    show_skeletal_bones: bool,
    show_cinematic_studio_dialog: bool,
    show_light_effects_studio_dialog: bool,
    show_zone_occlusion_studio_dialog: bool,
    show_music_studio_dialog: bool,
    show_corona_studio_dialog: bool,
    show_fluid_studio_dialog: bool,
    show_ballistics_studio_dialog: bool,
    show_teleporter_studio_dialog: bool,
    show_translocator_studio_dialog: bool,
    show_lift_pathing_studio_dialog: bool,
    show_water_surface_studio_dialog: bool,
    show_mover_studio_dialog: bool,
    acoustic_enabled: bool,
    acoustic_show_radii_spheres: bool,
    selected_sound_emitter_tag: Option<String>,
    skeletal_settings: crate::editor_state::SkeletalStudioSettings,
    skeletal_meshes: Vec<ut99_bsp::SkeletalMesh>,
    cinematic_settings: crate::editor_state::CinematicStudioSettings,
    light_effects_settings: crate::editor_state::LightEffectsStudioSettings,
    zone_occlusion_settings: crate::editor_state::ZoneOcclusionStudioSettings,
    antiportals: Vec<ut99_bsp::AntiPortalVolume>,
    music_studio_settings: crate::editor_state::MusicStudioSettings,
    corona_studio_settings: crate::editor_state::CoronaStudioSettings,
    fluid_studio_settings: crate::editor_state::FluidStudioSettings,
    ballistics_studio_settings: crate::editor_state::BallisticsStudioSettings,
    teleporter_studio_settings: crate::editor_state::TeleporterStudioSettings,
    translocator_studio_settings: crate::editor_state::TranslocatorStudioSettings,
    lift_pathing_studio_settings: crate::editor_state::LiftPathingStudioSettings,
    water_surface_studio_settings: crate::editor_state::WaterSurfaceStudioSettings,
    mover_studio_settings: crate::editor_state::MoverStudioSettings,
}

impl CachedStudioLinesKey {
    #[inline]
    fn builder_loc_bits(state: &crate::editor_state::EditorState) -> [u32; 3] {
        let p = state.builder_brush.location;
        [p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]
    }

    fn matches(&self, state: &crate::editor_state::EditorState, actors_rev: u64) -> bool {
        self.map_instance_id == state.map_instance_id
            && self.world_brushes_rev == state.world_brushes.rev()
            && self.geometry_revision == state.geometry_revision
            && self.actors_rev == actors_rev
            && self.selected_actor_index == state.selected_actor_index
            && self.builder_brush_location == Self::builder_loc_bits(state)
            && self.show_skeletal_bones == state.show_skeletal_bones
            && self.show_cinematic_studio_dialog == state.show_cinematic_studio_dialog
            && self.show_light_effects_studio_dialog == state.show_light_effects_studio_dialog
            && self.show_zone_occlusion_studio_dialog == state.show_zone_occlusion_studio_dialog
            && self.show_music_studio_dialog == state.show_music_studio_dialog
            && self.show_corona_studio_dialog == state.show_corona_studio_dialog
            && self.show_fluid_studio_dialog == state.show_fluid_studio_dialog
            && self.show_ballistics_studio_dialog == state.show_ballistics_studio_dialog
            && self.show_teleporter_studio_dialog == state.show_teleporter_studio_dialog
            && self.show_translocator_studio_dialog == state.show_translocator_studio_dialog
            && self.show_lift_pathing_studio_dialog == state.show_lift_pathing_studio_dialog
            && self.show_water_surface_studio_dialog == state.show_water_surface_studio_dialog
            && self.show_mover_studio_dialog == state.show_mover_studio_dialog
            && self.acoustic_enabled == state.acoustic_settings.enabled
            && self.acoustic_show_radii_spheres == state.acoustic_settings.show_radii_spheres
            && self.selected_sound_emitter_tag == state.selected_sound_emitter_tag
            && self.skeletal_settings == state.skeletal_settings
            && self.skeletal_meshes == state.skeletal_meshes
            && self.cinematic_settings == state.cinematic_settings
            && self.light_effects_settings == state.light_effects_settings
            && self.zone_occlusion_settings == state.zone_occlusion_settings
            && self.antiportals == state.antiportals
            && self.music_studio_settings == state.music_studio_settings
            && self.corona_studio_settings == state.corona_studio_settings
            && self.fluid_studio_settings == state.fluid_studio_settings
            && self.ballistics_studio_settings == state.ballistics_studio_settings
            && self.teleporter_studio_settings == state.teleporter_studio_settings
            && self.translocator_studio_settings == state.translocator_studio_settings
            && self.lift_pathing_studio_settings == state.lift_pathing_studio_settings
            && self.water_surface_studio_settings == state.water_surface_studio_settings
            && self.mover_studio_settings == state.mover_studio_settings
    }

    fn capture(state: &crate::editor_state::EditorState, actors_rev: u64) -> Self {
        Self {
            map_instance_id: state.map_instance_id,
            world_brushes_rev: state.world_brushes.rev(),
            geometry_revision: state.geometry_revision,
            actors_rev,
            selected_actor_index: state.selected_actor_index,
            builder_brush_location: Self::builder_loc_bits(state),
            show_skeletal_bones: state.show_skeletal_bones,
            show_cinematic_studio_dialog: state.show_cinematic_studio_dialog,
            show_light_effects_studio_dialog: state.show_light_effects_studio_dialog,
            show_zone_occlusion_studio_dialog: state.show_zone_occlusion_studio_dialog,
            show_music_studio_dialog: state.show_music_studio_dialog,
            show_corona_studio_dialog: state.show_corona_studio_dialog,
            show_fluid_studio_dialog: state.show_fluid_studio_dialog,
            show_ballistics_studio_dialog: state.show_ballistics_studio_dialog,
            show_teleporter_studio_dialog: state.show_teleporter_studio_dialog,
            show_translocator_studio_dialog: state.show_translocator_studio_dialog,
            show_lift_pathing_studio_dialog: state.show_lift_pathing_studio_dialog,
            show_water_surface_studio_dialog: state.show_water_surface_studio_dialog,
            show_mover_studio_dialog: state.show_mover_studio_dialog,
            acoustic_enabled: state.acoustic_settings.enabled,
            acoustic_show_radii_spheres: state.acoustic_settings.show_radii_spheres,
            selected_sound_emitter_tag: state.selected_sound_emitter_tag.clone(),
            skeletal_settings: state.skeletal_settings.clone(),
            skeletal_meshes: state.skeletal_meshes.clone(),
            cinematic_settings: state.cinematic_settings.clone(),
            light_effects_settings: state.light_effects_settings.clone(),
            zone_occlusion_settings: state.zone_occlusion_settings.clone(),
            antiportals: state.antiportals.clone(),
            music_studio_settings: state.music_studio_settings.clone(),
            corona_studio_settings: state.corona_studio_settings.clone(),
            fluid_studio_settings: state.fluid_studio_settings.clone(),
            ballistics_studio_settings: state.ballistics_studio_settings.clone(),
            teleporter_studio_settings: state.teleporter_studio_settings.clone(),
            translocator_studio_settings: state.translocator_studio_settings.clone(),
            lift_pathing_studio_settings: state.lift_pathing_studio_settings.clone(),
            water_surface_studio_settings: state.water_surface_studio_settings.clone(),
            mover_studio_settings: state.mover_studio_settings.clone(),
        }
    }
}

const BG_CENTER_RECTS: [(f32, f32, f32, f32); 1] = [(0.0, 0.0, 512.0, 512.0)];
const BG_STRETCH_RECTS: [(f32, f32, f32, f32); 1] = [(0.0, 0.0, 1024.0, 1024.0)];
const BG_TILE_RECTS: [(f32, f32, f32, f32); 9] = [
    (-512.0, -512.0, 256.0, 256.0),
    (-512.0, 0.0, 256.0, 256.0),
    (-512.0, 512.0, 256.0, 256.0),
    (0.0, -512.0, 256.0, 256.0),
    (0.0, 0.0, 256.0, 256.0),
    (0.0, 512.0, 256.0, 256.0),
    (512.0, -512.0, 256.0, 256.0),
    (512.0, 0.0, 256.0, 256.0),
    (512.0, 512.0, 256.0, 256.0),
];

fn background_blueprint_rects(
    mode: crate::editor_state::BackgroundDisplayMode,
) -> &'static [(f32, f32, f32, f32)] {
    match mode {
        crate::editor_state::BackgroundDisplayMode::Center => &BG_CENTER_RECTS,
        crate::editor_state::BackgroundDisplayMode::Tile => &BG_TILE_RECTS,
        crate::editor_state::BackgroundDisplayMode::Stretch => &BG_STRETCH_RECTS,
    }
}

struct ScaledViewportTarget {
    width: u32,
    height: u32,
    color_view: wgpu::TextureView,
    depth_view: wgpu::TextureView,
    blit_bind_group: wgpu::BindGroup,
}

pub struct Renderer {
    grid_cache: super::grid_cache::GridCache,
    cached_lightmap_keys: Vec<Option<u64>>,
    cached_lightmap_keys_valid: bool,
    pub last_lightmap_fallbacks: usize,
    pub last_texture_fallbacks: usize,
    last_frame_stats: crate::graphics::RenderFrameStats,
    gpu_timer: Option<super::gpu_timing::GpuTimer>,
    frame_lines: Vec<LineVertex>,
    frame_solids: Vec<SolidVertex>,
    frame_solid_indices: Vec<u32>,
    frame_indices_twosided: Vec<u32>,
    frame_indices_detail: Vec<u32>,
    frame_indices_masked: Vec<u32>,
    frame_indices_translucent: Vec<u32>,
    world_wireframe: super::wireframe::WireframeScratch,
    pub line_pipeline: wgpu::RenderPipeline,
    pub solid_opaque_pipeline: wgpu::RenderPipeline,
    pub solid_opaque_culled_pipeline: wgpu::RenderPipeline,
    pub solid_detail_pipeline: wgpu::RenderPipeline,
    pub solid_pipeline: wgpu::RenderPipeline,
    pub solid_translucent_pipeline: wgpu::RenderPipeline,
    pub backface_culling_enabled: bool,
    pub front_to_back_sorting_enabled: bool,
    pub last_culled_opaque_triangles: usize,
    pub last_winding_swaps: usize,
    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,
    pub uniform_buffers: [wgpu::Buffer; 4],
    pub uniform_bind_groups: [wgpu::BindGroup; 4],
    pub texture_array: wgpu::Texture,
    #[cfg(test)]
    texture_array_view: wgpu::TextureView,
    #[cfg(test)]
    texture_sampler: wgpu::Sampler,
    #[cfg(test)]
    texture_bind_group_layout: wgpu::BindGroupLayout,
    pub texture_bind_group: wgpu::BindGroup,
    pub uploaded_texture_layers: std::collections::HashMap<String, (u32, f32, f32)>,
    pub next_texture_layer: u32,
    max_diffuse_texture_layers: u32,
    max_retained_vertex_bytes: usize,
    last_rendered_map_instance_id: u64,
    texture_resource_revision: u64,
    missing_texture_lookups: super::texture_misses::MissingTextureCache,
    pub lightmap_atlas: LightmapAtlas,
    pub cached_csg_signature: u64,
    cached_csg_map_instance_id: u64,
    pub cached_carved_polys: Vec<(usize, ut99_bsp::Polygon)>,
    async_csg: Option<super::async_csg::AsyncCsgCoordinator>,
    csg_in_flight: bool,
    pub cached_zone_flood_sig: u64,
    pub cached_zone_flood: Option<(Vec<ut99_bsp::Plane>, ut99_bsp::ZoneFloodFillResult)>,
    overlay_cache: super::overlay_cache::OverlaySceneCache,
    cached_brush_bounds_key: Option<(u64, u64)>,
    cached_brush_bounds: Vec<(glam::Vec3, glam::Vec3)>,
    cached_static_solid_key: Option<StaticSolidKey>,
    cached_solid_ranges: StaticSolidRanges,
    cached_solid_chunks: [Vec<super::spatial_chunks::SpatialChunk>; super::spatial_chunks::SOLID_BUCKET_COUNT],
    cached_solid_vertex_count: u32,
    cached_solid_lightmap_fallbacks: usize,
    cached_solid_texture_fallbacks: usize,
    frame_solids_retained: bool,
    cached_static_line_key: Option<StaticLineKey>,
    cached_quadrant_grid_ranges: [(u32, u32); 4],
    cached_world_lines_start: u32,
    cached_static_line_count: u32,
    cached_line_chunks: Vec<super::spatial_chunks::SpatialChunk>,
    cached_visible_brushes: usize,
    cached_visible_actors: usize,
    frame_lines_retained: bool,
    chunk_opaque_indices_scratch: Vec<u32>,
    chunk_line_scratch: Vec<LineVertex>,
    chunk_slot_scratch: Vec<u8>,
    visible_chunk_sort_scratch: Vec<(f32, f32, u32, u32)>,
    visible_runs_scratch: Vec<(u32, u32)>,
    cached_studio_lines_key: Option<CachedStudioLinesKey>,
    cached_studio_line_count: u32,
    frame_studio_lines: Vec<LineVertex>,
    pub studio_line_buffer: Option<wgpu::Buffer>,
    pub studio_line_buffer_capacity: usize,
    frame_dynamic_lines: Vec<LineVertex>,
    pub dynamic_line_buffer: Option<wgpu::Buffer>,
    pub dynamic_line_buffer_capacity: usize,
    pub depth_texture_view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub line_buffer: Option<wgpu::Buffer>,
    pub line_buffer_capacity: usize,
    pub solid_buffer: Option<wgpu::Buffer>,
    pub solid_buffer_capacity: usize,
    pub solid_index_buffer: Option<wgpu::Buffer>,
    pub solid_index_buffer_capacity: usize,
    pub last_cull_stats: (usize, usize, usize, usize),
    surface_format: wgpu::TextureFormat,
    blit_pipeline: wgpu::RenderPipeline,
    blit_bind_group_layout: wgpu::BindGroupLayout,
    blit_sampler: wgpu::Sampler,
    scaled_target: Option<ScaledViewportTarget>,
}

fn zone_portal_color(poly_flags: u32, brush_index: usize) -> [f32; 4] {
    if poly_flags & ut99_bsp::PF_PORTAL != 0 {
        [0.0, 0.95, 0.85, 1.0]
    } else {
        let h = ((brush_index * 137 + 43) % 360) as f32;
        let s = 0.65_f32;
        let v = 0.85_f32;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;
        let (r1, g1, b1) = if h < 60.0 { (c, x, 0.0) }
            else if h < 120.0 { (x, c, 0.0) }
            else if h < 180.0 { (0.0, c, x) }
            else if h < 240.0 { (0.0, x, c) }
            else if h < 300.0 { (x, 0.0, c) }
            else { (c, 0.0, x) };
        [r1 + m, g1 + m, b1 + m, 1.0]
    }
}

fn texture_usage_color(texture_name: &str) -> [f32; 4] {
    let mut h: u32 = 2166136261;
    for b in texture_name.bytes() {
        h = (h ^ (b.to_ascii_lowercase() as u32)).wrapping_mul(16777619);
    }
    let hue = (h % 360) as f32;
    let s = 0.72_f32;
    let v = 0.88_f32;
    let c = v * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if hue < 60.0 { (c, x, 0.0) }
        else if hue < 120.0 { (x, c, 0.0) }
        else if hue < 180.0 { (0.0, c, x) }
        else if hue < 240.0 { (0.0, x, c) }
        else if hue < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    [r1 + m, g1 + m, b1 + m, 1.0]
}

fn bsp_cuts_color(carved_poly_index: usize, brush_index: usize) -> [f32; 4] {
    let hue = ((carved_poly_index.wrapping_mul(97).wrapping_add(brush_index.wrapping_mul(31)) + 19) % 360) as f32;
    let s = 0.68_f32;
    let v = 0.86_f32;
    let c = v * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if hue < 60.0 { (c, x, 0.0) }
        else if hue < 120.0 { (x, c, 0.0) }
        else if hue < 180.0 { (0.0, c, x) }
        else if hue < 240.0 { (0.0, x, c) }
        else if hue < 300.0 { (x, 0.0, c) }
        else { (c, 0.0, x) };
    [r1 + m, g1 + m, b1 + m, 1.0]
}

fn depth_complexity_color(poly_vertex_count: usize, poly_index: usize) -> [f32; 4] {
    let level = (poly_vertex_count.saturating_sub(3) + (poly_index % 5)) % 6;
    match level {
        0 => [0.1, 0.85, 0.2, 1.0],
        1 => [0.1, 0.7, 0.9, 1.0],
        2 => [0.2, 0.4, 0.95, 1.0],
        3 => [0.95, 0.8, 0.1, 1.0],
        4 => [0.95, 0.5, 0.1, 1.0],
        _ => [0.95, 0.15, 0.2, 1.0],
    }
}

impl Renderer {
    pub fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let budgets = crate::graphics::HardwareBudgets::default_for_limits(&device.limits());
        Self::new_with_budgets(device, surface_format, width, height, budgets)
    }

    pub fn new_with_budgets(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
        budgets: crate::graphics::HardwareBudgets,
    ) -> Self {
        let max_diffuse_texture_layers = budgets
            .diffuse_texture_layers
            .min(device.limits().max_texture_array_layers)
            .max(2);
        let lightmap_atlas_dim = budgets
            .lightmap_atlas_dim
            .min(device.limits().max_texture_dimension_2d)
            .max(256);
        let max_retained_vertex_bytes = budgets.max_retained_vertex_bytes;
        // Uniform Buffer
        let uniforms = Uniforms::default();
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // 4 Viewport Uniform Buffers (for 2x2 multi-viewport layout)
        let mut u_bufs = Vec::with_capacity(4);
        let mut u_bgs = Vec::with_capacity(4);
        for i in 0..4 {
            let buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("Uniform Buffer Viewport {}", i)),
                contents: bytemuck::cast_slice(&[uniforms]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });
            let bg = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("Uniform Bind Group Viewport {}", i)),
                layout: &uniform_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buf.as_entire_binding(),
                }],
            });
            u_bufs.push(buf);
            u_bgs.push(bg);
        }
        let uniform_buffers: [wgpu::Buffer; 4] = [
            u_bufs.remove(0),
            u_bufs.remove(0),
            u_bufs.remove(0),
            u_bufs.remove(0),
        ];
        let uniform_bind_groups: [wgpu::BindGroup; 4] = [
            u_bgs.remove(0),
            u_bgs.remove(0),
            u_bgs.remove(0),
            u_bgs.remove(0),
        ];

        // 2D Texture Array + Repeating Sampler for real UTX texture rendering
        let texture_array = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("UTX 2D Texture Array"),
            size: wgpu::Extent3d {
                width: TEXTURE_ARRAY_DIM,
                height: TEXTURE_ARRAY_DIM,
                depth_or_array_layers: max_diffuse_texture_layers,
            },
            mip_level_count: TEXTURE_ARRAY_MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let texture_array_view = texture_array.create_view(&wgpu::TextureViewDescriptor {
            label: Some("UTX 2D Texture Array View"),
            dimension: Some(wgpu::TextureViewDimension::D2Array),
            ..Default::default()
        });

        let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("UTX Repeating Sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let lightmap_atlas = LightmapAtlas::new(device, lightmap_atlas_dim);

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Array & Lightmap Atlas Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2Array,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Array & Lightmap Atlas Bind Group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&lightmap_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&lightmap_atlas.sampler),
                },
            ],
        });

        let line_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Line Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        let solid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Solid Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout, &texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Depth Texture
        let depth_texture_view = Self::create_depth_view(device, width, height);

        // Line Pipeline (Wireframe and Grid)
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Line Shader"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });

        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Pipeline"),
            layout: Some(&line_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: Some("vs_main"),
                buffers: &[LineVertex::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Solid Pipelines (Early-Z Opaque, Detail/SemiSolid/Mover, Masked, and Alpha-Blended Translucent)
        let solid_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Solid Shader"),
            source: wgpu::ShaderSource::Wgsl(SOLID_SHADER.into()),
        });
        let solid_buffers = [SolidVertex::desc()];
        let make_solid_pipeline = |label: &str,
                                   entry: &str,
                                   blend: Option<wgpu::BlendState>,
                                   depth_write_enabled: bool,
                                   depth_compare: wgpu::CompareFunction,
                                   bias: wgpu::DepthBiasState,
                                   front_face: wgpu::FrontFace,
                                   cull_mode: Option<wgpu::Face>| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&solid_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &solid_shader,
                    entry_point: Some("vs_main"),
                    buffers: &solid_buffers,
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &solid_shader,
                    entry_point: Some(entry),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: surface_format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face,
                    cull_mode,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled,
                    depth_compare,
                    stencil: wgpu::StencilState::default(),
                    bias,
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        let solid_opaque_pipeline = make_solid_pipeline(
            "Solid Opaque Pipeline",
            "fs_opaque",
            None,
            true,
            wgpu::CompareFunction::Less,
            wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
            wgpu::FrontFace::Ccw,
            None,
        );
        // Perspective camera uses LH view/proj, where outward-facing polygons wind Cw in NDC.
        let solid_opaque_culled_pipeline = make_solid_pipeline(
            "Solid Opaque Culled Pipeline",
            "fs_opaque",
            None,
            true,
            wgpu::CompareFunction::Less,
            wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
            wgpu::FrontFace::Cw,
            Some(wgpu::Face::Back),
        );
        let solid_detail_pipeline = make_solid_pipeline(
            "Solid Detail Pipeline",
            "fs_opaque",
            None,
            true,
            wgpu::CompareFunction::Less,
            wgpu::DepthBiasState {
                constant: 1,
                slope_scale: 1.0,
                clamp: 0.0,
            },
            wgpu::FrontFace::Ccw,
            None,
        );
        let solid_pipeline = make_solid_pipeline(
            "Solid Masked Pipeline",
            "fs_main",
            None,
            true,
            wgpu::CompareFunction::Less,
            wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.0,
                clamp: 0.0,
            },
            wgpu::FrontFace::Ccw,
            None,
        );
        let solid_translucent_pipeline = make_solid_pipeline(
            "Solid Translucent Pipeline",
            "fs_translucent",
            Some(wgpu::BlendState::ALPHA_BLENDING),
            false,
            wgpu::CompareFunction::LessEqual,
            wgpu::DepthBiasState::default(),
            wgpu::FrontFace::Ccw,
            None,
        );

        let blit_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Viewport Blit Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let blit_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Viewport Blit Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });
        let blit_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Viewport Blit Shader"),
            source: wgpu::ShaderSource::Wgsl(VIEWPORT_BLIT_SHADER.into()),
        });
        let blit_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Viewport Blit Pipeline Layout"),
            bind_group_layouts: &[&blit_bind_group_layout],
            push_constant_ranges: &[],
        });
        let blit_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Viewport Blit Pipeline"),
            layout: Some(&blit_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &blit_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &blit_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            line_pipeline,
            solid_opaque_pipeline,
            solid_opaque_culled_pipeline,
            solid_detail_pipeline,
            solid_pipeline,
            solid_translucent_pipeline,
            backface_culling_enabled: false,
            last_culled_opaque_triangles: 0,
            last_winding_swaps: 0,
            uniform_buffer,
            uniform_bind_group,
            uniform_buffers,
            uniform_bind_groups,
            texture_array,
            #[cfg(test)]
            texture_array_view,
            #[cfg(test)]
            texture_sampler,
            #[cfg(test)]
            texture_bind_group_layout,
            texture_bind_group,
            uploaded_texture_layers: std::collections::HashMap::new(),
            next_texture_layer: 1,
            max_diffuse_texture_layers,
            max_retained_vertex_bytes,
            last_rendered_map_instance_id: 0,
            texture_resource_revision: 0,
            missing_texture_lookups: super::texture_misses::MissingTextureCache::default(),
            lightmap_atlas,
            grid_cache: Default::default(),
            cached_lightmap_keys: Vec::new(),
            cached_lightmap_keys_valid: false,
            last_lightmap_fallbacks: 0,
            last_texture_fallbacks: 0,
            last_frame_stats: Default::default(),
            gpu_timer: None,
            frame_lines: Vec::new(),
            frame_solids: Vec::new(),
            frame_solid_indices: Vec::new(),
            frame_indices_twosided: Vec::new(),
            frame_indices_detail: Vec::new(),
            frame_indices_masked: Vec::new(),
            frame_indices_translucent: Vec::new(),
            world_wireframe: super::wireframe::WireframeScratch::default(),
            cached_csg_signature: 0,
            cached_csg_map_instance_id: 0,
            cached_carved_polys: Vec::new(),
            async_csg: None,
            csg_in_flight: false,
            cached_zone_flood_sig: 0,
            cached_zone_flood: None,
            overlay_cache: Default::default(),
            cached_brush_bounds_key: None,
            cached_brush_bounds: Vec::new(),
            cached_static_solid_key: None,
            cached_solid_ranges: StaticSolidRanges::default(),
            cached_solid_chunks: std::array::from_fn(|_| Vec::new()),
            cached_solid_vertex_count: 0,
            cached_solid_lightmap_fallbacks: 0,
            cached_solid_texture_fallbacks: 0,
            frame_solids_retained: false,
            cached_static_line_key: None,
            cached_quadrant_grid_ranges: [(0, 0); 4],
            cached_world_lines_start: 0,
            cached_static_line_count: 0,
            cached_line_chunks: Vec::new(),
            cached_visible_brushes: 0,
            cached_visible_actors: 0,
            frame_lines_retained: false,
            chunk_opaque_indices_scratch: Vec::new(),
            chunk_line_scratch: Vec::new(),
            chunk_slot_scratch: Vec::new(),
            visible_chunk_sort_scratch: Vec::new(),
            visible_runs_scratch: Vec::new(),
            front_to_back_sorting_enabled: false,
            cached_studio_lines_key: None,
            cached_studio_line_count: 0,
            frame_studio_lines: Vec::new(),
            studio_line_buffer: None,
            studio_line_buffer_capacity: 0,
            frame_dynamic_lines: Vec::new(),
            dynamic_line_buffer: None,
            dynamic_line_buffer_capacity: 0,
            depth_texture_view,
            width,
            height,
            line_buffer: None,
            line_buffer_capacity: 0,
            solid_buffer: None,
            solid_buffer_capacity: 0,
            solid_index_buffer: None,
            solid_index_buffer_capacity: 0,
            last_cull_stats: (0, 0, 0, 0),
            surface_format,
            blit_pipeline,
            blit_bind_group_layout,
            blit_sampler,
            scaled_target: None,
        }
    }

    pub fn compute_world_brushes_signature(brushes: &[ut99_bsp::Brush]) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        brushes.len().hash(&mut h);
        for b in brushes {
            std::mem::discriminant(&b.csg_oper).hash(&mut h);
            b.location.x.to_bits().hash(&mut h);
            b.location.y.to_bits().hash(&mut h);
            b.location.z.to_bits().hash(&mut h);
            b.rotation.x.to_bits().hash(&mut h);
            b.rotation.y.to_bits().hash(&mut h);
            b.rotation.z.to_bits().hash(&mut h);
            b.scale.x.to_bits().hash(&mut h);
            b.scale.y.to_bits().hash(&mut h);
            b.scale.z.to_bits().hash(&mut h);
            b.polys.len().hash(&mut h);
            for p in &b.polys {
                p.flags.hash(&mut h);
                p.texture.hash(&mut h);
                p.base.x.to_bits().hash(&mut h);
                p.base.y.to_bits().hash(&mut h);
                p.base.z.to_bits().hash(&mut h);
                p.texture_u.x.to_bits().hash(&mut h);
                p.texture_u.y.to_bits().hash(&mut h);
                p.texture_u.z.to_bits().hash(&mut h);
                p.texture_v.x.to_bits().hash(&mut h);
                p.texture_v.y.to_bits().hash(&mut h);
                p.texture_v.z.to_bits().hash(&mut h);
                p.pan_u.hash(&mut h);
                p.pan_v.hash(&mut h);
                p.vertices.len().hash(&mut h);
                for v in &p.vertices {
                    v.x.to_bits().hash(&mut h);
                    v.y.to_bits().hash(&mut h);
                    v.z.to_bits().hash(&mut h);
                }
                p.vertex_colors.len().hash(&mut h);
                if let Some(vc) = p.vertex_colors.first() {
                    vc[0].to_bits().hash(&mut h);
                    vc[1].to_bits().hash(&mut h);
                    vc[2].to_bits().hash(&mut h);
                }
                if let Some(ref lm) = p.lightmap {
                    lm.width.hash(&mut h);
                    lm.height.hash(&mut h);
                    if let Some(lx) = lm.luxels.first() {
                        lx[0].to_bits().hash(&mut h);
                        lx[1].to_bits().hash(&mut h);
                        lx[2].to_bits().hash(&mut h);
                    }
                    if let Some(lx) = lm.luxels.get(lm.luxels.len() / 2) {
                        lx[0].to_bits().hash(&mut h);
                        lx[1].to_bits().hash(&mut h);
                        lx[2].to_bits().hash(&mut h);
                    }
                }
            }
        }
        let sig = h.finish();
        if sig == 0 { 1 } else { sig }
    }

    #[cfg(test)]
    pub(crate) fn rebuild_texture_bind_group(&mut self, device: &wgpu::Device) {
        self.texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Texture Array & Lightmap Atlas Bind Group"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&self.texture_array_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.texture_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&self.lightmap_atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&self.lightmap_atlas.sampler),
                },
            ],
        });
    }

    pub fn resolve_or_upload_lightmap_rect(
        &mut self,
        queue: &wgpu::Queue,
        lm: &ut99_bsp::LightmapData,
    ) -> Option<LightmapAtlasRect> {
        let key = lightmap_content_key(lm)?;
        self.resolve_lightmap_with_key(queue, lm, key)
    }

    // Called before building draw vertices, so reused tiles cannot invalidate this frame.
    fn refresh_cached_lightmap_keys(&mut self) {
        self.cached_lightmap_keys = self
            .cached_carved_polys
            .iter()
            .map(|(_, poly)| poly.lightmap.as_ref().and_then(lightmap_content_key))
            .collect();
        let mut live = std::collections::HashSet::new();
        let mut new_required_texels = 0u64;
        for ((_, poly), key_opt) in self
            .cached_carved_polys
            .iter()
            .zip(self.cached_lightmap_keys.iter())
        {
            let Some(key) = *key_opt else { continue; };
            if live.insert(key) && !self.lightmap_atlas.contains_key(key) {
                if let Some(ref lm) = poly.lightmap {
                    let w = lm.width.min(super::lightmap_atlas::MAX_LIGHTMAP_TILE_DIM) as u64 + 2;
                    let h = lm.height.min(super::lightmap_atlas::MAX_LIGHTMAP_TILE_DIM) as u64 + 2;
                    new_required_texels = new_required_texels.saturating_add(w * h);
                }
            }
        }
        self.lightmap_atlas.retain_live(&live, new_required_texels);
        self.cached_lightmap_keys_valid = true;
    }

    fn resolve_lightmap_with_key(
        &mut self,
        queue: &wgpu::Queue,
        lm: &ut99_bsp::LightmapData,
        key: u64,
    ) -> Option<LightmapAtlasRect> {
        let rect = self.lightmap_atlas.resolve_or_upload(queue, lm, key);
        if rect.is_none() {
            self.last_lightmap_fallbacks = self.last_lightmap_fallbacks.saturating_add(1);
        }
        rect
    }

    /// Resolves or uploads a loaded UTX texture into a layer of `self.texture_array`.
    /// Returns `(layer_index, tex_width, tex_height)`, or `(0, 256.0, 256.0)` for procedural fallback.
    pub fn resolve_or_upload_texture_layer(
        &mut self,
        queue: &wgpu::Queue,
        state: &crate::editor_state::EditorState,
        tex_name: &str,
    ) -> (u32, f32, f32) {
        let trimmed = tex_name.trim();
        if trimmed.is_empty()
            || trimmed.eq_ignore_ascii_case("None")
            || trimmed.eq_ignore_ascii_case("Default")
            || trimmed.eq_ignore_ascii_case("DefaultTexture")
        {
            return (0, 256.0, 256.0);
        }
        // Cached short-name lookups are valid only while the loaded namespace is
        // unchanged. This also refreshes pixels after replacement or conversion.
        if self.texture_resource_revision != state.texture_manager.resource_revision {
            self.uploaded_texture_layers.clear();
            self.missing_texture_lookups.clear();
            self.next_texture_layer = 1;
            self.texture_resource_revision = state.texture_manager.resource_revision;
        }
        let key = trimmed.to_ascii_lowercase();
        if let Some(&cached) = self.uploaded_texture_layers.get(&key) {
            return cached;
        }
        if self.next_texture_layer >= self.max_diffuse_texture_layers {
            self.last_texture_fallbacks = self.last_texture_fallbacks.saturating_add(1);
            return (0, 256.0, 256.0);
        }
        // Capacity fallback above is not a failed lookup: it must remain retryable.
        let Some(utx_tex) = self.missing_texture_lookups.resolve(&key, || {
            state.texture_manager.find_loaded_texture(trimmed).filter(|texture| {
                texture.width > 0 && texture.height > 0
                    && texture.rgba_pixels.len() >= (texture.width * texture.height * 4) as usize
            })
        }) else { return (0, 256.0, 256.0); };
        let layer = self.next_texture_layer;
        self.next_texture_layer += 1;
        let mut resampled = vec![255u8; (TEXTURE_ARRAY_DIM * TEXTURE_ARRAY_DIM * 4) as usize];
        for y in 0..TEXTURE_ARRAY_DIM {
            let sy = ((y * utx_tex.height) / TEXTURE_ARRAY_DIM).min(utx_tex.height - 1);
            for x in 0..TEXTURE_ARRAY_DIM {
                let sx = ((x * utx_tex.width) / TEXTURE_ARRAY_DIM).min(utx_tex.width - 1);
                let src_i = ((sy * utx_tex.width + sx) * 4) as usize;
                let dst_i = ((y * TEXTURE_ARRAY_DIM + x) * 4) as usize;
                resampled[dst_i..dst_i + 4].copy_from_slice(&utx_tex.rgba_pixels[src_i..src_i + 4]);
            }
        }
        upload_layer_rgba_with_mipmaps(queue, &self.texture_array, layer, &resampled);
        let info = (layer, utx_tex.width as f32, utx_tex.height as f32);
        self.uploaded_texture_layers.insert(key, info);
        info
    }

    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width > 0 && height > 0 && (self.width != width || self.height != height) {
            self.width = width;
            self.height = height;
            self.depth_texture_view = Self::create_depth_view(device, width, height);
        }
    }

    fn create_depth_view(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
        let size = wgpu::Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    pub fn update_camera(&self, queue: &wgpu::Queue, camera: &Camera) {
        let uniforms = Uniforms {
            view_proj: camera.view_projection_matrix().to_cols_array_2d(),
            light_dir: [-0.5, -0.6, 0.65, 0.0],
            fog_params: [200.0, 3000.0, 1.0, 0.0],
            fog_color: [0.1, 0.12, 0.15, 1.0],
            cam_pos: [camera.position.x, camera.position.y, camera.position.z, 1.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    pub fn enable_gpu_timing(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        self.gpu_timer = super::gpu_timing::GpuTimer::new(device, queue);
    }

    pub fn gpu_timing_summary(&self) -> Option<(usize, f64, f64)> {
        self.gpu_timer.as_ref().and_then(|timer| timer.summary())
    }

    pub fn grid_cache_bytes(&self) -> usize { self.grid_cache.bytes() }

    pub fn cpu_vertex_storage_bytes(&self) -> usize {
        (self.frame_lines.capacity()
            + self.frame_studio_lines.capacity()
            + self.frame_dynamic_lines.capacity())
            * std::mem::size_of::<LineVertex>()
            + self.frame_solids.capacity() * std::mem::size_of::<SolidVertex>()
            + (self.frame_solid_indices.capacity()
                + self.frame_indices_twosided.capacity()
                + self.frame_indices_detail.capacity()
                + self.frame_indices_masked.capacity()
                + self.frame_indices_translucent.capacity())
                * std::mem::size_of::<u32>()
    }

    pub fn gpu_vertex_buffer_bytes(&self) -> usize {
        self.line_buffer_capacity
            + self.studio_line_buffer_capacity
            + self.dynamic_line_buffer_capacity
            + self.solid_buffer_capacity
            + self.solid_index_buffer_capacity
    }

    pub fn max_diffuse_texture_layers(&self) -> u32 {
        self.max_diffuse_texture_layers
    }

    pub fn max_retained_vertex_bytes(&self) -> usize {
        self.max_retained_vertex_bytes
    }

    pub fn diffuse_texture_array_bytes(&self) -> usize {
        let mut w = TEXTURE_ARRAY_DIM;
        let mut h = TEXTURE_ARRAY_DIM;
        let mut per_layer = 0usize;
        for _ in 0..TEXTURE_ARRAY_MIP_LEVELS {
            per_layer += (w as usize) * (h as usize) * 4;
            w = (w / 2).max(1);
            h = (h / 2).max(1);
        }
        per_layer * (self.max_diffuse_texture_layers as usize)
    }

    pub fn apply_runtime_budgets(&mut self, budgets: crate::graphics::HardwareBudgets) {
        self.max_retained_vertex_bytes = budgets.max_retained_vertex_bytes;
    }

    pub fn scaled_target_dimensions(&self) -> Option<(u32, u32)> {
        self.scaled_target.as_ref().map(|st| (st.width, st.height))
    }

    pub fn scaled_target_bytes(&self) -> usize {
        self.scaled_target
            .as_ref()
            .map_or(0, |st| (st.width as usize) * (st.height as usize) * 8)
    }

    fn ensure_scaled_target(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self
            .scaled_target
            .as_ref()
            .is_some_and(|st| st.width == width && st.height == height)
        {
            return;
        }
        let color_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("3D Scaled Viewport Color Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.surface_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let depth_view = Self::create_depth_view(device, width, height);
        let blit_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Viewport Blit Bind Group"),
            layout: &self.blit_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.blit_sampler),
                },
            ],
        });
        self.scaled_target = Some(ScaledViewportTarget {
            width,
            height,
            color_view,
            depth_view,
            blit_bind_group,
        });
    }

    fn reclaim_on_map_switch(&mut self) {
        self.scaled_target = None;
        self.csg_in_flight = false;
        self.cached_carved_polys = Vec::new();
        self.cached_csg_signature = 0;
        self.cached_csg_map_instance_id = 0;
        self.cached_solid_ranges = StaticSolidRanges::default();
        self.cached_solid_chunks = std::array::from_fn(|_| Vec::new());
        self.cached_solid_vertex_count = 0;
        self.cached_solid_lightmap_fallbacks = 0;
        self.cached_solid_texture_fallbacks = 0;
        self.cached_static_solid_key = None;
        self.cached_lightmap_keys = Vec::new();
        self.cached_lightmap_keys_valid = false;
        self.frame_solids_retained = false;
        self.cached_static_line_key = None;
        self.cached_quadrant_grid_ranges = [(0, 0); 4];
        self.cached_world_lines_start = 0;
        self.cached_static_line_count = 0;
        self.cached_line_chunks = Vec::new();
        self.cached_visible_brushes = 0;
        self.cached_visible_actors = 0;
        self.frame_lines_retained = false;
        self.cached_studio_lines_key = None;
        self.cached_studio_line_count = 0;
        self.cached_zone_flood_sig = 0;
        self.cached_zone_flood = None;
        self.cached_brush_bounds_key = None;
        self.cached_brush_bounds = Vec::new();
        self.overlay_cache = Default::default();
        self.world_wireframe = Default::default();

        self.lightmap_atlas.clear();
        self.uploaded_texture_layers.clear();
        self.next_texture_layer = 1;
        self.texture_resource_revision = 0;
        self.missing_texture_lookups.clear();

        self.line_buffer = None;
        self.line_buffer_capacity = 0;
        self.studio_line_buffer = None;
        self.studio_line_buffer_capacity = 0;
        self.dynamic_line_buffer = None;
        self.dynamic_line_buffer_capacity = 0;
        self.solid_buffer = None;
        self.solid_buffer_capacity = 0;
        self.solid_index_buffer = None;
        self.solid_index_buffer_capacity = 0;

        self.frame_lines = Vec::new();
        self.frame_solids = Vec::new();
        self.frame_solid_indices = Vec::new();
        self.frame_indices_twosided = Vec::new();
        self.frame_indices_detail = Vec::new();
        self.frame_indices_masked = Vec::new();
        self.frame_indices_translucent = Vec::new();
        self.frame_studio_lines = Vec::new();
        self.frame_dynamic_lines = Vec::new();
        self.chunk_opaque_indices_scratch = Vec::new();
        self.chunk_line_scratch = Vec::new();
        self.chunk_slot_scratch = Vec::new();
        self.visible_chunk_sort_scratch = Vec::new();
        self.visible_runs_scratch = Vec::new();
        self.last_cull_stats = (0, 0, 0, 0);
    }

    fn upload_persistent_buffer<T: bytemuck::Pod>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        buffer: &mut Option<wgpu::Buffer>,
        capacity: &mut usize,
        vertices: &[T],
    ) -> usize {
        Self::upload_persistent_buffer_with_usage(
            device,
            queue,
            label,
            wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            buffer,
            capacity,
            vertices,
        )
    }

    fn upload_persistent_buffer_with_usage<T: bytemuck::Pod>(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        label: &str,
        usage: wgpu::BufferUsages,
        buffer: &mut Option<wgpu::Buffer>,
        capacity: &mut usize,
        items: &[T],
    ) -> usize {
        if items.is_empty() {
            return 0;
        }
        let bytes = bytemuck::cast_slice(items);
        if let Some(ref buf) = *buffer {
            if *capacity >= bytes.len() {
                queue.write_buffer(buf, 0, bytes);
                return bytes.len();
            }
        }
        let new_cap = (bytes.len() * 3 / 2).max(65536);
        let new_buf = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: new_cap as u64,
            usage,
            mapped_at_creation: false,
        });
        queue.write_buffer(&new_buf, 0, bytes);
        *buffer = Some(new_buf);
        *capacity = new_cap;
        bytes.len()
    }

    pub fn last_frame_stats(&self) -> crate::graphics::RenderFrameStats {
        self.last_frame_stats
    }

    pub fn enable_async_csg(&mut self, wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>) {
        if let Some(ref coord) = self.async_csg {
            coord.set_wake(wake);
        } else {
            self.async_csg = Some(super::async_csg::AsyncCsgCoordinator::new(wake));
        }
    }

    pub fn disable_async_csg(&mut self) {
        self.async_csg = None;
        self.csg_in_flight = false;
    }

    pub fn is_csg_in_flight(&self) -> bool {
        self.csg_in_flight
    }

    pub fn csg_cancelled_total(&self) -> u64 {
        self.async_csg
            .as_ref()
            .map_or(0, super::async_csg::AsyncCsgCoordinator::cancelled_total)
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        target_view: &wgpu::TextureView,
        state: &crate::editor_state::EditorState,
    ) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        self.encode_render(device, queue, &mut encoder, target_view, state);
        let submit_started = std::time::Instant::now();
        queue.submit(std::iter::once(encoder.finish()));
        self.mark_submitted();
        self.last_frame_stats.encode_submit_ms += submit_started.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn mark_submitted(&mut self) {
        if let Some(timer) = &mut self.gpu_timer {
            timer.submitted();
        }
    }

    pub fn encode_render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        target_view: &wgpu::TextureView,
        state: &crate::editor_state::EditorState,
    ) {
        self.last_lightmap_fallbacks = 0;
        self.last_texture_fallbacks = 0;
        if let Some(ref coord) = self.async_csg {
            coord.cancel_for_other_map(state.map_instance_id);
        }
        if (self.last_rendered_map_instance_id != 0
            && self.last_rendered_map_instance_id != state.map_instance_id)
            || (self.cached_csg_map_instance_id != 0
                && self.cached_csg_map_instance_id != state.map_instance_id)
        {
            self.reclaim_on_map_switch();
        }
        self.last_rendered_map_instance_id = state.map_instance_id;
        if let Some(timer) = &mut self.gpu_timer { timer.begin_frame(device); }
        let fog_enabled_f32 = if state.distance_fog_enabled { 1.0 } else { 0.0 };
        let is_flat_diagnostic = |sh: crate::camera::ViewportShading| {
            matches!(
                sh,
                crate::camera::ViewportShading::Unlit
                    | crate::camera::ViewportShading::ZonePortal
                    | crate::camera::ViewportShading::TextureUsage
                    | crate::camera::ViewportShading::BspCuts
                    | crate::camera::ViewportShading::DepthComplexity
            )
        };
        let unlit_f32 = if is_flat_diagnostic(state.viewport_shading) { 1.0 } else { 0.0 };
        let effective_shading = if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
            state.shading_for_quadrant(3)
        } else {
            state.viewport_shading
        };
        let any_solid_viewport = if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
            (0..4).any(|i| state.shading_for_quadrant(i) != crate::camera::ViewportShading::Wireframe)
        } else {
            state.viewport_shading != crate::camera::ViewportShading::Wireframe
        };
        let vx = (state.viewport_rect[0].max(0.0) as u32).min(self.width.saturating_sub(1));
        let vy = (state.viewport_rect[1].max(0.0) as u32).min(self.height.saturating_sub(1));
        let vw = (state.viewport_rect[2].max(1.0) as u32).min(self.width.saturating_sub(vx)).max(1);
        let vh = (state.viewport_rect[3].max(1.0) as u32).min(self.height.saturating_sub(vy)).max(1);

        let mut quad_frustums = [
            crate::culling::Frustum::from_view_projection(&glam::Mat4::IDENTITY),
            crate::culling::Frustum::from_view_projection(&glam::Mat4::IDENTITY),
            crate::culling::Frustum::from_view_projection(&glam::Mat4::IDENTITY),
            crate::culling::Frustum::from_view_projection(&glam::Mat4::IDENTITY),
        ];

        if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
            let hw = vw / 2;
            let hh = vh / 2;
            let rw = vw.saturating_sub(hw);
            let rh = vh.saturating_sub(hh);

            let quad_specs = [
                (crate::camera::ViewportType::Top, hw as f32 / hh.max(1) as f32),
                (crate::camera::ViewportType::Side, rw as f32 / hh.max(1) as f32),
                (crate::camera::ViewportType::Front, hw as f32 / rh.max(1) as f32),
                (crate::camera::ViewportType::Perspective, rw as f32 / rh.max(1) as f32),
            ];

            for (idx, (v_type, aspect)) in quad_specs.iter().enumerate() {
                let q_shading = state.shading_for_quadrant(idx);
                let q_unlit_f32 = if is_flat_diagnostic(q_shading) { 1.0 } else { 0.0 };
                let vp_mat = state.camera.view_projection_matrix_for_type(*v_type, *aspect);
                quad_frustums[idx] = crate::culling::Frustum::from_view_projection(&vp_mat);
                let uniforms = Uniforms {
                    view_proj: vp_mat.to_cols_array_2d(),
                    light_dir: [-0.5, -0.6, 0.65, q_unlit_f32],
                    fog_params: [state.fog_start, state.fog_end, state.fog_density, fog_enabled_f32],
                    fog_color: state.fog_color,
                    cam_pos: [state.camera.position.x, state.camera.position.y, state.camera.position.z, 1.0],
                };
                queue.write_buffer(&self.uniform_buffers[idx], 0, bytemuck::cast_slice(&[uniforms]));
            }
        } else {
            let aspect = vw as f32 / vh.max(1) as f32;
            let vp_mat = state.camera.view_projection_matrix_for_type(state.active_viewport_type, aspect);
            quad_frustums[0] = crate::culling::Frustum::from_view_projection(&vp_mat);
            let uniforms = Uniforms {
                view_proj: vp_mat.to_cols_array_2d(),
                light_dir: [-0.5, -0.6, 0.65, unlit_f32],
                fog_params: [state.fog_start, state.fog_end, state.fog_density, fog_enabled_f32],
                fog_color: state.fog_color,
                cam_pos: [state.camera.position.x, state.camera.position.y, state.camera.position.z, 1.0],
            };
            queue.write_buffer(&self.uniform_buffers[0], 0, bytemuck::cast_slice(&[uniforms]));
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        }

        // O(1) revision-based signature covering all world geometry and revisions.
        let sig_started = std::time::Instant::now();
        let world_signature = if any_solid_viewport
            || effective_shading == crate::camera::ViewportShading::ZonePortal
        {
            let mut h = SceneHash::new();
            h.mix_u64(state.map_instance_id);
            h.mix_u64(state.world_brushes.rev());
            h.mix_u64(state.geometry_revision);
            let sig = h.finish();
            if sig == 0 { 1 } else { sig }
        } else {
            0
        };
        let actors_rev = state.loaded_unr.as_ref().map_or(0, |unr| unr.actors.rev());
        let signature_ms = sig_started.elapsed().as_secs_f64() * 1000.0;

        // Precompute portal planes, 3D BSP zone flood-fill, and 3D leak path tracer when ZonePortal view is active (cached via O(1) signature).
        let mut zone_flood_rebuilt = false;
        let (frame_zone_cache, zone_sig_for_keys) =
            if effective_shading == crate::camera::ViewportShading::ZonePortal {
                let mut zh = SceneHash::new();
                zh.mix_u64(world_signature);
                zh.mix_u64(actors_rev);
                let zone_sig = zh.finish();
                if zone_sig != self.cached_zone_flood_sig || self.cached_zone_flood.is_none() {
                    zone_flood_rebuilt = true;
                    let zone_seeds: Vec<(String, Vec3)> = state.loaded_unr.as_ref().into_iter()
                        .flat_map(|unr| unr.actors.iter())
                        .filter(|actor| actor.class_name.contains("ZoneInfo"))
                        .map(|actor| (actor.name.clone(), actor.location)).collect();
                    let mut planes = Vec::new();
                    for b in &state.world_brushes {
                        for p in b.to_world_polys() {
                            if (p.flags & ut99_bsp::PF_PORTAL) != 0 && planes.len() < 16 {
                                planes.push(p.plane);
                            }
                        }
                    }
                    let flood = ut99_bsp::compute_bsp_zone_flood_fill(&state.world_brushes, &zone_seeds);
                    self.cached_zone_flood_sig = zone_sig;
                    self.cached_zone_flood = Some((planes, flood));
                }
                (Some(self.cached_zone_flood.take().unwrap()), zone_sig)
            } else {
                (None, 0)
            };
        let (portal_planes, zone_flood_result): (&[ut99_bsp::Plane], Option<&ut99_bsp::ZoneFloodFillResult>) =
            frame_zone_cache.as_ref().map_or((&[], None), |(planes, flood)| (planes.as_slice(), Some(flood)));

        let static_solid_key = StaticSolidKey {
            map_instance_id: state.map_instance_id,
            world_brushes_rev: state.world_brushes.rev(),
            geometry_revision: state.geometry_revision,
            movers_rev: state.movers.rev(),
            static_meshes_rev: state.static_meshes.rev(),
            decals_rev: state.active_projector_decals.rev(),
            texture_resource_revision: state.texture_manager.resource_revision,
            effective_shading: shading_tag(effective_shading),
            any_solid_viewport,
            render_lux_heatmap: state.render_lux_heatmap,
            show_mover_polys: state.show_mover_polys,
            selected_mover_index: state.selected_mover_index,
            visibility_hash: compute_solid_visibility_hash(state),
            zone_flood_sig: zone_sig_for_keys,
            bg_hash: compute_bg_hash(state),
        };
        let static_solids_hit = self.cached_static_solid_key == Some(static_solid_key)
            && !zone_flood_rebuilt
            && (self.cached_solid_ranges.total == 0
                || (self.solid_buffer.is_some() && self.solid_index_buffer.is_some()))
            && (!self.frame_solids_retained
                || (self.frame_solids.len() == self.cached_solid_vertex_count as usize
                    && self.frame_solid_indices.len() == self.cached_solid_ranges.total as usize));

        // 1B. Persistent GPU Static Solid Cache (world CSG, movers, static meshes, decals, background quad)
        let mut csg_ms = 0.0;
        let mut csg_rebuilt = false;
        let mut csg_rebuild_reason = "";
        let mut solids_ms = 0.0;
        let mut uploaded_solid_bytes = 0usize;
        let mut solid_upload_ms = 0.0;
        let mut skip_solid_rebuild_pending_csg = false;

        if !static_solids_hit && any_solid_viewport {
            let sig = world_signature
                ^ state.geometry_revision.wrapping_mul(0x9E3779B97F4A7C15);
            if sig != self.cached_csg_signature {
                let job_key = super::async_csg::CsgJobKey {
                    map_instance_id: state.map_instance_id,
                    csg_signature: sig,
                };
                let use_async = self.async_csg.is_some()
                    && super::async_csg::should_run_async_csg(&state.world_brushes);
                if use_async {
                    let completed = self
                        .async_csg
                        .as_ref()
                        .and_then(|coord| coord.take_matching_result(job_key));
                    if let Some(done) = completed {
                        let was_empty = self.cached_csg_signature == 0;
                        self.cached_carved_polys = done.carved_polys;
                        self.cached_csg_signature = sig;
                        self.cached_csg_map_instance_id = state.map_instance_id;
                        self.cached_lightmap_keys_valid = false;
                        csg_ms = done.compute_ms;
                        csg_rebuilt = true;
                        csg_rebuild_reason = if was_empty {
                            "async_initial"
                        } else {
                            "async_completed"
                        };
                        self.csg_in_flight = false;
                    } else {
                        if let Some(ref coord) = self.async_csg {
                            coord.request(job_key, state.world_brushes.shared_snapshot());
                        }
                        self.csg_in_flight = true;
                        skip_solid_rebuild_pending_csg = true;
                    }
                } else {
                    if let Some(ref coord) = self.async_csg {
                        coord.cancel_if_busy();
                    }
                    let csg_started = std::time::Instant::now();
                    self.cached_carved_polys = ut99_bsp::evaluate_csg_brushes_indexed(&state.world_brushes);
                    csg_ms = csg_started.elapsed().as_secs_f64() * 1000.0;
                    csg_rebuilt = true;
                    csg_rebuild_reason = if self.cached_csg_signature == 0 {
                        "initial_or_reset"
                    } else {
                        "signature_or_revision_changed"
                    };
                    self.cached_csg_signature = sig;
                    self.cached_csg_map_instance_id = state.map_instance_id;
                    self.cached_lightmap_keys_valid = false;
                    self.csg_in_flight = false;
                }
            } else {
                if self.csg_in_flight {
                    if let Some(ref coord) = self.async_csg {
                        coord.cancel_if_busy();
                    }
                }
                self.csg_in_flight = false;
            }
        } else if any_solid_viewport {
            if self.csg_in_flight {
                if let Some(ref coord) = self.async_csg {
                    coord.cancel_if_busy();
                }
            }
            self.csg_in_flight = false;
        } else {
            self.csg_in_flight = false;
        }

        if static_solids_hit || skip_solid_rebuild_pending_csg {
            self.last_lightmap_fallbacks = self.cached_solid_lightmap_fallbacks;
            self.last_texture_fallbacks = self.cached_solid_texture_fallbacks;
        } else {
            let cached_brush_bounds = if effective_shading == crate::camera::ViewportShading::DepthComplexity {
                let bounds_key = (state.map_instance_id, state.world_brushes.rev());
                if self.cached_brush_bounds_key != Some(bounds_key) {
                    self.cached_brush_bounds.clear();
                    self.cached_brush_bounds.extend(state.world_brushes.iter().map(|b| b.world_bounds()));
                    self.cached_brush_bounds_key = Some(bounds_key);
                }
                Some(std::mem::take(&mut self.cached_brush_bounds))
            } else {
                None
            };
            let mut overlap_counts = cached_brush_bounds
                .as_deref()
                .map(super::overlap_counts::FrameOverlapCounts::new);

            let mut solid_verts = std::mem::take(&mut self.frame_solids);
            solid_verts.clear();
            let mut solid_indices = std::mem::take(&mut self.frame_solid_indices);
            solid_indices.clear();
            let mut twosided_indices = std::mem::take(&mut self.frame_indices_twosided);
            twosided_indices.clear();
            let mut detail_indices = std::mem::take(&mut self.frame_indices_detail);
            detail_indices.clear();
            let mut masked_indices = std::mem::take(&mut self.frame_indices_masked);
            masked_indices.clear();
            let mut translucent_indices = std::mem::take(&mut self.frame_indices_translucent);
            translucent_indices.clear();
            let mut winding_swaps = 0usize;

            let solids_started = std::time::Instant::now();
            if any_solid_viewport {
                if effective_shading == crate::camera::ViewportShading::Lit
                    && !state.render_lux_heatmap && !self.cached_lightmap_keys_valid
                {
                    self.refresh_cached_lightmap_keys();
                }
                let carved_world_polys = std::mem::take(&mut self.cached_carved_polys);
                for (carved_idx, (b_idx, poly)) in carved_world_polys.iter().enumerate() {
                    let b_idx = *b_idx;
                    if !state.is_brush_visible(b_idx) {
                        continue;
                    }
                    let Some(brush) = state.world_brushes.get(b_idx) else {
                        continue;
                    };
                    let n = poly.vertices.len();
                    if n < 3 {
                        continue;
                    }

                    let brush_cut_complexity = overlap_counts.as_mut().map_or(0, |counts| counts.count(b_idx));

                    let solidity = state.get_brush_solidity(b_idx);
                    let color = match brush.csg_oper {
                        CsgOperation::Add => match solidity {
                            crate::editor_state::BrushSolidity::SemiSolid => [0.15, 0.85, 0.35, 1.0],
                            crate::editor_state::BrushSolidity::NonSolid => [0.85, 0.90, 0.20, 1.0],
                            crate::editor_state::BrushSolidity::Solid => [0.15, 0.45, 0.85, 1.0],
                        },
                        CsgOperation::Subtract => [0.85, 0.75, 0.15, 1.0],
                        _ => [0.6, 0.6, 0.6, 1.0],
                    };

                    let (tex_layer, tex_w, tex_h) =
                        self.resolve_or_upload_texture_layer(queue, state, &poly.texture);
                    let uv_scale_x = TEXTURE_ARRAY_DIM as f32 / tex_w.max(1.0);
                    let uv_scale_y = TEXTURE_ARRAY_DIM as f32 / tex_h.max(1.0);

                    let norm = poly.plane.normal.to_array();
                    let has_lighting = poly.vertex_colors.len() == n;
                    let surf_color = if tex_layer > 0 {
                        [1.0, 1.0, 1.0, 1.0]
                    } else if !poly.texture.is_empty()
                        && !poly.texture.eq_ignore_ascii_case("None")
                        && !poly.texture.eq_ignore_ascii_case("DefaultTexture")
                    {
                        let mut h: u32 = 2166136261;
                        for b in poly.texture.bytes() {
                            h = (h ^ (b.to_ascii_lowercase() as u32)).wrapping_mul(16777619);
                        }
                        let tr = 0.35 + ((h & 0xFF) as f32 / 255.0) * 0.50;
                        let tg = 0.35 + (((h >> 8) & 0xFF) as f32 / 255.0) * 0.50;
                        let tb = 0.35 + (((h >> 16) & 0xFF) as f32 / 255.0) * 0.50;
                        [
                            color[0] * 0.72 + tr * 0.28,
                            color[1] * 0.72 + tg * 0.28,
                            color[2] * 0.72 + tb * 0.28,
                            1.0,
                        ]
                    } else {
                        color
                    };

                    let poly_zone_id = if effective_shading == crate::camera::ViewportShading::ZonePortal {
                        let flood_zid = zone_flood_result
                            .as_ref()
                            .and_then(|flood| flood.poly_zones.get(carved_idx))
                            .map(|&(front_zid, back_zid)| if front_zid > 0 { front_zid } else { back_zid });
                        if let Some(zid) = flood_zid.filter(|&z| z > 0) {
                            zid
                        } else if !portal_planes.is_empty() {
                            let centroid = poly.vertices.iter().copied().fold(Vec3::ZERO, |a, b| a + b) / (n as f32);
                            let mut z_mask = 1usize;
                            for (pi, pl) in portal_planes.iter().enumerate() {
                                if pl.normal.dot(centroid) - pl.dist >= 0.0 {
                                    z_mask ^= 1usize.wrapping_shl((pi + 1) as u32);
                                }
                            }
                            z_mask
                        } else {
                            b_idx
                        }
                    } else {
                        b_idx
                    };

                    let use_tex_layer = match effective_shading {
                        crate::camera::ViewportShading::ZonePortal
                        | crate::camera::ViewportShading::TextureUsage
                        | crate::camera::ViewportShading::BspCuts
                        | crate::camera::ViewportShading::DepthComplexity => 0,
                        crate::camera::ViewportShading::Lit if state.render_lux_heatmap => 0,
                        _ => tex_layer,
                    };

                    let lm_rect = if effective_shading == crate::camera::ViewportShading::Lit
                        && !state.render_lux_heatmap
                    {
                        match (poly.lightmap.as_ref(), self.cached_lightmap_keys.get(carved_idx).copied().flatten()) {
                            (Some(lm), Some(key)) => self.resolve_lightmap_with_key(queue, lm, key),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let use_lm_layer: u32 = u32::from(lm_rect.is_some());

                    let u_dir = if poly.texture_u.length_squared() > 1e-6 {
                        poly.texture_u.normalize()
                    } else {
                        Vec3::X
                    };
                    let v_dir = if poly.texture_v.length_squared() > 1e-6 {
                        poly.texture_v.normalize()
                    } else {
                        Vec3::Y
                    };
                    let (min_u, max_u, min_v, max_v) = if lm_rect.is_some() {
                        let mut mnu = f32::MAX;
                        let mut mxu = f32::MIN;
                        let mut mnv = f32::MAX;
                        let mut mxv = f32::MIN;
                        for &vv in &poly.vertices {
                            let d = vv - poly.base;
                            let pu = d.dot(u_dir);
                            let pv = d.dot(v_dir);
                            mnu = mnu.min(pu);
                            mxu = mxu.max(pu);
                            mnv = mnv.min(pv);
                            mxv = mxv.max(pv);
                        }
                        (mnu, mxu, mnv, mxv)
                    } else {
                        (0.0, 1.0, 0.0, 1.0)
                    };
                    let span_u = (max_u - min_u).max(1.0);
                    let span_v = (max_v - min_v).max(1.0);

                    let is_culled_structural = if (poly.flags
                        & (ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_MODULATED))
                        != 0
                        || (use_tex_layer > 0 && (poly.flags & ut99_bsp::PF_MASKED) != 0)
                        || solidity == crate::editor_state::BrushSolidity::SemiSolid
                        || (poly.flags & ut99_bsp::PF_SEMISOLID) != 0
                    {
                        false
                    } else {
                        solidity == crate::editor_state::BrushSolidity::Solid
                            && (poly.flags & ut99_bsp::PF_TWO_SIDED) == 0
                    };
                    let dst_indices = if (poly.flags & (ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_MODULATED)) != 0 {
                        &mut translucent_indices
                    } else if use_tex_layer > 0 && (poly.flags & ut99_bsp::PF_MASKED) != 0 {
                        &mut masked_indices
                    } else if solidity == crate::editor_state::BrushSolidity::SemiSolid
                        || (poly.flags & ut99_bsp::PF_SEMISOLID) != 0
                    {
                        &mut detail_indices
                    } else if is_culled_structural {
                        &mut solid_indices
                    } else {
                        &mut twosided_indices
                    };

                    let base_v = solid_verts.len() as u32;
                    for v_idx in 0..n {
                        let v = poly.vertices[v_idx];
                        let raw_uv = poly.compute_uv(v);
                        let uv = [raw_uv[0] * uv_scale_x, raw_uv[1] * uv_scale_y];
                        let lm_uv = if let Some(rect) = lm_rect {
                            let d = v - poly.base;
                            rect.map_uv((d.dot(u_dir) - min_u) / span_u, (d.dot(v_dir) - min_v) / span_v)
                        } else {
                            [0.0, 0.0]
                        };
                        let v_col = match effective_shading {
                            crate::camera::ViewportShading::ZonePortal => {
                                zone_portal_color(poly.flags, poly_zone_id)
                            }
                            crate::camera::ViewportShading::TextureUsage => {
                                texture_usage_color(&poly.texture)
                            }
                            crate::camera::ViewportShading::BspCuts => {
                                bsp_cuts_color(carved_idx, b_idx)
                            }
                            crate::camera::ViewportShading::DepthComplexity => {
                                depth_complexity_color(n, brush_cut_complexity)
                            }
                            crate::camera::ViewportShading::Unlit => surf_color,
                            crate::camera::ViewportShading::Lit => {
                                if state.render_lux_heatmap {
                                    let lum = if has_lighting {
                                        let vc = poly.vertex_colors[v_idx];
                                        0.2126 * vc[0] + 0.7152 * vc[1] + 0.0722 * vc[2]
                                    } else {
                                        0.5
                                    };
                                    crate::editor_state::lux_to_heatmap_color(lum)
                                } else if use_lm_layer > 0 {
                                    surf_color
                                } else if has_lighting {
                                    let vc = poly.vertex_colors[v_idx];
                                    [surf_color[0] * vc[0], surf_color[1] * vc[1], surf_color[2] * vc[2], 1.0]
                                } else {
                                    surf_color
                                }
                            }
                            crate::camera::ViewportShading::Wireframe => surf_color,
                        };
                        solid_verts.push(SolidVertex::with_texture_and_lightmap(
                            v.to_array(),
                            norm,
                            v_col,
                            uv,
                            use_tex_layer,
                            use_lm_layer,
                            poly.flags,
                            lm_uv,
                        ));
                    }

                    let v0 = poly.vertices[0];
                    for i in 1..n - 1 {
                        let (idx1, idx2) = if is_culled_structural
                            && (poly.vertices[i] - v0)
                                .cross(poly.vertices[i + 1] - v0)
                                .dot(poly.plane.normal)
                                < 0.0
                        {
                            winding_swaps += 1;
                            (i + 1, i)
                        } else {
                            (i, i + 1)
                        };
                        dst_indices.push(base_v);
                        dst_indices.push(base_v + idx1 as u32);
                        dst_indices.push(base_v + idx2 as u32);
                    }
                }
                self.cached_carved_polys = carved_world_polys;

                // Movers solid polys
                if state.show_mover_polys {
                    let mover_normal_color = [0.85, 0.25, 0.95, 1.0];
                    let mover_sel_color = [1.0, 0.45, 1.0, 1.0];
                    for (m_idx, mover) in state.movers.iter().enumerate() {
                        if !state.is_mover_visible(m_idx) {
                            continue;
                        }
                        let color = if state.selected_mover_index == Some(m_idx) {
                            mover_sel_color
                        } else {
                            mover_normal_color
                        };
                        let polys = mover.brush.to_world_polys();
                        for poly in &polys {
                            let n = poly.vertices.len();
                            if n < 3 {
                                continue;
                            }
                            let norm = poly.plane.normal.to_array();
                            let (tex_layer, tex_w, tex_h) =
                                self.resolve_or_upload_texture_layer(queue, state, &poly.texture);
                            let uv_scale_x = TEXTURE_ARRAY_DIM as f32 / tex_w.max(1.0);
                            let uv_scale_y = TEXTURE_ARRAY_DIM as f32 / tex_h.max(1.0);
                            let m_surf_color = if tex_layer > 0 {
                                [1.0, 0.85, 1.0, 1.0]
                            } else {
                                [color[0] * 0.8, color[1] * 0.8, color[2] * 0.8, 0.92]
                            };
                            let m_dst_indices = if (poly.flags & (ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_MODULATED)) != 0
                                || m_surf_color[3] < 0.999
                            {
                                &mut translucent_indices
                            } else if tex_layer > 0 && (poly.flags & ut99_bsp::PF_MASKED) != 0 {
                                &mut masked_indices
                            } else {
                                &mut detail_indices
                            };
                            let base_v = solid_verts.len() as u32;
                            for &v in &poly.vertices {
                                let raw_uv = poly.compute_uv(v);
                                solid_verts.push(SolidVertex::with_texture(
                                    v.to_array(),
                                    norm,
                                    m_surf_color,
                                    [raw_uv[0] * uv_scale_x, raw_uv[1] * uv_scale_y],
                                    tex_layer,
                                    poly.flags,
                                ));
                            }
                            for i in 1..n - 1 {
                                m_dst_indices.push(base_v);
                                m_dst_indices.push(base_v + i as u32);
                                m_dst_indices.push(base_v + (i + 1) as u32);
                            }
                        }
                    }
                }

                // Static Meshes solid polys
                let static_mesh_solid = [0.35, 0.70, 0.40, 1.0];
                for (m_idx, mesh) in state.static_meshes.iter().enumerate() {
                    if !state.is_static_mesh_visible(m_idx) {
                        continue;
                    }
                    let world_polys = mesh.to_world_polys();
                    for (p_idx, poly) in world_polys.iter().enumerate() {
                        let n = poly.vertices.len();
                        if n < 3 {
                            continue;
                        }
                        let norm = poly.plane.normal.to_array();
                        let (tex_layer, tex_w, tex_h) =
                            self.resolve_or_upload_texture_layer(queue, state, &poly.texture);
                        let uv_scale_x = TEXTURE_ARRAY_DIM as f32 / tex_w.max(1.0);
                        let uv_scale_y = TEXTURE_ARRAY_DIM as f32 / tex_h.max(1.0);
                        let use_tex_layer = match effective_shading {
                            crate::camera::ViewportShading::ZonePortal
                            | crate::camera::ViewportShading::DepthComplexity => 0,
                            _ => tex_layer,
                        };
                        let mesh_color = match effective_shading {
                            crate::camera::ViewportShading::ZonePortal => {
                                zone_portal_color(0, m_idx + 100)
                            }
                            crate::camera::ViewportShading::DepthComplexity => {
                                depth_complexity_color(n, p_idx + m_idx * 3)
                            }
                            _ if use_tex_layer > 0 => [1.0, 1.0, 1.0, 1.0],
                            _ => static_mesh_solid,
                        };
                        let sm_dst_indices = if (poly.flags & (ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_MODULATED)) != 0 {
                            &mut translucent_indices
                        } else if use_tex_layer > 0 && (poly.flags & ut99_bsp::PF_MASKED) != 0 {
                            &mut masked_indices
                        } else {
                            &mut twosided_indices
                        };
                        let base_v = solid_verts.len() as u32;
                        for &v in &poly.vertices {
                            let raw_uv = poly.compute_uv(v);
                            solid_verts.push(SolidVertex::with_texture(
                                v.to_array(),
                                norm,
                                mesh_color,
                                [raw_uv[0] * uv_scale_x, raw_uv[1] * uv_scale_y],
                                use_tex_layer,
                                poly.flags,
                            ));
                        }
                        for i in 1..n - 1 {
                            sm_dst_indices.push(base_v);
                            sm_dst_indices.push(base_v + i as u32);
                            sm_dst_indices.push(base_v + (i + 1) as u32);
                        }
                    }
                }
            }
            drop(overlap_counts);
            if let Some(bounds) = cached_brush_bounds {
                self.cached_brush_bounds = bounds;
            }

            // Projected Decal solid polygons
            let decal_solid_color = [0.85, 0.20, 0.20, 0.95];
            for decal in &state.active_projector_decals {
                let n = decal.vertices.len();
                if n < 3 {
                    continue;
                }
                let norm = decal.normal.to_array();
                let (tex_layer, _, _) =
                    self.resolve_or_upload_texture_layer(queue, state, &decal.texture);
                let decal_color = if tex_layer > 0 {
                    [1.0, 1.0, 1.0, 0.95]
                } else {
                    decal_solid_color
                };
                let base_v = solid_verts.len() as u32;
                for v_idx in 0..n {
                    solid_verts.push(SolidVertex::with_texture(
                        decal.vertices[v_idx].to_array(),
                        norm,
                        decal_color,
                        decal.uvs[v_idx],
                        tex_layer,
                        0,
                    ));
                }
                for i in 1..n - 1 {
                    translucent_indices.push(base_v);
                    translucent_indices.push(base_v + i as u32);
                    translucent_indices.push(base_v + (i + 1) as u32);
                }
            }

            // 2D Viewport Reference Background Blueprint Textured Quad
            if let Some(ref bg_path) = state.background_image_path {
                let (bg_tex_layer, _, _) =
                    self.resolve_or_upload_texture_layer(queue, state, bg_path);
                let quad_col = if bg_tex_layer > 0 {
                    [1.0, 1.0, 1.0, 0.45]
                } else {
                    [0.20, 0.45, 0.75, 0.25]
                };
                let norm = [0.0, 0.0, 1.0];
                let flags = ut99_bsp::PF_UNLIT | ut99_bsp::PF_TRANSLUCENT;
                for &(cx, cy, half_w, half_h) in background_blueprint_rects(state.background_display_mode) {
                    let p0 = [cx - half_w, cy - half_h, -1.0];
                    let p1 = [cx + half_w, cy - half_h, -1.0];
                    let p2 = [cx + half_w, cy + half_h, -1.0];
                    let p3 = [cx - half_w, cy + half_h, -1.0];
                    let base_v = solid_verts.len() as u32;
                    solid_verts.push(SolidVertex::with_texture(p0, norm, quad_col, [0.0, 256.0], bg_tex_layer, flags));
                    solid_verts.push(SolidVertex::with_texture(p1, norm, quad_col, [256.0, 256.0], bg_tex_layer, flags));
                    solid_verts.push(SolidVertex::with_texture(p2, norm, quad_col, [256.0, 0.0], bg_tex_layer, flags));
                    solid_verts.push(SolidVertex::with_texture(p3, norm, quad_col, [0.0, 0.0], bg_tex_layer, flags));
                    translucent_indices.extend_from_slice(&[
                        base_v,
                        base_v + 1,
                        base_v + 2,
                        base_v,
                        base_v + 2,
                        base_v + 3,
                    ]);
                }
            }

            let mut packed_indices = std::mem::take(&mut self.chunk_opaque_indices_scratch);
            packed_indices.clear();
            let mut slot_scratch = std::mem::take(&mut self.chunk_slot_scratch);
            super::spatial_chunks::partition_solid_bucket_into_chunks(
                &solid_indices,
                &solid_verts,
                packed_indices.len() as u32,
                &mut packed_indices,
                &mut self.cached_solid_chunks[0],
                &mut slot_scratch,
            );
            let opaque_end = packed_indices.len() as u32;
            solid_indices.clear();
            super::spatial_chunks::partition_solid_bucket_into_chunks(
                &twosided_indices,
                &solid_verts,
                opaque_end,
                &mut packed_indices,
                &mut self.cached_solid_chunks[1],
                &mut slot_scratch,
            );
            let twosided_end = packed_indices.len() as u32;
            twosided_indices.clear();
            super::spatial_chunks::partition_solid_bucket_into_chunks(
                &detail_indices,
                &solid_verts,
                twosided_end,
                &mut packed_indices,
                &mut self.cached_solid_chunks[2],
                &mut slot_scratch,
            );
            let detail_end = packed_indices.len() as u32;
            detail_indices.clear();
            super::spatial_chunks::partition_solid_bucket_into_chunks(
                &masked_indices,
                &solid_verts,
                detail_end,
                &mut packed_indices,
                &mut self.cached_solid_chunks[3],
                &mut slot_scratch,
            );
            let masked_end = packed_indices.len() as u32;
            masked_indices.clear();
            super::spatial_chunks::partition_solid_bucket_into_chunks(
                &translucent_indices,
                &solid_verts,
                masked_end,
                &mut packed_indices,
                &mut self.cached_solid_chunks[4],
                &mut slot_scratch,
            );
            translucent_indices.clear();
            self.chunk_slot_scratch = slot_scratch;
            let total_indices = packed_indices.len() as u32;
            let vertex_count = solid_verts.len() as u32;
            let ranges = StaticSolidRanges {
                opaque_end,
                twosided_end,
                detail_end,
                masked_end,
                total: total_indices,
            };
            self.last_culled_opaque_triangles = (opaque_end / 3) as usize;
            self.last_winding_swaps = winding_swaps;

            solids_ms = if any_solid_viewport {
                solids_started.elapsed().as_secs_f64() * 1000.0
            } else {
                0.0
            };

            let solid_up_started = std::time::Instant::now();
            uploaded_solid_bytes = Self::upload_persistent_buffer(
                device,
                queue,
                "Persistent Solid Vertex Buffer",
                &mut self.solid_buffer,
                &mut self.solid_buffer_capacity,
                &solid_verts,
            ) + Self::upload_persistent_buffer_with_usage(
                device,
                queue,
                "Persistent Solid Index Buffer",
                wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                &mut self.solid_index_buffer,
                &mut self.solid_index_buffer_capacity,
                &packed_indices,
            );
            solid_upload_ms = solid_up_started.elapsed().as_secs_f64() * 1000.0;

            self.cached_solid_ranges = ranges;
            self.cached_solid_vertex_count = vertex_count;
            self.cached_static_solid_key = Some(static_solid_key);
            self.cached_solid_lightmap_fallbacks = self.last_lightmap_fallbacks;
            self.cached_solid_texture_fallbacks = self.last_texture_fallbacks;
            self.frame_solids = retain_vertex_storage(solid_verts, self.max_retained_vertex_bytes);
            self.frame_solid_indices = retain_vertex_storage(packed_indices, self.max_retained_vertex_bytes);
            self.frame_solids_retained = self.frame_solids.len() == vertex_count as usize
                && self.frame_solid_indices.len() == total_indices as usize;
            self.chunk_opaque_indices_scratch = retain_vertex_storage(solid_indices, self.max_retained_vertex_bytes);
            self.frame_indices_twosided = retain_vertex_storage(twosided_indices, self.max_retained_vertex_bytes);
            self.frame_indices_detail = retain_vertex_storage(detail_indices, self.max_retained_vertex_bytes);
            self.frame_indices_masked = retain_vertex_storage(masked_indices, self.max_retained_vertex_bytes);
            self.frame_indices_translucent = retain_vertex_storage(translucent_indices, self.max_retained_vertex_bytes);
        }

        let traj_csg_sig = if state.show_trajectories && any_solid_viewport {
            self.cached_csg_signature
        } else {
            0
        };
        let static_line_key = StaticLineKey {
            map_instance_id: state.map_instance_id,
            grid_step_bits: state.grid.step.to_bits(),
            grid_extent_bits: state.grid.extent.to_bits(),
            viewport_layout: match state.viewport_layout {
                crate::camera::ViewportLayout::Single => 0,
                crate::camera::ViewportLayout::FourViewports => 1,
            },
            active_viewport_type: if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
                0
            } else {
                viewport_type_tag(state.active_viewport_type)
            },
            effective_shading: shading_tag(effective_shading),
            any_solid_viewport,
            zone_flood_sig: zone_sig_for_keys,
            world_brushes_rev: state.world_brushes.rev(),
            geometry_revision: state.geometry_revision,
            movers_rev: state.movers.rev(),
            static_meshes_rev: state.static_meshes.rev(),
            projectors_rev: state.projectors.rev(),
            decals_rev: state.active_projector_decals.rev(),
            actors_rev,
            map_actors_rev: state.map.actors.rev(),
            ai_path_links_rev: state.ai_path_links.rev(),
            visibility_selection_hash: compute_line_visibility_selection_hash(state),
            builder_brush_hash: compute_builder_brush_hash(state),
            overlay_flags_hash: compute_overlay_flags_hash(state),
            nav_graph_hash: compute_nav_graph_hash(state),
            traj_csg_sig,
        };
        let static_lines_hit = self.cached_static_line_key == Some(static_line_key)
            && !zone_flood_rebuilt
            && (self.cached_static_line_count == 0 || self.line_buffer.is_some())
            && (!self.frame_lines_retained
                || self.frame_lines.len() == self.cached_static_line_count as usize);

        let overlay_sig =
            super::overlay_cache::compute_overlay_scene_signature(state, world_signature)
                ^ traj_csg_sig;
        self.overlay_cache.sync(overlay_sig);

        let mut wireframe_ms = 0.0;
        let mut ovl_movers_meshes_ms = 0.0;
        let mut ovl_actors_links_ms = 0.0;
        let mut ovl_trajectories_ms = 0.0;
        let mut ovl_radii_portals_sky_ms = 0.0;
        let mut ovl_zone_seals_ms = 0.0;
        let mut ovl_dispatchers_movtraj_ms = 0.0;
        let mut uploaded_static_line_bytes = 0usize;
        let mut static_line_upload_ms = 0.0;

        if !static_lines_hit {
            let mut line_verts = std::mem::take(&mut self.frame_lines);
            line_verts.clear();
            let mut visible_brushes = 0usize;
            let mut visible_actors = 0usize;

            let (quadrant_grid_ranges, world_lines_start) =
                if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
                    let xy_start = 0u32;
                    self.grid_cache.append(&state.grid, crate::camera::ViewportType::Top, &mut line_verts);
                    let xy_end = line_verts.len() as u32;

                    let yz_start = line_verts.len() as u32;
                    self.grid_cache.append(&state.grid, crate::camera::ViewportType::Side, &mut line_verts);
                    let yz_end = line_verts.len() as u32;

                    let xz_start = line_verts.len() as u32;
                    self.grid_cache.append(&state.grid, crate::camera::ViewportType::Front, &mut line_verts);
                    let xz_end = line_verts.len() as u32;

                    (
                        [
                            (xy_start, xy_end),
                            (yz_start, yz_end),
                            (xz_start, xz_end),
                            (xy_start, xy_end),
                        ],
                        xz_end,
                    )
                } else {
                    self.grid_cache.append(&state.grid, state.active_viewport_type, &mut line_verts);
                    let end = line_verts.len() as u32;
                    ([(0, end); 4], end)
                };

            if let Some(flood) = zone_flood_result {
                for leak in &flood.leak_paths {
                    for seg in leak.waypoints.windows(2) {
                        line_verts.push(LineVertex::new(seg[0].to_array(), [1.0, 0.15, 0.10, 1.0]));
                        line_verts.push(LineVertex::new(seg[1].to_array(), [1.0, 0.65, 0.05, 1.0]));
                    }
                }
            }

            // 1A. World Brushes Wireframe Outlines
            let wire_started = std::time::Instant::now();
            for (b_idx, brush) in state.world_brushes.iter().enumerate() {
                if !state.is_brush_visible(b_idx) {
                    continue;
                }
                visible_brushes += 1;
                let is_sel = state.is_brush_selected(b_idx);
                let solidity = state.get_brush_solidity(b_idx);
                let color = match brush.csg_oper {
                    CsgOperation::Add => match solidity {
                        crate::editor_state::BrushSolidity::SemiSolid => [0.15, 0.85, 0.35, 1.0],
                        crate::editor_state::BrushSolidity::NonSolid => [0.85, 0.90, 0.20, 1.0],
                        crate::editor_state::BrushSolidity::Solid => [0.15, 0.45, 0.85, 1.0],
                    },
                    CsgOperation::Subtract => [0.85, 0.75, 0.15, 1.0],
                    _ => [0.6, 0.6, 0.6, 1.0],
                };

                self.world_wireframe.append(brush, &mut line_verts, |p_idx| {
                    if state.is_surface_selected(b_idx, p_idx) {
                        [1.0, 0.75, 0.1, 1.0]
                    } else if is_sel {
                        [1.0, 1.0, 1.0, 1.0]
                    } else {
                        [color[0] * 1.2, color[1] * 1.2, color[2] * 1.2, 1.0]
                    }
                });
            }
            wireframe_ms = wire_started.elapsed().as_secs_f64() * 1000.0;

            let ovl_movers_started = std::time::Instant::now();

            // 2. Active Red Builder Brush
            if state.show_builder_brush {
                let red_builder_color = if state.is_builder_selected {
                    [1.0, 0.45, 0.45, 1.0]
                } else {
                    [0.95, 0.15, 0.15, 1.0]
                };
                let builder_polys = state.builder_brush.to_world_polys();
                for poly in &builder_polys {
                    let n = poly.vertices.len();
                    for i in 0..n {
                        let next = (i + 1) % n;
                        line_verts.push(LineVertex::new(poly.vertices[i].to_array(), red_builder_color));
                        line_verts.push(LineVertex::new(poly.vertices[next].to_array(), red_builder_color));
                    }
                }
            }

        // 2b. Movers Wireframes (Purple/Magenta [0.85, 0.25, 0.95, 1.0]) and Keyframe Paths
        let mover_normal_color = [0.85, 0.25, 0.95, 1.0];
        let mover_sel_color = [1.0, 0.45, 1.0, 1.0];
        let key_line_color = [0.95, 0.60, 1.0, 0.85];

        for (m_idx, mover) in state.movers.iter().enumerate() {
            if !state.is_mover_visible(m_idx) {
                continue;
            }
            visible_brushes += 1;

            let is_sel = state.selected_mover_index == Some(m_idx);
            let color = if is_sel { mover_sel_color } else { mover_normal_color };
            let polys = mover.brush.to_world_polys();
            for poly in &polys {
                let n = poly.vertices.len();
                for i in 0..n {
                    let next = (i + 1) % n;
                    line_verts.push(LineVertex::new(poly.vertices[i].to_array(), color));
                    line_verts.push(LineVertex::new(poly.vertices[next].to_array(), color));
                }
            }

            // Draw line connecting keyframes (Key 0 -> Key 1 -> ...)
            let mut prev_loc: Option<glam::Vec3> = None;
            for kf_opt in &mover.keyframes {
                if let Some(ref kf) = kf_opt {
                    add_wire_box(&mut line_verts, kf.location, glam::Vec3::splat(8.0), key_line_color);
                    if let Some(prev) = prev_loc {
                        line_verts.push(LineVertex::new(prev.to_array(), key_line_color));
                        line_verts.push(LineVertex::new(kf.location.to_array(), key_line_color));
                    }
                    prev_loc = Some(kf.location);
                }
            }
        }

        // 2B. Static Meshes Wireframes
        let static_mesh_wire = [0.45, 0.95, 0.55, 1.0];
        for (m_idx, mesh) in state.static_meshes.iter().enumerate() {
            if !state.is_static_mesh_visible(m_idx) {
                continue;
            }
            visible_brushes += 1;

            let is_sel = state.selected_static_mesh_index == Some(m_idx);
            let wire_color = if is_sel { [1.0, 1.0, 0.2, 1.0] } else { static_mesh_wire };

            let world_polys = mesh.to_world_polys();
            for poly in &world_polys {
                let n = poly.vertices.len();
                if n < 3 { continue; }
                for i in 0..n {
                    let next = (i + 1) % n;
                    line_verts.push(LineVertex::new(poly.vertices[i].to_array(), wire_color));
                    line_verts.push(LineVertex::new(poly.vertices[next].to_array(), wire_color));
                }
            }
        }

        // 2C. Projectors & Baked Decals Wireframes
        let proj_frustum_color = [1.0, 0.55, 0.05, 0.8];
        for (p_idx, proj) in state.projectors.iter().enumerate() {
            let is_sel = state.selected_projector_index == Some(p_idx);
            let color = if is_sel { [1.0, 0.9, 0.2, 1.0] } else { proj_frustum_color };
            let (fwd, right, up) = proj.get_axes();
            let hw = proj.width * 0.5;
            let hh = proj.height * 0.5;
            let far_center = proj.location + fwd * proj.max_trace_distance;

            // Near quad
            let n0 = proj.location + right * hw + up * hh;
            let n1 = proj.location - right * hw + up * hh;
            let n2 = proj.location - right * hw - up * hh;
            let n3 = proj.location + right * hw - up * hh;
            line_verts.push(LineVertex::new(n0.to_array(), color));
            line_verts.push(LineVertex::new(n1.to_array(), color));
            line_verts.push(LineVertex::new(n1.to_array(), color));
            line_verts.push(LineVertex::new(n2.to_array(), color));
            line_verts.push(LineVertex::new(n2.to_array(), color));
            line_verts.push(LineVertex::new(n3.to_array(), color));
            line_verts.push(LineVertex::new(n3.to_array(), color));
            line_verts.push(LineVertex::new(n0.to_array(), color));

            // Far quad
            let f0 = far_center + right * hw + up * hh;
            let f1 = far_center - right * hw + up * hh;
            let f2 = far_center - right * hw - up * hh;
            let f3 = far_center + right * hw - up * hh;
            line_verts.push(LineVertex::new(f0.to_array(), color));
            line_verts.push(LineVertex::new(f1.to_array(), color));
            line_verts.push(LineVertex::new(f1.to_array(), color));
            line_verts.push(LineVertex::new(f2.to_array(), color));
            line_verts.push(LineVertex::new(f2.to_array(), color));
            line_verts.push(LineVertex::new(f3.to_array(), color));
            line_verts.push(LineVertex::new(f3.to_array(), color));
            line_verts.push(LineVertex::new(f0.to_array(), color));

            // Connecting rays
            line_verts.push(LineVertex::new(n0.to_array(), color));
            line_verts.push(LineVertex::new(f0.to_array(), color));
            line_verts.push(LineVertex::new(n1.to_array(), color));
            line_verts.push(LineVertex::new(f1.to_array(), color));
            line_verts.push(LineVertex::new(n2.to_array(), color));
            line_verts.push(LineVertex::new(f2.to_array(), color));
            line_verts.push(LineVertex::new(n3.to_array(), color));
            line_verts.push(LineVertex::new(f3.to_array(), color));
        }

        // Decal polygon wireframes
        let decal_wire_color = [1.0, 0.35, 0.35, 1.0];
        for decal in &state.active_projector_decals {
            let n = decal.vertices.len();
            if n < 3 { continue; }
            for i in 0..n {
                let next = (i + 1) % n;
                line_verts.push(LineVertex::new(decal.vertices[i].to_array(), decal_wire_color));
                line_verts.push(LineVertex::new(decal.vertices[next].to_array(), decal_wire_color));
            }
        }

        // 2D. Navigation Graph from state.nav_graph
        if state.show_ai_paths {
            let nav_edge_color = [0.10, 0.85, 0.35, 0.6];
            for spec in &state.nav_graph.reach_specs {
                if let (Some(n1), Some(n2)) = (state.nav_graph.nodes.get(spec.from), state.nav_graph.nodes.get(spec.to)) {
                    line_verts.push(LineVertex::new(n1.location.to_array(), nav_edge_color));
                    line_verts.push(LineVertex::new(n2.location.to_array(), nav_edge_color));
                }
            }
        }

        ovl_movers_meshes_ms = ovl_movers_started.elapsed().as_secs_f64() * 1000.0;

        // 3. Actor Gizmos for loaded .unr level (authoritative single source of truth: level.actors)
        let mut ovl_radii_part_ms = 0.0;
        if let Some(ref level) = state.loaded_unr {
            let ovl_actors_started = std::time::Instant::now();
            let light_color = [1.0, 0.88, 0.20, 1.0];
            let light_circle_color = [1.0, 0.85, 0.20, 0.20];
            let spawn_color = [0.20, 0.95, 0.35, 1.0];
            let path_color = [0.15, 0.85, 0.95, 1.0];
            let pickup_color = [1.0, 0.55, 0.15, 1.0];
            let sky_color = [0.35, 0.75, 1.0, 1.0];
            let water_color = [0.15, 0.45, 0.95, 1.0];
            let zone_color = [0.65, 0.35, 0.90, 1.0];
            let trigger_color = [0.20, 0.95, 0.55, 1.0];
            let teleporter_color = [0.15, 0.90, 1.0, 1.0];
            let kicker_color = [1.0, 0.55, 0.10, 1.0];
            let sound_icon_color = [0.30, 0.75, 1.0, 1.0];
            let generic_actor_color = [0.85, 0.80, 0.55, 0.95];
            const ROT_TO_RAD: f32 = std::f32::consts::PI / 32768.0;

            const PICKUP_SUBSTRS: [&str; 19] = [
                "weapon",
                "ammo",
                "health",
                "armor",
                "shield",
                "vial",
                "keg",
                "udamage",
                "pickup",
                "enforcer",
                "gesbiorifle",
                "shockrifle",
                "pulsegun",
                "ripper",
                "minigun",
                "flakcannon",
                "ut_eightball",
                "sniperrifle",
                "warheadlauncher",
            ];

            for (act_idx, actor) in level.actors.iter().enumerate() {
                if !state.is_actor_visible(act_idx) {
                    continue;
                }
                let cls = actor.class_name.as_str();
                let short_cls = cls.rsplit('.').next().unwrap_or(cls);
                if short_cls.eq_ignore_ascii_case("brush")
                    || short_cls.eq_ignore_ascii_case("levelinfo")
                    || short_cls.eq_ignore_ascii_case("camera")
                {
                    continue;
                }

                let is_light = super::overlay_cache::contains_ascii_icase(cls, "light")
                    || actor.light_brightness > 0;
                let is_playerstart =
                    super::overlay_cache::contains_ascii_icase(cls, "playerstart");
                let cull_rad = if is_light {
                    if actor.light_radius > 0 { (actor.light_radius as f32) * 25.0 } else { 30.0 }
                } else if is_playerstart {
                    75.0
                } else {
                    40.0
                };
                visible_actors += 1;

                if is_light {
                    add_octahedron(&mut line_verts, actor.location, 20.0, light_color);
                    if actor.light_radius > 0 {
                        add_wire_circle(&mut line_verts, actor.location, cull_rad, light_circle_color);
                    }
                    if cls.contains("Spotlight") || cls.contains("DirectionalLight") {
                        let pitch_rad = actor.rotation.0 as f32 * ROT_TO_RAD;
                        let yaw_rad = actor.rotation.1 as f32 * ROT_TO_RAD;
                        let fwd = glam::Vec3::new(
                            pitch_rad.cos() * yaw_rad.cos(),
                            pitch_rad.cos() * yaw_rad.sin(),
                            pitch_rad.sin(),
                        ) * 48.0;
                        line_verts.push(LineVertex::new(actor.location.to_array(), light_color));
                        line_verts.push(LineVertex::new((actor.location + fwd).to_array(), light_color));
                    }
                } else if is_playerstart {
                    add_wire_cylinder(&mut line_verts, actor.location + glam::Vec3::new(0.0, 0.0, 36.0), 18.0, 72.0, spawn_color);
                    let yaw_rad = actor.rotation.1 as f32 * ROT_TO_RAD;
                    let forward = glam::Vec3::new(yaw_rad.cos(), yaw_rad.sin(), 0.0) * 45.0;
                    line_verts.push(LineVertex::new(actor.location.to_array(), spawn_color));
                    line_verts.push(LineVertex::new((actor.location + forward).to_array(), spawn_color));
                } else if super::overlay_cache::contains_ascii_icase(cls, "pathnode")
                    || super::overlay_cache::contains_ascii_icase(cls, "inventoryspot")
                    || super::overlay_cache::contains_ascii_icase(cls, "translocdest")
                {
                    add_octahedron(&mut line_verts, actor.location, 16.0, path_color);
                } else if cls.contains("SkyZoneInfo") {
                    add_wire_box(&mut line_verts, actor.location, glam::Vec3::splat(22.0), sky_color);
                    add_octahedron(&mut line_verts, actor.location, 16.0, sky_color);
                } else if cls.contains("WaterZone") {
                    add_wire_box(&mut line_verts, actor.location, glam::Vec3::splat(20.0), water_color);
                } else if cls.contains("ZoneInfo") {
                    add_octahedron(&mut line_verts, actor.location, 20.0, zone_color);
                } else if cls.contains("Trigger") || cls.contains("Dispatcher") || cls.contains("SpecialEvent") || cls.contains("MusicEvent") {
                    add_octahedron(&mut line_verts, actor.location, 18.0, trigger_color);
                    add_wire_circle(&mut line_verts, actor.location, 20.0, trigger_color);
                } else if cls.contains("Teleporter") || cls.contains("WarpZone") {
                    add_wire_circle(&mut line_verts, actor.location, 22.0, teleporter_color);
                    add_octahedron(&mut line_verts, actor.location, 16.0, teleporter_color);
                    let yaw_rad = actor.rotation.1 as f32 * ROT_TO_RAD;
                    let fwd = glam::Vec3::new(yaw_rad.cos(), yaw_rad.sin(), 0.0) * 40.0;
                    line_verts.push(LineVertex::new(actor.location.to_array(), teleporter_color));
                    line_verts.push(LineVertex::new((actor.location + fwd).to_array(), teleporter_color));
                } else if cls.contains("Kicker") || cls.contains("JumpPad") || cls.contains("JumpSpot") || cls.contains("LiftExit") || cls.contains("LiftCenter") {
                    add_octahedron(&mut line_verts, actor.location, 18.0, kicker_color);
                    add_wire_box(&mut line_verts, actor.location, glam::Vec3::new(14.0, 14.0, 6.0), kicker_color);
                    if let Some(prop) = actor.properties.get("KickVelocity") {
                        if let ut99_format::TaggedValue::Vector(v) = prop.value {
                            if v.length_squared() > 1.0 {
                                let dir_tip = actor.location + v.normalize() * 52.0;
                                line_verts.push(LineVertex::new(actor.location.to_array(), [1.0, 0.85, 0.15, 1.0]));
                                line_verts.push(LineVertex::new(dir_tip.to_array(), [1.0, 0.85, 0.15, 1.0]));
                            }
                        }
                    }
                } else if cls.contains("Sound") {
                    add_octahedron(&mut line_verts, actor.location, 14.0, sound_icon_color);
                    add_wire_circle(&mut line_verts, actor.location, 18.0, sound_icon_color);
                } else if PICKUP_SUBSTRS
                    .iter()
                    .any(|&kw| super::overlay_cache::contains_ascii_icase(cls, kw))
                {
                    add_wire_box(&mut line_verts, actor.location, glam::Vec3::splat(16.0), pickup_color);
                } else {
                    add_octahedron(&mut line_verts, actor.location, 15.0, generic_actor_color);
                }
            }

            // D. Navigation Path Links (from EditorState precalculated reachability graph)
            if state.show_ai_paths {
                for (from, to, color) in &state.ai_path_links {
                    line_verts.push(LineVertex::new(from.to_array(), *color));
                    line_verts.push(LineVertex::new(to.to_array(), *color));
                }
            }

            // D2. Tag & Event Trigger Wiring Links
            if state.show_event_links {
                let event_color = [1.0, 0.85, 0.15, 0.90]; // Radiant Amber / Gold
                for (from, to, _) in self.overlay_cache.event_tag_links(|| state.get_event_tag_links()) {
                    line_verts.push(LineVertex::new(from.to_array(), event_color));
                    line_verts.push(LineVertex::new(to.to_array(), event_color));
                }
            }

            ovl_actors_links_ms = ovl_actors_started.elapsed().as_secs_f64() * 1000.0;
            let ovl_traj_started = std::time::Instant::now();

            // D3. Ballistic Flight Trajectories & Teleport Arcs
            if state.show_trajectories {
                let kicker_color = [1.0, 0.45, 0.05, 0.95];   // Radiant Orange / Amber
                let tele_color = [0.15, 0.85, 1.0, 0.95];     // Electric Cyan / Teal
                let bullseye_color = [1.0, 0.25, 0.05, 1.0];  // Scarlet Target Ring

                let carved_opt: Option<&[(usize, ut99_bsp::Polygon)]> =
                    if any_solid_viewport && self.cached_csg_map_instance_id == state.map_instance_id {
                        (!self.cached_carved_polys.is_empty())
                            .then_some(self.cached_carved_polys.as_slice())
                    } else if any_solid_viewport && self.csg_in_flight {
                        Some(&[][..])
                    } else {
                        None
                    };
                for arc in self
                    .overlay_cache
                    .trajectories(|| state.compute_trajectories_with_carved(carved_opt))
                {
                    let arc_color = if arc.is_teleport { tele_color } else { kicker_color };
                    for w in arc.points.windows(2) {
                        line_verts.push(LineVertex::new(w[0].to_array(), arc_color));
                        line_verts.push(LineVertex::new(w[1].to_array(), arc_color));
                    }

                    if let Some(landing) = arc.landing_point {
                        if !arc.is_teleport {
                            // Target Bullseye rings on floor
                            add_wire_circle(&mut line_verts, landing, 16.0, bullseye_color);
                            add_wire_circle(&mut line_verts, landing, 32.0, arc_color);
                            add_wire_circle(&mut line_verts, landing, 48.0, bullseye_color);

                            // Crosshair ticks
                            line_verts.push(LineVertex::new((landing + glam::Vec3::new(-56.0, 0.0, 1.0)).to_array(), bullseye_color));
                            line_verts.push(LineVertex::new((landing + glam::Vec3::new(56.0, 0.0, 1.0)).to_array(), bullseye_color));
                            line_verts.push(LineVertex::new((landing + glam::Vec3::new(0.0, -56.0, 1.0)).to_array(), bullseye_color));
                            line_verts.push(LineVertex::new((landing + glam::Vec3::new(0.0, 56.0, 1.0)).to_array(), bullseye_color));
                        } else {
                            // Entry and exit portal rings
                            add_wire_circle(&mut line_verts, landing, 24.0, tele_color);
                            add_wire_circle(&mut line_verts, arc.start, 24.0, tele_color);
                        }
                    }
                }
            }
            ovl_trajectories_ms = ovl_traj_started.elapsed().as_secs_f64() * 1000.0;

            let ovl_radii_started = std::time::Instant::now();

            // F. Selected Actor Highlight & Influence Spheres / Radii View (UnrealEd 2.0 Parity)
            for (act_idx, actor) in level.actors.iter().enumerate() {
                let is_sel = state.is_actor_selected(act_idx);
                if is_sel || state.show_actor_radii {
                    if is_sel {
                        let highlight_color = [1.0, 1.0, 1.0, 1.0];
                        add_wire_box(&mut line_verts, actor.location, glam::Vec3::splat(28.0), highlight_color);
                    }

                    // Light radius sphere
                    if actor.class_name.contains("Light") || actor.light_radius > 0 || actor.light_brightness > 0 {
                        let radius = if actor.light_radius > 0 {
                            (actor.light_radius as f32) * 25.0
                        } else {
                            400.0
                        };
                        let mut light_color = ut99_light_color_rgba(actor.light_hue, actor.light_saturation, actor.light_brightness);
                        if !is_sel {
                            light_color[3] = 0.35; // Dimmer for unselected actors in full Radii View
                        }
                        add_wire_sphere(&mut line_verts, actor.location, radius, light_color);
                    }

                    // Sound radius sphere
                    if actor.class_name.contains("Sound") || actor.properties.contains_key("SoundRadius") || actor.properties.contains_key("AmbientSound") {
                        let radius = actor.properties.get("SoundRadius")
                            .and_then(|p| match p.value {
                                ut99_format::unr::tagged_property::TaggedValue::Byte(b) => Some(b as f32 * 40.0),
                                ut99_format::unr::tagged_property::TaggedValue::Int(i) => Some(i as f32 * 40.0),
                                _ => None,
                            })
                            .unwrap_or(350.0);
                        let sound_color = if is_sel { [0.2, 0.75, 1.0, 0.75] } else { [0.2, 0.75, 1.0, 0.3] };
                        add_wire_sphere(&mut line_verts, actor.location, radius, sound_color);
                    }

                    // Collision cylinder (UnrealEd 2.0 Radii View & collision verification)
                    let (col_radius, col_height) = {
                        let r = actor.properties.get("CollisionRadius")
                            .and_then(|p| match p.value {
                                ut99_format::unr::tagged_property::TaggedValue::Float(f) => Some(f),
                                ut99_format::unr::tagged_property::TaggedValue::Int(i) => Some(i as f32),
                                ut99_format::unr::tagged_property::TaggedValue::Byte(b) => Some(b as f32),
                                _ => None,
                            });
                        let h = actor.properties.get("CollisionHeight")
                            .and_then(|p| match p.value {
                                ut99_format::unr::tagged_property::TaggedValue::Float(f) => Some(f),
                                ut99_format::unr::tagged_property::TaggedValue::Int(i) => Some(i as f32),
                                ut99_format::unr::tagged_property::TaggedValue::Byte(b) => Some(b as f32),
                                _ => None,
                            });

                        if let (Some(r), Some(h)) = (r, h) {
                            (r, h)
                        } else if actor.class_name.contains("PlayerStart") {
                            (18.0, 40.0)
                        } else if actor.class_name.contains("PathNode") {
                            (20.0, 30.0)
                        } else if actor.class_name.contains("Trigger") {
                            (r.unwrap_or(40.0), h.unwrap_or(40.0))
                        } else if actor.class_name.contains("Pawn") || actor.class_name.contains("Bot") {
                            (17.0, 39.0)
                        } else if actor.class_name.contains("Weapon") || actor.class_name.contains("Ammo") || actor.class_name.contains("Pickup") {
                            (20.0, 20.0)
                        } else {
                            (r.unwrap_or(0.0), h.unwrap_or(0.0))
                        }
                    };

                    if col_radius > 0.0 && col_height > 0.0 {
                        let col_color = if is_sel { [0.2, 0.95, 0.4, 0.85] } else { [0.2, 0.95, 0.4, 0.35] };
                        add_wire_cylinder(&mut line_verts, actor.location, col_radius, col_height * 2.0, col_color);
                    }
                }
            }
            ovl_radii_part_ms = ovl_radii_started.elapsed().as_secs_f64() * 1000.0;
        }

        let ovl_portals_sky_started = std::time::Instant::now();

        // Zone Portal Visualizer (PF_PORTAL)
        let portal_color = [0.0, 1.0, 0.6, 0.95];
        for (b_idx, b) in state.world_brushes.iter().enumerate() {
            if !state.is_brush_visible(b_idx) {
                continue;
            }
            let model = brush_model_matrix(b);
            for poly in &b.polys {
                if (poly.flags & ut99_bsp::PF_PORTAL) != 0 && poly.vertices.len() >= 3 {
                    let n = poly.vertices.len();
                    for i in 0..n {
                        let v0 = model.transform_point3(poly.vertices[i]);
                        let v1 = model.transform_point3(poly.vertices[(i + 1) % n]);
                        line_verts.push(LineVertex::new(v0.to_array(), portal_color));
                        line_verts.push(LineVertex::new(v1.to_array(), portal_color));
                    }
                    if n >= 4 {
                        let v0 = model.transform_point3(poly.vertices[0]);
                        let v2 = model.transform_point3(poly.vertices[2]);
                        let v1 = model.transform_point3(poly.vertices[1]);
                        let v3 = model.transform_point3(poly.vertices[3]);
                        let inner_color = [0.0, 0.8, 0.5, 0.6];
                        line_verts.push(LineVertex::new(v0.to_array(), inner_color));
                        line_verts.push(LineVertex::new(v2.to_array(), inner_color));
                        line_verts.push(LineVertex::new(v1.to_array(), inner_color));
                        line_verts.push(LineVertex::new(v3.to_array(), inner_color));
                    }
                }
            }
        }

        // SkyZone & Fake Backdrop Visualizer
        if state.show_skybox_links {
            let backdrop_border_color = [0.20, 0.75, 1.0, 0.95];   // Sky cyan
            let backdrop_inner_color = [0.35, 0.85, 1.0, 0.50];    // Ethereal cyan
            let sky_beam_color = [0.25, 0.80, 1.0, 0.75];          // Celestial dashed ray

            let links = self.overlay_cache.skybox_links(|| state.get_skybox_links());
            for link in links {
                let n = link.backdrop_vertices.len();
                if n >= 3 {
                    // Perimeter outline
                    for i in 0..n {
                        let v0 = link.backdrop_vertices[i];
                        let v1 = link.backdrop_vertices[(i + 1) % n];
                        line_verts.push(LineVertex::new(v0.to_array(), backdrop_border_color));
                        line_verts.push(LineVertex::new(v1.to_array(), backdrop_border_color));
                    }

                    // Inner celestial star / diagonal cross
                    if n >= 4 {
                        let v0 = link.backdrop_vertices[0];
                        let v2 = link.backdrop_vertices[2];
                        let v1 = link.backdrop_vertices[1];
                        let v3 = link.backdrop_vertices[3];
                        line_verts.push(LineVertex::new(v0.to_array(), backdrop_inner_color));
                        line_verts.push(LineVertex::new(v2.to_array(), backdrop_inner_color));
                        line_verts.push(LineVertex::new(v1.to_array(), backdrop_inner_color));
                        line_verts.push(LineVertex::new(v3.to_array(), backdrop_inner_color));
                    }

                    // Dashed celestial ray from SkyZoneInfo to backdrop centroid
                    let start = link.skyzone_location;
                    let end = link.backdrop_centroid;
                    let segments = 12;
                    for s in 0..segments {
                        if s % 2 == 0 {
                            let t0 = s as f32 / segments as f32;
                            let t1 = (s + 1) as f32 / segments as f32;
                            let p0 = start.lerp(end, t0);
                            let p1 = start.lerp(end, t1);
                            line_verts.push(LineVertex::new(p0.to_array(), sky_beam_color));
                            line_verts.push(LineVertex::new(p1.to_array(), sky_beam_color));
                        }
                    }
                }
            }

            // SkyZone beacon rings at SkyZoneInfo actors
            if let Some(ref level) = state.loaded_unr {
                for actor in &level.actors {
                    if actor.class_name.contains("SkyZoneInfo") {
                        add_wire_circle(&mut line_verts, actor.location, 24.0, [0.2, 0.85, 1.0, 0.95]);
                        add_wire_circle(&mut line_verts, actor.location, 48.0, [0.1, 0.65, 1.0, 0.70]);
                        add_wire_box(&mut line_verts, actor.location, glam::Vec3::splat(16.0), [0.3, 0.9, 1.0, 1.0]);
                    }
                }
            }
        }

        // Sky Backdrop Surface Highlighter (PF_FAKE_BACKDROP)
        if state.sky_zone_settings.highlight_sky_surfaces {
            let sky_surf_color = [0.85, 0.55, 1.0, 0.88];
            for brush in &state.world_brushes {
                for poly in &brush.polys {
                    if (poly.flags & ut99_bsp::PF_FAKE_BACKDROP) != 0 && poly.vertices.len() >= 3 {
                        let n = poly.vertices.len();
                        for i in 0..n {
                            let v0 = poly.vertices[i] + brush.location;
                            let v1 = poly.vertices[(i + 1) % n] + brush.location;
                            line_verts.push(LineVertex::new(v0.to_array(), sky_surf_color));
                            line_verts.push(LineVertex::new(v1.to_array(), sky_surf_color));
                        }
                    }
                }
            }
        }

        // Bot Navigation Coverage Heatmap
        if state.show_nav_coverage {
            let report = state.compute_nav_coverage();
            let optimal_color = [0.15, 0.90, 0.25, 0.95];   // Bright green (< 400 uu)
            let reachable_color = [0.95, 0.85, 0.10, 0.95]; // Amber yellow (400..800 uu)
            let deadzone_color = [1.00, 0.20, 0.20, 0.95];  // Scarlet red (> 800 uu)

            for sample in &report.samples {
                let center = sample.position + glam::Vec3::new(0.0, 0.0, 2.0);
                match sample.rating {
                    ut99_bsp::NavCoverageRating::Optimal => {
                        add_wire_circle(&mut line_verts, center, 16.0, optimal_color);
                    }
                    ut99_bsp::NavCoverageRating::Reachable => {
                        add_wire_circle(&mut line_verts, center, 20.0, reachable_color);
                        add_wire_circle(&mut line_verts, center, 6.0, reachable_color);
                    }
                    ut99_bsp::NavCoverageRating::DeadZone => {
                        add_wire_circle(&mut line_verts, center, 28.0, deadzone_color);
                        let x_arm = 18.0;
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(-x_arm, -x_arm, 0.0)).to_array(), deadzone_color));
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(x_arm, x_arm, 0.0)).to_array(), deadzone_color));
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(-x_arm, x_arm, 0.0)).to_array(), deadzone_color));
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(x_arm, -x_arm, 0.0)).to_array(), deadzone_color));
                    }
                }
            }
        }

        // Zone Audio & Reverb Resonance Visualization
        if state.show_audio_zones {
            let reports = self
                .overlay_cache
                .zone_audio_reports(|| state.get_zone_audio_reports());
            let reverb_cage_color = [0.85, 0.25, 0.95, 0.85]; // Radiant Magenta / Violet
            let reverb_wave_color = [0.70, 0.20, 0.90, 0.50]; // Ethereal Purple
            let ambient_sound_color = [1.00, 0.60, 0.10, 0.85]; // Amber Orange

            for report in reports {
                let color = if report.is_reverb { reverb_cage_color } else { ambient_sound_color };
                let center = report.location;

                // Central acoustic node glyph: octahedron + inner box
                add_octahedron(&mut line_verts, center, 24.0, color);
                add_wire_box(&mut line_verts, center, glam::Vec3::splat(12.0), color);

                // Sound radius outer boundary cylinder/cage
                let radius = report.sound_radius.max(64.0);
                add_wire_cylinder(&mut line_verts, center, radius, radius * 0.5, color);

                // Waveform propagation concentric rings
                let wave_color = if report.is_reverb { reverb_wave_color } else { [1.0, 0.7, 0.2, 0.4] };
                add_wire_circle(&mut line_verts, center, radius * 0.33, wave_color);
                add_wire_circle(&mut line_verts, center, radius * 0.66, wave_color);
                add_wire_circle(&mut line_verts, center, radius, wave_color);

                // Vertical sound resonance pillar
                let height_vec = glam::Vec3::new(0.0, 0.0, radius * 0.25);
                line_verts.push(LineVertex::new((center - height_vec).to_array(), wave_color));
                line_verts.push(LineVertex::new((center + height_vec).to_array(), wave_color));
            }
        }

        // WarpZone & Non-Euclidean Portal Visualizer
        if state.show_warpzones {
            let topology = self
                .overlay_cache
                .warpzone_topology(|| state.collect_warpzone_topology());
            let selected_idx = state.warp_zone_settings.selected_warp_index;

            for entry in topology {
                let status_color = entry.status.badge_color();
                let is_selected = selected_idx == Some(entry.actor_index);

                // 1. Gateway portal rings & core
                add_wire_circle(&mut line_verts, entry.location, 36.0, status_color);
                add_wire_circle(&mut line_verts, entry.location, 54.0, [status_color[0] * 0.7, status_color[1] * 0.7, status_color[2] * 0.7, 0.45]);
                add_octahedron(&mut line_verts, entry.location, 20.0, status_color);

                // Highlight halo for selected portal
                if is_selected {
                    let halo_color = [1.0, 0.9, 0.2, 0.95]; // Golden Radiance
                    add_wire_circle(&mut line_verts, entry.location, 70.0, halo_color);
                    add_wire_box(&mut line_verts, entry.location, glam::Vec3::splat(24.0), halo_color);
                }

                // 2. 3D Directional Forward Vector
                if state.warp_zone_settings.show_portal_normal_vectors {
                    let fwd_len = 64.0;
                    let tip = entry.location + entry.forward_vector * fwd_len;
                    let vec_color = [status_color[0], status_color[1], status_color[2], 0.95];
                    line_verts.push(LineVertex::new(entry.location.to_array(), vec_color));
                    line_verts.push(LineVertex::new(tip.to_array(), vec_color));

                    // Arrowhead fins
                    let right = entry.forward_vector.cross(glam::Vec3::Z).normalize_or_zero() * 8.0;
                    let up = glam::Vec3::Z * 8.0;
                    let barb = tip - entry.forward_vector * 14.0;
                    line_verts.push(LineVertex::new(tip.to_array(), vec_color));
                    line_verts.push(LineVertex::new((barb + right).to_array(), vec_color));
                    line_verts.push(LineVertex::new(tip.to_array(), vec_color));
                    line_verts.push(LineVertex::new((barb - right).to_array(), vec_color));
                    line_verts.push(LineVertex::new(tip.to_array(), vec_color));
                    line_verts.push(LineVertex::new((barb + up).to_array(), vec_color));
                }

                // 3. Inter-dimensional Helical Connection Beam
                if state.warp_zone_settings.show_connecting_helices {
                    if let Some(target_loc) = entry.target_location {
                        let beam_color = [status_color[0] * 0.85, status_color[1] * 0.85, status_color[2] * 0.95, 0.70];
                        let segs = 24;
                        for s in 0..segs {
                            let t0 = s as f32 / segs as f32;
                            let t1 = (s + 1) as f32 / segs as f32;
                            let p0 = entry.location.lerp(target_loc, t0);
                            let p1 = entry.location.lerp(target_loc, t1);

                            let angle0 = t0 * std::f32::consts::TAU * 3.0;
                            let angle1 = t1 * std::f32::consts::TAU * 3.0;
                            let helix_offset0 = glam::Vec3::new(angle0.cos() * 16.0, 0.0, angle0.sin() * 16.0);
                            let helix_offset1 = glam::Vec3::new(angle1.cos() * 16.0, 0.0, angle1.sin() * 16.0);

                            line_verts.push(LineVertex::new((p0 + helix_offset0).to_array(), beam_color));
                            line_verts.push(LineVertex::new((p1 + helix_offset1).to_array(), beam_color));
                        }
                    }
                }

                // 4. Portal Camera Exit Frustum Preview
                if state.warp_zone_settings.show_portal_camera_frustum {
                    if let (Some(tgt_loc), Some(tgt_fwd)) = (entry.target_location, entry.target_forward_vector) {
                        let frustum_color = [0.20, 0.85, 1.00, 0.55]; // Cyan Portal Lens
                        let fwd = tgt_fwd;
                        let right = fwd.cross(glam::Vec3::Z).normalize_or_zero() * 32.0;
                        let up = right.cross(fwd).normalize_or_zero() * 24.0;
                        let apex = tgt_loc;
                        let base_center = tgt_loc + fwd * 80.0;

                        let c0 = base_center - right - up;
                        let c1 = base_center + right - up;
                        let c2 = base_center + right + up;
                        let c3 = base_center - right + up;

                        // Rays from apex
                        line_verts.push(LineVertex::new(apex.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c0.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(apex.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c1.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(apex.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c2.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(apex.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c3.to_array(), frustum_color));

                        // Base rectangle
                        line_verts.push(LineVertex::new(c0.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c1.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c1.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c2.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c2.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c3.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c3.to_array(), frustum_color));
                        line_verts.push(LineVertex::new(c0.to_array(), frustum_color));
                    }
                }

                // 5. Unpaired Orphan Hazard Marker
                if entry.status == crate::editor_state::WarpConnectionStatus::OrphanBroken {
                    let alert_color = [1.0, 0.2, 0.2, 0.95];
                    add_wire_box(&mut line_verts, entry.location, glam::Vec3::splat(18.0), alert_color);
                    line_verts.push(LineVertex::new(entry.location.to_array(), alert_color));
                    line_verts.push(LineVertex::new((entry.location + glam::Vec3::new(0.0, 0.0, 96.0)).to_array(), alert_color));
                }
            }
        }

        // Anti-Portal Occluder Crimson Hazard Visualization
        if state.show_anti_portals {
            let anti_portal_color = [1.00, 0.22, 0.28, 0.95]; // Crimson Hazard
            let hatch_color = [1.00, 0.40, 0.15, 0.60];       // Amber Cross-Hatch

            for brush in &state.world_brushes {
                let has_anti_portal = brush.polys.iter().any(|p| (p.flags & ut99_bsp::PF_ANTI_PORTAL) != 0);
                if has_anti_portal {
                    for poly in &brush.polys {
                        let n = poly.vertices.len();
                        if n >= 3 {
                            // Perimeter wireframe
                            for i in 0..n {
                                let v0 = poly.vertices[i] + brush.location;
                                let v1 = poly.vertices[(i + 1) % n] + brush.location;
                                line_verts.push(LineVertex::new(v0.to_array(), anti_portal_color));
                                line_verts.push(LineVertex::new(v1.to_array(), anti_portal_color));
                            }
                            // Cross-bracing diagonals
                            if n >= 4 {
                                let v0 = poly.vertices[0] + brush.location;
                                let v2 = poly.vertices[2] + brush.location;
                                let v1 = poly.vertices[1] + brush.location;
                                let v3 = poly.vertices[3] + brush.location;
                                line_verts.push(LineVertex::new(v0.to_array(), hatch_color));
                                line_verts.push(LineVertex::new(v2.to_array(), hatch_color));
                                line_verts.push(LineVertex::new(v1.to_array(), hatch_color));
                                line_verts.push(LineVertex::new(v3.to_array(), hatch_color));
                            }
                        }
                    }
                }
            }
        }

        ovl_radii_portals_sky_ms = ovl_radii_part_ms + ovl_portals_sky_started.elapsed().as_secs_f64() * 1000.0;
        let ovl_zone_seals_started = std::time::Instant::now();

        // Zone Boundary Leak Detector Visualization
        if state.show_zone_leaks_visual {
            let leak_ray_color = [1.00, 0.15, 0.15, 0.95];   // Scarlet red
            let leak_marker_color = [1.00, 0.35, 0.10, 1.00]; // Amber-crimson hazard
            for report in self.overlay_cache.zone_seals(|| state.analyze_zone_seals()) {
                for leak in &report.detected_leaks {
                    // Dashed escape beam from zone location to leak point
                    let segs = 16;
                    for s in 0..segs {
                        if s % 2 == 0 {
                            let t0 = s as f32 / segs as f32;
                            let t1 = (s + 1) as f32 / segs as f32;
                            let p0 = leak.zone_location.lerp(leak.leak_location, t0);
                            let p1 = leak.zone_location.lerp(leak.leak_location, t1);
                            line_verts.push(LineVertex::new(p0.to_array(), leak_ray_color));
                            line_verts.push(LineVertex::new(p1.to_array(), leak_ray_color));
                        }
                    }
                    // Hazard marker at leak site
                    add_octahedron(&mut line_verts, leak.leak_location, 16.0, leak_marker_color);
                    add_wire_circle(&mut line_verts, leak.leak_location, 24.0, leak_ray_color);
                }
            }
        }

        ovl_zone_seals_ms = ovl_zone_seals_started.elapsed().as_secs_f64() * 1000.0;
        let ovl_dispatchers_started = std::time::Instant::now();

        // Mover Trajectory Spline & Continuous 3D Ribbon Paths
        if state.show_mover_trajectories {
            let rail_color = [1.00, 0.75, 0.15, 0.95]; // Golden Amber
            let arrow_color = [1.00, 0.95, 0.25, 1.00]; // Luminous Yellow
            let key_dot_color = [0.95, 0.35, 1.00, 1.00]; // Magenta
            let base_key_color = [0.25, 0.85, 1.00, 1.00]; // Cyan

            for traj in self
                .overlay_cache
                .mover_trajectories(|| state.get_mover_trajectories())
            {
                // Continuous 3D ribbon with directional flow arrows
                add_3d_ribbon_trajectory(&mut line_verts, &traj.spline_points, 12.0, rail_color, arrow_color);

                // Keyframe nodes with orientation stalks
                for (k_idx, key) in traj.keyframes.iter().enumerate() {
                    let is_base = k_idx == 0;
                    let node_col = if is_base { base_key_color } else { key_dot_color };

                    add_octahedron(&mut line_verts, key.position, 12.0, node_col);
                    add_wire_box(&mut line_verts, key.position, glam::Vec3::splat(6.0), node_col);

                    // 3D Directional orientation stalk (Pitch + Yaw)
                    let p_rad = key.rotation.x.to_radians();
                    let y_rad = key.rotation.y.to_radians();
                    let forward_dir = glam::Vec3::new(
                        p_rad.cos() * y_rad.cos(),
                        p_rad.cos() * y_rad.sin(),
                        -p_rad.sin(),
                    ).normalize() * 28.0;

                    let tip = key.position + forward_dir;
                    line_verts.push(LineVertex::new(key.position.to_array(), [1.0, 0.9, 0.2, 1.0]));
                    line_verts.push(LineVertex::new(tip.to_array(), [1.0, 0.9, 0.2, 1.0]));

                    // Directional arrowhead
                    let perp = if forward_dir.z.abs() > 24.0 { glam::Vec3::X } else { glam::Vec3::Z };
                    let head_wing = forward_dir.cross(perp).normalize() * 6.0;
                    line_verts.push(LineVertex::new(tip.to_array(), [1.0, 0.9, 0.2, 1.0]));
                    line_verts.push(LineVertex::new((tip - forward_dir * 0.25 + head_wing).to_array(), [1.0, 0.9, 0.2, 1.0]));
                    line_verts.push(LineVertex::new(tip.to_array(), [1.0, 0.9, 0.2, 1.0]));
                    line_verts.push(LineVertex::new((tip - forward_dir * 0.25 - head_wing).to_array(), [1.0, 0.9, 0.2, 1.0]));
                }
            }
        }

        // Dispatcher Timeline & Sequential Trigger Arcs
        let dispatcher_color = [0.25, 0.85, 1.00, 0.90]; // Electric cyan
        let arc_stage_color = [0.85, 0.40, 1.00, 0.75];  // Violet arc
        let selected_disp_name = state.dispatcher_settings.selected_dispatcher_name.as_deref()
            .or(state.selected_dispatcher_name.as_deref());
        let current_time = state.dispatcher_settings.timeline_time;

        let (dispatcher_seqs, dispatcher_chains) = self.overlay_cache.dispatchers(|| {
            let seqs = state.get_dispatcher_sequences();
            let chains = ut99_bsp::detect_cascading_dispatcher_chains(&seqs);
            (seqs, chains)
        });
        for seq in dispatcher_seqs {
            let is_selected_disp = selected_disp_name == Some(&seq.dispatcher_name);
            let disp_col = if is_selected_disp { [1.00, 0.90, 0.20, 1.00] } else { dispatcher_color };

            add_octahedron(&mut line_verts, seq.location, if is_selected_disp { 28.0 } else { 22.0 }, disp_col);
            add_wire_box(&mut line_verts, seq.location, glam::Vec3::splat(if is_selected_disp { 18.0 } else { 14.0 }), disp_col);

            if is_selected_disp {
                add_wire_circle(&mut line_verts, seq.location, 36.0, [1.00, 0.90, 0.20, 0.60]);
            }

            for stage in &seq.stages {
                let is_stage_active = (current_time >= stage.cumulative_time_secs - 0.2)
                    && (current_time <= stage.cumulative_time_secs + 0.4);
                let current_arc_col = if is_stage_active {
                    [1.00, 0.95, 0.30, 0.95] // Glowing gold for currently triggered stage
                } else {
                    arc_stage_color
                };

                for (_tgt_name, tgt_loc) in &stage.matched_targets {
                    // Parabolic event arc connecting dispatcher to target actor
                    let segs = 16;
                    let mid_height = ((seq.location.distance(*tgt_loc)) * 0.25).clamp(32.0, 220.0);
                    for s in 0..segs {
                        let t0 = s as f32 / segs as f32;
                        let t1 = (s + 1) as f32 / segs as f32;
                        let mut p0 = seq.location.lerp(*tgt_loc, t0);
                        let mut p1 = seq.location.lerp(*tgt_loc, t1);
                        p0.z += (1.0 - (2.0 * t0 - 1.0).powi(2)) * mid_height;
                        p1.z += (1.0 - (2.0 * t1 - 1.0).powi(2)) * mid_height;
                        line_verts.push(LineVertex::new(p0.to_array(), current_arc_col));
                        line_verts.push(LineVertex::new(p1.to_array(), current_arc_col));
                    }

                    // Target indicator ring
                    add_wire_circle(&mut line_verts, *tgt_loc, if is_stage_active { 32.0 } else { 20.0 }, current_arc_col);

                    if is_stage_active {
                        add_octahedron(&mut line_verts, *tgt_loc + glam::Vec3::new(0.0, 0.0, 24.0), 14.0, [1.0, 0.8, 0.1, 0.9]);
                    }
                }
            }
        }

        // Cascading Multi-Dispatcher Chains & Circular Loop Hazard Warnings
        for chain in dispatcher_chains {
            let chain_col = if chain.is_circular {
                [1.00, 0.20, 0.20, 0.95] // Red hazard for circular recursion
            } else {
                [0.20, 0.95, 0.50, 0.85] // Emerald green for cascading chain
            };

            if let Some(ref level) = state.loaded_unr {
                let src_pos = level.actors.iter().find(|a| a.name == chain.source_dispatcher).map(|a| a.location);
                let tgt_pos = level.actors.iter().find(|a| a.name == chain.target_dispatcher).map(|a| a.location);

                if let (Some(p_src), Some(p_tgt)) = (src_pos, tgt_pos) {
                    let segs = 20;
                    for s in 0..segs {
                        let t0 = s as f32 / segs as f32;
                        let t1 = (s + 1) as f32 / segs as f32;
                        let p0 = p_src.lerp(p_tgt, t0);
                        let p1 = p_src.lerp(p_tgt, t1);
                        line_verts.push(LineVertex::new(p0.to_array(), chain_col));
                        line_verts.push(LineVertex::new(p1.to_array(), chain_col));
                    }
                    if chain.is_circular {
                        add_wire_box(&mut line_verts, p_tgt, glam::Vec3::splat(26.0), chain_col);
                    }
                }
            }
        }

            // Zone PVS Graph & Portal Cell Boundaries
            if state.show_zone_pvs {
                let pvs_portal_col = [0.15, 0.95, 0.65, 0.95]; // Mint Green
                for (b_idx, brush) in state.world_brushes.iter().enumerate() {
                    if !state.is_brush_visible(b_idx) {
                        continue;
                    }
                    let model = brush_model_matrix(brush);
                    for poly in &brush.polys {
                        if (poly.flags & ut99_bsp::poly::PF_PORTAL) != 0 && poly.vertices.len() >= 3 {
                            let n = poly.vertices.len();
                            for i in 0..n {
                                let v0 = model.transform_point3(poly.vertices[i]);
                                let v1 = model.transform_point3(poly.vertices[(i + 1) % n]);
                                line_verts.push(LineVertex::new(v0.to_array(), pvs_portal_col));
                                line_verts.push(LineVertex::new(v1.to_array(), pvs_portal_col));
                            }
                        }
                    }
                }
            }

            // Bot AI Navigation Graph, Reachability Network & JumpPad Arcs
            if state.show_bot_paths {
                let graph = &state.nav_graph;
                for spec in &graph.reach_specs {
                    if spec.from < graph.nodes.len() && spec.to < graph.nodes.len() {
                        let p1 = graph.nodes[spec.from].location;
                        let p2 = graph.nodes[spec.to].location;
                        let col = if (spec.reach_flags & ut99_bsp::R_SPECIAL) != 0 {
                            [0.95, 0.20, 0.90, 0.85] // Magenta / Lift
                        } else if (spec.reach_flags & ut99_bsp::R_JUMP) != 0 {
                            [0.20, 0.70, 1.00, 0.85] // Cyan / Jump
                        } else {
                            [0.20, 0.95, 0.35, 0.80] // Green / Walk
                        };
                        line_verts.push(LineVertex::new(p1.to_array(), col));
                        line_verts.push(LineVertex::new(p2.to_array(), col));
                    }
                }

                // Render Nodes & Tactical Cover Markers
                for (idx, node) in graph.nodes.iter().enumerate() {
                    let is_sel = state.bot_nav_settings.selected_node_id == Some(idx);
                    let node_col = if is_sel {
                        [1.00, 0.85, 0.20, 1.00] // Yellow / Selected
                    } else if node.node_type.contains("PlayerStart") {
                        [0.20, 0.85, 1.00, 0.80] // Cyan
                    } else if node.node_type.contains("Lift") {
                        [0.95, 0.20, 0.90, 0.85] // Magenta
                    } else if node.node_type.contains("Inventory") {
                        [0.40, 0.95, 0.40, 0.85] // Lime
                    } else {
                        [0.25, 0.65, 0.95, 0.75] // Blue
                    };

                    add_wire_box(&mut line_verts, node.location, glam::Vec3::splat(12.0), node_col);

                    if state.bot_nav_settings.show_tactical_cover {
                        add_wire_circle(&mut line_verts, node.location, 20.0, node_col);
                    }
                }

                // Render JumpPad Trajectory Arcs
                if state.bot_nav_settings.show_jumppad_arcs {
                    let arcs = self
                        .overlay_cache
                        .jumppad_arcs(|| state.compute_jumppad_trajectories());
                    for arc in arcs {
                        let arc_col = if arc.has_overshoot_hazard {
                            [1.00, 0.20, 0.20, 0.90] // Red / Hazard
                        } else {
                            [0.15, 0.85, 1.00, 0.90] // Electric Cyan
                        };

                        for window in arc.trajectory_points.windows(2) {
                            line_verts.push(LineVertex::new(window[0].to_array(), arc_col));
                            line_verts.push(LineVertex::new(window[1].to_array(), arc_col));
                        }

                        // Landing target helper
                        add_wire_circle(&mut line_verts, arc.estimated_landing_pos, 24.0, arc_col);
                    }
                }
            }

            // L (Wireframe). 2D Viewport Reference Background Blueprint Frame
            if state.background_image_path.is_some() {
                let bg_col = [0.30, 0.65, 0.95, 0.45];
                for &(cx, cy, half_w, half_h) in background_blueprint_rects(state.background_display_mode) {
                    let p0 = [cx - half_w, cy - half_h, -1.0];
                    let p1 = [cx + half_w, cy - half_h, -1.0];
                    let p2 = [cx + half_w, cy + half_h, -1.0];
                    let p3 = [cx - half_w, cy + half_h, -1.0];
                    line_verts.push(LineVertex::new(p0, bg_col));
                    line_verts.push(LineVertex::new(p1, bg_col));
                    line_verts.push(LineVertex::new(p1, bg_col));
                    line_verts.push(LineVertex::new(p2, bg_col));
                    line_verts.push(LineVertex::new(p2, bg_col));
                    line_verts.push(LineVertex::new(p3, bg_col));
                    line_verts.push(LineVertex::new(p3, bg_col));
                    line_verts.push(LineVertex::new(p0, bg_col));
                    line_verts.push(LineVertex::new(p0, bg_col));
                    line_verts.push(LineVertex::new(p2, bg_col));
                    line_verts.push(LineVertex::new(p1, bg_col));
                    line_verts.push(LineVertex::new(p3, bg_col));
                }
            }

            ovl_dispatchers_movtraj_ms = ovl_dispatchers_started.elapsed().as_secs_f64() * 1000.0;

            let mut line_chunks = std::mem::take(&mut self.cached_line_chunks);
            let mut chunk_line_scratch = std::mem::take(&mut self.chunk_line_scratch);
            let mut slot_scratch = std::mem::take(&mut self.chunk_slot_scratch);
            let line_partition = super::spatial_chunks::partition_world_lines_into_chunks(
                &mut line_verts[world_lines_start as usize..],
                world_lines_start,
                &mut line_chunks,
                &mut chunk_line_scratch,
                &mut slot_scratch,
            );
            self.chunk_slot_scratch = slot_scratch;
            if !line_chunks.is_empty() {
                if line_partition.grid.is_none() {
                    line_chunks[0].brush_count = visible_brushes as u32;
                    line_chunks[0].actor_count = visible_actors as u32;
                } else {
                    state.ensure_world_brush_bounds();
                    state.ensure_mover_bounds();
                    state.ensure_static_mesh_bounds();
                    let bounds_cache = state.scene_bounds_cache.borrow();
                    for (b_idx, &(bmin, bmax)) in bounds_cache.world_brushes.iter().enumerate() {
                        if !state.is_brush_visible(b_idx) {
                            continue;
                        }
                        let c_idx = line_partition.chunk_for_point((bmin + bmax) * 0.5, (bmax - bmin).max_element());
                        line_chunks[c_idx].brush_count += 1;
                    }
                    for (m_idx, &(bmin, bmax)) in bounds_cache.movers.iter().enumerate() {
                        if !state.is_mover_visible(m_idx) {
                            continue;
                        }
                        let c_idx = line_partition.chunk_for_point((bmin + bmax) * 0.5, (bmax - bmin).max_element());
                        line_chunks[c_idx].brush_count += 1;
                    }
                    for (m_idx, &(bmin, bmax)) in bounds_cache.static_meshes.iter().enumerate() {
                        if !state.is_static_mesh_visible(m_idx) {
                            continue;
                        }
                        let c_idx = line_partition.chunk_for_point((bmin + bmax) * 0.5, (bmax - bmin).max_element());
                        line_chunks[c_idx].brush_count += 1;
                    }
                    drop(bounds_cache);
                    if let Some(ref level) = state.loaded_unr {
                        for (act_idx, actor) in level.actors.iter().enumerate() {
                            if !state.is_actor_visible(act_idx) {
                                continue;
                            }
                            let cls = actor.class_name.as_str();
                            let short_cls = cls.rsplit('.').next().unwrap_or(cls);
                            if short_cls.eq_ignore_ascii_case("brush")
                                || short_cls.eq_ignore_ascii_case("levelinfo")
                                || short_cls.eq_ignore_ascii_case("camera")
                            {
                                continue;
                            }
                            let c_idx = line_partition.chunk_for_point(actor.location, 0.0);
                            line_chunks[c_idx].actor_count += 1;
                        }
                    }
                }
            }
            self.cached_line_chunks = line_chunks;
            self.chunk_line_scratch = retain_vertex_storage(chunk_line_scratch, self.max_retained_vertex_bytes);

            let static_line_up_started = std::time::Instant::now();
            uploaded_static_line_bytes = Self::upload_persistent_buffer(
                device,
                queue,
                "Persistent Line Vertex Buffer",
                &mut self.line_buffer,
                &mut self.line_buffer_capacity,
                &line_verts,
            );
            static_line_upload_ms = static_line_up_started.elapsed().as_secs_f64() * 1000.0;

            self.cached_static_line_key = Some(static_line_key);
            self.cached_quadrant_grid_ranges = quadrant_grid_ranges;
            self.cached_world_lines_start = world_lines_start;
            self.cached_static_line_count = line_verts.len() as u32;
            self.cached_visible_brushes = visible_brushes;
            self.cached_visible_actors = visible_actors;
            self.frame_lines = retain_vertex_storage(line_verts, self.max_retained_vertex_bytes);
            self.frame_lines_retained =
                self.frame_lines.len() == self.cached_static_line_count as usize;
        }

        if let Some(cache) = frame_zone_cache {
            self.cached_zone_flood = Some(cache);
        }

        let studio_lines_hit = self
            .cached_studio_lines_key
            .as_ref()
            .is_some_and(|k| k.matches(state, actors_rev))
            && (self.cached_studio_line_count == 0 || self.studio_line_buffer.is_some());
        let mut studio_build_ms = 0.0;
        let mut uploaded_studio_line_bytes = 0usize;
        let mut studio_line_upload_ms = 0.0;
        if !studio_lines_hit {
            let studio_started = std::time::Instant::now();
            let mut line_verts = std::mem::take(&mut self.frame_studio_lines);
            line_verts.clear();

            // Skeletal Mesh, Rigging & Socket Visualizer
            if state.show_skeletal_bones || state.skeletal_settings.show_bones_wireframe {
                let bone_joint_color = [0.20, 0.95, 0.65, 0.95]; // Bright mint/cyan joint
                let bone_link_color = [0.15, 0.80, 0.50, 0.80];  // Emerald bone line
                let socket_color = [1.00, 0.85, 0.20, 0.95];     // Gold socket
                let weapon_box_color = [0.20, 0.85, 1.00, 0.85]; // Electric cyan weapon marker

                for mesh in state.get_skeletal_meshes() {
                    for bone in &mesh.bones {
                        let bone_pos = bone.world_transform.transform_point3(glam::Vec3::ZERO);
                        add_octahedron(&mut line_verts, bone_pos, 4.0, bone_joint_color);

                        if let Some(p_idx) = bone.parent_index {
                            if let Some(parent_bone) = mesh.bones.get(p_idx) {
                                let parent_pos = parent_bone.world_transform.transform_point3(glam::Vec3::ZERO);
                                line_verts.push(LineVertex::new(parent_pos.to_array(), bone_link_color));
                                line_verts.push(LineVertex::new(bone_pos.to_array(), bone_link_color));
                            }
                        }
                    }

                    if state.skeletal_settings.show_socket_helpers {
                        for socket in &mesh.sockets {
                            if let Some(mat) = mesh.get_socket_world_transform(&socket.name) {
                                let sock_pos = mat.transform_point3(glam::Vec3::ZERO);
                                let sock_forward = mat.transform_vector3(glam::Vec3::new(0.0, 1.0, 0.0)).normalize_or_zero() * 12.0;

                                add_wire_circle(&mut line_verts, sock_pos, 6.0, socket_color);
                                line_verts.push(LineVertex::new(sock_pos.to_array(), socket_color));
                                line_verts.push(LineVertex::new((sock_pos + sock_forward).to_array(), socket_color));

                                if let Some(ref weapon_name) = socket.attached_mesh_name {
                                    let w_size = if weapon_name.contains("Flak") {
                                        glam::Vec3::new(6.0, 18.0, 8.0)
                                    } else {
                                        glam::Vec3::new(4.0, 22.0, 6.0)
                                    };
                                    add_wire_box(&mut line_verts, sock_pos + sock_forward * 0.5, w_size, weapon_box_color);
                                }
                            }
                        }
                    }
                }
            }

            // Cinematic Matinee Camera Path & Cutscene Sequencer Live Preview
            if state.show_cinematic_studio_dialog || state.cinematic_settings.show_spline_path {
                let spline_pts = state.cinematic_settings.active_track.sample_polyline(16);
                let path_col = [1.00, 0.75, 0.10, 0.95]; // Golden Amber

                for i in 0..spline_pts.len().saturating_sub(1) {
                    line_verts.push(LineVertex::new(spline_pts[i].to_array(), path_col));
                    line_verts.push(LineVertex::new(spline_pts[i + 1].to_array(), path_col));
                }

                // Keyframe node points
                let node_col = [1.00, 0.90, 0.30, 1.00];
                for kf in &state.cinematic_settings.active_track.keyframes {
                    add_wire_circle(&mut line_verts, kf.position, 16.0, node_col);
                }

                // Animated Camera Frustum at Playhead
                if state.cinematic_settings.show_camera_frustum {
                    let frustum_lines = state.get_camera_frustum_lines_at_playhead();
                    let frustum_col = [0.20, 0.75, 1.00, 0.95]; // Cinema Blue
                    for (start, end) in frustum_lines {
                        line_verts.push(LineVertex::new(start.to_array(), frustum_col));
                        line_verts.push(LineVertex::new(end.to_array(), frustum_col));
                    }
                }
            }

            // Dynamic Light FX & Animating Spotlight/Searchlight Visualizer
            if state.show_light_effects_studio_dialog || state.light_effects_settings.show_animated_beams_viewport {
                let t = state.light_effects_settings.current_time_sec;
                let dyn_state = ut99_bsp::evaluate_dynamic_light(&state.light_effects_settings.preview_light, t);
                let pivot = state.builder_brush.location;

                if (dyn_state.is_directional || state.light_effects_settings.show_spotlight_cones_viewport) && dyn_state.spot_cone_angle_rad > 0.05 {
                    let beam_length = dyn_state.current_radius_uu.max(256.0);
                    let beam_end = pivot + dyn_state.spot_direction * beam_length;
                    let beam_radius = beam_length * dyn_state.spot_cone_angle_rad.tan();

                    let beam_col = [
                        dyn_state.effective_rgb.x.clamp(0.2, 1.0),
                        dyn_state.effective_rgb.y.clamp(0.2, 1.0),
                        dyn_state.effective_rgb.z.clamp(0.2, 1.0),
                        0.90,
                    ];

                    line_verts.push(LineVertex::new(pivot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(beam_end.to_array(), beam_col));

                    let up_hint = if dyn_state.spot_direction.z.abs() > 0.85 { Vec3::Y } else { Vec3::Z };
                    let right = dyn_state.spot_direction.cross(up_hint).normalize() * beam_radius;
                    let true_up = right.cross(dyn_state.spot_direction).normalize() * beam_radius;

                    let c0 = beam_end - right;
                    let c1 = beam_end + right;
                    let c2 = beam_end - true_up;
                    let c3 = beam_end + true_up;

                    line_verts.push(LineVertex::new(pivot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(c0.to_array(), beam_col));
                    line_verts.push(LineVertex::new(pivot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(c1.to_array(), beam_col));
                    line_verts.push(LineVertex::new(pivot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(c2.to_array(), beam_col));
                    line_verts.push(LineVertex::new(pivot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(c3.to_array(), beam_col));

                    add_wire_circle(&mut line_verts, beam_end, beam_radius, beam_col);
                }
            }

            // Zone Portals & AntiPortal Occluder Volume Visualizer
            if state.show_zone_occlusion_studio_dialog || state.zone_occlusion_settings.show_antiportal_shadows {
                let ap_col = [0.85, 0.20, 0.85, 0.90]; // Magenta / Violet
                for ap in &state.antiportals {
                    if ap.is_active {
                        let min_p = ap.location - ap.size;
                        let max_p = ap.location + ap.size;
                        let corners = [
                            Vec3::new(min_p.x, min_p.y, min_p.z),
                            Vec3::new(max_p.x, min_p.y, min_p.z),
                            Vec3::new(max_p.x, max_p.y, min_p.z),
                            Vec3::new(min_p.x, max_p.y, min_p.z),
                            Vec3::new(min_p.x, min_p.y, max_p.z),
                            Vec3::new(max_p.x, min_p.y, max_p.z),
                            Vec3::new(max_p.x, max_p.y, max_p.z),
                            Vec3::new(min_p.x, max_p.y, max_p.z),
                        ];
                        let edges = [
                            (0, 1), (1, 2), (2, 3), (3, 0),
                            (4, 5), (5, 6), (6, 7), (7, 4),
                            (0, 4), (1, 5), (2, 6), (3, 7),
                        ];
                        for &(i0, i1) in &edges {
                            line_verts.push(LineVertex::new(corners[i0].to_array(), ap_col));
                            line_verts.push(LineVertex::new(corners[i1].to_array(), ap_col));
                        }
                    }
                }
            }

            // MusicEvent Dynamic Sound Cues Visualizer
            if state.show_music_studio_dialog || state.music_studio_settings.show_music_event_spheres_viewport {
                let music_col = [0.30, 0.85, 1.00, 0.90]; // Cyan-blue
                let events = self.overlay_cache.music_events(|| state.collect_music_events());
                for ev in events {
                    let p = ev.location;
                    let s = 16.0;
                    // Draw 3D diamond octahedron for MusicEvent
                    let top = p + Vec3::new(0.0, 0.0, s);
                    let bot = p - Vec3::new(0.0, 0.0, s);
                    let m0 = p + Vec3::new(s, 0.0, 0.0);
                    let m1 = p + Vec3::new(0.0, s, 0.0);
                    let m2 = p - Vec3::new(s, 0.0, 0.0);
                    let m3 = p - Vec3::new(0.0, s, 0.0);

                    line_verts.push(LineVertex::new(top.to_array(), music_col));
                    line_verts.push(LineVertex::new(m0.to_array(), music_col));
                    line_verts.push(LineVertex::new(top.to_array(), music_col));
                    line_verts.push(LineVertex::new(m1.to_array(), music_col));
                    line_verts.push(LineVertex::new(top.to_array(), music_col));
                    line_verts.push(LineVertex::new(m2.to_array(), music_col));
                    line_verts.push(LineVertex::new(top.to_array(), music_col));
                    line_verts.push(LineVertex::new(m3.to_array(), music_col));

                    line_verts.push(LineVertex::new(bot.to_array(), music_col));
                    line_verts.push(LineVertex::new(m0.to_array(), music_col));
                    line_verts.push(LineVertex::new(bot.to_array(), music_col));
                    line_verts.push(LineVertex::new(m1.to_array(), music_col));
                    line_verts.push(LineVertex::new(bot.to_array(), music_col));
                    line_verts.push(LineVertex::new(m2.to_array(), music_col));
                    line_verts.push(LineVertex::new(bot.to_array(), music_col));
                    line_verts.push(LineVertex::new(m3.to_array(), music_col));

                    add_wire_circle(&mut line_verts, p, 48.0, music_col);
                }
            }

            // Sun, Corona & Lens Flare Visualizer (rings + starburst; camera LoS ray emitted in dynamic lines)
            if state.show_corona_studio_dialog || state.corona_studio_settings.show_flares_viewport {
                let config = &state.corona_studio_settings.preview_config;
                let corona_col = [
                    config.color_rgb.x.clamp(0.2, 1.0),
                    config.color_rgb.y.clamp(0.2, 1.0),
                    config.color_rgb.z.clamp(0.2, 1.0),
                    0.95,
                ];

                // Render at selected light or pivot
                let target_pos = if let Some(sel_idx) = state.selected_actor_index {
                    state.loaded_unr.as_ref().and_then(|u| u.actors.get(sel_idx).map(|a| a.location)).unwrap_or(state.builder_brush.location)
                } else {
                    state.builder_brush.location
                };

                let r = (config.draw_scale * 24.0).max(12.0);
                add_wire_circle(&mut line_verts, target_pos, r, corona_col);
                add_wire_circle(&mut line_verts, target_pos, r * 1.5, [corona_col[0], corona_col[1], corona_col[2], 0.4]);

                // Draw 8-point diffraction starburst
                for k in 0..8 {
                    let a = (k as f32) * std::f32::consts::PI / 4.0;
                    let spike_len = r * 2.2;
                    let dir = Vec3::new(a.cos(), a.sin(), 0.0);
                    line_verts.push(LineVertex::new(target_pos.to_array(), corona_col));
                    line_verts.push(LineVertex::new((target_pos + dir * spike_len).to_array(), corona_col));
                }
            }

            // Fluid Physics & Hazard Volumes Visualizer
            if state.show_fluid_studio_dialog || state.fluid_studio_settings.show_current_streamers_viewport || state.fluid_studio_settings.show_hazard_boxes_viewport {
                let zones = self
                    .overlay_cache
                    .zone_physics(|| state.collect_level_zone_physics());
                for (_idx, _name, phys) in zones {
                    let col = if phys.b_pain_zone {
                        if phys.damage_type.contains("Burn") {
                            [1.0, 0.25, 0.05, 0.9] // Lava red-orange
                        } else {
                            [0.2, 0.95, 0.2, 0.9] // Slime green
                        }
                    } else if phys.b_water_zone {
                        [0.15, 0.65, 1.0, 0.85] // Water blue
                    } else {
                        [0.8, 0.8, 0.9, 0.7] // Air / Neutral
                    };

                    // Draw flow velocity vector arrow at pivot/zone
                    let flow = phys.zone_velocity;
                    let flow_len = flow.length();
                    if flow_len > 1.0 && state.fluid_studio_settings.show_current_streamers_viewport {
                        let start = state.builder_brush.location;
                        let end = start + flow.normalize() * (flow_len * 0.15).clamp(32.0, 160.0);
                        line_verts.push(LineVertex::new(start.to_array(), col));
                        line_verts.push(LineVertex::new(end.to_array(), col));

                        // Arrowhead
                        let side = if flow.z.abs() > 0.85 * flow_len { Vec3::X } else { Vec3::Z };
                        let perp = flow.cross(side).normalize_or_zero() * 8.0;
                        line_verts.push(LineVertex::new(end.to_array(), col));
                        line_verts.push(LineVertex::new((end - flow.normalize() * 12.0 + perp).to_array(), col));
                        line_verts.push(LineVertex::new(end.to_array(), col));
                        line_verts.push(LineVertex::new((end - flow.normalize() * 12.0 - perp).to_array(), col));
                    }

                    // Draw hazard boundary wireframe box
                    if phys.b_pain_zone && state.fluid_studio_settings.show_hazard_boxes_viewport {
                        let center = state.builder_brush.location;
                        let hs = Vec3::splat(128.0);
                        let min = center - hs;
                        let max = center + hs;

                        // 12 edges of AABB
                        let p = [
                            Vec3::new(min.x, min.y, min.z), Vec3::new(max.x, min.y, min.z),
                            Vec3::new(max.x, max.y, min.z), Vec3::new(min.x, max.y, min.z),
                            Vec3::new(min.x, min.y, max.z), Vec3::new(max.x, min.y, max.z),
                            Vec3::new(max.x, max.y, max.z), Vec3::new(min.x, max.y, max.z),
                        ];
                        let edges = [
                            (0, 1), (1, 2), (2, 3), (3, 0),
                            (4, 5), (5, 6), (6, 7), (7, 4),
                            (0, 4), (1, 5), (2, 6), (3, 7),
                        ];
                        for (i, j) in edges {
                            line_verts.push(LineVertex::new(p[i].to_array(), col));
                            line_verts.push(LineVertex::new(p[j].to_array(), col));
                        }
                    }
                }
            }

            // Weapon Ballistics & Combat Trajectory Visualizer
            if state.show_ballistics_studio_dialog || state.ballistics_studio_settings.show_trajectories_viewport {
                let traj = &state.ballistics_studio_settings.active_trajectory;
                let proj = &state.ballistics_studio_settings.preview_projectile;
                let trail_col = proj.trail_color_rgba;

                // Draw trajectory flight arc
                if traj.len() > 1 && state.ballistics_studio_settings.show_trajectories_viewport {
                    for i in 0..(traj.len() - 1) {
                        line_verts.push(LineVertex::new(traj[i].position.to_array(), trail_col));
                        line_verts.push(LineVertex::new(traj[i + 1].position.to_array(), trail_col));
                    }
                }

                // Draw bounce markers and surface normals
                for pt in traj {
                    if pt.is_bounce {
                        add_wire_circle(&mut line_verts, pt.position, 12.0, [1.0, 1.0, 0.2, 0.9]);
                        if state.ballistics_studio_settings.show_bounce_normals_viewport {
                            if let Some(norm) = pt.normal {
                                let n_end = pt.position + norm * 32.0;
                                line_verts.push(LineVertex::new(pt.position.to_array(), [0.2, 1.0, 0.2, 0.9]));
                                line_verts.push(LineVertex::new(n_end.to_array(), [0.2, 1.0, 0.2, 0.9]));
                            }
                        }
                    }

                    if pt.is_impact && state.ballistics_studio_settings.show_blast_spheres_viewport {
                        let blast_col = [1.0, 0.3, 0.1, 0.85];
                        add_wire_circle(&mut line_verts, pt.position, proj.damage_radius.max(16.0), blast_col);
                        // Draw perpendicular circle for 3D sphere illusion
                        let top = pt.position + Vec3::new(0.0, 0.0, proj.damage_radius.max(16.0));
                        let bot = pt.position - Vec3::new(0.0, 0.0, proj.damage_radius.max(16.0));
                        line_verts.push(LineVertex::new(top.to_array(), blast_col));
                        line_verts.push(LineVertex::new(bot.to_array(), blast_col));
                    }
                }

                // Draw placed ballistic probes
                for probe in &state.ballistics_studio_settings.probes {
                    let probe_col = [0.95, 0.35, 1.0, 0.95]; // Magenta
                    let loc = probe.location;
                    let hs = 12.0;
                    line_verts.push(LineVertex::new((loc - Vec3::X * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::X * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc - Vec3::Y * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::Y * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc - Vec3::Z * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::Z * hs).to_array(), probe_col));

                    // Aiming heading line
                    let pr = probe.pitch_deg.to_radians();
                    let yr = probe.yaw_deg.to_radians();
                    let dir = Vec3::new(pr.cos() * yr.cos(), pr.cos() * yr.sin(), pr.sin());
                    line_verts.push(LineVertex::new(loc.to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + dir * 48.0).to_array(), probe_col));
                }
            }

            // Teleporter Destination & Network Link Visualizer
            if state.show_teleporter_studio_dialog || state.teleporter_studio_settings.show_portal_discs_viewport {
                let graph = self
                    .overlay_cache
                    .teleporter_network(|| state.collect_teleporter_network());

                for node in &graph.nodes {
                    let disc_col = if node.b_changes_velocity {
                        [0.0, 0.9, 1.0, 0.95] // Electric Cyan
                    } else {
                        [0.8, 0.3, 1.0, 0.95] // Portal Violet
                    };

                    // Portal base horizontal disc
                    if state.teleporter_studio_settings.show_portal_discs_viewport {
                        add_wire_circle(&mut line_verts, node.location, 34.0, disc_col);
                        add_wire_circle(&mut line_verts, node.location + Vec3::new(0.0, 0.0, 24.0), 28.0, disc_col);
                        add_wire_circle(&mut line_verts, node.location + Vec3::new(0.0, 0.0, 48.0), 20.0, disc_col);
                    }

                    // Launch velocity vector
                    if state.teleporter_studio_settings.show_exit_vectors_viewport {
                        let fwd = node.forward_direction();
                        let launch_dir = if node.b_changes_velocity && node.target_velocity.length_squared() > 1.0 {
                            node.target_velocity.normalize()
                        } else {
                            fwd
                        };
                        let arrow_col = [1.0, 0.9, 0.2, 0.95];
                        let v_end = node.location + launch_dir * 54.0;
                        line_verts.push(LineVertex::new(node.location.to_array(), arrow_col));
                        line_verts.push(LineVertex::new(v_end.to_array(), arrow_col));
                    }
                }

                // Directional link lines between teleporters
                if state.teleporter_studio_settings.show_network_lines_viewport {
                    for link in &graph.links {
                        let src = graph.nodes.iter().find(|n| n.id == link.source_id);
                        let dst = graph.nodes.iter().find(|n| n.id == link.dest_id);
                        if let (Some(s), Some(d)) = (src, dst) {
                            let link_col = if link.is_bidirectional {
                                [0.2, 0.95, 0.95, 0.85]
                            } else {
                                [0.9, 0.4, 1.0, 0.85]
                            };
                            line_verts.push(LineVertex::new((s.location + Vec3::new(0.0, 0.0, 30.0)).to_array(), link_col));
                            line_verts.push(LineVertex::new((d.location + Vec3::new(0.0, 0.0, 30.0)).to_array(), link_col));
                        }
                    }
                }
            }

            // Translocator Trajectory & Teleport Beacon Visualizer
            if state.show_translocator_studio_dialog || state.translocator_studio_settings.show_arcs_viewport {
                if let Some(ref sim) = state.translocator_studio_settings.active_simulation {
                    let arc_col = [1.0, 0.82, 0.20, 0.95]; // Translocator Gold

                    // Draw parabolic flight arc
                    if sim.points.len() > 1 && state.translocator_studio_settings.show_arcs_viewport {
                        for i in 0..(sim.points.len() - 1) {
                            line_verts.push(LineVertex::new(sim.points[i].position.to_array(), arc_col));
                            line_verts.push(LineVertex::new(sim.points[i + 1].position.to_array(), arc_col));
                        }
                    }

                    // Bounce rings and normal markers
                    if state.translocator_studio_settings.show_bounce_rings_viewport {
                        for pt in &sim.points {
                            if pt.is_bounce {
                                let bounce_col = [1.0, 0.50, 0.15, 0.9];
                                add_wire_circle(&mut line_verts, pt.position, 10.0, bounce_col);
                                if let Some(norm) = pt.surface_normal {
                                    let n_end = pt.position + norm * 24.0;
                                    line_verts.push(LineVertex::new(pt.position.to_array(), bounce_col));
                                    line_verts.push(LineVertex::new(n_end.to_array(), bounce_col));
                                }
                            }
                        }
                    }

                    // Landing Beacon
                    if state.translocator_studio_settings.show_beacons_viewport {
                        if let Some(beacon_pos) = sim.final_beacon_location {
                            let beacon_col = [0.2, 1.0, 0.5, 0.95]; // Emerald
                            add_wire_circle(&mut line_verts, beacon_pos, 16.0, beacon_col);
                            add_wire_circle(&mut line_verts, beacon_pos + Vec3::new(0.0, 0.0, 10.0), 10.0, beacon_col);

                            // Vertical locator stalk
                            line_verts.push(LineVertex::new(beacon_pos.to_array(), beacon_col));
                            line_verts.push(LineVertex::new((beacon_pos + Vec3::new(0.0, 0.0, 36.0)).to_array(), beacon_col));
                        }
                    }
                }

                // Draw placed translocator probes
                for probe in &state.translocator_studio_settings.probes {
                    let probe_col = [1.0, 0.85, 0.3, 0.95];
                    let loc = probe.location;
                    let hs = 14.0;
                    line_verts.push(LineVertex::new((loc - Vec3::X * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::X * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc - Vec3::Y * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::Y * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc - Vec3::Z * hs).to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + Vec3::Z * hs).to_array(), probe_col));

                    let pr = probe.pitch_deg.to_radians();
                    let yr = probe.yaw_deg.to_radians();
                    let dir = Vec3::new(pr.cos() * yr.cos(), pr.cos() * yr.sin(), pr.sin());
                    line_verts.push(LineVertex::new(loc.to_array(), probe_col));
                    line_verts.push(LineVertex::new((loc + dir * 42.0).to_array(), probe_col));
                }
            }

            // LiftCenter, LiftExit & Dynamic Elevator Pathing Visualizer
            if state.show_lift_pathing_studio_dialog || state.lift_pathing_studio_settings.show_visualizer_in_viewport {
                if let Some(ref audit) = state.lift_pathing_studio_settings.cached_audit {
                    let idx = state.lift_pathing_studio_settings.selected_elevator_idx;
                    if let Some(sys) = audit.systems.get(idx).or_else(|| audit.systems.first()) {
                        let shaft_col = [0.95, 0.75, 0.20, 0.85];
                        let floor_col = [0.25, 0.75, 1.00, 0.75];
                        let center_col = [0.20, 1.00, 0.40, 0.95];
                        let exit_col = [0.30, 0.85, 1.00, 0.95];
                        let hazard_col = [1.00, 0.25, 0.20, 0.95];

                        // 1. Draw shaft lines connecting keyframes
                        if state.lift_pathing_studio_settings.show_shaft_lines_viewport && sys.keyframes.len() > 1 {
                            for i in 0..(sys.keyframes.len() - 1) {
                                let p0 = sys.keyframes[i].position;
                                let p1 = sys.keyframes[i + 1].position;
                                line_verts.push(LineVertex::new(p0.to_array(), shaft_col));
                                line_verts.push(LineVertex::new(p1.to_array(), shaft_col));

                                let r = (sys.platform_radius * 0.7).max(48.0);
                                for &(dx, dy) in &[(-r, -r), (r, -r), (r, r), (-r, r)] {
                                    let c0 = p0 + Vec3::new(dx, dy, 0.0);
                                    let c1 = p1 + Vec3::new(dx, dy, 0.0);
                                    line_verts.push(LineVertex::new(c0.to_array(), [shaft_col[0], shaft_col[1], shaft_col[2], 0.45]));
                                    line_verts.push(LineVertex::new(c1.to_array(), [shaft_col[0], shaft_col[1], shaft_col[2], 0.45]));
                                }
                            }
                        }

                        // 2. Draw Keyframe floor outlines
                        if state.lift_pathing_studio_settings.show_keyframe_floors_viewport {
                            for k in &sys.keyframes {
                                add_wire_circle(&mut line_verts, k.position, sys.platform_radius.max(64.0), floor_col);
                                add_wire_circle(&mut line_verts, k.position + Vec3::new(0.0, 0.0, 12.0), sys.platform_radius.max(64.0) * 0.8, floor_col);
                            }
                        }

                        // 3. Draw LiftCenter on platform
                        if let Some(ref center) = sys.lift_center {
                            let c_pos = center.location;
                            add_wire_circle(&mut line_verts, c_pos, 20.0, center_col);
                            add_wire_circle(&mut line_verts, c_pos + Vec3::new(0.0, 0.0, 16.0), 12.0, center_col);

                            line_verts.push(LineVertex::new(c_pos.to_array(), center_col));
                            line_verts.push(LineVertex::new((c_pos + Vec3::new(0.0, 0.0, 36.0)).to_array(), center_col));

                            let hs = 14.0;
                            line_verts.push(LineVertex::new((c_pos - Vec3::X * hs).to_array(), center_col));
                            line_verts.push(LineVertex::new((c_pos + Vec3::X * hs).to_array(), center_col));
                            line_verts.push(LineVertex::new((c_pos - Vec3::Y * hs).to_array(), center_col));
                            line_verts.push(LineVertex::new((c_pos + Vec3::Y * hs).to_array(), center_col));
                        }

                        // 4. Draw LiftExits at each floor
                        for exit in &sys.lift_exits {
                            let e_pos = exit.location;
                            add_wire_circle(&mut line_verts, e_pos, 18.0, exit_col);
                            add_wire_circle(&mut line_verts, e_pos + Vec3::new(0.0, 0.0, 24.0), 12.0, exit_col);

                            if let Some(center_ref) = &sys.lift_center {
                                let dir_to_center = (center_ref.location - e_pos).normalize_or_zero();
                                line_verts.push(LineVertex::new(e_pos.to_array(), exit_col));
                                line_verts.push(LineVertex::new((e_pos + dir_to_center * 36.0).to_array(), exit_col));
                            }
                        }

                        // 5. Draw Animated Bot Riding Avatar
                        if state.lift_pathing_studio_settings.show_bot_avatar_viewport {
                            let sim_steps = &state.lift_pathing_studio_settings.cached_simulation;
                            if !sim_steps.is_empty() {
                                let total_cycle = sys.total_cycle_time.max(0.5);
                                let norm_t = (state.lift_pathing_studio_settings.playback_time / total_cycle).clamp(0.0, 1.0);
                                let s_idx = ((norm_t * (sim_steps.len() - 1) as f32).round() as usize).min(sim_steps.len() - 1);
                                let bot_pos = sim_steps[s_idx].bot_pos;
                                let bot_col = [0.95, 0.85, 0.25, 0.95];

                                add_wire_circle(&mut line_verts, bot_pos, 17.0, bot_col);
                                add_wire_circle(&mut line_verts, bot_pos + Vec3::new(0.0, 0.0, 39.0), 17.0, bot_col);
                                add_wire_circle(&mut line_verts, bot_pos + Vec3::new(0.0, 0.0, 78.0), 17.0, bot_col);

                                for &(dx, dy) in &[(-17.0, 0.0), (17.0, 0.0), (0.0, -17.0), (0.0, 17.0)] {
                                    let b_bot = bot_pos + Vec3::new(dx, dy, 0.0);
                                    let b_top = bot_pos + Vec3::new(dx, dy, 78.0);
                                    line_verts.push(LineVertex::new(b_bot.to_array(), bot_col));
                                    line_verts.push(LineVertex::new(b_top.to_array(), bot_col));
                                }
                            }
                        }

                        // 6. Draw Hazard alerts if any
                        for diag in &sys.diagnostics {
                            if diag.severity == ut99_bsp::LiftDiagnosticSeverity::CriticalHazard {
                                let h_pos = diag.location;
                                add_wire_circle(&mut line_verts, h_pos, 28.0, hazard_col);
                                add_wire_circle(&mut line_verts, h_pos + Vec3::new(0.0, 0.0, 15.0), 20.0, hazard_col);
                                line_verts.push(LineVertex::new((h_pos - Vec3::Z * 20.0).to_array(), hazard_col));
                                line_verts.push(LineVertex::new((h_pos + Vec3::Z * 40.0).to_array(), hazard_col));
                            }
                        }
                    }
                }
            }

            // Dynamic Zone Water Surface & Wave Reflection Visualizer
            if state.show_water_surface_studio_dialog || state.water_surface_studio_settings.show_wave_wireframe_viewport {
                let water_set = &state.water_surface_studio_settings;
                let center = water_set.sheet_center;
                let sx = water_set.sheet_size_x.max(32.0);
                let sy = water_set.sheet_size_y.max(32.0);
                let half_x = sx * 0.5;
                let half_y = sy * 0.5;

                // 1. Boundary box
                if water_set.show_boundary_box_viewport {
                    let box_col = [0.20, 0.60, 0.95, 0.70];
                    add_wire_box(&mut line_verts, center, Vec3::new(half_x, half_y, 25.0), box_col);
                }

                // 2. Animated Undulating Wave Grid Wireframe
                if water_set.show_wave_wireframe_viewport {
                    let sim = &water_set.simulation;
                    let w = sim.width;
                    let h = sim.height;
                    if w > 1 && h > 1 {
                        let step_x = sx / (w - 1) as f32;
                        let step_y = sy / (h - 1) as f32;
                        let wave_col = [0.25, 0.85, 1.00, 0.80];
                        let disp_scale = water_set.wave_displacement_scale;

                        // Grid lines along X
                        for y in 0..h {
                            let fy = -half_y + y as f32 * step_y;
                            for x in 0..(w - 1) {
                                let fx0 = -half_x + x as f32 * step_x;
                                let fx1 = fx0 + step_x;
                                let z0 = sim.current_heights[y * w + x] * disp_scale;
                                let z1 = sim.current_heights[y * w + x + 1] * disp_scale;

                                let p0 = center + Vec3::new(fx0, fy, z0);
                                let p1 = center + Vec3::new(fx1, fy, z1);
                                line_verts.push(LineVertex::new(p0.to_array(), wave_col));
                                line_verts.push(LineVertex::new(p1.to_array(), wave_col));
                            }
                        }

                        // Grid lines along Y
                        for x in 0..w {
                            let fx = -half_x + x as f32 * step_x;
                            for y in 0..(h - 1) {
                                let fy0 = -half_y + y as f32 * step_y;
                                let fy1 = fy0 + step_y;
                                let z0 = sim.current_heights[y * w + x] * disp_scale;
                                let z1 = sim.current_heights[(y + 1) * w + x] * disp_scale;

                                let p0 = center + Vec3::new(fx, fy0, z0);
                                let p1 = center + Vec3::new(fx, fy1, z1);
                                line_verts.push(LineVertex::new(p0.to_array(), wave_col));
                                line_verts.push(LineVertex::new(p1.to_array(), wave_col));
                            }
                        }
                    }
                }

                // 3. Caustics Light Rays & Projected Floor Pattern
                if water_set.show_caustics_viewport {
                    let sim = &water_set.simulation;
                    let w = sim.width;
                    let h = sim.height;
                    let floor_depth = -100.0;
                    let caustic_col = [1.00, 0.92, 0.45, 0.85];

                    if w > 1 && h > 1 {
                        let step_x = sx / (w - 1) as f32;
                        let step_y = sy / (h - 1) as f32;
                        // Draw focused caustics rays for top-focus cells
                        for y in (1..(h - 1)).step_by(2) {
                            for x in (1..(w - 1)).step_by(2) {
                                let idx = y * w + x;
                                let c_val = sim.caustics_intensity_map[idx];
                                if c_val > 1.25 {
                                    let fx = -half_x + x as f32 * step_x;
                                    let fy = -half_y + y as f32 * step_y;
                                    let z_surf = sim.current_heights[idx] * water_set.wave_displacement_scale;
                                    let normal = sim.get_normal(x, y);

                                    let p_surf = center + Vec3::new(fx, fy, z_surf);
                                    let refr_dir = Vec3::new(-normal.x * 0.3, -normal.y * 0.3, -1.0).normalize();
                                    let t = floor_depth / refr_dir.z;
                                    let p_floor = p_surf + refr_dir * t;

                                    let ray_alpha = ((c_val - 1.0) / 1.5).clamp(0.2, 0.85);
                                    let col = [caustic_col[0], caustic_col[1], caustic_col[2], ray_alpha];
                                    line_verts.push(LineVertex::new(p_surf.to_array(), col));
                                    line_verts.push(LineVertex::new(p_floor.to_array(), col));

                                    // Small caustic sparkle circle at floor
                                    add_wire_circle(&mut line_verts, p_floor, 8.0 * (c_val - 1.0), col);
                                }
                            }
                        }
                    }
                }

                // 4. Directional Flow Vectors
                if water_set.show_flow_vectors_viewport {
                    let flow = water_set.active_config.flow_velocity;
                    if flow.x.abs() > 1.0 || flow.y.abs() > 1.0 {
                        let flow_col = [1.00, 0.85, 0.20, 0.95];
                        let flow_dir = flow.normalize();
                        let arrow_len = (flow.length() * 0.5).clamp(24.0, 96.0);

                        // Draw 3 flow arrows across the sheet
                        for &offset_pct in &[-0.35, 0.0, 0.35] {
                            let p_start = center + Vec3::new(half_x * offset_pct, half_y * offset_pct, 4.0);
                            let p_end = p_start + flow_dir * arrow_len;
                            line_verts.push(LineVertex::new(p_start.to_array(), flow_col));
                            line_verts.push(LineVertex::new(p_end.to_array(), flow_col));

                            // Arrowhead
                            let side = Vec3::new(-flow_dir.y, flow_dir.x, 0.0) * 10.0;
                            let head_left = p_end - flow_dir * 12.0 + side;
                            let head_right = p_end - flow_dir * 12.0 - side;
                            line_verts.push(LineVertex::new(p_end.to_array(), flow_col));
                            line_verts.push(LineVertex::new(head_left.to_array(), flow_col));
                            line_verts.push(LineVertex::new(p_end.to_array(), flow_col));
                            line_verts.push(LineVertex::new(head_right.to_array(), flow_col));
                        }
                    }
                }
            }

            // Mover Encroachment, Door Sequencing & Crushing Trap Visualizer
            if state.show_mover_studio_dialog || state.mover_studio_settings.show_swept_bounds_viewport {
                let m_set = &state.mover_studio_settings;
                let cfg = &m_set.active_config;
                let mover_half = Vec3::new(32.0, 32.0, 48.0);

                // 1. Swept Bounds Box enclosing entire trajectory
                if m_set.show_swept_bounds_viewport {
                    let (b_min, b_max) = cfg.compute_swept_bounds(mover_half);
                    let swept_center = (b_min + b_max) * 0.5;
                    let swept_half = (b_max - b_min) * 0.5;
                    let swept_col = [0.20, 0.85, 0.95, 0.60];
                    add_wire_box(&mut line_verts, swept_center, swept_half, swept_col);
                }

                // 2. Keyframe Ghost Outlines with Floor Labels
                if m_set.show_keyframe_ghosts_viewport {
                    for (idx, k) in cfg.keyframes.iter().enumerate() {
                        let ghost_col = if idx == 0 {
                            [0.20, 1.00, 0.40, 0.80] // Green for Key 0 (Home/Closed)
                        } else if idx == cfg.keyframes.len() - 1 {
                            [1.00, 0.80, 0.20, 0.80] // Gold for Key N (Open)
                        } else {
                            [0.40, 0.70, 1.00, 0.70] // Cyan for Intermediate
                        };

                        add_wire_box(&mut line_verts, k.position, mover_half, ghost_col);
                        add_wire_circle(&mut line_verts, k.position, 16.0, ghost_col);
                    }
                }

                // 3. Animated 3D Mover Brush at Current Playhead
                if m_set.show_animated_brush_viewport {
                    let move_t = cfg.move_time.max(0.1);
                    let stay_t = cfg.stay_open_time.max(0.0);
                    let cur_t = m_set.playback_time;

                    let progress = if cur_t <= move_t {
                        cur_t / move_t
                    } else if cur_t <= move_t + stay_t {
                        1.0
                    } else {
                        1.0 - ((cur_t - move_t - stay_t) / move_t)
                    };

                    let (cur_pos, _) = cfg.evaluate_pose(progress);
                    let brush_col = [1.00, 0.92, 0.25, 0.95];
                    add_wire_box(&mut line_verts, cur_pos, mover_half * 1.02, brush_col);

                    // Center sphere indicator
                    add_wire_circle(&mut line_verts, cur_pos, 20.0, brush_col);
                    add_wire_circle(&mut line_verts, cur_pos + Vec3::new(0.0, 0.0, mover_half.z), 14.0, brush_col);
                }

                // 4. Pinch-Point & Crushing Hazard Highlights
                if m_set.show_pinch_hazards_viewport {
                    let hazard_col = [1.00, 0.20, 0.20, 0.95];
                    for h in &m_set.cached_hazards {
                        if h.severity == ut99_bsp::MoverHazardSeverity::CriticalLethalHazard {
                            add_wire_circle(&mut line_verts, h.location, 24.0, hazard_col);
                            add_wire_circle(&mut line_verts, h.location + Vec3::new(0.0, 0.0, 16.0), 16.0, hazard_col);
                            line_verts.push(LineVertex::new((h.location - Vec3::Z * 20.0).to_array(), hazard_col));
                            line_verts.push(LineVertex::new((h.location + Vec3::Z * 30.0).to_array(), hazard_col));
                        }
                    }

                    // Bot test position indicator
                    let bot_col = [0.95, 0.50, 0.20, 0.90];
                    let bp = m_set.bot_test_position;
                    add_wire_circle(&mut line_verts, bp, 17.0, bot_col);
                    add_wire_circle(&mut line_verts, bp + Vec3::new(0.0, 0.0, 39.0), 17.0, bot_col);
                    add_wire_circle(&mut line_verts, bp + Vec3::new(0.0, 0.0, 78.0), 17.0, bot_col);
                }

                // 5. Sensor Trigger Radius Sphere
                if m_set.show_trigger_radius_viewport {
                    let door_pos = cfg.keyframes.first().map(|k| k.position).unwrap_or(Vec3::ZERO);
                    let approach_dist = m_set.trigger_approach_distance;
                    let trig_pos = door_pos + Vec3::X * approach_dist;
                    let trig_col = [0.85, 0.40, 1.00, 0.85];

                    add_wire_circle(&mut line_verts, trig_pos, m_set.trigger_radius, trig_col);
                    add_wire_circle(&mut line_verts, trig_pos + Vec3::new(0.0, 0.0, 24.0), m_set.trigger_radius * 0.8, trig_col);
                    line_verts.push(LineVertex::new(door_pos.to_array(), trig_col));
                    line_verts.push(LineVertex::new(trig_pos.to_array(), trig_col));
                }
            }

            // Static Acoustic Emitter Attenuation Spheres (camera-independent)
            if state.acoustic_settings.enabled && state.acoustic_settings.show_radii_spheres {
                let sel_tag = state.selected_sound_emitter_tag.as_deref();
                let emitters = self
                    .overlay_cache
                    .sound_emitters(|| state.collect_sound_emitters());
                for emitter in emitters {
                    let radius = emitter.sound_radius_units;
                    if radius > 0.0 {
                        let is_sel = sel_tag == Some(emitter.actor_tag.as_str());
                        let center = emitter.location;
                        let outer_col = if is_sel { [1.0, 0.85, 0.2, 0.9] } else { [0.3, 0.8, 1.0, 0.6] };
                        let half_col = if is_sel { [1.0, 0.65, 0.1, 0.75] } else { [0.2, 0.6, 0.9, 0.45] };
                        let core_col = if is_sel { [0.2, 0.95, 0.4, 0.9] } else { [0.15, 0.8, 0.35, 0.6] };

                        add_wire_circle(&mut line_verts, center, radius, outer_col);
                        add_wire_circle(&mut line_verts, center, radius * 0.5, half_col);
                        add_wire_circle(&mut line_verts, center, radius * 0.25, core_col);
                    }
                }
            }

            studio_build_ms = studio_started.elapsed().as_secs_f64() * 1000.0;
            let studio_up_started = std::time::Instant::now();
            uploaded_studio_line_bytes = Self::upload_persistent_buffer(
                device,
                queue,
                "Persistent Studio Line Vertex Buffer",
                &mut self.studio_line_buffer,
                &mut self.studio_line_buffer_capacity,
                &line_verts,
            );
            studio_line_upload_ms = studio_up_started.elapsed().as_secs_f64() * 1000.0;
            self.cached_studio_line_count = line_verts.len() as u32;
            self.cached_studio_lines_key = Some(CachedStudioLinesKey::capture(state, actors_rev));
            self.frame_studio_lines = retain_vertex_storage(line_verts, self.max_retained_vertex_bytes);
        }

        let mut line_verts = std::mem::take(&mut self.frame_dynamic_lines);
        line_verts.clear();

        {
            let dyn_env_started = std::time::Instant::now();

            // SkyBox 3D Real-time Parallax Frustum & Zone Visualizer
            if state.sky_zone_settings.show_frustum {
                if let Some(sky_cam) = state.calculate_skybox_camera(&state.camera) {
                    let sky_pos = sky_cam.position;
                    let fwd = sky_cam.forward();
                    let right = sky_cam.right();
                    let up = sky_cam.up();

                    let dist = 320.0_f32;
                    let half_h = dist * (sky_cam.fov_y * 0.5).tan();
                    let half_w = half_h * sky_cam.aspect.max(0.1);

                    let c0 = sky_pos + fwd * dist - right * half_w - up * half_h;
                    let c1 = sky_pos + fwd * dist + right * half_w - up * half_h;
                    let c2 = sky_pos + fwd * dist + right * half_w + up * half_h;
                    let c3 = sky_pos + fwd * dist - right * half_w + up * half_h;

                    let frustum_color = [0.80, 0.45, 1.0, 0.92]; // Celestial Orchid
                    let ray_color = [0.65, 0.35, 0.95, 0.65];

                    // Frustum rays from sky camera anchor to far plane corners
                    line_verts.push(LineVertex::new(sky_pos.to_array(), ray_color));
                    line_verts.push(LineVertex::new(c0.to_array(), ray_color));
                    line_verts.push(LineVertex::new(sky_pos.to_array(), ray_color));
                    line_verts.push(LineVertex::new(c1.to_array(), ray_color));
                    line_verts.push(LineVertex::new(sky_pos.to_array(), ray_color));
                    line_verts.push(LineVertex::new(c2.to_array(), ray_color));
                    line_verts.push(LineVertex::new(sky_pos.to_array(), ray_color));
                    line_verts.push(LineVertex::new(c3.to_array(), ray_color));

                    // Far plane rectangle
                    line_verts.push(LineVertex::new(c0.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c1.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c1.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c2.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c2.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c3.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c3.to_array(), frustum_color));
                    line_verts.push(LineVertex::new(c0.to_array(), frustum_color));

                    // Center view target anchor
                    let center_tgt = sky_pos + fwd * dist;
                    line_verts.push(LineVertex::new((center_tgt - right * 12.0).to_array(), frustum_color));
                    line_verts.push(LineVertex::new((center_tgt + right * 12.0).to_array(), frustum_color));
                    line_verts.push(LineVertex::new((center_tgt - up * 12.0).to_array(), frustum_color));
                    line_verts.push(LineVertex::new((center_tgt + up * 12.0).to_array(), frustum_color));
                }
            }

            // 3D Acoustic Attenuation & Occlusion Profiler (listener rays depend on listener position)
            if state.acoustic_settings.enabled && state.acoustic_settings.show_listener_rays {
                let listener_pos = match state.acoustic_settings.listener_source {
                    crate::editor_state::AcousticListenerSource::CameraPosition => state.camera.position,
                    crate::editor_state::AcousticListenerSource::SelectedActor => {
                        state.loaded_unr.as_ref()
                            .and_then(|u| u.actors.iter().find(|a| a.class_name.eq_ignore_ascii_case("PlayerStart")))
                            .map(|a| a.location)
                            .unwrap_or(state.camera.position)
                    }
                };

                for &(center, is_in_range, is_occluded) in
                    self.overlay_cache.acoustic_ray_states(state, listener_pos)
                {
                    let (ray_col, is_dashed) = if !is_in_range {
                        ([0.4, 0.4, 0.45, 0.35], true)
                    } else if is_occluded {
                        ([0.95, 0.3, 0.2, 0.85], true)
                    } else {
                        ([0.2, 0.95, 0.4, 0.85], false)
                    };

                    if is_dashed {
                        let segs = 10;
                        for s in 0..segs {
                            if s % 2 == 0 {
                                let t0 = s as f32 / segs as f32;
                                let t1 = (s + 1) as f32 / segs as f32;
                                let p0 = center.lerp(listener_pos, t0);
                                let p1 = center.lerp(listener_pos, t1);
                                line_verts.push(LineVertex::new(p0.to_array(), ray_col));
                                line_verts.push(LineVertex::new(p1.to_array(), ray_col));
                            }
                        }
                    } else {
                        line_verts.push(LineVertex::new(center.to_array(), ray_col));
                        line_verts.push(LineVertex::new(listener_pos.to_array(), ray_col));
                    }
                }
            }

            // Volumetric Zone Fog Distance Attenuation Shells
            if state.distance_fog_enabled && state.zone_fog_settings.show_distance_shells {
                let center = match state.zone_fog_settings.shell_target_mode {
                    crate::editor_state::FogShellTargetMode::ActiveCamera => state.camera.position,
                    crate::editor_state::FogShellTargetMode::SelectedZone => {
                        let mut found_pos = state.camera.position;
                        if let Some(ref level) = state.loaded_unr {
                            if let Some(sel_idx) = state.zone_fog_settings.selected_zone_index.or(state.selected_actor_index) {
                                if let Some(actor) = level.actors.get(sel_idx) {
                                    found_pos = actor.location;
                                }
                            }
                        }
                        found_pos
                    }
                };

                let base_color = state.fog_color;
                let start = state.fog_start;
                let end = state.fog_end;
                let span = (end - start).max(1.0);

                // 4 concentric shells representing 25%, 50%, 75%, 100% fog obscurity
                let steps = [
                    (0.25, 0.25),
                    (0.50, 0.45),
                    (0.75, 0.70),
                    (1.00, 0.95),
                ];

                for (frac, alpha) in steps {
                    let r = start + span * frac;
                    let shell_color = [base_color[0], base_color[1], base_color[2], alpha];

                    // 3 Orthogonal circles (XY, XZ, YZ)
                    add_wire_circle(&mut line_verts, center, r, shell_color);

                    // Vertical XZ circle
                    let segs = 32;
                    for s in 0..segs {
                        let a0 = s as f32 / segs as f32 * std::f32::consts::TAU;
                        let a1 = (s + 1) as f32 / segs as f32 * std::f32::consts::TAU;
                        let p0 = center + glam::Vec3::new(a0.cos() * r, 0.0, a0.sin() * r);
                        let p1 = center + glam::Vec3::new(a1.cos() * r, 0.0, a1.sin() * r);
                        line_verts.push(LineVertex::new(p0.to_array(), shell_color));
                        line_verts.push(LineVertex::new(p1.to_array(), shell_color));
                    }

                    // Vertical YZ circle
                    for s in 0..segs {
                        let a0 = s as f32 / segs as f32 * std::f32::consts::TAU;
                        let a1 = (s + 1) as f32 / segs as f32 * std::f32::consts::TAU;
                        let p0 = center + glam::Vec3::new(0.0, a0.cos() * r, a0.sin() * r);
                        let p1 = center + glam::Vec3::new(0.0, a1.cos() * r, a1.sin() * r);
                        line_verts.push(LineVertex::new(p0.to_array(), shell_color));
                        line_verts.push(LineVertex::new(p1.to_array(), shell_color));
                    }
                }

                // Radial axis tick indicators
                let axis_color = [base_color[0] * 1.2, base_color[1] * 1.2, base_color[2] * 1.2, 0.8];
                line_verts.push(LineVertex::new((center - glam::Vec3::X * end).to_array(), axis_color));
                line_verts.push(LineVertex::new((center + glam::Vec3::X * end).to_array(), axis_color));
                line_verts.push(LineVertex::new((center - glam::Vec3::Y * end).to_array(), axis_color));
                line_verts.push(LineVertex::new((center + glam::Vec3::Y * end).to_array(), axis_color));
            }

            // Sun, Corona & Lens Flare camera LoS ray (only camera-dependent Corona primitive)
            if (state.show_corona_studio_dialog || state.corona_studio_settings.show_flares_viewport)
                && state.corona_studio_settings.show_occlusion_rays_viewport
            {
                let target_pos = if let Some(sel_idx) = state.selected_actor_index {
                    state
                        .loaded_unr
                        .as_ref()
                        .and_then(|u| u.actors.get(sel_idx).map(|a| a.location))
                        .unwrap_or(state.builder_brush.location)
                } else {
                    state.builder_brush.location
                };
                let los_col = [1.0, 0.9, 0.3, 0.5];
                line_verts.push(LineVertex::new(state.camera.position.to_array(), los_col));
                line_verts.push(LineVertex::new(target_pos.to_array(), los_col));
            }

            ovl_radii_portals_sky_ms += dyn_env_started.elapsed().as_secs_f64() * 1000.0;
        }

        let ovl_studios_started = std::time::Instant::now();

        // Dynamic Particle FX & Volumetric Emitter Simulator
        if state.show_particles {
            let emitter_beacon_color = [1.00, 0.55, 0.15, 0.90]; // Amber
            let sel_beacon_color = [0.20, 0.85, 1.00, 1.00];     // Electric Cyan
            let collision_plane_color = [0.20, 0.60, 0.90, 0.60]; // Floor collision plane

            for (idx, emitter) in state.particle_emitters.iter().enumerate() {
                let is_selected = state.particle_settings.selected_emitter_index == idx;
                let b_col = if is_selected { sel_beacon_color } else { emitter_beacon_color };

                if state.particle_settings.show_emitter_bounds {
                    add_wire_box(&mut line_verts, emitter.location, glam::Vec3::splat(14.0), b_col);
                    add_wire_circle(&mut line_verts, emitter.location, 16.0, b_col);
                }

                // Render elastic floor bounce plane helper
                if state.particle_settings.show_collision_plane {
                    if let Some(floor_z) = emitter.floor_z {
                        let center = glam::Vec3::new(emitter.location.x, emitter.location.y, floor_z);
                        add_wire_circle(&mut line_verts, center, 48.0, collision_plane_color);
                        line_verts.push(LineVertex::new((center - glam::Vec3::new(48.0, 0.0, 0.0)).to_array(), collision_plane_color));
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(48.0, 0.0, 0.0)).to_array(), collision_plane_color));
                        line_verts.push(LineVertex::new((center - glam::Vec3::new(0.0, 48.0, 0.0)).to_array(), collision_plane_color));
                        line_verts.push(LineVertex::new((center + glam::Vec3::new(0.0, 48.0, 0.0)).to_array(), collision_plane_color));
                    }
                }

                for p in &emitter.particles {
                    let half_s = (p.size * 0.5).max(1.0);
                    // 3D 3-axis volumetric cross marker for particle
                    line_verts.push(LineVertex::new((p.position - glam::Vec3::new(half_s, 0.0, 0.0)).to_array(), p.color));
                    line_verts.push(LineVertex::new((p.position + glam::Vec3::new(half_s, 0.0, 0.0)).to_array(), p.color));
                    line_verts.push(LineVertex::new((p.position - glam::Vec3::new(0.0, half_s, 0.0)).to_array(), p.color));
                    line_verts.push(LineVertex::new((p.position + glam::Vec3::new(0.0, half_s, 0.0)).to_array(), p.color));
                    line_verts.push(LineVertex::new((p.position - glam::Vec3::new(0.0, 0.0, half_s)).to_array(), p.color));
                    line_verts.push(LineVertex::new((p.position + glam::Vec3::new(0.0, 0.0, half_s)).to_array(), p.color));
                }
            }
        }

        // Bot Combat Sightlines & Vantage Perches
        if state.show_combat_sightlines {
            if let Some(ref report) = state.combat_coverage_report {
                for line in &report.sightlines {
                    let col = match line.recommended_weapon {
                        "SniperRifle" => [1.00, 0.15, 0.15, 0.85],   // Laser Red
                        "ShockRifle" => [0.15, 0.85, 1.00, 0.75],    // Electric Cyan
                        "RocketLauncher" => [1.00, 0.65, 0.15, 0.75],// Amber
                        _ => [0.35, 0.95, 0.25, 0.70],               // Flak Green
                    };
                    line_verts.push(LineVertex::new(line.from_pos.to_array(), col));
                    line_verts.push(LineVertex::new(line.to_pos.to_array(), col));
                }
                for vp in &report.vantage_points {
                    if vp.elevation_advantage_uu >= 120.0 {
                        let perch_col = [1.00, 0.85, 0.20, 0.95]; // Gold
                        add_wire_circle(&mut line_verts, vp.location, 28.0, perch_col);
                        add_octahedron(&mut line_verts, vp.location + glam::Vec3::new(0.0, 0.0, 48.0), 12.0, perch_col);
                    }
                }
            }
        }

        // Procedural Architectural Prefab Live 3D Preview
        if state.show_architectural_studio_dialog && state.architectural_settings.show_preview {
            let preview_polys = state.get_prefab_preview_polygons();
            let preview_col = [1.00, 0.72, 0.18, 0.95]; // Warm Gold / Amber

            for poly in &preview_polys {
                let n = poly.vertices.len();
                if n >= 3 {
                    for i in 0..n {
                        let v0 = poly.vertices[i];
                        let v1 = poly.vertices[(i + 1) % n];
                        line_verts.push(LineVertex::new(v0.to_array(), preview_col));
                        line_verts.push(LineVertex::new(v1.to_array(), preview_col));
                    }
                }
            }
        }

        // Terrain & Heightmap Mesh Sculptor Live 3D Preview
        if state.show_terrain_studio_dialog && state.terrain_settings.show_preview {
            let preview_polys = state.get_terrain_preview_polygons();
            let terrain_col = [0.22, 0.88, 0.55, 0.90]; // Emerald Green

            for poly in &preview_polys {
                let n = poly.vertices.len();
                if n >= 3 {
                    for i in 0..n {
                        let v0 = poly.vertices[i];
                        let v1 = poly.vertices[(i + 1) % n];
                        line_verts.push(LineVertex::new(v0.to_array(), terrain_col));
                        line_verts.push(LineVertex::new(v1.to_array(), terrain_col));
                    }
                }
            }

            // Sculpting Brush Radius Helper Circle
            let center_3d = glam::Vec3::new(
                state.terrain_settings.brush_center.x + state.builder_brush.location.x,
                state.terrain_settings.brush_center.y + state.builder_brush.location.y,
                state.builder_brush.location.z + state.terrain_settings.grid.base_z + 16.0,
            );
            let brush_col = [1.00, 0.35, 0.35, 0.95]; // Sculpting Red
            add_wire_circle(&mut line_verts, center_3d, state.terrain_settings.brush_radius, brush_col);
        }

        // Static Mesh LOD & Convex Collision Hull Studio Live Preview
        if state.show_lod_collision_dialog && state.lod_collision_settings.show_preview_hull {
            if let Some(hull_brush) = state.generate_collision_hull_for_selected_brush(
                state.lod_collision_settings.hull_type,
                state.lod_collision_settings.hull_margin,
            ) {
                let hull_col = [0.15, 0.85, 1.00, 0.95]; // Bright Cyan
                for poly in &hull_brush.polys {
                    let n = poly.vertices.len();
                    if n >= 3 {
                        for i in 0..n {
                            let v0 = poly.vertices[i] + hull_brush.location;
                            let v1 = poly.vertices[(i + 1) % n] + hull_brush.location;
                            line_verts.push(LineVertex::new(v0.to_array(), hull_col));
                            line_verts.push(LineVertex::new(v1.to_array(), hull_col));
                        }
                    }
                }
            }
        }

        // Team Base Objective, CTF Flag Run Route & Defense AI Studio
        if state.show_team_objective_dialog {
            let t_set = &state.team_objective_settings;

            // 1. Holographic Flag Banners & Objective Pedestals
            if t_set.show_flag_holograms_viewport {
                for obj in &t_set.detected_objectives {
                    let (banner_col, pole_col) = match obj.team {
                        Some(ut99_bsp::TeamColor::Red) => ([1.0, 0.25, 0.25, 0.95], [1.0, 0.5, 0.5, 0.8]),
                        Some(ut99_bsp::TeamColor::Blue) => ([0.25, 0.55, 1.0, 0.95], [0.5, 0.75, 1.0, 0.8]),
                        Some(ut99_bsp::TeamColor::Green) => ([0.25, 0.9, 0.35, 0.95], [0.5, 1.0, 0.6, 0.8]),
                        Some(ut99_bsp::TeamColor::Gold) => ([1.0, 0.85, 0.2, 0.95], [1.0, 0.95, 0.5, 0.8]),
                        None => ([0.8, 0.35, 1.0, 0.95], [0.9, 0.6, 1.0, 0.8]),
                    };

                    let base = obj.location;
                    let pole_top = base + Vec3::new(0.0, 0.0, 120.0);
                    // Base pedestal ring
                    add_wire_circle(&mut line_verts, base, obj.collision_radius.max(32.0), banner_col);
                    add_wire_circle(&mut line_verts, base + Vec3::new(0.0, 0.0, 16.0), obj.collision_radius.max(32.0) * 0.8, banner_col);

                    // Flag pole
                    line_verts.push(LineVertex::new(base.to_array(), pole_col));
                    line_verts.push(LineVertex::new(pole_top.to_array(), pole_col));

                    // Holographic banner cloth quad
                    let banner_w = 40.0;
                    let banner_h = 28.0;
                    let b0 = pole_top;
                    let b1 = pole_top + Vec3::new(banner_w, 0.0, 0.0);
                    let b2 = pole_top + Vec3::new(banner_w, 0.0, -banner_h);
                    let b3 = pole_top + Vec3::new(0.0, 0.0, -banner_h);

                    line_verts.push(LineVertex::new(b0.to_array(), banner_col)); line_verts.push(LineVertex::new(b1.to_array(), banner_col));
                    line_verts.push(LineVertex::new(b1.to_array(), banner_col)); line_verts.push(LineVertex::new(b2.to_array(), banner_col));
                    line_verts.push(LineVertex::new(b2.to_array(), banner_col)); line_verts.push(LineVertex::new(b3.to_array(), banner_col));
                    line_verts.push(LineVertex::new(b3.to_array(), banner_col)); line_verts.push(LineVertex::new(b0.to_array(), banner_col));
                    line_verts.push(LineVertex::new(b0.to_array(), banner_col)); line_verts.push(LineVertex::new(b2.to_array(), banner_col));
                }
            }

            // 2. 3D Route Ribbons & Waypoints
            if t_set.show_route_ribbons_viewport {
                for route in &t_set.routes {
                    let route_col = if route.is_primary {
                        [1.00, 0.82, 0.20, 0.95] // Golden primary
                    } else {
                        [0.40, 0.75, 1.00, 0.85] // Cyan/azure flank
                    };

                    if route.waypoints.len() >= 2 {
                        for i in 0..route.waypoints.len() - 1 {
                            let w1 = route.waypoints[i] + Vec3::new(0.0, 0.0, 16.0);
                            let w2 = route.waypoints[i + 1] + Vec3::new(0.0, 0.0, 16.0);
                            line_verts.push(LineVertex::new(w1.to_array(), route_col));
                            line_verts.push(LineVertex::new(w2.to_array(), route_col));
                            add_octahedron(&mut line_verts, w1, 8.0, route_col);
                        }
                        if let Some(&last) = route.waypoints.last() {
                            add_octahedron(&mut line_verts, last + Vec3::new(0.0, 0.0, 16.0), 8.0, route_col);
                        }
                    }
                }
            }

            // 3. Defense Vision Cones & Line-of-Sight Rays
            if t_set.show_defense_cones_viewport {
                for dp in &t_set.detected_defense_points {
                    let dp_col = match dp.team {
                        ut99_bsp::TeamColor::Red => [1.00, 0.30, 0.30, 0.90],
                        ut99_bsp::TeamColor::Blue => [0.30, 0.60, 1.00, 0.90],
                        ut99_bsp::TeamColor::Green => [0.30, 0.95, 0.40, 0.90],
                        ut99_bsp::TeamColor::Gold => [1.00, 0.85, 0.25, 0.90],
                    };

                    let eye_pos = dp.location + Vec3::new(0.0, 0.0, 24.0);
                    let yaw_rad = dp.rotation_yaw_deg.to_radians();
                    let forward = Vec3::new(yaw_rad.cos(), yaw_rad.sin(), 0.0);
                    let cone_len = if dp.is_sniping { 180.0 } else { 100.0 };
                    let half_fov_rad = (dp.fov_degrees * 0.5).to_radians();

                    let left_dir = Vec3::new((yaw_rad - half_fov_rad).cos(), (yaw_rad - half_fov_rad).sin(), 0.0);
                    let right_dir = Vec3::new((yaw_rad + half_fov_rad).cos(), (yaw_rad + half_fov_rad).sin(), 0.0);

                    let c_center = eye_pos + forward * cone_len;
                    let c_left = eye_pos + left_dir * cone_len;
                    let c_right = eye_pos + right_dir * cone_len;

                    line_verts.push(LineVertex::new(eye_pos.to_array(), dp_col)); line_verts.push(LineVertex::new(c_center.to_array(), dp_col));
                    line_verts.push(LineVertex::new(eye_pos.to_array(), dp_col)); line_verts.push(LineVertex::new(c_left.to_array(), dp_col));
                    line_verts.push(LineVertex::new(eye_pos.to_array(), dp_col)); line_verts.push(LineVertex::new(c_right.to_array(), dp_col));
                    line_verts.push(LineVertex::new(c_left.to_array(), dp_col)); line_verts.push(LineVertex::new(c_center.to_array(), dp_col));
                    line_verts.push(LineVertex::new(c_center.to_array(), dp_col)); line_verts.push(LineVertex::new(c_right.to_array(), dp_col));

                    // Octahedron marker at defense position
                    add_octahedron(&mut line_verts, dp.location, 12.0, dp_col);
                }
            }

            // 4. Chokepoint Warning Rings
            if t_set.show_chokepoints_viewport {
                let cp_col = [1.00, 0.45, 0.15, 0.95]; // amber/orange
                for route in &t_set.routes {
                    for &cp in &route.chokepoint_locations {
                        add_wire_circle(&mut line_verts, cp + Vec3::new(0.0, 0.0, 8.0), 32.0, cp_col);
                        add_wire_circle(&mut line_verts, cp + Vec3::new(0.0, 0.0, 24.0), 20.0, cp_col);
                        line_verts.push(LineVertex::new(cp.to_array(), cp_col));
                        line_verts.push(LineVertex::new((cp + Vec3::new(0.0, 0.0, 48.0)).to_array(), cp_col));
                        add_octahedron(&mut line_verts, cp + Vec3::new(0.0, 0.0, 48.0), 8.0, cp_col);
                    }
                }
            }
        }

        // Destructible Architecture, Breakable Glass & Fracture Studio
        if state.show_destructible_studio_dialog {
            let d_set = &state.destructible_studio_settings;
            let cfg = &d_set.active_config;
            let sim = &d_set.simulation;

            let base_col = match cfg.archetype {
                ut99_bsp::DestructibleArchetype::BreakableWindowGlass | ut99_bsp::DestructibleArchetype::FragileSkylightDome => {
                    [0.25, 0.85, 1.0, 0.9] // ice blue
                }
                ut99_bsp::DestructibleArchetype::ExplodingStoneWall | ut99_bsp::DestructibleArchetype::VentGratingShatter => {
                    [1.0, 0.70, 0.25, 0.9] // amber / stone
                }
                ut99_bsp::DestructibleArchetype::ReinforcedBlastDoor => {
                    [1.0, 0.30, 0.30, 0.95] // blast crimson
                }
            };

            let norm = if cfg.normal.length_squared() > 0.001 {
                cfg.normal.normalize()
            } else {
                Vec3::new(0.0, 1.0, 0.0)
            };

            let up_ref = if norm.dot(Vec3::new(0.0, 0.0, 1.0)).abs() > 0.95 {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                Vec3::new(0.0, 0.0, 1.0)
            };

            let right = norm.cross(up_ref).normalize();
            let ortho_up = right.cross(norm).normalize();

            let hw = cfg.size.x * 0.5;
            let hh = cfg.size.y * 0.5;
            let ht = 8.0_f32;

            // 1. Intact Boundary Slab & Projected Fracture Lines
            if !sim.is_shattered {
                // Front and back face corners
                let f0 = cfg.center + right * -hw + ortho_up * -hh + norm * ht;
                let f1 = cfg.center + right * hw + ortho_up * -hh + norm * ht;
                let f2 = cfg.center + right * hw + ortho_up * hh + norm * ht;
                let f3 = cfg.center + right * -hw + ortho_up * hh + norm * ht;

                let b0 = cfg.center + right * -hw + ortho_up * -hh - norm * ht;
                let b1 = cfg.center + right * hw + ortho_up * -hh - norm * ht;
                let b2 = cfg.center + right * hw + ortho_up * hh - norm * ht;
                let b3 = cfg.center + right * -hw + ortho_up * hh - norm * ht;

                // Front quad
                line_verts.push(LineVertex::new(f0.to_array(), base_col)); line_verts.push(LineVertex::new(f1.to_array(), base_col));
                line_verts.push(LineVertex::new(f1.to_array(), base_col)); line_verts.push(LineVertex::new(f2.to_array(), base_col));
                line_verts.push(LineVertex::new(f2.to_array(), base_col)); line_verts.push(LineVertex::new(f3.to_array(), base_col));
                line_verts.push(LineVertex::new(f3.to_array(), base_col)); line_verts.push(LineVertex::new(f0.to_array(), base_col));

                // Back quad
                let back_col = [base_col[0] * 0.7, base_col[1] * 0.7, base_col[2] * 0.7, base_col[3] * 0.6];
                line_verts.push(LineVertex::new(b0.to_array(), back_col)); line_verts.push(LineVertex::new(b1.to_array(), back_col));
                line_verts.push(LineVertex::new(b1.to_array(), back_col)); line_verts.push(LineVertex::new(b2.to_array(), back_col));
                line_verts.push(LineVertex::new(b2.to_array(), back_col)); line_verts.push(LineVertex::new(b3.to_array(), back_col));
                line_verts.push(LineVertex::new(b3.to_array(), back_col)); line_verts.push(LineVertex::new(b0.to_array(), back_col));

                // Side connecting struts
                line_verts.push(LineVertex::new(f0.to_array(), back_col)); line_verts.push(LineVertex::new(b0.to_array(), back_col));
                line_verts.push(LineVertex::new(f1.to_array(), back_col)); line_verts.push(LineVertex::new(b1.to_array(), back_col));
                line_verts.push(LineVertex::new(f2.to_array(), back_col)); line_verts.push(LineVertex::new(b2.to_array(), back_col));
                line_verts.push(LineVertex::new(f3.to_array(), back_col)); line_verts.push(LineVertex::new(b3.to_array(), back_col));

                // Projected 2D Fracture Shards on Front Surface
                if d_set.show_fracture_pattern_viewport {
                    let shard_line_col = [1.0, 1.0, 1.0, 0.7];
                    for shard in &d_set.cached_shards {
                        if shard.local_vertices.len() >= 2 {
                            for i in 0..shard.local_vertices.len() {
                                let v1 = shard.local_vertices[i];
                                let v2 = shard.local_vertices[(i + 1) % shard.local_vertices.len()];
                                let p1 = cfg.center + right * v1.x + ortho_up * v1.y + norm * ht;
                                let p2 = cfg.center + right * v2.x + ortho_up * v2.y + norm * ht;
                                line_verts.push(LineVertex::new(p1.to_array(), shard_line_col));
                                line_verts.push(LineVertex::new(p2.to_array(), shard_line_col));
                            }
                        }
                    }
                }
            }

            // 2. Blast Force Direction Vector & Cone
            if d_set.show_debris_cone_viewport {
                let blast_col = [1.0, 0.45, 0.1, 0.95]; // fiery orange
                let blast_vec = norm;
                let arrow_len = (cfg.blast_force * 0.25).clamp(40.0, 180.0);
                let tip = cfg.center + blast_vec * arrow_len;
                let mid = cfg.center + blast_vec * (arrow_len * 0.7);

                line_verts.push(LineVertex::new(cfg.center.to_array(), blast_col));
                line_verts.push(LineVertex::new(tip.to_array(), blast_col));

                // Cone rings around mid
                add_wire_circle(&mut line_verts, mid, arrow_len * 0.2, blast_col);
                add_octahedron(&mut line_verts, tip, 8.0, blast_col);
            }

            // 3. Dynamic Debris Flying Particles & Velocity Motion Trails
            if sim.is_shattered && d_set.show_debris_cone_viewport {
                // Ground collision plane ring
                let floor_col = [0.4, 0.4, 0.5, 0.5];
                let blast_floor_center = Vec3::new(cfg.center.x, cfg.center.y, sim.floor_z);
                add_wire_circle(&mut line_verts, blast_floor_center, (cfg.size.x + cfg.size.y) * 0.6, floor_col);

                for p in &sim.particles {
                    let p_col = if p.is_at_rest {
                        [0.55, 0.55, 0.60, 0.75]
                    } else {
                        [1.00, 0.85, 0.25, 0.95] // glowing flying fragment
                    };

                    let p_size = (p.scale * 4.0).clamp(3.0, 14.0);
                    add_octahedron(&mut line_verts, p.position, p_size, p_col);

                    // Velocity motion trail
                    if !p.is_at_rest && p.velocity.length_squared() > 10.0 {
                        let trail_start = p.position - p.velocity * 0.04;
                        line_verts.push(LineVertex::new(trail_start.to_array(), [p_col[0], p_col[1], p_col[2], 0.3]));
                        line_verts.push(LineVertex::new(p.position.to_array(), p_col));
                    }
                }
            }
        }

        // Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio
        if state.show_kicker_studio_dialog {
            let k_set = &state.kicker_studio_settings;
            let cfg = &k_set.active_config;
            let analysis = &k_set.trajectory_analysis;

            // 1. Launch Pad Volume & Kick Velocity Vector Arrow
            let pad_col = [0.25, 0.90, 0.40, 0.95]; // emerald green
            add_wire_box(&mut line_verts, cfg.pad_location, Vec3::new(32.0, 32.0, 8.0), pad_col);
            add_wire_circle(&mut line_verts, cfg.pad_location + Vec3::new(0.0, 0.0, 8.0), 28.0, pad_col);

            let arrow_len = (cfg.kick_velocity.length() * 0.15).clamp(40.0, 200.0);
            let vel_dir = cfg.kick_velocity.normalize_or_zero();
            let vel_tip = cfg.pad_location + Vec3::new(0.0, 0.0, 16.0) + vel_dir * arrow_len;
            line_verts.push(LineVertex::new((cfg.pad_location + Vec3::new(0.0, 0.0, 16.0)).to_array(), pad_col));
            line_verts.push(LineVertex::new(vel_tip.to_array(), pad_col));
            add_octahedron(&mut line_verts, vel_tip, 8.0, pad_col);

            // 2. Parabolic Trajectory Ribbon & Apex Marker
            if k_set.show_trajectory_ribbon_viewport && analysis.samples.len() >= 2 {
                for i in 0..analysis.samples.len() - 1 {
                    let s1 = &analysis.samples[i];
                    let s2 = &analysis.samples[i + 1];

                    let arc_factor = (s1.time_secs / analysis.total_flight_time_secs).clamp(0.0, 1.0);
                    let seg_col = if arc_factor < 0.5 {
                        [0.2 + arc_factor * 1.6, 0.85 + arc_factor * 0.15, 0.3, 0.95] // emerald -> gold
                    } else {
                        [1.0 - (arc_factor - 0.5) * 1.6, 0.95 - (arc_factor - 0.5) * 0.35, 0.3 + (arc_factor - 0.5) * 1.4, 0.95] // gold -> cyan
                    };

                    line_verts.push(LineVertex::new(s1.position.to_array(), seg_col));
                    line_verts.push(LineVertex::new(s2.position.to_array(), seg_col));
                }

                // Golden Apex Octahedron Marker & Vertical Height Stalk
                let apex_col = [1.0, 0.85, 0.1, 0.95];
                add_octahedron(&mut line_verts, analysis.apex_position, 12.0, apex_col);
                let apex_ground = Vec3::new(analysis.apex_position.x, analysis.apex_position.y, cfg.pad_location.z);
                line_verts.push(LineVertex::new(analysis.apex_position.to_array(), [1.0, 0.85, 0.1, 0.4]));
                line_verts.push(LineVertex::new(apex_ground.to_array(), [1.0, 0.85, 0.1, 0.4]));
                add_wire_circle(&mut line_verts, apex_ground, 24.0, [1.0, 0.85, 0.1, 0.5]);
            }

            // 3. Target Platform & Landing Rings
            if k_set.show_landing_target_viewport {
                let target_col = [0.25, 0.65, 1.0, 0.95]; // cyan/azure
                add_wire_box(&mut line_verts, cfg.target_location, Vec3::new(32.0, 32.0, 8.0), target_col);
                add_wire_circle(&mut line_verts, cfg.target_location + Vec3::new(0.0, 0.0, 8.0), 32.0, target_col);
                add_wire_circle(&mut line_verts, cfg.target_location + Vec3::new(0.0, 0.0, 8.0), 16.0, target_col);
                add_octahedron(&mut line_verts, cfg.target_location + Vec3::new(0.0, 0.0, 32.0), 10.0, target_col);

                // Actual landing position vs target discrepancy line
                let miss_col = if analysis.target_miss_distance_uu < 50.0 {
                    [0.2, 0.9, 0.3, 0.8]
                } else {
                    [1.0, 0.4, 0.2, 0.9]
                };
                line_verts.push(LineVertex::new(analysis.landing_position.to_array(), miss_col));
                line_verts.push(LineVertex::new(cfg.target_location.to_array(), miss_col));
                add_wire_circle(&mut line_verts, analysis.landing_position, 20.0, miss_col);
            }

            // 4. Ceiling Clearance Boundary Plane & Impact Warning
            if k_set.show_ceiling_clearance_viewport {
                let ceil_z = cfg.ceiling_height_z;
                let ceil_center = Vec3::new(analysis.apex_position.x, analysis.apex_position.y, ceil_z);
                let ceil_col = if analysis.hit_ceiling {
                    [1.0, 0.2, 0.2, 0.9]
                } else {
                    [0.7, 0.4, 0.4, 0.5]
                };
                add_wire_circle(&mut line_verts, ceil_center, 120.0, ceil_col);

                if analysis.hit_ceiling {
                    if let Some(c_pt) = analysis.ceiling_impact_point {
                        add_octahedron(&mut line_verts, c_pt, 16.0, [1.0, 0.15, 0.15, 0.95]);
                    }
                }
            }

            // 5. Animated Flying Player Avatar
            let scrub_t = k_set.flight_scrubber_secs;
            if !analysis.samples.is_empty() {
                let mut avatar_pos = analysis.samples[0].position;
                for i in 0..analysis.samples.len() - 1 {
                    if scrub_t >= analysis.samples[i].time_secs && scrub_t <= analysis.samples[i + 1].time_secs {
                        let seg_dt = analysis.samples[i + 1].time_secs - analysis.samples[i].time_secs;
                        let alpha = if seg_dt > 0.0001 { (scrub_t - analysis.samples[i].time_secs) / seg_dt } else { 0.0 };
                        avatar_pos = analysis.samples[i].position.lerp(analysis.samples[i + 1].position, alpha);
                        break;
                    }
                }
                add_wire_box(&mut line_verts, avatar_pos, Vec3::new(17.0, 17.0, 39.0), [1.0, 1.0, 1.0, 0.95]);
            }
        }

        // BSP Tree Quality, Cut Minimizer & Sliver Polygon Purging Studio
        if state.show_bsp_quality_studio_dialog {
            let bsp_set = &state.bsp_quality_studio_settings;

            // 1. Sliver polygon markers (Red pulsating octahedrons & polygon wireframes)
            if bsp_set.show_sliver_markers_viewport {
                let sliv_col = [1.0, 0.2, 0.2, 0.95]; // glowing red
                for sliv in &bsp_set.detected_slivers {
                    add_octahedron(&mut line_verts, sliv.centroid, 8.0, sliv_col);
                    // Draw micro-polygon outline
                    if sliv.vertices.len() >= 2 {
                        for i in 0..sliv.vertices.len() {
                            let next_i = (i + 1) % sliv.vertices.len();
                            line_verts.push(LineVertex::new(sliv.vertices[i].to_array(), sliv_col));
                            line_verts.push(LineVertex::new(sliv.vertices[next_i].to_array(), sliv_col));
                        }
                    }
                }
            }

            // 2. Coplanar overlap markers (Magenta warning circles & line stems)
            if bsp_set.show_coplanar_markers_viewport {
                let cop_col = [1.0, 0.3, 0.9, 0.95]; // vibrant magenta
                for cop in &bsp_set.detected_coplanar {
                    add_wire_circle(&mut line_verts, cop.approximate_center, 24.0, cop_col);
                    add_octahedron(&mut line_verts, cop.approximate_center, 6.0, cop_col);
                    // Normal vector stalk
                    let norm_tip = cop.approximate_center + cop.plane.normal * 32.0;
                    line_verts.push(LineVertex::new(cop.approximate_center.to_array(), cop_col));
                    line_verts.push(LineVertex::new(norm_tip.to_array(), cop_col));
                }
            }

            // 3. Cut offender brush bounding box highlights (Amber warning boxes)
            if bsp_set.show_cut_heatmap_viewport {
                let offender_col = [1.0, 0.65, 0.1, 0.85]; // amber
                for offender in &bsp_set.cut_offenders {
                    if offender.estimated_cuts_induced > 0 {
                        if let Some((b_min, b_max)) = state.world_brush_bound(offender.brush_index) {
                            let b_center = (b_min + b_max) * 0.5;
                            let b_extent = (b_max - b_min) * 0.5;
                            add_wire_box(&mut line_verts, b_center, b_extent, offender_col);
                        }
                    }
                }
            }
        }

        // 2D Shape Loft, Curved Pipe & Architectural Molding Studio
        if state.show_loft_studio_dialog {
            let loft_set = &state.loft_studio_settings;

            // 1. Spline Path Centerline & Control Stations
            let path_col = [0.2, 0.85, 0.4, 0.95]; // emerald green
            if loft_set.cached_frames.len() >= 2 {
                for i in 0..(loft_set.cached_frames.len() - 1) {
                    let p0 = loft_set.cached_frames[i].position;
                    let p1 = loft_set.cached_frames[i + 1].position;
                    line_verts.push(LineVertex::new(p0.to_array(), path_col));
                    line_verts.push(LineVertex::new(p1.to_array(), path_col));
                }
            }

            for (idx, kf) in loft_set.path_keyframes.iter().enumerate() {
                let kf_col = if loft_set.selected_keyframe_idx == Some(idx) {
                    [1.0, 0.85, 0.1, 0.95] // gold for selected
                } else {
                    [0.9, 0.25, 0.2, 0.95] // red
                };
                add_octahedron(&mut line_verts, kf.position, 10.0, kf_col);
            }

            // 2. RMF Tangent, Normal & Binormal Coordinate Frames
            if loft_set.show_tangent_frames_viewport {
                for frame in &loft_set.cached_frames {
                    let origin = frame.position;
                    let t_tip = origin + frame.tangent * 24.0;
                    let n_tip = origin + frame.normal * 24.0;
                    let b_tip = origin + frame.binormal * 24.0;

                    // Tangent (Blue), Normal (Red), Binormal (Green)
                    line_verts.push(LineVertex::new(origin.to_array(), [0.2, 0.5, 1.0, 0.9]));
                    line_verts.push(LineVertex::new(t_tip.to_array(), [0.2, 0.5, 1.0, 0.9]));

                    line_verts.push(LineVertex::new(origin.to_array(), [1.0, 0.2, 0.2, 0.9]));
                    line_verts.push(LineVertex::new(n_tip.to_array(), [1.0, 0.2, 0.2, 0.9]));

                    line_verts.push(LineVertex::new(origin.to_array(), [0.2, 0.9, 0.3, 0.9]));
                    line_verts.push(LineVertex::new(b_tip.to_array(), [0.2, 0.9, 0.3, 0.9]));
                }
            }

            // 3. Holographic Cross-Section Rings
            if loft_set.show_profile_cross_sections_viewport {
                let ring_col = [0.3, 0.75, 1.0, 0.85]; // cyan
                let pts = &loft_set.custom_profile.points;
                if pts.len() >= 3 {
                    for frame in &loft_set.cached_frames {
                        let n_pts = pts.len();
                        for i in 0..n_pts {
                            let next_i = (i + 1) % n_pts;
                            let p0 = frame.transform_point(pts[i]);
                            let p1 = frame.transform_point(pts[next_i]);
                            line_verts.push(LineVertex::new(p0.to_array(), ring_col));
                            line_verts.push(LineVertex::new(p1.to_array(), ring_col));
                        }
                    }
                }
            }

            // 4. 3D Mesh Ghost Wireframe
            if loft_set.show_wireframe_ghost_viewport {
                if let Some(ref brush) = loft_set.cached_preview_brush {
                    let ghost_col = [1.0, 0.9, 0.2, 0.4]; // translucent yellow
                    for poly in &brush.polys {
                        let n = poly.vertices.len();
                        for i in 0..n {
                            let next = (i + 1) % n;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), ghost_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), ghost_col));
                        }
                    }
                }
            }

            // 5. Curvature & Pinch Hazard Spheres
            for h in &loft_set.cached_hazards {
                let haz_col = if h.is_critical { [1.0, 0.15, 0.15, 0.95] } else { [1.0, 0.85, 0.1, 0.9] };
                add_wire_circle(&mut line_verts, h.position, 28.0, haz_col);
                add_octahedron(&mut line_verts, h.position, 14.0, haz_col);
            }
        }

        // Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio
        if state.show_stairway_studio_dialog {
            let stair_set = &state.stairway_studio_settings;

            // 1. Stair Steps Ghost Wireframe (Amber Gold)
            if stair_set.show_stair_ghost_viewport {
                if let Some(ref brush) = stair_set.cached_stair_brush {
                    let stair_col = [1.0, 0.78, 0.20, 0.85];
                    for poly in &brush.polys {
                        let n = poly.vertices.len();
                        for i in 0..n {
                            let next = (i + 1) % n;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), stair_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), stair_col));
                        }
                    }
                }
            }

            // 2. Smooth Collision Wedge Ramp (Cyan / Holographic Turquoise)
            if stair_set.show_collision_ramp_ghost_viewport {
                if let Some(ref brush) = stair_set.cached_ramp_brush {
                    let ramp_col = [0.20, 0.90, 1.00, 0.90];
                    for poly in &brush.polys {
                        let n = poly.vertices.len();
                        for i in 0..n {
                            let next = (i + 1) % n;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), ramp_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), ramp_col));
                        }
                    }
                }
            }

            // 3. Handrail Banister (Warm Orange)
            if stair_set.show_handrail_ghost_viewport {
                if let Some(ref brush) = stair_set.cached_handrail_brush {
                    let rail_col = [1.0, 0.50, 0.15, 0.85];
                    for poly in &brush.polys {
                        let n = poly.vertices.len();
                        for i in 0..n {
                            let next = (i + 1) % n;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), rail_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), rail_col));
                        }
                    }
                }
            }

            // 4. Bot Navigation Centerline Path & Waypoint Nodes
            if stair_set.show_bot_waypoints_viewport {
                let wp_count = stair_set.cached_waypoints.len();
                if wp_count >= 2 {
                    let path_col = [0.20, 0.90, 0.40, 0.85]; // Emerald
                    for i in 0..(wp_count - 1) {
                        let p0 = stair_set.cached_waypoints[i].position;
                        let p1 = stair_set.cached_waypoints[i + 1].position;
                        line_verts.push(LineVertex::new(p0.to_array(), path_col));
                        line_verts.push(LineVertex::new(p1.to_array(), path_col));
                    }
                }

                for wp in &stair_set.cached_waypoints {
                    let node_col = if wp.reachspec_valid {
                        [0.20, 0.90, 0.40, 0.95] // Emerald green
                    } else {
                        [1.00, 0.20, 0.20, 0.95] // Red
                    };
                    add_octahedron(&mut line_verts, wp.position, 8.0, node_col);
                    add_wire_circle(&mut line_verts, wp.position, 16.0, node_col);
                }
            }

            // 5. Critical MaxStepHeight Warning Overlays
            let has_critical = stair_set.cached_hazards.iter().any(|h| h.severity == ut99_bsp::StairHazardSeverity::Critical);
            if has_critical {
                if let Some(ref brush) = stair_set.cached_stair_brush {
                    let crit_col = [1.0, 0.15, 0.15, 0.95];
                    for poly in &brush.polys {
                        if poly.plane.normal.z > 0.9 {
                            add_wire_circle(&mut line_verts, poly.base, 24.0, crit_col);
                        }
                    }
                }
            }
        }

        // Surface Clutter, Procedural Foliage & Decoration Scatter Studio
        if state.show_scatter_studio_dialog {
            let sc_set = &state.scatter_studio_settings;

            // 1. Instances: Wireframe Box, Upward Orientation Stalk & Anchor Point
            if sc_set.show_instances_viewport {
                for inst in &sc_set.preview_instances {
                    let col = if inst.is_valid_clearance {
                        [0.25, 0.90, 0.45, 0.90] // Emerald Green
                    } else {
                        [1.00, 0.20, 0.20, 0.95] // Blocked Red
                    };

                    add_octahedron(&mut line_verts, inst.position, 6.0 * inst.scale.x.max(0.5), col);

                    let stalk_len = 24.0 * inst.scale.x.max(0.5);
                    let tip = inst.position + inst.surface_normal * stalk_len;
                    line_verts.push(LineVertex::new(inst.position.to_array(), col));
                    line_verts.push(LineVertex::new(tip.to_array(), col));

                    let half = 12.0 * inst.scale.x.max(0.5);
                    let min = inst.position - Vec3::new(half, half, 0.0);
                    let max = inst.position + Vec3::new(half, half, stalk_len);

                    let corners = [
                        Vec3::new(min.x, min.y, min.z),
                        Vec3::new(max.x, min.y, min.z),
                        Vec3::new(max.x, max.y, min.z),
                        Vec3::new(min.x, max.y, min.z),
                        Vec3::new(min.x, min.y, max.z),
                        Vec3::new(max.x, min.y, max.z),
                        Vec3::new(max.x, max.y, max.z),
                        Vec3::new(min.x, max.y, max.z),
                    ];
                    let edges = [
                        (0, 1), (1, 2), (2, 3), (3, 0),
                        (4, 5), (5, 6), (6, 7), (7, 4),
                        (0, 4), (1, 5), (2, 6), (3, 7),
                    ];
                    for (i0, i1) in edges {
                        line_verts.push(LineVertex::new(corners[i0].to_array(), col));
                        line_verts.push(LineVertex::new(corners[i1].to_array(), col));
                    }
                }
            }

            // 2. Poisson-Disc Exclusion Radii on Surface (Cyan Circles)
            if sc_set.show_exclusion_discs_viewport {
                let disc_r = sc_set.active_config.min_spacing_radius * 0.5;
                let cyan_col = [0.20, 0.80, 1.00, 0.40];
                for inst in &sc_set.preview_instances {
                    add_wire_circle(&mut line_verts, inst.position, disc_r, cyan_col);
                }
            }

            // 3. Surface Normal Vectors (Yellow Stalks)
            if sc_set.show_surface_normals_viewport {
                let yellow_col = [1.00, 0.90, 0.20, 0.85];
                for inst in &sc_set.preview_instances {
                    let stalk_tip = inst.position + inst.surface_normal * 36.0;
                    line_verts.push(LineVertex::new(inst.position.to_array(), yellow_col));
                    line_verts.push(LineVertex::new(stalk_tip.to_array(), yellow_col));
                }
            }

            // 4. Bot Navigation Node Clearance Buffers (Red Exclusion Spheres)
            if sc_set.show_bot_exclusion_zones_viewport {
                let red_buf = [1.00, 0.25, 0.25, 0.50];
                if let Some(ref unr) = state.loaded_unr {
                    for actor in &unr.actors {
                        let cls = &actor.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "pathnode")
                            || super::overlay_cache::contains_ascii_icase(cls, "playerstart")
                            || super::overlay_cache::contains_ascii_icase(cls, "jumpspot")
                        {
                            add_wire_circle(&mut line_verts, actor.location, sc_set.active_config.bot_clearance_radius, red_buf);
                            add_octahedron(&mut line_verts, actor.location, 10.0, red_buf);
                        }
                    }
                }
            }
        }

        // Volumetric Light Shaft & Window God Ray Studio
        if state.show_light_shaft_dialog {
            let ls_set = &state.light_shaft_settings;
            let cfg = &ls_set.active_config;
            let rgb = cfg.color_rgb;
            let beam_col = [rgb[0], rgb[1], rgb[2], 0.85];
            let core_col = [1.0, 1.0, 0.8, 0.95];

            // 1. Volumetric Beam Mesh Frustum Wireframe
            if ls_set.show_beam_ghost_viewport {
                if let Some(ref brush) = ls_set.cached_preview_brush {
                    for poly in &brush.polys {
                        let is_core = poly.texture.ends_with("_Core");
                        let is_pool = poly.texture.ends_with("_Pool");

                        let col = if is_core {
                            if !ls_set.show_core_hotspot_viewport {
                                continue;
                            }
                            core_col
                        } else if is_pool {
                            if !ls_set.show_floor_disc_viewport {
                                continue;
                            }
                            [rgb[0] * 1.1, rgb[1] * 1.1, rgb[2] * 1.1, 0.90]
                        } else {
                            beam_col
                        };

                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), col));
                        }
                    }
                }
            }

            // 2. Aperture Window Frame at Apex
            if ls_set.show_aperture_frame_viewport {
                let rot = ut99_bsp::light_shaft::compute_aim_rotation(cfg.pitch_deg, cfg.yaw_deg, cfg.roll_deg);
                let half_w = cfg.aperture_width * 0.5;
                let half_h = cfg.aperture_height * 0.5;
                let frame_col = [1.0, 0.9, 0.3, 0.95];

                let corners = [
                    rot * Vec3::new(-half_w, -half_h, 0.0),
                    rot * Vec3::new(half_w, -half_h, 0.0),
                    rot * Vec3::new(half_w, half_h, 0.0),
                    rot * Vec3::new(-half_w, half_h, 0.0),
                ];
                for i in 0..4 {
                    let next = (i + 1) % 4;
                    line_verts.push(LineVertex::new(corners[i].to_array(), frame_col));
                    line_verts.push(LineVertex::new(corners[next].to_array(), frame_col));
                }
            }

            // 3. Ambient Floating Dust Mote Sparkles
            if ls_set.show_dust_motes_viewport {
                for mote in &ls_set.cached_dust_motes {
                    let alpha = (mote.brightness * 0.9).clamp(0.2, 0.95);
                    let mote_col = [rgb[0] * 1.2, rgb[1] * 1.2, rgb[2] * 1.2, alpha];
                    add_octahedron(&mut line_verts, mote.position, mote.size, mote_col);
                }
            }
        }

        // Spline Rail, Kinetic Track Mover & Rollercoaster Studio
        if state.show_track_studio_dialog {
            let tr_set = &state.track_studio_settings;
            let cfg = &tr_set.active_config;

            // 1. Dual Rails Wireframe
            if tr_set.show_rails_viewport {
                if let Some(ref brush) = tr_set.cached_rails_brush {
                    let rail_col = [0.30, 0.85, 1.00, 0.90]; // Luminous Cyan Steel
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), rail_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), rail_col));
                        }
                    }
                }
            }

            // 2. Cross-Tie Sleepers
            if tr_set.show_ties_viewport {
                if let Some(ref brush) = tr_set.cached_ties_brush {
                    let tie_col = [0.85, 0.55, 0.25, 0.85]; // Amber Wood/Concrete
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), tie_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), tie_col));
                        }
                    }
                }
            }

            // 3. Mover Keyframe Markers (Key0..Key7)
            if tr_set.show_keyframes_viewport {
                let kf_col = [1.00, 0.85, 0.20, 0.95]; // Gold
                for kf in &tr_set.cached_keyframes {
                    add_octahedron(&mut line_verts, kf.position, 12.0, kf_col);
                    // Tangent direction stalk
                    let sample = ut99_bsp::sample_track_spline(cfg, kf.key_index as f32 / (tr_set.cached_keyframes.len() - 1).max(1) as f32);
                    let tip = kf.position + sample.tangent * 28.0;
                    line_verts.push(LineVertex::new(kf.position.to_array(), [1.0, 0.3, 0.3, 0.9]));
                    line_verts.push(LineVertex::new(tip.to_array(), [1.0, 0.3, 0.3, 0.9]));
                }
            }

            // 4. Kinetic Cart Simulation at Playhead
            if tr_set.show_animated_cart_viewport {
                let cur_s = ut99_bsp::sample_track_spline(cfg, tr_set.timeline_t);
                let cart_dims = cfg.cart_dimensions;
                let half_w = cart_dims.x * 0.5;
                let half_l = cart_dims.y * 0.5;
                let half_h = cart_dims.z * 0.5;

                let cart_pos = cur_s.position + cur_s.normal * (half_h + cfg.rail_thickness * 0.5);

                // 8 corners of the cart box oriented along track tangent/normal/binormal
                let fwd = cur_s.tangent * half_l;
                let up = cur_s.normal * half_h;
                let side = cur_s.binormal * half_w;

                let c = [
                    cart_pos - fwd - up - side,
                    cart_pos - fwd - up + side,
                    cart_pos - fwd + up - side,
                    cart_pos - fwd + up + side,
                    cart_pos + fwd - up - side,
                    cart_pos + fwd - up + side,
                    cart_pos + fwd + up - side,
                    cart_pos + fwd + up + side,
                ];

                let cart_col = [0.20, 1.00, 0.40, 0.95]; // Emerald Green Cart
                let edges = [
                    (0, 1), (1, 3), (3, 2), (2, 0),
                    (4, 5), (5, 7), (7, 6), (6, 4),
                    (0, 4), (1, 5), (2, 6), (3, 7),
                ];
                for (a, b) in edges {
                    line_verts.push(LineVertex::new(c[a].to_array(), cart_col));
                    line_verts.push(LineVertex::new(c[b].to_array(), cart_col));
                }
            }

            // 5. Bot AI Station Waypoints (LiftExit / LiftCenter)
            if tr_set.show_station_waypoints_viewport {
                for wp in &tr_set.cached_waypoints {
                    let col = if wp.is_cart_mounted {
                        [0.20, 0.95, 0.35, 0.95] // Green for LiftCenter
                    } else {
                        [0.25, 0.60, 1.00, 0.95] // Blue for LiftExit
                    };
                    add_wire_circle(&mut line_verts, wp.location, 24.0, col);
                    add_octahedron(&mut line_verts, wp.location, 8.0, col);
                }
            }
        }

        // Security CCTV Camera, Live Video Monitor & Surveillance Studio
        if state.show_surveillance_dialog {
            let sv_set = &state.surveillance_settings;
            let cfg = &sv_set.active_config;

            if let Some(ref frustum) = sv_set.cached_frustum {
                // 1. Camera Housing & Lens
                let cam_pos = cfg.camera_mount.position;
                add_octahedron(&mut line_verts, cam_pos, 10.0, [1.00, 0.85, 0.20, 0.95]); // Gold housing
                let lens_tip = cam_pos + frustum.forward * 16.0;
                line_verts.push(LineVertex::new(cam_pos.to_array(), [0.3, 0.85, 1.0, 0.9]));
                line_verts.push(LineVertex::new(lens_tip.to_array(), [0.3, 0.85, 1.0, 0.9]));
                add_wire_circle(&mut line_verts, lens_tip, 8.0, [0.3, 0.85, 1.0, 0.95]);

                // 2. Camera Viewing Frustum Pyramid
                if sv_set.show_camera_frustum_viewport {
                    let frustum_edge_col = [0.30, 0.75, 0.95, 0.70];
                    let far_quad_col = [0.20, 0.80, 1.00, 0.85];

                    // 4 edges from camera origin to far corners
                    for c in &frustum.far_corners {
                        line_verts.push(LineVertex::new(cam_pos.to_array(), frustum_edge_col));
                        line_verts.push(LineVertex::new(c.to_array(), frustum_edge_col));
                    }

                    // Far plane quad perimeter (TL -> TR -> BR -> BL -> TL)
                    let fc = &frustum.far_corners;
                    line_verts.push(LineVertex::new(fc[0].to_array(), far_quad_col));
                    line_verts.push(LineVertex::new(fc[1].to_array(), far_quad_col));

                    line_verts.push(LineVertex::new(fc[1].to_array(), far_quad_col));
                    line_verts.push(LineVertex::new(fc[2].to_array(), far_quad_col));

                    line_verts.push(LineVertex::new(fc[2].to_array(), far_quad_col));
                    line_verts.push(LineVertex::new(fc[3].to_array(), far_quad_col));

                    line_verts.push(LineVertex::new(fc[3].to_array(), far_quad_col));
                    line_verts.push(LineVertex::new(fc[0].to_array(), far_quad_col));
                }

                // 3. Sightline Target Ray & Centerline Reticle
                if sv_set.show_sightline_target_viewport {
                    let sight_col = [0.20, 1.00, 0.60, 0.90]; // Luminous Green
                    line_verts.push(LineVertex::new(cam_pos.to_array(), sight_col));
                    line_verts.push(LineVertex::new(frustum.target_point.to_array(), sight_col));
                    add_octahedron(&mut line_verts, frustum.target_point, 14.0, [1.00, 0.25, 0.25, 0.95]);
                }

                // 4. Patrol Sweep Arc Fan
                if sv_set.show_patrol_sweep_viewport {
                    let sweep_col = [0.95, 0.65, 0.15, 0.65]; // Amber Sweep
                    let sweep_rad = cfg.camera_mount.max_range * 0.4;
                    let steps = 16;
                    let base_yaw = cfg.camera_mount.base_yaw_deg;
                    let pan = cfg.patrol.pan_sweep_deg;

                    let mut prev_pt = None;
                    for i in 0..=steps {
                        let t = i as f32 / steps as f32;
                        let yaw = base_yaw - pan + t * (pan * 2.0);
                        let rot = Quat::from_rotation_z(yaw.to_radians()) * Quat::from_rotation_y(cfg.camera_mount.base_pitch_deg.to_radians());
                        let pt = cam_pos + (rot * Vec3::X) * sweep_rad;

                        line_verts.push(LineVertex::new(cam_pos.to_array(), sweep_col));
                        line_verts.push(LineVertex::new(pt.to_array(), sweep_col));

                        if let Some(prev) = prev_pt {
                            line_verts.push(LineVertex::new(prev, sweep_col));
                            line_verts.push(LineVertex::new(pt.to_array(), sweep_col));
                        }
                        prev_pt = Some(pt.to_array());
                    }
                }
            }

            // 5. CRT Monitor Housing Box
            if sv_set.show_monitor_box_viewport {
                if let Some(ref brush) = sv_set.cached_monitor_brush {
                    let mon_col = [0.45, 0.60, 0.85, 0.85]; // Slate Blue
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), mon_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), mon_col));
                        }
                    }
                }
            }
        }

        // Security Laser Grid, Tripwire Alarm & Energy Barrier Studio
        if state.show_laser_grid_dialog {
            let lg_set = &state.laser_grid_settings;
            let cfg = &lg_set.active_config;

            // 1. Glowing Laser Beams
            if lg_set.show_beams_viewport {
                let col = cfg.beam_color;
                let halo_col = [col[0], col[1], col[2], 0.35];
                let norm = cfg.normal.normalize_or_zero();
                let up = Vec3::Z;

                for beam in &lg_set.cached_beams {
                    let s = beam.start_point;
                    let e = beam.end_point;

                    // Core bright beam
                    line_verts.push(LineVertex::new(s.to_array(), col));
                    line_verts.push(LineVertex::new(e.to_array(), col));

                    // Luminous Bloom Halo lines
                    let r = beam.beam_radius * 1.5;
                    let o1 = norm * r;
                    let o2 = up * r;

                    line_verts.push(LineVertex::new((s + o1).to_array(), halo_col));
                    line_verts.push(LineVertex::new((e + o1).to_array(), halo_col));

                    line_verts.push(LineVertex::new((s - o1).to_array(), halo_col));
                    line_verts.push(LineVertex::new((e - o1).to_array(), halo_col));

                    line_verts.push(LineVertex::new((s + o2).to_array(), halo_col));
                    line_verts.push(LineVertex::new((e + o2).to_array(), halo_col));

                    line_verts.push(LineVertex::new((s - o2).to_array(), halo_col));
                    line_verts.push(LineVertex::new((e - o2).to_array(), halo_col));
                }
            }

            // 2. Diode Wall Mounts
            if lg_set.show_diodes_viewport {
                let emitter_col = [0.95, 0.95, 1.00, 0.95]; // Bright Diode
                let receptor_col = [0.65, 0.75, 0.90, 0.95]; // Metal Housing

                for beam in &lg_set.cached_beams {
                    add_octahedron(&mut line_verts, beam.start_point, 5.0, emitter_col);
                    add_wire_circle(&mut line_verts, beam.start_point, 6.0, emitter_col);

                    add_octahedron(&mut line_verts, beam.end_point, 5.0, receptor_col);
                    add_wire_circle(&mut line_verts, beam.end_point, 6.0, receptor_col);
                }
            }

            // 3. Trigger Touch Detection Zones
            if lg_set.show_triggers_viewport {
                let trig_col = if cfg.trigger.damage_per_second >= 100.0 {
                    [1.00, 0.15, 0.15, 0.60] // Lethal Red
                } else {
                    [1.00, 0.60, 0.15, 0.50] // Alarm Orange
                };

                for spec in &lg_set.cached_trigger_specs {
                    add_wire_circle(&mut line_verts, spec.location, spec.collision_height, trig_col);
                    add_octahedron(&mut line_verts, spec.location, 6.0, trig_col);
                }
            }

            // 4. Kinematic Sweep Range
            if lg_set.show_sweep_arcs_viewport && cfg.sweep.is_sweeping {
                let sweep_col = [1.00, 0.85, 0.20, 0.40]; // Amber sweep boundary
                let axis = cfg.sweep.sweep_axis.normalize_or_zero();
                let amp = cfg.sweep.sweep_amplitude;

                for beam in &lg_set.cached_beams {
                    let s_top = beam.start_point + axis * amp;
                    let s_bot = beam.start_point - axis * amp;
                    let e_top = beam.end_point + axis * amp;
                    let e_bot = beam.end_point - axis * amp;

                    line_verts.push(LineVertex::new(s_top.to_array(), sweep_col));
                    line_verts.push(LineVertex::new(s_bot.to_array(), sweep_col));

                    line_verts.push(LineVertex::new(e_top.to_array(), sweep_col));
                    line_verts.push(LineVertex::new(e_bot.to_array(), sweep_col));
                }
            }
        }

        // Environmental Ventilation Fan, Air Current & Wind Tunnel Studio
        if state.show_ventilator_dialog {
            let vent_set = &state.ventilator_settings;
            let cfg = &vent_set.active_config;

            // 1. Rotating Fan Rotor Wireframe
            if vent_set.show_fan_viewport {
                if let Some(ref fan_b) = vent_set.cached_fan_brush {
                    let fan_col = [0.95, 0.75, 0.20, 0.90]; // Warm Bronze/Gold
                    for poly in &fan_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), fan_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), fan_col));
                        }
                    }
                }
            }

            // 2. Duct Housing Ring Wireframe
            if vent_set.show_housing_viewport {
                if let Some(ref house_b) = vent_set.cached_housing_brush {
                    let house_col = [0.40, 0.60, 0.85, 0.65]; // Steel Blue
                    for poly in &house_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), house_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), house_col));
                        }
                    }
                }
            }

            // 3. Air Current Dynamic Streamlines
            if vent_set.show_streamlines_viewport {
                let stream_col = [0.20, 0.85, 1.00, 0.90]; // Cyan Streamlines
                for stream in &vent_set.cached_streamlines {
                    line_verts.push(LineVertex::new(stream.start_point.to_array(), stream_col));
                    line_verts.push(LineVertex::new(stream.end_point.to_array(), stream_col));
                    add_octahedron(&mut line_verts, stream.end_point, 2.5, stream_col);
                }
            }

            // 4. Blade Danger Zone
            if vent_set.show_hazard_zone_viewport && cfg.lethality.is_lethal_contact {
                let hazard_col = [1.00, 0.15, 0.15, 0.85]; // Crimson Hazard Circle
                let r = cfg.geometry.hub_radius + cfg.geometry.blade_length;
                add_wire_circle(&mut line_verts, cfg.position, r, hazard_col);
            }

            // 5. Hover Height Equilibrium Plane
            if vent_set.show_hover_height_viewport && vent_set.cached_lift.can_lift_player {
                let hover_col = [0.20, 0.95, 0.40, 0.85]; // Emerald Hover Plane
                let flow_dir = cfg.rotation_axis.normalize_or_zero();
                let hover_pt = cfg.position + flow_dir * vent_set.cached_lift.equilibrium_hover_height;
                add_wire_circle(&mut line_verts, hover_pt, cfg.air_current.duct_radius * 0.9, hover_col);
            }
        }

        // Industrial Conveyor Belt, Treadmill & Surface Velocity Studio
        if state.show_conveyor_dialog {
            let conv_set = &state.conveyor_settings;
            let cfg = &conv_set.active_config;

            // 1. Conveyor Deck Wireframe
            if conv_set.show_deck_viewport {
                if let Some(ref deck_b) = conv_set.cached_deck_brush {
                    let deck_col = [0.95, 0.70, 0.15, 0.90]; // Warm Amber
                    for poly in &deck_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), deck_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), deck_col));
                        }
                    }
                }
            }

            // 2. End Rollers Wireframe
            if conv_set.show_rollers_viewport {
                if let Some(ref rollers_b) = conv_set.cached_rollers_brush {
                    let roller_col = [0.50, 0.70, 0.95, 0.75]; // Metallic Steel Blue
                    for poly in &rollers_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), roller_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), roller_col));
                        }
                    }
                }
            }

            // 3. Side Guardrails & Support Frame
            if conv_set.show_guardrails_viewport {
                if let Some(ref frame_b) = conv_set.cached_frame_brush {
                    let frame_col = [0.60, 0.70, 0.80, 0.65]; // Industrial Slate
                    for poly in &frame_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), frame_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), frame_col));
                        }
                    }
                }
            }

            // 4. Moving Surface Velocity Arrows along Centerline
            if conv_set.show_velocity_vectors_viewport {
                let arrow_col = [0.20, 0.85, 1.00, 0.95]; // Electric Cyan
                let fwd = cfg.forward_axis.normalize_or_zero();
                let right = fwd.cross(Vec3::Z).normalize_or_zero();
                let incline_rad = cfg.deck.incline_angle_deg.to_radians();
                let pitch_quat = Quat::from_axis_angle(right, incline_rad);
                let fwd_inclined = (pitch_quat * fwd).normalize_or_zero();
                let up_inclined = (pitch_quat * Vec3::Z).normalize_or_zero();

                let half_l = cfg.deck.length * 0.5;
                let top_z = cfg.deck.thickness * 0.5 + 4.0;
                let dir_sign = match cfg.kinematics.direction {
                    ut99_bsp::ConveyorDirection::Forward => 1.0,
                    ut99_bsp::ConveyorDirection::Backward => -1.0,
                    ut99_bsp::ConveyorDirection::Stopped => 0.0,
                };

                let arrow_count = 5;
                for a in 0..arrow_count {
                    let u = ((a as f32 + 0.5) / arrow_count as f32) * 2.0 - 1.0;
                    let base_pt = cfg.position + fwd_inclined * (u * half_l * 0.8) + up_inclined * top_z;
                    let tip_pt = base_pt + fwd_inclined * (24.0 * dir_sign);

                    line_verts.push(LineVertex::new(base_pt.to_array(), arrow_col));
                    line_verts.push(LineVertex::new(tip_pt.to_array(), arrow_col));
                    add_octahedron(&mut line_verts, tip_pt, 2.5, arrow_col);
                }
            }

            // 5. Bot AI Navigation Waypoints
            if conv_set.show_waypoints_viewport && cfg.bot_nav.generate_bot_waypoints {
                let (intake, discharge) = &conv_set.cached_waypoints;
                let in_col = [0.20, 0.95, 0.40, 0.90]; // Emerald Intake
                let out_col = [1.00, 0.55, 0.15, 0.90]; // Orange Discharge

                add_wire_circle(&mut line_verts, intake.location, 16.0, in_col);
                add_octahedron(&mut line_verts, intake.location, 4.0, in_col);

                add_wire_circle(&mut line_verts, discharge.location, 16.0, out_col);
                add_octahedron(&mut line_verts, discharge.location, 4.0, out_col);
            }
        }

        // Automated Defense Turret, Sentry Cannon & Emplacement Studio
        if state.show_turret_dialog {
            let turr_set = &state.turret_settings;
            let cfg = &turr_set.active_config;
            let metrics = &turr_set.cached_metrics;

            let mount_up = match cfg.mount.mount_type {
                ut99_bsp::TurretMountType::FloorPedestal => Vec3::Z,
                ut99_bsp::TurretMountType::CeilingSuspended => -Vec3::Z,
                ut99_bsp::TurretMountType::WallBracket => Vec3::Z,
            };
            let pivot_pos = cfg.position + mount_up * cfg.mount.pedestal_height;

            // 1. Structural Mount Wireframe
            if turr_set.show_mount_viewport {
                if let Some(ref mount_b) = turr_set.cached_mount_brush {
                    let mount_col = [0.45, 0.55, 0.65, 0.85]; // Gunmetal Slate
                    for poly in &mount_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), mount_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), mount_col));
                        }
                    }
                }
            }

            // 2. Armored Shield Visor Wireframe
            if turr_set.show_shield_viewport {
                if let Some(ref shield_b) = turr_set.cached_shield_brush {
                    let shield_col = match cfg.team {
                        ut99_bsp::TurretTeamOwnership::RedTeam => [0.95, 0.25, 0.25, 0.90],
                        ut99_bsp::TurretTeamOwnership::BlueTeam => [0.25, 0.65, 0.95, 0.90],
                        _ => [0.95, 0.75, 0.25, 0.90],
                    };
                    for poly in &shield_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), shield_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), shield_col));
                        }
                    }
                }
            }

            // 3. Swept Traversal & Elevation Tracking Cone
            if turr_set.show_tracking_cone_viewport {
                let cone_col = [0.30, 0.80, 1.00, 0.35]; // Translucent Cyan
                let range = cfg.tracking.tracking_range;
                let min_yaw_rad = (cfg.base_rotation_yaw_deg + cfg.tracking.yaw_min_deg).to_radians();
                let max_yaw_rad = (cfg.base_rotation_yaw_deg + cfg.tracking.yaw_max_deg).to_radians();

                let arc_steps = 12;
                for s in 0..=arc_steps {
                    let frac = s as f32 / arc_steps as f32;
                    let yaw = min_yaw_rad + frac * (max_yaw_rad - min_yaw_rad);
                    let dir = Vec3::new(yaw.cos(), yaw.sin(), 0.0);
                    let rim_pt = pivot_pos + dir * range;

                    line_verts.push(LineVertex::new(pivot_pos.to_array(), cone_col));
                    line_verts.push(LineVertex::new(rim_pt.to_array(), cone_col));

                    if s < arc_steps {
                        let next_yaw = min_yaw_rad + ((s + 1) as f32 / arc_steps as f32) * (max_yaw_rad - min_yaw_rad);
                        let next_dir = Vec3::new(next_yaw.cos(), next_yaw.sin(), 0.0);
                        let next_rim = pivot_pos + next_dir * range;
                        line_verts.push(LineVertex::new(rim_pt.to_array(), [0.30, 0.80, 1.00, 0.75]));
                        line_verts.push(LineVertex::new(next_rim.to_array(), [0.30, 0.80, 1.00, 0.75]));
                    }
                }
            }

            // 4. Live Muzzle Aiming Line-of-Fire Raycast
            if turr_set.show_line_of_fire_viewport {
                let beam_col = match cfg.combat.weapon_type {
                    ut99_bsp::TurretWeaponType::ShockBeam => [0.75, 0.25, 1.00, 0.95],
                    ut99_bsp::TurretWeaponType::RocketSalvo => [1.00, 0.50, 0.10, 0.95],
                    ut99_bsp::TurretWeaponType::PlasmaBolt => [0.20, 0.90, 1.00, 0.95],
                    _ => [1.00, 0.85, 0.20, 0.95],
                };

                let muzzle = metrics.muzzle_origin;
                let target_pt = muzzle + metrics.aiming_direction * cfg.tracking.tracking_range;

                line_verts.push(LineVertex::new(muzzle.to_array(), beam_col));
                line_verts.push(LineVertex::new(target_pt.to_array(), beam_col));

                // Target crosshair at range
                add_octahedron(&mut line_verts, target_pt, 6.0, beam_col);
                add_wire_circle(&mut line_verts, target_pt, 18.0, beam_col);
            }

            // 5. Bot Cover & Evasion Waypoints
            if turr_set.show_waypoints_viewport {
                for wp in &turr_set.cached_waypoints {
                    let wp_col = if wp.has_direct_line_of_sight {
                        [1.00, 0.25, 0.25, 0.90] // Red Killzone Node
                    } else {
                        [0.20, 0.95, 0.40, 0.90] // Green Flank Cover Node
                    };

                    add_wire_circle(&mut line_verts, wp.location, 16.0, wp_col);
                    add_octahedron(&mut line_verts, wp.location, 4.0, wp_col);
                    line_verts.push(LineVertex::new(pivot_pos.to_array(), [wp_col[0], wp_col[1], wp_col[2], 0.25]));
                    line_verts.push(LineVertex::new(wp.location.to_array(), [wp_col[0], wp_col[1], wp_col[2], 0.25]));
                }
            }
        }

        // Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
        if state.show_soundscape_dialog {
            let snd_set = &state.soundscape_settings;
            let cfg = &snd_set.active_config;
            let metrics = &snd_set.cached_metrics;
            let pivot = cfg.position;

            // 1. Central Emitter Speaker Icon
            if snd_set.show_emitter_viewport {
                let speaker_col = [1.00, 0.85, 0.20, 0.95]; // Gold
                add_octahedron(&mut line_verts, pivot, 8.0, speaker_col);
                add_wire_circle(&mut line_verts, pivot, 12.0, speaker_col);

                // Small directional horn stub
                let horn_tip = pivot + Vec3::new(16.0, 0.0, 0.0);
                line_verts.push(LineVertex::new(pivot.to_array(), speaker_col));
                line_verts.push(LineVertex::new(horn_tip.to_array(), speaker_col));
                add_wire_circle(&mut line_verts, horn_tip, 8.0, speaker_col);
            }

            // 2. Inner 100% Volume Shell (Yellow 3-axis circles)
            if snd_set.show_inner_radius_viewport && metrics.inner_radius_uu > 10.0 {
                let inner_col = [1.00, 0.90, 0.25, 0.80];
                let r = metrics.inner_radius_uu;
                let steps = 24;
                for s in 0..steps {
                    let a1 = (s as f32 / steps as f32) * std::f32::consts::TAU;
                    let a2 = ((s + 1) as f32 / steps as f32) * std::f32::consts::TAU;

                    // XY circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a1.cos() * r, a1.sin() * r, 0.0)).to_array(), inner_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a2.cos() * r, a2.sin() * r, 0.0)).to_array(), inner_col));

                    // XZ circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a1.cos() * r, 0.0, a1.sin() * r)).to_array(), inner_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a2.cos() * r, 0.0, a2.sin() * r)).to_array(), inner_col));

                    // YZ circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(0.0, a1.cos() * r, a1.sin() * r)).to_array(), inner_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(0.0, a2.cos() * r, a2.sin() * r)).to_array(), inner_col));
                }
            }

            // 3. Outer Threshold Shell (Cyan 3-axis circles)
            if snd_set.show_outer_radius_viewport && metrics.outer_radius_uu > 20.0 {
                let outer_col = [0.30, 0.75, 1.00, 0.45];
                let r = metrics.outer_radius_uu;
                let steps = 32;
                for s in 0..steps {
                    let a1 = (s as f32 / steps as f32) * std::f32::consts::TAU;
                    let a2 = ((s + 1) as f32 / steps as f32) * std::f32::consts::TAU;

                    // XY circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a1.cos() * r, a1.sin() * r, 0.0)).to_array(), outer_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a2.cos() * r, a2.sin() * r, 0.0)).to_array(), outer_col));

                    // XZ circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a1.cos() * r, 0.0, a1.sin() * r)).to_array(), outer_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(a2.cos() * r, 0.0, a2.sin() * r)).to_array(), outer_col));

                    // YZ circle
                    line_verts.push(LineVertex::new((pivot + Vec3::new(0.0, a1.cos() * r, a1.sin() * r)).to_array(), outer_col));
                    line_verts.push(LineVertex::new((pivot + Vec3::new(0.0, a2.cos() * r, a2.sin() * r)).to_array(), outer_col));
                }
            }

            // 4. Pulsing Acoustic Wave Ripples
            if snd_set.show_soundwaves_viewport {
                let wave_col = [0.40, 0.90, 1.00, 0.65];
                for w in 1..=4 {
                    let wave_r = (metrics.outer_radius_uu * (w as f32 / 5.0)).max(32.0);
                    add_wire_circle(&mut line_verts, pivot, wave_r, wave_col);
                }
            }

            // 5. Acoustic Room Zone Bounds
            if snd_set.show_zone_bounds_viewport {
                if let Some(ref zone_b) = snd_set.cached_zone_brush {
                    let zone_col = [0.25, 0.90, 0.50, 0.55]; // Reverb Emerald
                    for poly in &zone_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), zone_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), zone_col));
                        }
                    }
                }
            }

            // 6. Virtual Listener Probe Marker
            let probe_loc = pivot + Vec3::new(snd_set.probe_listener_distance_uu, 0.0, 0.0);
            let probe_col = if snd_set.cached_probe_sample.is_occluded_by_wall {
                [0.95, 0.35, 0.20, 0.95] // Muffled Occluded Red
            } else {
                [0.30, 1.00, 0.50, 0.95] // Clear Green
            };
            add_octahedron(&mut line_verts, probe_loc, 6.0, probe_col);
            add_wire_circle(&mut line_verts, probe_loc, 14.0, probe_col);
            line_verts.push(LineVertex::new(pivot.to_array(), [probe_col[0], probe_col[1], probe_col[2], 0.35]));
            line_verts.push(LineVertex::new(probe_loc.to_array(), [probe_col[0], probe_col[1], probe_col[2], 0.35]));
        }

        // Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio
        if state.show_hazard_chamber_dialog {
            let ch_set = &state.hazard_chamber_settings;
            let cfg = &ch_set.active_config;

            if let Some(ref brushes) = ch_set.cached_brushes {
                // 1. Chamber Outer Shell Wireframe
                if ch_set.show_chamber_viewport {
                    let shell_col = [0.45, 0.65, 0.85, 0.75];
                    for poly in &brushes.chamber_brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), shell_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), shell_col));
                        }
                    }
                }

                // 2. Sliding Blast Doors
                if ch_set.show_doors_viewport {
                    let door_col = [1.00, 0.70, 0.15, 0.95]; // Hazard Amber
                    for b in [&brushes.door_left_brush, &brushes.door_right_brush] {
                        for poly in &b.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), door_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), door_col));
                            }
                        }
                    }
                }

                // 3. Observation Window Glass
                if ch_set.show_window_viewport {
                    if let Some(ref win_b) = brushes.window_glass_brush {
                        let win_col = [0.25, 0.85, 1.00, 0.90]; // Glass Cyan
                        for poly in &win_b.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), win_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), win_col));
                            }
                        }
                    }
                }

                // 4. Control Console Pedestal & Button
                if ch_set.show_console_viewport {
                    let con_col = [0.60, 0.60, 0.70, 0.85];
                    for poly in &brushes.console_brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), con_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), con_col));
                        }
                    }

                    let btn_pos = cfg.position + cfg.console_offset + Vec3::new(0.0, 0.0, -cfg.chamber_height * 0.5 + 40.0);
                    let btn_col = [1.00, 0.25, 0.25, 0.95];
                    add_octahedron(&mut line_verts, btn_pos, 6.0, btn_col);
                    add_wire_circle(&mut line_verts, btn_pos, 12.0, btn_col);
                }

                // 5. Hazard Atmosphere / Gas Cloud
                if ch_set.show_gas_volume_viewport {
                    let gas_col = [cfg.strobe_color[0], cfg.strobe_color[1], cfg.strobe_color[2], 0.60];
                    let half_size = Vec3::new(cfg.chamber_width * 0.40, cfg.chamber_length * 0.40, cfg.chamber_height * 0.40);
                    add_wire_box(&mut line_verts, cfg.position, half_size * 2.0, gas_col);
                    add_wire_circle(&mut line_verts, cfg.position, half_size.x.min(half_size.y), gas_col);
                }

                // 6. Bot Navigation Waypoints
                if ch_set.show_waypoints_viewport {
                    let wp = &ch_set.cached_waypoints;
                    let bait_col = [1.00, 0.85, 0.20, 0.95]; // Gold
                    let con_col = [1.00, 0.35, 0.35, 0.95];  // Red
                    let safe_col = [0.30, 1.00, 0.50, 0.95]; // Green

                    add_octahedron(&mut line_verts, wp.bait_inside, 6.0, bait_col);
                    add_wire_circle(&mut line_verts, wp.bait_inside, 16.0, bait_col);

                    add_octahedron(&mut line_verts, wp.console_activator, 6.0, con_col);
                    add_wire_circle(&mut line_verts, wp.console_activator, 16.0, con_col);

                    add_octahedron(&mut line_verts, wp.safe_outside_door, 6.0, safe_col);
                    add_wire_circle(&mut line_verts, wp.safe_outside_door, 16.0, safe_col);

                    // Conjoining path lines
                    let path_col = [0.85, 0.85, 0.85, 0.40];
                    line_verts.push(LineVertex::new(wp.bait_inside.to_array(), path_col));
                    line_verts.push(LineVertex::new(wp.safe_outside_door.to_array(), path_col));

                    line_verts.push(LineVertex::new(wp.safe_outside_door.to_array(), path_col));
                    line_verts.push(LineVertex::new(wp.console_activator.to_array(), path_col));
                }
            }
        }

        // Industrial Pipe Network, Steam Valve & Conduit Junction Studio
        if state.show_pipe_conduit_dialog {
            let p_set = &state.pipe_conduit_settings;
            let cfg = &p_set.active_config;

            if let Some(ref brushes) = p_set.cached_brushes {
                // 1. Straight Run Pipe Wireframe
                if p_set.show_pipe_viewport {
                    let pipe_col = [0.50, 0.65, 0.80, 0.85];
                    for poly in &brushes.straight_run_brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), pipe_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), pipe_col));
                        }
                    }
                }

                // 2. Elbow Bend Wireframe
                if p_set.show_elbow_viewport {
                    if let Some(ref eb) = brushes.elbow_bend_brush {
                        let elbow_col = [0.30, 0.85, 1.00, 0.90];
                        for poly in &eb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), elbow_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), elbow_col));
                            }
                        }
                    }
                }

                // 3. Bolt Flange Brushes
                if p_set.show_flanges_viewport {
                    let fl_col = [1.00, 0.80, 0.20, 0.95]; // Gold
                    for fb in &brushes.flange_brushes {
                        for poly in &fb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), fl_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), fl_col));
                            }
                        }
                    }
                }

                // 4. Wall Collar Brush
                if let Some(ref cb) = brushes.wall_collar_brush {
                    let col_col = [0.70, 0.75, 0.85, 0.90];
                    for poly in &cb.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), col_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), col_col));
                        }
                    }
                }

                // 5. Valve Body & Handwheel
                if p_set.show_valve_viewport {
                    if let Some(ref vb) = brushes.valve_body_brush {
                        let vb_col = [0.85, 0.70, 0.25, 0.90];
                        for poly in &vb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), vb_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), vb_col));
                            }
                        }
                    }
                    if let Some(ref wb) = brushes.valve_handwheel_brush {
                        let wh_col = [1.00, 0.20, 0.20, 0.95]; // Crimson
                        for poly in &wb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), wh_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), wh_col));
                            }
                        }
                    }
                }

                // 6. Steam Scald Hazard Conical Jet
                if p_set.show_steam_hazard_viewport && cfg.steam_hazard_enabled {
                    let v_center = cfg.position + Vec3::new(0.0, cfg.total_run_length_uu * 0.5, 0.0);
                    let vent_origin = v_center + Vec3::new(cfg.pipe_outer_radius + 4.0, 0.0, 0.0);
                    let jet_end = vent_origin + Vec3::new(48.0, 0.0, 0.0);
                    let jet_col = [0.90, 0.95, 1.00, 0.75];

                    line_verts.push(LineVertex::new(vent_origin.to_array(), jet_col));
                    line_verts.push(LineVertex::new((jet_end + Vec3::new(0.0, -18.0, -18.0)).to_array(), jet_col));
                    line_verts.push(LineVertex::new(vent_origin.to_array(), jet_col));
                    line_verts.push(LineVertex::new((jet_end + Vec3::new(0.0, 18.0, -18.0)).to_array(), jet_col));
                    line_verts.push(LineVertex::new(vent_origin.to_array(), jet_col));
                    line_verts.push(LineVertex::new((jet_end + Vec3::new(0.0, 18.0, 18.0)).to_array(), jet_col));
                    line_verts.push(LineVertex::new(vent_origin.to_array(), jet_col));
                    line_verts.push(LineVertex::new((jet_end + Vec3::new(0.0, -18.0, 18.0)).to_array(), jet_col));

                    add_wire_circle(&mut line_verts, jet_end, 18.0, jet_col);
                    add_octahedron(&mut line_verts, vent_origin, 5.0, [1.0, 0.3, 0.3, 0.95]);
                }

                // 7. Flow Centerline Spine
                if p_set.show_flow_centerline_viewport {
                    let spine_col = [0.20, 1.00, 0.60, 0.85]; // Emerald
                    let start_c = cfg.position;
                    let end_c = cfg.position + Vec3::new(0.0, cfg.total_run_length_uu, 0.0);
                    line_verts.push(LineVertex::new(start_c.to_array(), spine_col));
                    line_verts.push(LineVertex::new(end_c.to_array(), spine_col));

                    add_octahedron(&mut line_verts, start_c, 5.0, spine_col);
                    add_octahedron(&mut line_verts, end_c, 5.0, spine_col);
                }
            }
        }

        // Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
        if state.show_hologram_dialog {
            let h_set = &state.hologram_settings;
            let cfg = &h_set.active_config;

            if let Some(ref brushes) = h_set.cached_brushes {
                let pri_col = [cfg.primary_color[0], cfg.primary_color[1], cfg.primary_color[2], 0.90];
                let sec_col = [cfg.secondary_color[0], cfg.secondary_color[1], cfg.secondary_color[2], 0.85];

                // 1. Display Surface Wireframe
                if h_set.show_display_viewport {
                    for poly in &brushes.display_surface_brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), pri_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), pri_col));
                        }
                    }
                }

                // 2. Emitter Pedestal Wireframe
                if h_set.show_pedestal_viewport {
                    if let Some(ref pb) = brushes.emitter_pedestal_brush {
                        let ped_col = [0.45, 0.50, 0.60, 0.90];
                        for poly in &pb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), ped_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), ped_col));
                            }
                        }

                        // Projector lens circle on top of pedestal
                        let lens_z = cfg.position.z + cfg.pedestal_height;
                        let lens_pos = Vec3::new(cfg.position.x, cfg.position.y, lens_z);
                        add_wire_circle(&mut line_verts, lens_pos, cfg.pedestal_radius * 0.4, pri_col);
                    }
                }

                // 3. Volumetric Projection Beam Frustum
                if h_set.show_projection_beam_viewport {
                    if let Some(ref cb) = brushes.projection_cone_brush {
                        let beam_col = [0.40, 0.85, 1.00, 0.45];
                        for poly in &cb.polys {
                            let len = poly.vertices.len();
                            for i in 0..len {
                                let next = (i + 1) % len;
                                line_verts.push(LineVertex::new(poly.vertices[i].to_array(), beam_col));
                                line_verts.push(LineVertex::new(poly.vertices[next].to_array(), beam_col));
                            }
                        }
                    }
                }

                // 4. Scanlines Wave Grid
                if h_set.show_scanlines_viewport {
                    let center_z = cfg.position.z + cfg.elevation_offset_uu;
                    let hh = cfg.display_height * 0.5;
                    let hw = cfg.display_width * 0.5;
                    let scan_col = [1.0, 1.0, 1.0, 0.35];

                    let lines = cfg.scanline_count.clamp(4, 32);
                    for k in 0..=lines {
                        let frac = k as f32 / lines as f32;
                        let z = center_z - hh + frac * cfg.display_height;
                        let p_l = Vec3::new(cfg.position.x - hw, cfg.position.y, z);
                        let p_r = Vec3::new(cfg.position.x + hw, cfg.position.y, z);
                        line_verts.push(LineVertex::new(p_l.to_array(), scan_col));
                        line_verts.push(LineVertex::new(p_r.to_array(), scan_col));
                    }
                }

                // 5. Rotation Axis Stalk
                if h_set.show_rotation_axis_viewport {
                    let axis_col = sec_col;
                    let bottom_pos = cfg.position;
                    let top_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.elevation_offset_uu + cfg.display_height * 0.5 + 24.0);

                    line_verts.push(LineVertex::new(bottom_pos.to_array(), axis_col));
                    line_verts.push(LineVertex::new(top_pos.to_array(), axis_col));

                    add_octahedron(&mut line_verts, bottom_pos, 4.0, axis_col);
                    add_octahedron(&mut line_verts, top_pos, 4.0, axis_col);

                    if cfg.rotation_speed_deg_s.abs() > 0.001 {
                        add_wire_circle(&mut line_verts, top_pos, 16.0, axis_col);
                    }
                }
            }
        }

        // Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
        if state.show_weather_dialog {
            let w_set = &state.weather_settings;
            let cfg = &w_set.active_config;

            // 1. Weather Volume Bounds Wireframe
            if w_set.show_weather_volume_viewport {
                if let Some(ref vol_b) = w_set.cached_volume_brush {
                    let vol_col = [0.35, 0.65, 0.95, 0.60];
                    for poly in &vol_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), vol_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), vol_col));
                        }
                    }
                }
            }

            // 2. Procedural Fractal Lightning Bolt
            if w_set.show_lightning_viewport && w_set.cached_metrics.is_flashing_now {
                let bolt_pts = &w_set.cached_lightning_branch;
                if bolt_pts.len() >= 2 {
                    let flash = w_set.cached_metrics.flash_brightness;
                    let bolt_col = [
                        (cfg.lightning_color_rgb[0] * flash).clamp(0.2, 1.0),
                        (cfg.lightning_color_rgb[1] * flash).clamp(0.2, 1.0),
                        (cfg.lightning_color_rgb[2] * flash).clamp(0.2, 1.0),
                        1.0,
                    ];
                    for i in 0..bolt_pts.len() - 1 {
                        line_verts.push(LineVertex::new(bolt_pts[i].to_array(), bolt_col));
                        line_verts.push(LineVertex::new(bolt_pts[i + 1].to_array(), bolt_col));
                    }
                }
            }

            // 3. Wind Vector Arrow from Volume Center
            let wind_len = cfg.wind_vector.length();
            if wind_len > 1.0 {
                let wind_dir = cfg.wind_vector / wind_len;
                let wind_arrow_len = (wind_len * 0.5).clamp(64.0, 512.0);
                let p0 = cfg.position;
                let p1 = p0 + wind_dir * wind_arrow_len;
                let wind_col = [0.20, 0.95, 0.70, 0.85];
                line_verts.push(LineVertex::new(p0.to_array(), wind_col));
                line_verts.push(LineVertex::new(p1.to_array(), wind_col));
            }
        }

        // Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
        if state.show_armor_stacking_dialog {
            let arm_set = &state.armor_stacking_settings;
            let cfg = &arm_set.active_config;

            // 1. Armor Pedestal Wireframe
            if arm_set.show_pedestals_viewport {
                if let Some(ref ped_b) = arm_set.cached_pedestal_brush {
                    let ped_col = [0.85, 0.70, 0.20, 0.85]; // Golden Pedestal
                    for poly in &ped_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), ped_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), ped_col));
                        }
                    }
                }
            }

            // 2. Absorption Shield Bubbles (Gold for ShieldBelt, Blue for Body Armor, Green for ThighPads)
            if arm_set.show_absorption_bubbles_viewport {
                let center = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 40.0);
                if cfg.shield_belt_count > 0 {
                    let gold = [1.0, 0.85, 0.15, 0.75];
                    add_wire_circle(&mut line_verts, center, 48.0, gold);
                    add_octahedron(&mut line_verts, center, 16.0, gold);
                }
                if cfg.body_armor_count > 0 {
                    let blue = [0.2, 0.55, 1.0, 0.70];
                    add_wire_circle(&mut line_verts, center, 40.0, blue);
                }
                if cfg.thigh_pads_count > 0 {
                    let green = [0.25, 0.90, 0.40, 0.65];
                    add_wire_circle(&mut line_verts, center, 32.0, green);
                }
            }

            // 3. Clustering Hazard Links (< 400 uu)
            if arm_set.show_cluster_hazards_viewport {
                if let Some(ref unr) = state.loaded_unr {
                    let mut major_pos = Vec::new();
                    for actor in &unr.actors {
                        let cls = &actor.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "shieldbelt")
                            || super::overlay_cache::contains_ascii_icase(cls, "armor2")
                            || super::overlay_cache::contains_ascii_icase(cls, "keg")
                            || super::overlay_cache::contains_ascii_icase(cls, "healthpack")
                        {
                            major_pos.push(actor.location);
                        }
                    }

                    let hazard_col = [1.00, 0.20, 0.20, 0.95]; // Crimson Hazard Line
                    for i in 0..major_pos.len() {
                        for j in (i + 1)..major_pos.len() {
                            let dist = major_pos[i].distance(major_pos[j]);
                            if dist < 400.0 {
                                line_verts.push(LineVertex::new(major_pos[i].to_array(), hazard_col));
                                line_verts.push(LineVertex::new(major_pos[j].to_array(), hazard_col));
                                add_octahedron(&mut line_verts, (major_pos[i] + major_pos[j]) * 0.5, 8.0, hazard_col);
                            }
                        }
                    }
                }
            }
        }

        // Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
        if state.show_bot_jump_routes_dialog {
            let j_set = &state.bot_jump_routes_settings;
            let cfg = &j_set.active_config;

            // 1. Launch Jump Plate Wireframe
            if j_set.show_jump_plate_viewport {
                if let Some(ref plate_b) = j_set.cached_plate_brush {
                    let plate_col = [0.35, 0.75, 1.00, 0.85]; // Steel Cyan
                    for poly in &plate_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), plate_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), plate_col));
                        }
                    }
                }
            }

            // 2. Parabolic Ballistic Trajectory Arc with Apex Marker
            if j_set.show_trajectory_viewport {
                let arc_col = cfg.jump_type.trajectory_color_rgb();
                let pts = &j_set.cached_solution.trajectory_points;
                if pts.len() >= 2 {
                    for i in 0..pts.len() - 1 {
                        line_verts.push(LineVertex::new(pts[i].to_array(), arc_col));
                        line_verts.push(LineVertex::new(pts[i + 1].to_array(), arc_col));
                    }

                    // Draw apex peak octahedron marker
                    let apex_pt = pts.iter().max_by(|a, b| a.z.partial_cmp(&b.z).unwrap()).copied();
                    if let Some(apex) = apex_pt {
                        add_octahedron(&mut line_verts, apex, 8.0, [1.0, 1.0, 0.2, 0.95]);
                    }
                }
            }

            // 3. Landing Zone Ring & Error Tolerance Circle
            if j_set.show_landing_zone_viewport {
                let landing_col = if j_set.cached_solution.is_reachable {
                    [0.20, 0.95, 0.45, 0.90] // Green circle
                } else {
                    [1.00, 0.20, 0.20, 0.95] // Red hazard circle
                };
                add_wire_circle(&mut line_verts, cfg.target_pos, cfg.landing_tolerance_radius, landing_col);
                add_octahedron(&mut line_verts, cfg.target_pos, 6.0, landing_col);
            }
        }

        // Interactive Tournament Weapon Placement & Ammo Circuit Studio
        if state.show_weapon_circuit_dialog {
            let w_set = &state.weapon_circuit_settings;
            let cfg = &w_set.active_config;

            // 1. Weapon Altar Wireframe
            if w_set.show_altar_viewport {
                if let Some(ref altar_b) = w_set.cached_altar_brush {
                    let altar_col = cfg.primary_weapon.weapon_color_rgb();
                    for poly in &altar_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), altar_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), altar_col));
                        }
                    }
                    add_octahedron(&mut line_verts, cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height + 24.0), 10.0, altar_col);
                }
            }

            // 2. Ammo Tethers
            if w_set.show_ammo_tethers_viewport && cfg.flanking_ammo_count > 0 {
                let ammo_offsets = [
                    Vec3::new(-cfg.altar_width * 0.45, 0.0, cfg.altar_height + 12.0),
                    Vec3::new(cfg.altar_width * 0.45, 0.0, cfg.altar_height + 12.0),
                    Vec3::new(0.0, cfg.altar_depth * 0.45, cfg.altar_height + 12.0),
                ];
                let center_top = cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height);
                let tether_col = [1.0, 0.9, 0.3, 0.75]; // Gold
                for i in 0..(cfg.flanking_ammo_count as usize).min(ammo_offsets.len()) {
                    let ammo_pos = cfg.position + ammo_offsets[i];
                    line_verts.push(LineVertex::new(center_top.to_array(), tether_col));
                    line_verts.push(LineVertex::new(ammo_pos.to_array(), tether_col));
                    add_octahedron(&mut line_verts, ammo_pos, 4.0, tether_col);
                }
            }

            // 3. Circuit Sprint Links between existing level weapons
            if w_set.show_circuit_links_viewport {
                let link_col = [0.4, 0.8, 1.0, 0.5]; // Cyan sprint line
                if let Some(ref unr) = state.loaded_unr {
                    let mut weaps = Vec::new();
                    for a in &unr.actors {
                        let cls = &a.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "weapon")
                            || super::overlay_cache::contains_ascii_icase(cls, "shock")
                            || super::overlay_cache::contains_ascii_icase(cls, "flak")
                            || super::overlay_cache::contains_ascii_icase(cls, "eightball")
                            || super::overlay_cache::contains_ascii_icase(cls, "sniper")
                        {
                            weaps.push(a.location);
                        }
                    }
                    weaps.push(cfg.position);
                    for i in 0..weaps.len() {
                        for j in (i + 1)..weaps.len() {
                            line_verts.push(LineVertex::new(weaps[i].to_array(), link_col));
                            line_verts.push(LineVertex::new(weaps[j].to_array(), link_col));
                        }
                    }
                }
            }
        }

        // Interactive Tournament Powerup Control & Relic Altar Studio
        if state.show_powerup_control_dialog {
            let p_set = &state.powerup_control_settings;
            let cfg = &p_set.active_config;
            let p_col = cfg.powerup_type.powerup_color_rgb();

            // 1. Altar Brush Wireframe
            if p_set.show_altar_viewport {
                if let Some(ref altar_b) = p_set.cached_altar_brush {
                    for poly in &altar_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), p_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), p_col));
                        }
                    }
                    add_octahedron(&mut line_verts, cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height + 24.0), 12.0, p_col);
                }
            }

            // 2. Containment Cylinder & Energy Vortex
            if p_set.show_containment_cylinder_viewport {
                let center_bottom = cfg.position + Vec3::new(0.0, 0.0, cfg.altar_height);
                let center_top = center_bottom + Vec3::new(0.0, 0.0, cfg.containment_beam_height);
                let beam_col = [p_col[0], p_col[1], p_col[2], 0.4];

                add_wire_circle(&mut line_verts, center_bottom, cfg.altar_radius * 0.7, beam_col);
                add_wire_circle(&mut line_verts, center_top, cfg.altar_radius * 0.7, beam_col);

                let num_pillars = 4;
                for i in 0..num_pillars {
                    let angle = std::f32::consts::TAU * (i as f32) / (num_pillars as f32);
                    let ox = cfg.altar_radius * 0.7 * angle.cos();
                    let oy = cfg.altar_radius * 0.7 * angle.sin();
                    let p_bot = center_bottom + Vec3::new(ox, oy, 0.0);
                    let p_top = center_top + Vec3::new(ox, oy, 0.0);
                    line_verts.push(LineVertex::new(p_bot.to_array(), beam_col));
                    line_verts.push(LineVertex::new(p_top.to_array(), beam_col));
                }
            }

            // 3. Contention Sightline Rays
            if p_set.show_contention_rays_viewport {
                let ray_col = [1.0, 0.8, 0.2, 0.5]; // Golden contention
                if let Some(ref unr) = state.loaded_unr {
                    for a in &unr.actors {
                        let cls = &a.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "weapon")
                            || super::overlay_cache::contains_ascii_icase(cls, "sniper")
                            || super::overlay_cache::contains_ascii_icase(cls, "shieldbelt")
                        {
                            line_verts.push(LineVertex::new(cfg.position.to_array(), ray_col));
                            line_verts.push(LineVertex::new(a.location.to_array(), ray_col));
                        }
                    }
                }
            }
        }

        // Interactive Tournament Spawn Point Balancing Studio
        if state.show_spawn_distribution_dialog {
            let sp_set = &state.spawn_distribution_settings;
            let cfg = &sp_set.active_config;
            let sp_col = cfg.team_number.team_color_rgb();

            // 1. Spawn Pad Wireframe
            if sp_set.show_pad_viewport {
                if let Some(ref pad_b) = sp_set.cached_pad_brush {
                    for poly in &pad_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), sp_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), sp_col));
                        }
                    }
                }
            }

            // 2. Player Collision Hull (Radius = 17.0, Height = 78.0)
            if sp_set.show_player_hull_viewport {
                let hull_bottom = cfg.position + Vec3::new(0.0, 0.0, cfg.pad_height);
                let hull_top = hull_bottom + Vec3::new(0.0, 0.0, 78.0);
                let hull_col = [sp_col[0], sp_col[1], sp_col[2], 0.6];

                add_wire_circle(&mut line_verts, hull_bottom, 17.0, hull_col);
                add_wire_circle(&mut line_verts, hull_top, 17.0, hull_col);

                let num_pillars = 4;
                for i in 0..num_pillars {
                    let angle = std::f32::consts::TAU * (i as f32) / (num_pillars as f32);
                    let ox = 17.0 * angle.cos();
                    let oy = 17.0 * angle.sin();
                    let p_bot = hull_bottom + Vec3::new(ox, oy, 0.0);
                    let p_top = hull_top + Vec3::new(ox, oy, 0.0);
                    line_verts.push(LineVertex::new(p_bot.to_array(), hull_col));
                    line_verts.push(LineVertex::new(p_top.to_array(), hull_col));
                }
            }

            // 3. Spawn Direction Vector Arrow
            if sp_set.show_direction_vector_viewport {
                let dir_rad = cfg.yaw_degrees.to_radians();
                let dir_vec = Vec3::new(dir_rad.cos(), dir_rad.sin(), 0.0);
                let start_p = cfg.position + Vec3::new(0.0, 0.0, cfg.pad_height + 40.0);
                let end_p = start_p + dir_vec * 60.0;
                let arrow_col = [1.0, 1.0, 1.0, 0.95];

                line_verts.push(LineVertex::new(start_p.to_array(), arrow_col));
                line_verts.push(LineVertex::new(end_p.to_array(), arrow_col));

                let left_arrow = end_p - dir_vec * 12.0 + Vec3::new(-dir_vec.y, dir_vec.x, 0.0) * 8.0;
                let right_arrow = end_p - dir_vec * 12.0 - Vec3::new(-dir_vec.y, dir_vec.x, 0.0) * 8.0;
                line_verts.push(LineVertex::new(end_p.to_array(), arrow_col));
                line_verts.push(LineVertex::new(left_arrow.to_array(), arrow_col));
                line_verts.push(LineVertex::new(end_p.to_array(), arrow_col));
                line_verts.push(LineVertex::new(right_arrow.to_array(), arrow_col));
            }
        }

        // Interactive Domination Control Point Studio
        if state.show_domination_dialog {
            let dom_set = &state.domination_settings;
            let cfg = &dom_set.active_config;
            let dom_col = [1.0, 0.78, 0.20, 0.95]; // Domination Gold

            // 1. Pedestal Wireframe
            if dom_set.show_pedestal_viewport {
                if let Some(ref ped_b) = dom_set.cached_pedestal_brush {
                    for poly in &ped_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), dom_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), dom_col));
                        }
                    }
                }
            }

            // 2. Capture Cylinder Radius
            if dom_set.show_capture_radius_viewport {
                let cap_bottom = cfg.position;
                let cap_top = cap_bottom + Vec3::new(0.0, 0.0, 72.0);
                let cap_col = [1.0, 0.50, 0.10, 0.55]; // Translucent Amber

                add_wire_circle(&mut line_verts, cap_bottom, cfg.capture_radius_uu, cap_col);
                add_wire_circle(&mut line_verts, cap_top, cfg.capture_radius_uu, cap_col);

                let num_pillars = 6;
                for i in 0..num_pillars {
                    let angle = std::f32::consts::TAU * (i as f32) / (num_pillars as f32);
                    let ox = cfg.capture_radius_uu * angle.cos();
                    let oy = cfg.capture_radius_uu * angle.sin();
                    let p_bot = cap_bottom + Vec3::new(ox, oy, 0.0);
                    let p_top = cap_top + Vec3::new(ox, oy, 0.0);
                    line_verts.push(LineVertex::new(p_bot.to_array(), cap_col));
                    line_verts.push(LineVertex::new(p_top.to_array(), cap_col));
                }
            }

            // 3. Triangle Sightlines & Sprint Vectors
            if dom_set.show_triangle_sightlines_viewport {
                if let Some(ref unr) = state.loaded_unr {
                    let mut other_pts = Vec::new();
                    for act in &unr.actors {
                        let cls = &act.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "controlpoint")
                            || super::overlay_cache::contains_ascii_icase(cls, "dominationpoint")
                        {
                            other_pts.push(act.location);
                        }
                    }

                    let sightline_col = [0.2, 0.85, 1.0, 0.75]; // Cyan vector
                    let start_p = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 28.0);
                    for opt in &other_pts {
                        line_verts.push(LineVertex::new(start_p.to_array(), sightline_col));
                        line_verts.push(LineVertex::new(opt.to_array(), sightline_col));
                    }
                }
            }
        }

        // Interactive Assault Objective Studio
        if state.show_assault_dialog {
            let as_set = &state.assault_settings;
            let cfg = &as_set.active_config;
            let door_col = [1.0, 0.40, 0.20, 0.95]; // Assault Orange

            // 1. Blast Door Frame Wireframe
            if as_set.show_door_viewport {
                if let Some(ref door_b) = as_set.cached_door_brush {
                    for poly in &door_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), door_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), door_col));
                        }
                    }
                }
            }

            // 2. Objective Beacon & Touch Bounds
            if as_set.show_objective_beacon_viewport {
                let beacon_p = cfg.position + Vec3::new(0.0, 0.0, cfg.door_height * 0.5 + 24.0);
                let beacon_col = [1.0, 0.85, 0.10, 0.90]; // Golden beacon

                // Diamond icon around beacon
                let d_sz = 20.0_f32;
                let top_p = beacon_p + Vec3::new(0.0, 0.0, d_sz);
                let bot_p = beacon_p - Vec3::new(0.0, 0.0, d_sz);
                let x_pos = beacon_p + Vec3::new(d_sz, 0.0, 0.0);
                let x_neg = beacon_p - Vec3::new(d_sz, 0.0, 0.0);
                let y_pos = beacon_p + Vec3::new(0.0, d_sz, 0.0);
                let y_neg = beacon_p - Vec3::new(0.0, d_sz, 0.0);

                line_verts.push(LineVertex::new(top_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(x_pos.to_array(), beacon_col));
                line_verts.push(LineVertex::new(top_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(x_neg.to_array(), beacon_col));
                line_verts.push(LineVertex::new(top_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(y_pos.to_array(), beacon_col));
                line_verts.push(LineVertex::new(top_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(y_neg.to_array(), beacon_col));

                line_verts.push(LineVertex::new(bot_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(x_pos.to_array(), beacon_col));
                line_verts.push(LineVertex::new(bot_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(x_neg.to_array(), beacon_col));
                line_verts.push(LineVertex::new(bot_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(y_pos.to_array(), beacon_col));
                line_verts.push(LineVertex::new(bot_p.to_array(), beacon_col));
                line_verts.push(LineVertex::new(y_neg.to_array(), beacon_col));
            }

            // 3. Chain Progression & Forward Spawns
            if as_set.show_chain_links_viewport {
                // Forward spawn vector arrow
                let fwd_start = cfg.position;
                let fwd_end = fwd_start + Vec3::new(cfg.forward_spawn_advance_dist_uu, 0.0, 0.0);
                let fwd_col = [0.2, 1.0, 0.4, 0.85]; // Green advance vector

                line_verts.push(LineVertex::new(fwd_start.to_array(), fwd_col));
                line_verts.push(LineVertex::new(fwd_end.to_array(), fwd_col));
            }
        }

        // Interactive Team Base Radar & Jammer Studio
        if state.show_radar_dialog {
            let rad_set = &state.radar_settings;
            let cfg = &rad_set.active_config;
            let radar_col = if cfg.is_jammer {
                [0.9, 0.2, 0.9, 0.95] // Jammer Magenta
            } else if cfg.team == 0 {
                [1.0, 0.25, 0.25, 0.95] // Red Radar
            } else if cfg.team == 1 {
                [0.25, 0.55, 1.0, 0.95] // Blue Radar
            } else if cfg.team == 2 {
                [0.25, 0.9, 0.35, 0.95] // Green Radar
            } else {
                [1.0, 0.85, 0.2, 0.95] // Gold Radar
            };

            // 1. Mast & Dish Wireframe
            if rad_set.show_antenna_viewport {
                if let Some(ref mast_b) = rad_set.cached_antenna_brush {
                    for poly in &mast_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), radar_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), radar_col));
                        }
                    }
                }
            }

            // 2. Detection Perimeter Circle
            if rad_set.show_detection_radius_viewport {
                let center = cfg.position;
                let r = cfg.detection_radius_uu;
                let segs = 32;
                let circle_col = [radar_col[0], radar_col[1], radar_col[2], 0.65];

                for i in 0..segs {
                    let a1 = std::f32::consts::TAU * (i as f32) / (segs as f32);
                    let a2 = std::f32::consts::TAU * ((i + 1) as f32) / (segs as f32);
                    let p1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), 16.0);
                    let p2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), 16.0);
                    line_verts.push(LineVertex::new(p1.to_array(), circle_col));
                    line_verts.push(LineVertex::new(p2.to_array(), circle_col));
                }
            }

            // 3. Surveillance Sweep Frustum / Sector Cone
            if rad_set.show_sweep_cone_viewport {
                let apex = cfg.position + Vec3::new(0.0, 0.0, cfg.mast_height + 16.0);
                let r = cfg.detection_radius_uu * 0.75;
                let half_cone = (cfg.cone_angle_degrees * 0.5).to_radians();
                let left_ray = Vec3::new(r * (-half_cone).sin(), r * (-half_cone).cos(), 0.0);
                let right_ray = Vec3::new(r * half_cone.sin(), r * half_cone.cos(), 0.0);
                let cone_col = [radar_col[0], radar_col[1], radar_col[2], 0.85];

                line_verts.push(LineVertex::new(apex.to_array(), cone_col));
                line_verts.push(LineVertex::new((apex + left_ray).to_array(), cone_col));
                line_verts.push(LineVertex::new(apex.to_array(), cone_col));
                line_verts.push(LineVertex::new((apex + right_ray).to_array(), cone_col));
            }
        }

        // Interactive Capture The Flag Base & Transit Routes Studio
        if state.show_ctf_base_dialog {
            let ctf_set = &state.ctf_base_settings;
            let cfg = &ctf_set.active_config;
            let team_col = match cfg.team {
                ut99_bsp::CTFTeam::Red => [1.0, 0.25, 0.25, 0.95],
                ut99_bsp::CTFTeam::Blue => [0.25, 0.60, 1.0, 0.95],
                ut99_bsp::CTFTeam::Green => [0.25, 0.90, 0.35, 0.95],
                ut99_bsp::CTFTeam::Gold => [1.0, 0.85, 0.20, 0.95],
            };

            // 1. Stepped Dais Wireframe
            if ctf_set.show_pedestal_viewport {
                if let Some(ref ped_b) = ctf_set.cached_pedestal_brush {
                    for poly in &ped_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), team_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), team_col));
                        }
                    }
                }
            }

            // 2. Defense Perimeter Ring
            if ctf_set.show_defense_radius_viewport {
                let center = cfg.position;
                let r = cfg.defense_radius_uu;
                let segs = 32;
                let ring_col = [team_col[0], team_col[1], team_col[2], 0.60];

                for i in 0..segs {
                    let a1 = std::f32::consts::TAU * (i as f32) / (segs as f32);
                    let a2 = std::f32::consts::TAU * ((i + 1) as f32) / (segs as f32);
                    let p1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), 12.0);
                    let p2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), 12.0);
                    line_verts.push(LineVertex::new(p1.to_array(), ring_col));
                    line_verts.push(LineVertex::new(p2.to_array(), ring_col));
                }
            }

            // 3. Transit Route Vector to Opposing Flag Base
            if ctf_set.show_transit_route_viewport {
                if let Some(ref unr) = state.loaded_unr {
                    for actor in &unr.actors {
                        let cls = &actor.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "flagbase") {
                            let is_opposing = match cfg.team {
                                ut99_bsp::CTFTeam::Red => super::overlay_cache::contains_ascii_icase(cls, "blue"),
                                ut99_bsp::CTFTeam::Blue => super::overlay_cache::contains_ascii_icase(cls, "red"),
                                _ => true,
                            };
                            if is_opposing {
                                let start = cfg.position + Vec3::new(0.0, 0.0, cfg.pedestal_height + 24.0);
                                let end = actor.location + Vec3::new(0.0, 0.0, 24.0);
                                let route_col = [1.0, 0.85, 0.3, 0.85]; // Gold transit line
                                line_verts.push(LineVertex::new(start.to_array(), route_col));
                                line_verts.push(LineVertex::new(end.to_array(), route_col));
                            }
                        }
                    }
                }
            }
        }

        // Interactive Team Cannon & Turret Defense Studio
        if state.show_team_cannon_dialog {
            let tc_set = &state.team_cannon_settings;
            let cfg = &tc_set.active_config;
            let cannon_col = match cfg.team {
                0 => [1.0, 0.25, 0.25, 0.95],
                1 => [0.25, 0.60, 1.0, 0.95],
                2 => [0.25, 0.90, 0.35, 0.95],
                _ => [1.0, 0.85, 0.20, 0.95],
            };

            // 1. Bunker Mount Wireframe
            if tc_set.show_bunker_viewport {
                if let Some(ref bnk_b) = tc_set.cached_bunker_brush {
                    for poly in &bnk_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), cannon_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), cannon_col));
                        }
                    }
                }
            }

            // 2. Targeting Range Dome
            if tc_set.show_range_sphere_viewport {
                let center = cfg.position;
                let r = cfg.max_range_uu;
                let segs = 32;
                let dome_col = [cannon_col[0], cannon_col[1], cannon_col[2], 0.50];

                for i in 0..segs {
                    let a1 = std::f32::consts::TAU * (i as f32) / (segs as f32);
                    let a2 = std::f32::consts::TAU * ((i + 1) as f32) / (segs as f32);
                    let p1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), 0.0);
                    let p2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), 0.0);
                    line_verts.push(LineVertex::new(p1.to_array(), dome_col));
                    line_verts.push(LineVertex::new(p2.to_array(), dome_col));
                }
            }

            // 3. Aim Vector & Pitch Ray
            if tc_set.show_targeting_line_viewport {
                let z_offset = if cfg.is_inverted {
                    -(cfg.bunker_mount_height + 16.0)
                } else {
                    cfg.bunker_mount_height + 16.0
                };
                let muzzle = cfg.position + Vec3::new(0.0, 0.0, z_offset);
                let yaw_rad = cfg.yaw_degrees.to_radians();
                let pitch_rad = cfg.pitch_degrees.to_radians();
                let dir = Vec3::new(
                    yaw_rad.cos() * pitch_rad.cos(),
                    yaw_rad.sin() * pitch_rad.cos(),
                    pitch_rad.sin(),
                ).normalize_or_zero();

                let aim_end = muzzle + dir * (cfg.max_range_uu * 0.65);
                let aim_col = [1.0, 0.4, 0.1, 0.90]; // Tracer Orange
                line_verts.push(LineVertex::new(muzzle.to_array(), aim_col));
                line_verts.push(LineVertex::new(aim_end.to_array(), aim_col));
            }
        }

        // Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
        if state.show_bio_trap_dialog {
            let bio_set = &state.bio_trap_settings;
            let cfg = &bio_set.active_config;
            let bio_col = [0.35, 0.95, 0.20, 0.95]; // Toxic Slime Green

            // 1. Drain Grate Wireframe
            if bio_set.show_grate_viewport {
                if let Some(ref grate_b) = bio_set.cached_grate_brush {
                    for poly in &grate_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), bio_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), bio_col));
                        }
                    }
                }
            }

            // 2. Toxic Splash Footprint Perimeter
            if bio_set.show_splash_radius_viewport {
                let center = cfg.position;
                let r = cfg.splash_radius_uu;
                let segs = 32;
                let splash_col = [0.20, 1.0, 0.15, 0.60];

                for i in 0..segs {
                    let a1 = std::f32::consts::TAU * (i as f32) / (segs as f32);
                    let a2 = std::f32::consts::TAU * ((i + 1) as f32) / (segs as f32);
                    let p1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), 4.0);
                    let p2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), 4.0);
                    line_verts.push(LineVertex::new(p1.to_array(), splash_col));
                    line_verts.push(LineVertex::new(p2.to_array(), splash_col));
                }
            }

            // 3. Adhesion Normal Vector Ray
            if bio_set.show_adhesion_normal_viewport {
                let normal_dir = cfg.surface_normal.normalize_or_zero();
                let ray_start = cfg.position;
                let ray_end = cfg.position + normal_dir * 64.0;
                let ray_col = [0.8, 1.0, 0.3, 0.90]; // Slime Yellow-Green
                line_verts.push(LineVertex::new(ray_start.to_array(), ray_col));
                line_verts.push(LineVertex::new(ray_end.to_array(), ray_col));
            }
        }

        // Interactive Pressure Plate & Proximity Trigger Network Studio
        if state.show_trigger_network_dialog {
            let trig_set = &state.trigger_network_settings;
            let cfg = &trig_set.active_config;
            let plate_col = [0.20, 0.70, 1.0, 0.95]; // Cyan / Logic Blue

            // 1. Pressure Plate Frame Wireframe
            if trig_set.show_plate_viewport {
                if let Some(ref plate_b) = trig_set.cached_plate_brush {
                    for poly in &plate_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), plate_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), plate_col));
                        }
                    }
                }
            }

            // 2. Activation Volume Cylinder (Top and Bottom Rings + Vertical Struts)
            if trig_set.show_activation_volume_viewport {
                let center = cfg.position;
                let r = cfg.activation_radius_uu;
                let h = cfg.activation_height_uu;
                let segs = 32;
                let cyl_col = [0.10, 0.50, 1.0, 0.65];

                for i in 0..segs {
                    let a1 = std::f32::consts::TAU * (i as f32) / (segs as f32);
                    let a2 = std::f32::consts::TAU * ((i + 1) as f32) / (segs as f32);
                    let b1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), 0.0);
                    let b2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), 0.0);
                    let t1 = center + Vec3::new(r * a1.cos(), r * a1.sin(), h);
                    let t2 = center + Vec3::new(r * a2.cos(), r * a2.sin(), h);

                    line_verts.push(LineVertex::new(b1.to_array(), cyl_col));
                    line_verts.push(LineVertex::new(b2.to_array(), cyl_col));
                    line_verts.push(LineVertex::new(t1.to_array(), cyl_col));
                    line_verts.push(LineVertex::new(t2.to_array(), cyl_col));

                    if i % 8 == 0 {
                        line_verts.push(LineVertex::new(b1.to_array(), cyl_col));
                        line_verts.push(LineVertex::new(t1.to_array(), cyl_col));
                    }
                }
            }

            // 3. Event-to-Tag Link Lines to Recipient Actors
            if trig_set.show_event_wiring_lines_viewport && !cfg.event_name.trim().is_empty() {
                if let Some(ref unr) = state.loaded_unr {
                    let wire_col = [1.0, 0.85, 0.10, 0.85]; // Amber / Logic Line
                    let trig_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.plate_thickness_uu + 8.0);
                    for actor in &unr.actors {
                        if actor.tag.eq_ignore_ascii_case(&cfg.event_name) {
                            line_verts.push(LineVertex::new(trig_pos.to_array(), wire_col));
                            line_verts.push(LineVertex::new(actor.location.to_array(), wire_col));
                        }
                    }
                }
            }
        }

        // Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
        if state.show_elevator_dialog {
            let el_set = &state.elevator_settings;
            let cfg = &el_set.active_config;
            let plat_col = [0.95, 0.70, 0.20, 0.95]; // Industrial Yellow / Amber

            // 1. Elevator Platform Brush Wireframe at Key 0 and ghost box at Key 1
            if el_set.show_platform_viewport {
                if let Some(ref plat_b) = el_set.cached_platform_brush {
                    for poly in &plat_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), plat_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), plat_col));
                        }
                    }
                }

                // Ghost wireframe of platform at top travel height (Key 1)
                let hw = cfg.platform_width_uu * 0.5;
                let hl = cfg.platform_length_uu * 0.5;
                let ghost_col = [0.95, 0.80, 0.30, 0.40];

                let corners = [
                    cfg.position + Vec3::new(-hw, -hl, cfg.travel_height_uu),
                    cfg.position + Vec3::new(hw, -hl, cfg.travel_height_uu),
                    cfg.position + Vec3::new(hw, hl, cfg.travel_height_uu),
                    cfg.position + Vec3::new(-hw, hl, cfg.travel_height_uu),
                ];
                for i in 0..4 {
                    let next = (i + 1) % 4;
                    line_verts.push(LineVertex::new(corners[i].to_array(), ghost_col));
                    line_verts.push(LineVertex::new(corners[next].to_array(), ghost_col));
                }
            }

            // 2. Vertical Travel Axis Line
            if el_set.show_travel_axis_viewport {
                let axis_col = [0.30, 0.85, 1.0, 0.90]; // Transit Cyan
                let start = cfg.position;
                let end = cfg.position + Vec3::new(0.0, 0.0, cfg.travel_height_uu);
                line_verts.push(LineVertex::new(start.to_array(), axis_col));
                line_verts.push(LineVertex::new(end.to_array(), axis_col));

                // Small tick marks at bottom and top
                let tick_r = 24.0;
                line_verts.push(LineVertex::new((start + Vec3::new(-tick_r, 0.0, 0.0)).to_array(), axis_col));
                line_verts.push(LineVertex::new((start + Vec3::new(tick_r, 0.0, 0.0)).to_array(), axis_col));
                line_verts.push(LineVertex::new((end + Vec3::new(-tick_r, 0.0, 0.0)).to_array(), axis_col));
                line_verts.push(LineVertex::new((end + Vec3::new(tick_r, 0.0, 0.0)).to_array(), axis_col));
            }

            // 3. Bot Waypoints (LiftCenter diamond and LiftExit diamonds with link lines)
            if el_set.show_bot_waypoints_viewport {
                let center_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.platform_thickness_uu + 20.0);
                let exit0_pos = cfg.position + Vec3::new(cfg.platform_width_uu * 0.5 + 48.0, 0.0, 16.0);
                let exit1_pos = cfg.position + Vec3::new(cfg.platform_width_uu * 0.5 + 48.0, 0.0, cfg.travel_height_uu + 16.0);

                let center_col = [0.20, 1.00, 0.30, 0.95]; // LiftCenter Green

                // LiftCenter Diamond
                let d_r = 12.0;
                line_verts.push(LineVertex::new((center_pos + Vec3::new(-d_r, 0.0, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(0.0, d_r, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(0.0, d_r, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(d_r, 0.0, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(d_r, 0.0, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(0.0, -d_r, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(0.0, -d_r, 0.0)).to_array(), center_col));
                line_verts.push(LineVertex::new((center_pos + Vec3::new(-d_r, 0.0, 0.0)).to_array(), center_col));

                // Connection Lines from Center to Exits
                let link_col = [0.40, 1.00, 0.60, 0.65];
                line_verts.push(LineVertex::new(center_pos.to_array(), link_col));
                line_verts.push(LineVertex::new(exit0_pos.to_array(), link_col));

                let top_center_pos = center_pos + Vec3::new(0.0, 0.0, cfg.travel_height_uu);
                line_verts.push(LineVertex::new(top_center_pos.to_array(), link_col));
                line_verts.push(LineVertex::new(exit1_pos.to_array(), link_col));
            }
        }

        // Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
        if state.show_zone_velocity_dialog {
            let z_set = &state.zone_velocity_settings;
            let cfg = &z_set.active_config;

            // 1. Zone Portal Sheet Brush Wireframe
            let portal_col = match cfg.medium {
                ut99_bsp::ZoneMediumType::SubmergedWater => [0.20, 0.70, 1.00, 0.90],
                ut99_bsp::ZoneMediumType::CorrosiveAcidSlime => [0.20, 1.00, 0.20, 0.90],
                ut99_bsp::ZoneMediumType::MoltenLava => [1.00, 0.40, 0.10, 0.90],
                ut99_bsp::ZoneMediumType::DeepSpaceVacuum => [0.70, 0.30, 1.00, 0.90],
                ut99_bsp::ZoneMediumType::AtmosphericAir => [0.90, 0.90, 0.95, 0.80],
            };

            if z_set.show_portal_sheet_viewport {
                if let Some(ref sheet_b) = z_set.cached_portal_brush {
                    for poly in &sheet_b.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new(poly.vertices[i].to_array(), portal_col));
                            line_verts.push(LineVertex::new(poly.vertices[next].to_array(), portal_col));
                        }
                    }
                }
            }

            // 2. Velocity Vectors (Multiple directional arrows originating from portal base)
            if z_set.show_velocity_vectors_viewport {
                let speed = cfg.zone_velocity.length();
                if speed > 1.0 {
                    let dir = cfg.zone_velocity.normalize();
                    let arrow_len = (speed * 0.4).clamp(40.0, 300.0);
                    let vec_col = [1.00, 0.85, 0.15, 0.95]; // Bright Gold / Velocity
                    let offsets = [
                        Vec3::new(0.0, 0.0, 0.0),
                        Vec3::new(-cfg.portal_width_uu * 0.3, 0.0, 0.0),
                        Vec3::new(cfg.portal_width_uu * 0.3, 0.0, 0.0),
                    ];

                    for offset in &offsets {
                        let start = cfg.position + *offset;
                        let end = start + dir * arrow_len;
                        line_verts.push(LineVertex::new(start.to_array(), vec_col));
                        line_verts.push(LineVertex::new(end.to_array(), vec_col));

                        // Arrowhead
                        let side = dir.cross(Vec3::Z).normalize_or_zero();
                        let head_len = 16.0;
                        let head_p1 = end - dir * head_len + side * (head_len * 0.5);
                        let head_p2 = end - dir * head_len - side * (head_len * 0.5);
                        line_verts.push(LineVertex::new(end.to_array(), vec_col));
                        line_verts.push(LineVertex::new(head_p1.to_array(), vec_col));
                        line_verts.push(LineVertex::new(end.to_array(), vec_col));
                        line_verts.push(LineVertex::new(head_p2.to_array(), vec_col));
                    }
                }
            }

            // 3. Drift Trajectory Streamline (Line extending full zone length)
            if z_set.show_drift_trajectory_viewport {
                let speed = cfg.zone_velocity.length();
                if speed > 1.0 {
                    let dir = cfg.zone_velocity.normalize();
                    let stream_col = [0.35, 0.95, 1.00, 0.65]; // Cyan Streamline
                    let start = cfg.position;
                    let end = start + dir * cfg.zone_length_uu;
                    line_verts.push(LineVertex::new(start.to_array(), stream_col));
                    line_verts.push(LineVertex::new(end.to_array(), stream_col));

                    // End boundary tick
                    let side = dir.cross(Vec3::Z).normalize_or_zero();
                    let tick_w = 48.0;
                    line_verts.push(LineVertex::new((end - side * tick_w).to_array(), stream_col));
                    line_verts.push(LineVertex::new((end + side * tick_w).to_array(), stream_col));
                }
            }
        }

        // Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
        if state.show_hazard_barrel_dialog {
            let b_set = &state.hazard_barrel_settings;
            let cfg = &b_set.active_config;

            // 1. Barrel Mesh Wireframe (Hazard Orange)
            if b_set.show_barrel_mesh_viewport {
                if let Some(ref brush) = b_set.cached_barrel_brush {
                    let col = [1.00, 0.55, 0.15, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Blast Radius Sphere (Equatorial ring & Meridian ring in Red/Orange)
            if b_set.show_blast_radius_viewport {
                let r = cfg.blast_radius_uu;
                let segs = 32;
                let col = [1.00, 0.25, 0.15, 0.80];
                for i in 0..segs {
                    let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;

                    // XY circle
                    let p0 = cfg.position + Vec3::new(a0.cos() * r, a0.sin() * r, 0.0);
                    let p1 = cfg.position + Vec3::new(a1.cos() * r, a1.sin() * r, 0.0);
                    line_verts.push(LineVertex::new(p0.to_array(), col));
                    line_verts.push(LineVertex::new(p1.to_array(), col));

                    // XZ circle
                    let p2 = cfg.position + Vec3::new(a0.cos() * r, 0.0, a0.sin() * r);
                    let p3 = cfg.position + Vec3::new(a1.cos() * r, 0.0, a1.sin() * r);
                    line_verts.push(LineVertex::new(p2.to_array(), col));
                    line_verts.push(LineVertex::new(p3.to_array(), col));
                }
            }

            // 3. Toxic Spill Disc (Green ground ring)
            if b_set.show_toxic_spill_viewport && cfg.is_toxic_spill {
                let r = cfg.blast_radius_uu * 0.75;
                let segs = 24;
                let col = [0.20, 1.00, 0.20, 0.75];
                for i in 0..segs {
                    let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;
                    let p0 = cfg.position + Vec3::new(a0.cos() * r, a0.sin() * r, 2.0);
                    let p1 = cfg.position + Vec3::new(a1.cos() * r, a1.sin() * r, 2.0);
                    line_verts.push(LineVertex::new(p0.to_array(), col));
                    line_verts.push(LineVertex::new(p1.to_array(), col));
                }
            }

            // 4. Chain Detonation Links
            if b_set.show_chain_links_viewport {
                if let Some(ref unr) = state.loaded_unr {
                    let chain_col = [1.00, 0.90, 0.20, 0.90];
                    for actor in &unr.actors {
                        let cls = &actor.class_name;
                        if super::overlay_cache::contains_ascii_icase(cls, "barrel")
                            || super::overlay_cache::contains_ascii_icase(cls, "keg")
                            || super::overlay_cache::contains_ascii_icase(cls, "canister")
                        {
                            let dist = (actor.location - cfg.position).length();
                            if dist > 1.0 && dist <= cfg.blast_radius_uu {
                                line_verts.push(LineVertex::new(cfg.position.to_array(), chain_col));
                                line_verts.push(LineVertex::new(actor.location.to_array(), chain_col));
                            }
                        }
                    }
                }
            }
        }

        // Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
        if state.show_special_lit_dialog {
            let l_set = &state.special_lit_settings;
            let cfg = &l_set.active_config;

            // 1. Fixture Brush Wireframe
            if l_set.show_fixture_brush_viewport {
                if let Some(ref brush) = l_set.cached_fixture_brush {
                    let col = [0.45, 0.85, 1.00, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. SpecialLit Spherical Radius Rings
            if l_set.show_special_radius_viewport {
                let r = cfg.light_radius_uu;
                let segs = 32;
                let col = [1.00, 0.80, 0.20, 0.75];
                for i in 0..segs {
                    let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;

                    // XY circle
                    let p0 = cfg.position + Vec3::new(a0.cos() * r, a0.sin() * r, 0.0);
                    let p1 = cfg.position + Vec3::new(a1.cos() * r, a1.sin() * r, 0.0);
                    line_verts.push(LineVertex::new(p0.to_array(), col));
                    line_verts.push(LineVertex::new(p1.to_array(), col));

                    // YZ circle
                    let p2 = cfg.position + Vec3::new(0.0, a0.cos() * r, a0.sin() * r);
                    let p3 = cfg.position + Vec3::new(0.0, a1.cos() * r, a1.sin() * r);
                    line_verts.push(LineVertex::new(p2.to_array(), col));
                    line_verts.push(LineVertex::new(p3.to_array(), col));
                }
            }

            // 3. Surface Rays to PF_SpecialLit Surfaces
            if l_set.show_surface_rays_viewport {
                let ray_col = [0.30, 1.00, 0.60, 0.70];
                for brush in &state.world_brushes {
                    for poly in &brush.polys {
                        if (poly.flags & ut99_bsp::PF_SPECIAL_LIT) != 0 {
                            let center = if !poly.vertices.is_empty() {
                                poly.vertices.iter().copied().sum::<Vec3>() / poly.vertices.len() as f32 + brush.location
                            } else {
                                brush.location
                            };
                            let dist = (center - cfg.position).length();
                            if dist <= cfg.light_radius_uu {
                                line_verts.push(LineVertex::new(cfg.position.to_array(), ray_col));
                                line_verts.push(LineVertex::new(center.to_array(), ray_col));
                            }
                        }
                    }
                }
            }
        }

        // Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
        if state.show_sniper_vantage_dialog {
            let s_set = &state.sniper_vantage_settings;
            let cfg = &s_set.active_config;

            // 1. Parapet Balcony Wireframe (Teal / Cyan)
            if s_set.show_parapet_mesh_viewport {
                if let Some(ref brush) = s_set.cached_parapet_brush {
                    let col = [0.15, 0.85, 0.95, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Sightline Look Vector & Firing Frustum Cone
            if s_set.show_sightline_cone_viewport {
                let look_dir = cfg.look_direction.normalize_or_zero();
                let eye_pos = cfg.position + Vec3::new(0.0, 0.0, 64.0);
                let target_pos = eye_pos + look_dir * cfg.max_effective_range_uu;

                // Main sightline ray (Cyan)
                let ray_col = [0.20, 0.95, 1.00, 1.00];
                line_verts.push(LineVertex::new(eye_pos.to_array(), ray_col));
                line_verts.push(LineVertex::new(target_pos.to_array(), ray_col));

                // Cone perimeter rays (4 boundary edges based on coverage angle)
                let half_cone_rad = (cfg.coverage_angle_deg * 0.5).to_radians();
                let up = if look_dir.z.abs() > 0.9 { Vec3::X } else { Vec3::Z };
                let right = look_dir.cross(up).normalize_or_zero();
                let true_up = right.cross(look_dir).normalize_or_zero();

                let cone_col = [0.10, 0.75, 0.90, 0.65];
                let r = cfg.max_effective_range_uu * half_cone_rad.tan();
                let segs = 16;
                for i in 0..segs {
                    let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;

                    let c0 = target_pos + right * (a0.cos() * r) + true_up * (a0.sin() * r);
                    let c1 = target_pos + right * (a1.cos() * r) + true_up * (a1.sin() * r);

                    // Perimeter ring
                    line_verts.push(LineVertex::new(c0.to_array(), cone_col));
                    line_verts.push(LineVertex::new(c1.to_array(), cone_col));

                    // Radiating ray every 4 segments
                    if i % 4 == 0 {
                        line_verts.push(LineVertex::new(eye_pos.to_array(), cone_col));
                        line_verts.push(LineVertex::new(c0.to_array(), cone_col));
                    }
                }
            }

            // 3. Headshot Clearance Plane (Golden Amber plane above parapet)
            if s_set.show_headshot_plane_viewport {
                let headshot_z = cfg.position.z + 64.0;
                let hs_col = [1.00, 0.85, 0.20, 0.85];
                let w = cfg.parapet_width_uu * 0.5;
                let d = cfg.parapet_depth_uu * 0.5;

                let p0 = Vec3::new(cfg.position.x - w, cfg.position.y - d, headshot_z);
                let p1 = Vec3::new(cfg.position.x + w, cfg.position.y - d, headshot_z);
                let p2 = Vec3::new(cfg.position.x + w, cfg.position.y + d, headshot_z);
                let p3 = Vec3::new(cfg.position.x - w, cfg.position.y + d, headshot_z);

                line_verts.push(LineVertex::new(p0.to_array(), hs_col));
                line_verts.push(LineVertex::new(p1.to_array(), hs_col));
                line_verts.push(LineVertex::new(p1.to_array(), hs_col));
                line_verts.push(LineVertex::new(p2.to_array(), hs_col));
                line_verts.push(LineVertex::new(p2.to_array(), hs_col));
                line_verts.push(LineVertex::new(p3.to_array(), hs_col));
                line_verts.push(LineVertex::new(p3.to_array(), hs_col));
                line_verts.push(LineVertex::new(p0.to_array(), hs_col));
            }
        }

        // Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
        if state.show_secret_door_dialog {
            let d_set = &state.secret_door_settings;
            let cfg = &d_set.active_config;

            // 1. Door Mesh Wireframe (Key 0 in Purple/Magenta)
            if d_set.show_door_mesh_viewport {
                if let Some(ref brush) = d_set.cached_door_brush {
                    let col = [0.85, 0.25, 0.95, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Open Ghost Wireframe & Trajectory Path (Key 1 in Light Magenta)
            if d_set.show_displacement_path_viewport {
                if let Some(ref brush) = d_set.cached_door_brush {
                    let ghost_col = [0.95, 0.45, 1.00, 0.65];
                    let open_loc = brush.location + cfg.travel_displacement;
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + open_loc).to_array(), ghost_col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + open_loc).to_array(), ghost_col));
                        }
                    }

                    // Trajectory Arrow connecting Key 0 center to Key 1 center
                    let traj_col = [1.00, 0.60, 0.90, 1.00];
                    let center_k0 = brush.location + Vec3::new(0.0, 0.0, cfg.door_height_uu * 0.5);
                    let center_k1 = center_k0 + cfg.travel_displacement;
                    line_verts.push(LineVertex::new(center_k0.to_array(), traj_col));
                    line_verts.push(LineVertex::new(center_k1.to_array(), traj_col));
                }
            }

            // 3. Trigger Activation Radius Circle
            if d_set.show_trigger_radius_viewport {
                let trig_r = (cfg.door_width_uu * 0.75).max(48.0);
                let segs = 24;
                let trig_col = [0.75, 0.35, 0.95, 0.80];
                for i in 0..segs {
                    let a0 = (i as f32) / (segs as f32) * std::f32::consts::TAU;
                    let a1 = ((i + 1) as f32) / (segs as f32) * std::f32::consts::TAU;
                    let p0 = cfg.position + Vec3::new(a0.cos() * trig_r, a0.sin() * trig_r, 4.0);
                    let p1 = cfg.position + Vec3::new(a1.cos() * trig_r, a1.sin() * trig_r, 4.0);
                    line_verts.push(LineVertex::new(p0.to_array(), trig_col));
                    line_verts.push(LineVertex::new(p1.to_array(), trig_col));
                }
            }
        }

        // Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
        if state.show_teleporter_arch_dialog {
            let t_set = &state.teleporter_arch_settings;
            let cfg = &t_set.active_config;

            // 1. Archway Mesh Wireframe (Azure Blue)
            if t_set.show_arch_mesh_viewport {
                if let Some(ref brush) = t_set.cached_arch_brush {
                    let col = [0.20, 0.70, 1.00, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Planar Vortex Swirl Sheet (Bright Cyan / Violet with criss-cross diagonals)
            if t_set.show_vortex_sheet_viewport {
                let w = cfg.arch_width_uu - cfg.pillar_thickness_uu * 2.0;
                let h = cfg.arch_height_uu - cfg.pillar_thickness_uu;
                let half_w = w * 0.5;
                let v_col = [0.40, 0.85, 1.00, 0.85];

                let p0 = cfg.position + Vec3::new(0.0, -half_w, 0.0);
                let p1 = cfg.position + Vec3::new(0.0, half_w, 0.0);
                let p2 = cfg.position + Vec3::new(0.0, half_w, h);
                let p3 = cfg.position + Vec3::new(0.0, -half_w, h);

                // Perimeter
                line_verts.push(LineVertex::new(p0.to_array(), v_col));
                line_verts.push(LineVertex::new(p1.to_array(), v_col));
                line_verts.push(LineVertex::new(p1.to_array(), v_col));
                line_verts.push(LineVertex::new(p2.to_array(), v_col));
                line_verts.push(LineVertex::new(p2.to_array(), v_col));
                line_verts.push(LineVertex::new(p3.to_array(), v_col));
                line_verts.push(LineVertex::new(p3.to_array(), v_col));
                line_verts.push(LineVertex::new(p0.to_array(), v_col));

                // Diagonals (X-pattern swirl)
                line_verts.push(LineVertex::new(p0.to_array(), v_col));
                line_verts.push(LineVertex::new(p2.to_array(), v_col));
                line_verts.push(LineVertex::new(p1.to_array(), v_col));
                line_verts.push(LineVertex::new(p3.to_array(), v_col));
            }

            // 3. Exit Trajectory Launch Arrow
            if t_set.show_exit_trajectory_viewport && cfg.b_changes_velocity {
                let dir = cfg.facing_direction.normalize_or_zero();
                let arrow_len = (cfg.exit_boost_velocity * 0.5).max(64.0);
                let start_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.arch_height_uu * 0.5);
                let end_pos = start_pos + dir * arrow_len;
                let arrow_col = [0.50, 0.95, 1.00, 1.00];

                line_verts.push(LineVertex::new(start_pos.to_array(), arrow_col));
                line_verts.push(LineVertex::new(end_pos.to_array(), arrow_col));
            }
        }

        // Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
        if state.show_fluid_basin_dialog {
            let b_set = &state.fluid_basin_settings;
            let cfg = &b_set.active_config;

            // 1. Excavated Basin Trough Wireframe
            if b_set.show_basin_mesh_viewport {
                if let Some(ref brush) = b_set.cached_basin_brush {
                    let col = if cfg.dps > 50.0 {
                        [1.00, 0.40, 0.15, 0.90] // Lava orange
                    } else if cfg.dps > 0.0 {
                        [0.30, 0.85, 0.20, 0.90] // Acid green
                    } else {
                        [0.20, 0.60, 0.95, 0.90] // Coolant blue
                    };

                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Planar Fluid Surface Grid (at Z = 0)
            if b_set.show_fluid_surface_viewport {
                let half_w = cfg.width_uu * 0.5;
                let half_l = cfg.length_uu * 0.5;
                let surf_col = if cfg.dps > 50.0 {
                    [1.00, 0.55, 0.10, 0.75]
                } else if cfg.dps > 0.0 {
                    [0.40, 0.95, 0.15, 0.75]
                } else {
                    [0.30, 0.75, 1.00, 0.75]
                };

                let p0 = cfg.position + Vec3::new(-half_w, -half_l, 0.0);
                let p1 = cfg.position + Vec3::new(half_w, -half_l, 0.0);
                let p2 = cfg.position + Vec3::new(half_w, half_l, 0.0);
                let p3 = cfg.position + Vec3::new(-half_w, half_l, 0.0);

                line_verts.push(LineVertex::new(p0.to_array(), surf_col));
                line_verts.push(LineVertex::new(p1.to_array(), surf_col));
                line_verts.push(LineVertex::new(p1.to_array(), surf_col));
                line_verts.push(LineVertex::new(p2.to_array(), surf_col));
                line_verts.push(LineVertex::new(p2.to_array(), surf_col));
                line_verts.push(LineVertex::new(p3.to_array(), surf_col));
                line_verts.push(LineVertex::new(p3.to_array(), surf_col));
                line_verts.push(LineVertex::new(p0.to_array(), surf_col));
            }

            // 3. Escape Ramp / Ladder Indicator
            if b_set.show_escape_ramp_viewport && cfg.has_escape_ladder_or_ramp {
                let ramp_col = [1.00, 0.90, 0.20, 0.95]; // Yellow
                let half_w = cfg.width_uu * 0.5;
                let bot = cfg.position + Vec3::new(half_w * 0.5, 0.0, -cfg.depth_uu);
                let top = cfg.position + Vec3::new(half_w, 0.0, 0.0);

                line_verts.push(LineVertex::new(bot.to_array(), ramp_col));
                line_verts.push(LineVertex::new(top.to_array(), ramp_col));
            }
        }

        // Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
        if state.show_alarm_beacon_dialog {
            let a_set = &state.alarm_beacon_settings;
            let cfg = &a_set.active_config;

            // 1. Beacon Mount Wireframe
            if a_set.show_beacon_mesh_viewport {
                if let Some(ref brush) = a_set.cached_beacon_brush {
                    let col = [1.00, 0.25, 0.20, 0.95];
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Strobe Pulse / Rotor Beam (Spherical / Star Radii)
            if a_set.show_strobe_pulse_viewport {
                let r = cfg.light_radius_uu * 0.5;
                let beam_col = [1.00, 0.45, 0.15, 0.80];
                let pos = cfg.position + Vec3::new(0.0, 0.0, 16.0);
                add_wire_circle(&mut line_verts, pos, r, beam_col);
            }

            // 3. Audible Siren Range Circle (at Z = cfg.position.z)
            if a_set.show_audible_radius_viewport && cfg.sound_volume_db > 0.0 {
                let aud_r = cfg.sound_volume_db * 20.0;
                let aud_col = [1.00, 0.65, 0.25, 0.50];
                add_wire_circle(&mut line_verts, cfg.position, aud_r, aud_col);
            }
        }

        // Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
        if state.show_supply_locker_dialog {
            let l_set = &state.supply_locker_settings;
            let cfg = &l_set.active_config;

            // 1. Cabinet Recessed Housing Wireframe
            if l_set.show_locker_mesh_viewport {
                if let Some(ref brush) = l_set.cached_locker_brush {
                    let col = [0.85, 0.75, 0.30, 0.95]; // Gold/Bronze metal
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Contained Items / Shelf Indicators
            if l_set.show_contained_items_viewport {
                let shelf_col = [0.95, 0.85, 0.40, 0.80];
                let half_w = cfg.locker_width_uu * 0.5;
                let shelf_dz = cfg.locker_height_uu / (cfg.shelf_count as f32 + 1.0);

                for s in 1..=cfg.shelf_count {
                    let z = cfg.position.z + (s as f32) * shelf_dz;
                    let p0 = cfg.position + Vec3::new(-cfg.locker_depth_uu * 0.5, -half_w, z - cfg.position.z);
                    let p1 = cfg.position + Vec3::new(-cfg.locker_depth_uu * 0.5, half_w, z - cfg.position.z);
                    line_verts.push(LineVertex::new(p0.to_array(), shelf_col));
                    line_verts.push(LineVertex::new(p1.to_array(), shelf_col));
                }
            }

            // 3. Facing Direction Vector Arrow
            if l_set.show_shelf_clearance_viewport {
                let dir = cfg.facing_direction.normalize_or_zero();
                let arrow_col = [1.00, 0.90, 0.20, 1.00];
                let start_pos = cfg.position + Vec3::new(0.0, 0.0, cfg.locker_height_uu * 0.5);
                let end_pos = start_pos + dir * 48.0;

                line_verts.push(LineVertex::new(start_pos.to_array(), arrow_col));
                line_verts.push(LineVertex::new(end_pos.to_array(), arrow_col));
            }
        }

        // Interactive Glass Skylight, Observation Dome & Structural Truss Studio
        if state.show_skylight_dialog {
            let s_set = &state.skylight_settings;
            let cfg = &s_set.active_config;

            // 1. Glazed Frame Wireframe
            if s_set.show_skylight_mesh_viewport {
                if let Some(ref brush) = s_set.cached_skylight_brush {
                    let col = [0.30, 0.75, 1.00, 0.95]; // Azure glass
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Sunlight Shaft / Lux Ray Cone
            if s_set.show_sunlight_shaft_viewport {
                let sun_col = [1.00, 0.95, 0.50, 0.75]; // Warm solar ray
                let origin = cfg.position + Vec3::new(0.0, 0.0, cfg.height_or_rise_uu * 0.5);
                let shaft_depth = 256.0;

                let corners = [
                    cfg.position + Vec3::new(-cfg.length_uu * 0.5, -cfg.width_uu * 0.5, 0.0),
                    cfg.position + Vec3::new(cfg.length_uu * 0.5, -cfg.width_uu * 0.5, 0.0),
                    cfg.position + Vec3::new(cfg.length_uu * 0.5, cfg.width_uu * 0.5, 0.0),
                    cfg.position + Vec3::new(-cfg.length_uu * 0.5, cfg.width_uu * 0.5, 0.0),
                ];

                for c in &corners {
                    line_verts.push(LineVertex::new(origin.to_array(), sun_col));
                    line_verts.push(LineVertex::new((*c - Vec3::new(0.0, 0.0, shaft_depth)).to_array(), sun_col));
                }
            }

            // 3. Structural Mullion Ribs
            if s_set.show_mullion_truss_viewport && cfg.rib_count >= 2 {
                let rib_col = [0.50, 0.85, 1.00, 0.80];
                let dz = cfg.height_or_rise_uu;
                let half_w = cfg.width_uu * 0.5;
                let step_x = cfg.length_uu / (cfg.rib_count as f32 + 1.0);

                for r in 1..=cfg.rib_count {
                    let x = -cfg.length_uu * 0.5 + (r as f32) * step_x;
                    let p0 = cfg.position + Vec3::new(x, -half_w, 0.0);
                    let p_apex = cfg.position + Vec3::new(x, 0.0, dz);
                    let p1 = cfg.position + Vec3::new(x, half_w, 0.0);

                    line_verts.push(LineVertex::new(p0.to_array(), rib_col));
                    line_verts.push(LineVertex::new(p_apex.to_array(), rib_col));
                    line_verts.push(LineVertex::new(p_apex.to_array(), rib_col));
                    line_verts.push(LineVertex::new(p1.to_array(), rib_col));
                }
            }
        }

        // Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
        if state.show_vent_duct_dialog {
            let v_set = &state.vent_duct_settings;
            let cfg = &v_set.active_config;

            // 1. Conduit Casing Wireframe
            if v_set.show_duct_mesh_viewport {
                if let Some(ref brush) = v_set.cached_duct_brush {
                    let col = [0.25, 0.85, 0.55, 0.95]; // Mint green conduit
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Air Current Streamline
            if v_set.show_air_current_viewport && cfg.air_draft_velocity > 0.0 {
                let stream_col = [0.40, 1.00, 0.70, 0.85];
                let start_pos = cfg.position;
                let end_pos = cfg.position + Vec3::new(cfg.duct_length_uu, 0.0, 0.0);

                line_verts.push(LineVertex::new(start_pos.to_array(), stream_col));
                line_verts.push(LineVertex::new(end_pos.to_array(), stream_col));
            }

            // 3. Player Crouch Clearance Box
            if v_set.show_crouch_clearance_viewport {
                let crouch_col = [0.85, 1.00, 0.40, 0.80];
                let internal_h = cfg.duct_height_uu - cfg.wall_thickness_uu * 2.0;
                let internal_w = cfg.duct_width_uu - cfg.wall_thickness_uu * 2.0;

                let c0 = cfg.position + Vec3::new(16.0, -internal_w * 0.5, -internal_h * 0.5);
                let c1 = cfg.position + Vec3::new(16.0, internal_w * 0.5, internal_h * 0.5);
                add_wire_box(&mut line_verts, c0, c1, crouch_col);
            }
        }

        // Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
        if state.show_piston_dialog {
            let p_set = &state.piston_settings;
            let cfg = &p_set.active_config;

            // 1. Piston Head & Shaft Wireframe
            if p_set.show_piston_mesh_viewport {
                if let Some(ref brush) = p_set.cached_piston_brush {
                    let col = [1.0, 0.55, 0.20, 0.95]; // Industrial Orange
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Stroke Travel Axis & Ghost Head
            if p_set.show_stroke_trajectory_viewport {
                let stroke_dir = cfg.orientation.direction_vector();
                let axis_col = [1.0, 0.90, 0.20, 0.90];
                let start_pos = cfg.position;
                let end_pos = cfg.position + stroke_dir * cfg.stroke_length_uu;

                line_verts.push(LineVertex::new(start_pos.to_array(), axis_col));
                line_verts.push(LineVertex::new(end_pos.to_array(), axis_col));

                // Ghost box at full extension
                let half_ext = Vec3::new(cfg.head_width_uu * 0.5, cfg.head_depth_uu * 0.5, cfg.head_thickness_uu * 0.5);
                add_wire_box(&mut line_verts, end_pos, half_ext, [1.0, 0.85, 0.30, 0.65]);
            }

            // 3. Squish Hazard Danger Perimeter
            if p_set.show_squish_hazard_viewport && cfg.squish_damage >= 100 {
                let stroke_dir = cfg.orientation.direction_vector();
                let end_pos = cfg.position + stroke_dir * cfg.stroke_length_uu;
                let hazard_half_ext = Vec3::new(cfg.head_width_uu * 0.5 + 16.0, cfg.head_depth_uu * 0.5 + 16.0, cfg.head_thickness_uu * 0.5 + 8.0);
                add_wire_box(&mut line_verts, end_pos, hazard_half_ext, [1.0, 0.20, 0.20, 0.85]);
            }
        }

        // Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
        if state.show_jump_boots_dialog {
            let b_set = &state.jump_boots_settings;
            let cfg = &b_set.active_config;

            // 1. Cradle Wireframe
            if b_set.show_cradle_mesh_viewport {
                if let Some(ref brush) = b_set.cached_cradle_brush {
                    let col = [0.30, 0.75, 1.00, 0.95]; // Cyan / Electric Blue
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), col));
                        }
                    }
                }
            }

            // 2. Parabolic Vault Arc & Apex Ring
            if b_set.show_ballistic_arc_viewport {
                let metrics = &b_set.cached_metrics;
                let arc_col = [0.45, 0.90, 1.00, 0.90];
                let g = cfg.world_gravity_z.abs().max(50.0);
                let vz = cfg.launch_boost_vz;
                let total_t = metrics.total_hang_time_s.max(0.1);
                let steps = 24;

                let target_dir = if cfg.target_platform_offset.length_squared() > 1.0 {
                    Vec3::new(cfg.target_platform_offset.x, cfg.target_platform_offset.y, 0.0).normalize_or_zero()
                } else {
                    Vec3::new(1.0, 0.0, 0.0)
                };

                let mut prev_pt = cfg.position;
                for step in 1..=steps {
                    let t = (step as f32 / steps as f32) * total_t;
                    let horiz_dist = cfg.forward_velocity_vx * t;
                    let vert_dist = vz * t - 0.5 * g * t * t;
                    let curr_pt = cfg.position + target_dir * horiz_dist + Vec3::new(0.0, 0.0, vert_dist);

                    line_verts.push(LineVertex::new(prev_pt.to_array(), arc_col));
                    line_verts.push(LineVertex::new(curr_pt.to_array(), arc_col));
                    prev_pt = curr_pt;
                }

                // Apex ring
                let apex_t = vz / g;
                let apex_horiz = cfg.forward_velocity_vx * apex_t;
                let apex_pt = cfg.position + target_dir * apex_horiz + Vec3::new(0.0, 0.0, metrics.peak_apex_height_uu);
                add_wire_circle(&mut line_verts, apex_pt, 24.0, [1.0, 0.90, 0.30, 0.85]);
            }

            // 3. Target Platform Clearance Box
            if b_set.show_target_platform_viewport {
                let target_center = cfg.position + cfg.target_platform_offset;
                let half_ext = cfg.target_platform_size * 0.5;
                add_wire_box(&mut line_verts, target_center, half_ext, [0.35, 1.00, 0.55, 0.80]);
            }
        }

        // Interactive Force Field, Energy Barrier & Power Core Overload Studio
        if state.show_force_field_dialog {
            let f_set = &state.force_field_settings;
            let cfg = &f_set.active_config;
            let field_col = cfg.color_palette.to_rgba();

            // 1. Field Slab & Flux Lines
            if f_set.show_field_slab_viewport {
                if let Some(ref brush) = f_set.cached_field_brush {
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), field_col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), field_col));
                        }
                    }
                }
            }

            // 2. Emitter Pylons Wireframe
            if f_set.show_pylons_viewport {
                let pylon_col = [0.85, 0.70, 0.30, 0.90]; // Metallic Amber
                for pylon in &f_set.cached_pylon_brushes {
                    for poly in &pylon.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + pylon.location).to_array(), pylon_col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + pylon.location).to_array(), pylon_col));
                        }
                    }
                }
            }

            // 3. Power Generator Vector Link
            if f_set.show_generator_link_viewport && cfg.generator_linked {
                let link_col = [1.00, 0.30, 0.80, 0.85]; // Neon Magenta
                let field_center = cfg.position + Vec3::new(0.0, 0.0, cfg.height_uu * 0.5);

                let mut gen_pos = field_center + Vec3::new(0.0, 160.0, 0.0);
                if let Some(ref unr) = state.loaded_unr {
                    for act in &unr.actors {
                        if act.tag == cfg.generator_tag {
                            gen_pos = act.location;
                            break;
                        }
                    }
                }

                line_verts.push(LineVertex::new(field_center.to_array(), link_col));
                line_verts.push(LineVertex::new(gen_pos.to_array(), link_col));
                add_wire_box(&mut line_verts, gen_pos, Vec3::splat(16.0), link_col);
            }
        }

        // Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
        if state.show_reactor_core_dialog {
            let r_set = &state.reactor_core_settings;
            let cfg = &r_set.active_config;

            // 1. Plasma Core Wireframe
            if r_set.show_core_brush_viewport {
                let core_col = [0.75, 0.25, 1.00, 0.95]; // Purple Plasma
                if let Some(ref brush) = r_set.cached_core_brush {
                    for poly in &brush.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + brush.location).to_array(), core_col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + brush.location).to_array(), core_col));
                        }
                    }
                }
            }

            // 2. Concentric Kinetic Rings Wireframe
            if r_set.show_rings_viewport {
                let ring_colors = [
                    [0.25, 0.85, 1.00, 0.90], // Cyan Outer Ring
                    [1.00, 0.80, 0.20, 0.90], // Gold Mid Ring
                ];
                for (idx, ring) in r_set.cached_ring_brushes.iter().enumerate() {
                    let col = ring_colors[idx % ring_colors.len()];
                    for poly in &ring.polys {
                        let len = poly.vertices.len();
                        for i in 0..len {
                            let next = (i + 1) % len;
                            line_verts.push(LineVertex::new((poly.vertices[i] + ring.location).to_array(), col));
                            line_verts.push(LineVertex::new((poly.vertices[next] + ring.location).to_array(), col));
                        }
                    }
                }
            }

            // 3. Spherical Hazard Radius Boundary
            if r_set.show_hazard_radius_viewport && cfg.hazard_radius_uu > 0.0 {
                let haz_col = [1.00, 0.20, 0.20, 0.80]; // Crimson Hazard
                add_wire_circle(&mut line_verts, cfg.position, cfg.hazard_radius_uu, haz_col);
                // Also add XZ and YZ circles for a 3D spherical cage
                let segs = 32;
                let mut prev_xz = cfg.position + Vec3::new(cfg.hazard_radius_uu, 0.0, 0.0);
                let mut prev_yz = cfg.position + Vec3::new(0.0, cfg.hazard_radius_uu, 0.0);
                for i in 1..=segs {
                    let angle = (i as f32) * (std::f32::consts::TAU / segs as f32);
                    let curr_xz = cfg.position + Vec3::new(angle.cos() * cfg.hazard_radius_uu, 0.0, angle.sin() * cfg.hazard_radius_uu);
                    let curr_yz = cfg.position + Vec3::new(0.0, angle.cos() * cfg.hazard_radius_uu, angle.sin() * cfg.hazard_radius_uu);

                    line_verts.push(LineVertex::new(prev_xz.to_array(), haz_col));
                    line_verts.push(LineVertex::new(curr_xz.to_array(), haz_col));
                    prev_xz = curr_xz;

                    line_verts.push(LineVertex::new(prev_yz.to_array(), haz_col));
                    line_verts.push(LineVertex::new(curr_yz.to_array(), haz_col));
                    prev_yz = curr_yz;
                }
            }
        }


        // Soft Penumbra Shadow Profiler Raycast Visualizer
        if state.show_shadow_penumbra {
            if let Some(ref prof) = state.shadow_penumbra_profile {
                let ray_col = if prof.is_in_penumbra {
                    [0.85, 0.40, 1.00, 0.85] // Penumbra Violet
                } else if prof.penumbra_factor < 0.01 {
                    [0.90, 0.15, 0.15, 0.80] // Umbra Red
                } else {
                    [1.00, 0.90, 0.20, 0.85] // Full light Yellow
                };
                line_verts.push(LineVertex::new(prof.surface_point.to_array(), ray_col));
                line_verts.push(LineVertex::new(prof.light_position.to_array(), ray_col));

                if prof.light_radius > 0.0 {
                    add_wire_circle(&mut line_verts, prof.light_position, prof.light_radius, ray_col);
                }
                if let Some(d) = prof.first_blocker_dist {
                    let dir = (prof.light_position - prof.surface_point).normalize();
                    let blocker_pos = prof.surface_point + dir * d;
                    add_wire_box(&mut line_verts, blocker_pos, glam::Vec3::splat(12.0), [1.0, 0.2, 0.2, 0.9]);
                }
            }
        }

        // G. 3D Transform Gizmo (at selected mover, actor, brush, or builder brush)
        let gizmo_center = crate::transform_session::gizmo_pivot(state);

        if let Some(center) = gizmo_center {
            let gizmo_scale = state.get_gizmo_scale(center, state.active_viewport_type, vh as f32);
            let rot_scale = gizmo_scale * 0.875;
            let rot_mat = state.get_selected_rotation_matrix();
            match state.gizmo_mode {
                crate::raycast::GizmoMode::Translate => {
                    add_transform_gizmo(&mut line_verts, center, gizmo_scale, state.active_gizmo_axis, state.hovered_gizmo_axis, rot_mat);
                }
                crate::raycast::GizmoMode::Rotate => {
                    add_rotation_gizmo(&mut line_verts, center, rot_scale, state.active_gizmo_axis, state.hovered_gizmo_axis, rot_mat);
                }
                crate::raycast::GizmoMode::Scale => {
                    add_scale_gizmo(&mut line_verts, center, gizmo_scale, state.active_gizmo_axis, state.hovered_gizmo_axis, rot_mat);
                }
            }
            if state.active_gizmo_axis != crate::raycast::GizmoAxis::None && state.active_gizmo_delta.length_squared() > 4.0 {
                let orig_center = center - state.active_gizmo_delta;
                let ghost_col = [0.65, 0.85, 1.0, 0.45];
                add_wire_box(&mut line_verts, orig_center, glam::Vec3::splat(12.0), ghost_col);
                line_verts.push(LineVertex::new(orig_center.to_array(), ghost_col));
                line_verts.push(LineVertex::new(center.to_array(), ghost_col));
            }
        }

        // H. Vertex Editing Mode: Render interactive 3D handles at each unique vertex
        if state.editor_mode == crate::editor_state::EditorMode::VertexEdit {
            let unique_verts = state.get_active_brush_unique_vertices();
            let unsel_color = [1.0, 0.85, 0.1, 1.0]; // bright yellow
            let sel_color = [0.2, 1.0, 0.9, 1.0];   // vibrant cyan

            for (i, &v) in unique_verts.iter().enumerate() {
                let is_selected = state.selected_vertex_indices.contains(&i);
                let (color, size) = if is_selected {
                    (sel_color, 12.0)
                } else {
                    (unsel_color, 7.0)
                };
                add_wire_box(&mut line_verts, v, glam::Vec3::splat(size), color);
            }
        }

        // I. Brush Clipping Mode: Render Clip Markers and Cutting Plane
        if state.editor_mode == crate::editor_state::EditorMode::BrushClip {
            let marker_color = [1.0, 0.3, 0.2, 1.0]; // bright orange-red
            for &pt in &state.clip_points {
                add_octahedron(&mut line_verts, pt, 16.0, marker_color);
            }
            if state.clip_points.len() >= 2 {
                line_verts.push(LineVertex::new(state.clip_points[0].to_array(), marker_color));
                line_verts.push(LineVertex::new(state.clip_points[1].to_array(), marker_color));
            }
            if state.clip_points.len() == 3 {
                line_verts.push(LineVertex::new(state.clip_points[1].to_array(), marker_color));
                line_verts.push(LineVertex::new(state.clip_points[2].to_array(), marker_color));
                line_verts.push(LineVertex::new(state.clip_points[2].to_array(), marker_color));
                line_verts.push(LineVertex::new(state.clip_points[0].to_array(), marker_color));
            }

            // Draw cutting plane & normal arrow
            if let Some(plane) = state.get_clipping_plane() {
                let center = if !state.clip_points.is_empty() {
                    state.clip_points[0]
                } else {
                    state.builder_brush.location
                };
                let norm = plane.normal.normalize();
                let plane_color = [0.2, 0.9, 1.0, 0.7]; // cyan plane
                let arrow_color = [1.0, 1.0, 0.0, 1.0]; // yellow normal

                // Plane quad wireframe
                let u_axis = if norm.x.abs() < 0.9 {
                    norm.cross(glam::Vec3::X).normalize()
                } else {
                    norm.cross(glam::Vec3::Y).normalize()
                };
                let v_axis = norm.cross(u_axis).normalize();
                let ext = 300.0;
                let c0 = center - u_axis * ext - v_axis * ext;
                let c1 = center + u_axis * ext - v_axis * ext;
                let c2 = center + u_axis * ext + v_axis * ext;
                let c3 = center - u_axis * ext + v_axis * ext;

                line_verts.push(LineVertex::new(c0.to_array(), plane_color)); line_verts.push(LineVertex::new(c1.to_array(), plane_color));
                line_verts.push(LineVertex::new(c1.to_array(), plane_color)); line_verts.push(LineVertex::new(c2.to_array(), plane_color));
                line_verts.push(LineVertex::new(c2.to_array(), plane_color)); line_verts.push(LineVertex::new(c3.to_array(), plane_color));
                line_verts.push(LineVertex::new(c3.to_array(), plane_color)); line_verts.push(LineVertex::new(c0.to_array(), plane_color));

                // Normal arrow pointing towards the kept half-space
                let arrow_tip = center + norm * 80.0;
                line_verts.push(LineVertex::new(center.to_array(), arrow_color));
                line_verts.push(LineVertex::new(arrow_tip.to_array(), arrow_color));
                add_octahedron(&mut line_verts, arrow_tip, 6.0, arrow_color);
            }
        }

        // Level Doctor 3D Diagnostic Beacons in Viewport
        if state.show_diagnostic_markers && !state.map_audit_issues.is_empty() {
            for (idx, issue) in state.map_audit_issues.iter().enumerate() {
                if let Some(loc) = issue.location {
                    let is_selected = state.selected_issue_index == Some(idx);
                    let base_color = match issue.severity {
                        ut99_bsp::IssueSeverity::Error => [1.00, 0.20, 0.20, 0.95],
                        ut99_bsp::IssueSeverity::Warning => [1.00, 0.75, 0.10, 0.95],
                        ut99_bsp::IssueSeverity::Info => [0.20, 0.75, 1.00, 0.95],
                    };

                    let color = if is_selected {
                        [1.00, 0.90, 0.20, 1.00]
                    } else {
                        base_color
                    };

                    let radius = if is_selected { 22.0 } else { 14.0 };

                    // Primary 3D diamond (octahedron) beacon
                    add_octahedron(&mut line_verts, loc, radius, color);

                    // Vertical cadastral ground dropper line
                    let drop_bottom = loc - glam::Vec3::new(0.0, 0.0, radius * 2.5);
                    let drop_top = loc + glam::Vec3::new(0.0, 0.0, radius * 1.5);
                    line_verts.push(LineVertex::new(drop_bottom.to_array(), color));
                    line_verts.push(LineVertex::new(drop_top.to_array(), color));

                    // Ground target circle at bottom of dropper
                    add_wire_circle(&mut line_verts, drop_bottom, radius * 0.75, color);

                    if is_selected {
                        // Concentric halo ring and outer diamond box for selected issue
                        add_wire_circle(&mut line_verts, loc, radius * 1.5, [1.0, 1.0, 1.0, 0.85]);
                        add_wire_box(&mut line_verts, loc, glam::Vec3::splat(radius * 1.8), [1.0, 0.8, 0.2, 0.6]);
                    } else if issue.severity == ut99_bsp::IssueSeverity::Error {
                        // Hazard cross for error beacons
                        let cross_arm = radius * 0.7;
                        line_verts.push(LineVertex::new((loc + glam::Vec3::new(-cross_arm, -cross_arm, 0.0)).to_array(), color));
                        line_verts.push(LineVertex::new((loc + glam::Vec3::new(cross_arm, cross_arm, 0.0)).to_array(), color));
                        line_verts.push(LineVertex::new((loc + glam::Vec3::new(-cross_arm, cross_arm, 0.0)).to_array(), color));
                        line_verts.push(LineVertex::new((loc + glam::Vec3::new(cross_arm, -cross_arm, 0.0)).to_array(), color));
                    }
                }
            }
        }

        // J. 3D Drag & Drop Placement Preview Indicator (03)
        if state.active_drag_drop.is_some() {
            if let Some(pos) = state.drag_drop_preview_pos {
                let normal = state.drag_drop_preview_normal.unwrap_or(glam::Vec3::Z);
                let (marker_color, pulse_color, is_actor) = match &state.active_drag_drop {
                    Some(crate::editor_state::DragDropPayload::ActorClass(_)) => {
                        ([0.20, 0.90, 1.00, 0.95], [0.10, 0.70, 1.00, 0.60], true) // Electric Cyan
                    }
                    Some(crate::editor_state::DragDropPayload::Texture(_)) => {
                        ([1.00, 0.85, 0.15, 0.95], [0.95, 0.65, 0.10, 0.60], false) // Radiant Gold
                    }
                    None => ([1.0, 1.0, 1.0, 1.0], [0.5, 0.5, 0.5, 0.5], false),
                };

                // Surface-aligned rings
                add_wire_circle(&mut line_verts, pos + normal * 1.5, 24.0, marker_color);
                add_wire_circle(&mut line_verts, pos + normal * 1.5, 36.0, pulse_color);

                // Normal stem pointing out from surface
                let stem_tip = pos + normal * 40.0;
                line_verts.push(LineVertex::new(pos.to_array(), marker_color));
                line_verts.push(LineVertex::new(stem_tip.to_array(), marker_color));
                add_octahedron(&mut line_verts, stem_tip, 6.0, marker_color);

                if is_actor {
                    // Preview box for actor placement
                    add_wire_box(&mut line_verts, pos + normal * 20.0, glam::Vec3::splat(16.0), marker_color);
                } else {
                    // Surface disc / diagonal tick marks for texture drop
                    let tangent = if normal.z.abs() < 0.9 {
                        normal.cross(glam::Vec3::Z).normalize()
                    } else {
                        normal.cross(glam::Vec3::Y).normalize()
                    };
                    let bitangent = normal.cross(tangent).normalize();
                    let t_arm = 20.0;
                    line_verts.push(LineVertex::new((pos - tangent * t_arm).to_array(), pulse_color));
                    line_verts.push(LineVertex::new((pos + tangent * t_arm).to_array(), pulse_color));
                    line_verts.push(LineVertex::new((pos - bitangent * t_arm).to_array(), pulse_color));
                    line_verts.push(LineVertex::new((pos + bitangent * t_arm).to_array(), pulse_color));
                }
            }
        }

        // K. Freehand Polygon Drawing & Extrusion 3D Preview (Cycle 7 / BG-17)
        if !state.freehand_points.is_empty() {
            let poly_edge_col = [0.15, 0.95, 1.00, 1.0]; // Electric Cyan
            let poly_close_col = [1.00, 0.78, 0.20, 0.90]; // Amber closing segment
            let extrude_col = [0.25, 0.85, 0.55, 0.75]; // Emerald extruded prism preview
            let n = state.freehand_points.len();

            for (i, &pt) in state.freehand_points.iter().enumerate() {
                add_octahedron(&mut line_verts, pt, 6.0, poly_edge_col);
                if i + 1 < n {
                    let next = state.freehand_points[i + 1];
                    line_verts.push(LineVertex::new(pt.to_array(), poly_edge_col));
                    line_verts.push(LineVertex::new(next.to_array(), poly_edge_col));
                } else if n >= 3 {
                    let first = state.freehand_points[0];
                    line_verts.push(LineVertex::new(pt.to_array(), poly_close_col));
                    line_verts.push(LineVertex::new(first.to_array(), poly_close_col));
                }
            }

            if n >= 3 && state.freehand_extrude_depth.abs() >= 1.0 {
                let offset = glam::Vec3::new(0.0, 0.0, state.freehand_extrude_depth);
                for i in 0..n {
                    let p0 = state.freehand_points[i];
                    let p1 = state.freehand_points[(i + 1) % n];
                    let t0 = p0 + offset;
                    let t1 = p1 + offset;
                    line_verts.push(LineVertex::new(p0.to_array(), extrude_col));
                    line_verts.push(LineVertex::new(t0.to_array(), extrude_col));
                    line_verts.push(LineVertex::new(t0.to_array(), extrude_col));
                    line_verts.push(LineVertex::new(t1.to_array(), extrude_col));
                }
            }
        }

        let ovl_studios_gizmos_ms = studio_build_ms + ovl_studios_started.elapsed().as_secs_f64() * 1000.0;
        let overlays_ms = ovl_movers_meshes_ms
            + ovl_actors_links_ms
            + ovl_trajectories_ms
            + ovl_radii_portals_sky_ms
            + ovl_zone_seals_ms
            + ovl_dispatchers_movtraj_ms
            + ovl_studios_gizmos_ms;
        self.last_cull_stats = (self.cached_visible_brushes, 0, self.cached_visible_actors, 0);

        let dynamic_line_count = line_verts.len() as u32;
        let dyn_upload_started = std::time::Instant::now();
        let uploaded_dynamic_line_bytes = Self::upload_persistent_buffer(
            device,
            queue,
            "Persistent Dynamic Line Vertex Buffer",
            &mut self.dynamic_line_buffer,
            &mut self.dynamic_line_buffer_capacity,
            &line_verts,
        );
        let dyn_upload_ms = dyn_upload_started.elapsed().as_secs_f64() * 1000.0;
        let upload_ms = solid_upload_ms + static_line_upload_ms + studio_line_upload_ms + dyn_upload_ms;
        let uploaded_line_bytes =
            uploaded_static_line_bytes + uploaded_studio_line_bytes + uploaded_dynamic_line_bytes;

        let encode_started = std::time::Instant::now();
        let mut draw_calls = 0u32;
        let mut solid_chunks_total = 0u32;
        let mut solid_chunks_drawn = 0u32;
        let mut solid_tris_total = 0u32;
        let mut solid_tris_drawn = 0u32;
        let mut line_chunks_total = 0u32;
        let mut line_chunks_drawn = 0u32;
        let mut line_segs_total = 0u32;
        let mut line_segs_drawn = 0u32;

        let cam_pos = state.camera.position;
        let cam_fwd = state.camera.forward();
        let mut sort_scratch = std::mem::take(&mut self.visible_chunk_sort_scratch);
        let mut runs_scratch = std::mem::take(&mut self.visible_runs_scratch);
        let backface_cull = self.backface_culling_enabled;
        let sort_f2b = self.front_to_back_sorting_enabled;

        let draw_viewport_solids = |pass: &mut wgpu::RenderPass<'_>,
                                    solid_chunks: &[Vec<super::spatial_chunks::SpatialChunk>;
                                         super::spatial_chunks::SOLID_BUCKET_COUNT],
                                    total_indices: u32,
                                    vbuf: &wgpu::Buffer,
                                    ibuf: &wgpu::Buffer,
                                    tex_bg: &wgpu::BindGroup,
                                    opaque_culled: &wgpu::RenderPipeline,
                                    opaque: &wgpu::RenderPipeline,
                                    detail: &wgpu::RenderPipeline,
                                    masked: &wgpu::RenderPipeline,
                                    translucent: &wgpu::RenderPipeline,
                                    vp_type: crate::camera::ViewportType,
                                    frustum: Option<&crate::culling::Frustum>,
                                    sort_scratch: &mut Vec<(f32, f32, u32, u32)>,
                                    runs_scratch: &mut Vec<(u32, u32)>|
         -> (u32, u32, u32, u32, u32) {
            if total_indices == 0 {
                return (0, 0, 0, 0, 0);
            }
            let mut vc_total = 0u32;
            let mut vt_total = 0u32;
            for b in solid_chunks {
                for c in b {
                    vc_total += 1;
                    vt_total += c.prim_count;
                }
            }
            let mut vc_drawn = 0u32;
            let mut vt_drawn = 0u32;
            let mut calls = 0u32;
            let mut bound_common = false;
            let mut emit_runs = |pipeline: &wgpu::RenderPipeline,
                                 runs: &[(u32, u32)],
                                 chunks_cnt: u32,
                                 tris_cnt: u32| {
                vc_drawn += chunks_cnt;
                vt_drawn += tris_cnt;
                if runs.is_empty() {
                    return;
                }
                if !bound_common {
                    pass.set_bind_group(1, tex_bg, &[]);
                    pass.set_vertex_buffer(0, vbuf.slice(..));
                    pass.set_index_buffer(ibuf.slice(..), wgpu::IndexFormat::Uint32);
                    bound_common = true;
                }
                pass.set_pipeline(pipeline);
                for &(start, end) in runs {
                    pass.draw_indexed(start..end, 0, 0..1);
                    calls += 1;
                }
            };

            if backface_cull && vp_type == crate::camera::ViewportType::Perspective {
                let (cd, td) = super::spatial_chunks::collect_visible_opaque_runs(
                    &[&solid_chunks[0]],
                    frustum,
                    sort_f2b,
                    vp_type,
                    cam_pos,
                    cam_fwd,
                    sort_scratch,
                    runs_scratch,
                );
                emit_runs(opaque_culled, runs_scratch, cd, td);

                let (cd, td) = super::spatial_chunks::collect_visible_opaque_runs(
                    &[&solid_chunks[1]],
                    frustum,
                    sort_f2b,
                    vp_type,
                    cam_pos,
                    cam_fwd,
                    sort_scratch,
                    runs_scratch,
                );
                emit_runs(opaque, runs_scratch, cd, td);
            } else {
                let (cd, td) = super::spatial_chunks::collect_visible_opaque_runs(
                    &[&solid_chunks[0], &solid_chunks[1]],
                    frustum,
                    sort_f2b,
                    vp_type,
                    cam_pos,
                    cam_fwd,
                    sort_scratch,
                    runs_scratch,
                );
                emit_runs(opaque, runs_scratch, cd, td);
            }

            let (cd, td) = super::spatial_chunks::collect_visible_opaque_runs(
                &[&solid_chunks[2]],
                frustum,
                sort_f2b,
                vp_type,
                cam_pos,
                cam_fwd,
                sort_scratch,
                runs_scratch,
            );
            emit_runs(detail, runs_scratch, cd, td);

            let (cd, td) = super::spatial_chunks::collect_visible_coalesced_runs(
                &solid_chunks[3],
                frustum,
                runs_scratch,
            );
            emit_runs(masked, runs_scratch, cd, td);

            let (cd, td) = super::spatial_chunks::collect_visible_coalesced_runs(
                &solid_chunks[4],
                frustum,
                runs_scratch,
            );
            emit_runs(translucent, runs_scratch, cd, td);

            (calls, vc_total, vc_drawn, vt_total, vt_drawn)
        };

        let draw_viewport_static_lines = |pass: &mut wgpu::RenderPass<'_>,
                                          line_pipeline: &wgpu::RenderPipeline,
                                          buf: &wgpu::Buffer,
                                          static_line_count: u32,
                                          grid_range: (u32, u32),
                                          world_lines_start: u32,
                                          line_chunks: &[super::spatial_chunks::SpatialChunk],
                                          frustum: Option<&crate::culling::Frustum>,
                                          runs_scratch: &mut Vec<(u32, u32)>|
         -> (u32, u32, u32, u32, u32) {
            if static_line_count == 0 {
                return (0, 0, 0, 0, 0);
            }
            let mut lc_total = 0u32;
            let mut ls_total = 0u32;
            for c in line_chunks {
                lc_total += 1;
                ls_total += c.prim_count;
            }
            if frustum.is_none() && grid_range == (0, world_lines_start) {
                pass.set_pipeline(line_pipeline);
                pass.set_vertex_buffer(0, buf.slice(..));
                pass.draw(0..static_line_count, 0..1);
                return (1, lc_total, lc_total, ls_total, ls_total);
            }

            let (lc_drawn, ls_drawn) = super::spatial_chunks::collect_visible_coalesced_runs(
                line_chunks,
                frustum,
                runs_scratch,
            );
            let (g_start, g_end) = grid_range;
            let mut draw_grid_separate = g_end > g_start;
            if draw_grid_separate {
                if let Some(first) = runs_scratch.first_mut() {
                    if g_end == first.0 {
                        first.0 = g_start;
                        draw_grid_separate = false;
                    }
                }
            }
            if !draw_grid_separate && runs_scratch.is_empty() {
                return (0, lc_total, lc_drawn, ls_total, ls_drawn);
            }

            let mut calls = 0u32;
            pass.set_pipeline(line_pipeline);
            pass.set_vertex_buffer(0, buf.slice(..));
            if draw_grid_separate {
                pass.draw(g_start..g_end, 0..1);
                calls += 1;
            }
            for &(start, end) in runs_scratch.iter() {
                pass.draw(start..end, 0..1);
                calls += 1;
            }
            (calls, lc_total, lc_drawn, ls_total, ls_drawn)
        };

        let scale = state.viewport_render_scale.clamp(0.5, 1.0);
        let max_dim = device.limits().max_texture_dimension_2d.max(1);
        let sw = ((vw as f32) * scale).round().clamp(1.0, max_dim as f32) as u32;
        let sh = ((vh as f32) * scale).round().clamp(1.0, max_dim as f32) as u32;
        let use_scaled_target = scale < 0.999 && (sw < vw || sh < vh);
        let (pass_vx, pass_vy, pass_vw, pass_vh) = if use_scaled_target {
            self.ensure_scaled_target(device, sw, sh);
            (0, 0, sw, sh)
        } else {
            self.scaled_target = None;
            (vx, vy, vw, vh)
        };
        let (pass_color_view, pass_depth_view) = match self.scaled_target.as_ref() {
            Some(st) if use_scaled_target => (&st.color_view, &st.depth_view),
            _ => (target_view, &self.depth_texture_view),
        };

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("3D Viewport Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: pass_color_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.10,
                            g: 0.11,
                            b: 0.13,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: pass_depth_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: self.gpu_timer.as_ref().and_then(|timer| timer.writes()),
                occlusion_query_set: None,
            });

            if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
                let hw = pass_vw / 2;
                let hh = pass_vh / 2;
                let rw = pass_vw.saturating_sub(hw);
                let rh = pass_vh.saturating_sub(hh);

                let quadrants = [
                    (pass_vx, pass_vy, hw, hh, 0, crate::camera::ViewportType::Top),
                    (pass_vx + hw, pass_vy, rw, hh, 1, crate::camera::ViewportType::Side),
                    (pass_vx, pass_vy + hh, hw, rh, 2, crate::camera::ViewportType::Front),
                    (pass_vx + hw, pass_vy + hh, rw, rh, 3, crate::camera::ViewportType::Perspective),
                ];

                for (qx, qy, qw, qh, idx, vp_type) in quadrants {
                    if qw == 0 || qh == 0 {
                        continue;
                    }
                    let frustum_opt = state
                        .frustum_culling_enabled
                        .then_some(&quad_frustums[idx]);
                    pass.set_viewport(qx as f32, qy as f32, qw as f32, qh as f32, 0.0, 1.0);
                    pass.set_scissor_rect(qx, qy, qw, qh);
                    pass.set_bind_group(0, &self.uniform_bind_groups[idx], &[]);
                    if state.shading_for_quadrant(idx) != crate::camera::ViewportShading::Wireframe
                        || state.background_image_path.is_some()
                    {
                        if self.cached_solid_ranges.total > 0 {
                            if let (Some(ref vbuf), Some(ref ibuf)) =
                                (self.solid_buffer.as_ref(), self.solid_index_buffer.as_ref())
                            {
                                let (dc, ct, cd, tt, td) = draw_viewport_solids(
                                    &mut pass,
                                    &self.cached_solid_chunks,
                                    self.cached_solid_ranges.total,
                                    vbuf,
                                    ibuf,
                                    &self.texture_bind_group,
                                    &self.solid_opaque_culled_pipeline,
                                    &self.solid_opaque_pipeline,
                                    &self.solid_detail_pipeline,
                                    &self.solid_pipeline,
                                    &self.solid_translucent_pipeline,
                                    vp_type,
                                    frustum_opt,
                                    &mut sort_scratch,
                                    &mut runs_scratch,
                                );
                                draw_calls += dc;
                                solid_chunks_total += ct;
                                solid_chunks_drawn += cd;
                                solid_tris_total += tt;
                                solid_tris_drawn += td;
                            }
                        }
                    }
                    if self.cached_static_line_count > 0 {
                        if let Some(ref buf) = self.line_buffer {
                            let (dc, ct, cd, st, sd) = draw_viewport_static_lines(
                                &mut pass,
                                &self.line_pipeline,
                                buf,
                                self.cached_static_line_count,
                                self.cached_quadrant_grid_ranges[idx],
                                self.cached_world_lines_start,
                                &self.cached_line_chunks,
                                frustum_opt,
                                &mut runs_scratch,
                            );
                            draw_calls += dc;
                            line_chunks_total += ct;
                            line_chunks_drawn += cd;
                            line_segs_total += st;
                            line_segs_drawn += sd;
                        }
                    }
                    if self.cached_studio_line_count > 0 {
                        if let Some(ref buf) = self.studio_line_buffer {
                            pass.set_pipeline(&self.line_pipeline);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..self.cached_studio_line_count, 0..1);
                            draw_calls += 1;
                        }
                    }
                    if dynamic_line_count > 0 {
                        if let Some(ref buf) = self.dynamic_line_buffer {
                            pass.set_pipeline(&self.line_pipeline);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..dynamic_line_count, 0..1);
                            draw_calls += 1;
                        }
                    }
                }
            } else {
                if pass_vw > 0 && pass_vh > 0 {
                    let frustum_opt = state
                        .frustum_culling_enabled
                        .then_some(&quad_frustums[0]);
                    pass.set_viewport(pass_vx as f32, pass_vy as f32, pass_vw as f32, pass_vh as f32, 0.0, 1.0);
                    pass.set_scissor_rect(pass_vx, pass_vy, pass_vw, pass_vh);
                    pass.set_bind_group(0, &self.uniform_bind_groups[0], &[]);
                    if state.viewport_shading != crate::camera::ViewportShading::Wireframe
                        || state.background_image_path.is_some()
                    {
                        if self.cached_solid_ranges.total > 0 {
                            if let (Some(ref vbuf), Some(ref ibuf)) =
                                (self.solid_buffer.as_ref(), self.solid_index_buffer.as_ref())
                            {
                                let (dc, ct, cd, tt, td) = draw_viewport_solids(
                                    &mut pass,
                                    &self.cached_solid_chunks,
                                    self.cached_solid_ranges.total,
                                    vbuf,
                                    ibuf,
                                    &self.texture_bind_group,
                                    &self.solid_opaque_culled_pipeline,
                                    &self.solid_opaque_pipeline,
                                    &self.solid_detail_pipeline,
                                    &self.solid_pipeline,
                                    &self.solid_translucent_pipeline,
                                    state.active_viewport_type,
                                    frustum_opt,
                                    &mut sort_scratch,
                                    &mut runs_scratch,
                                );
                                draw_calls += dc;
                                solid_chunks_total += ct;
                                solid_chunks_drawn += cd;
                                solid_tris_total += tt;
                                solid_tris_drawn += td;
                            }
                        }
                    }
                    if self.cached_static_line_count > 0 {
                        if let Some(ref buf) = self.line_buffer {
                            let (dc, ct, cd, st, sd) = draw_viewport_static_lines(
                                &mut pass,
                                &self.line_pipeline,
                                buf,
                                self.cached_static_line_count,
                                (0, self.cached_world_lines_start),
                                self.cached_world_lines_start,
                                &self.cached_line_chunks,
                                frustum_opt,
                                &mut runs_scratch,
                            );
                            draw_calls += dc;
                            line_chunks_total += ct;
                            line_chunks_drawn += cd;
                            line_segs_total += st;
                            line_segs_drawn += sd;
                        }
                    }
                    if self.cached_studio_line_count > 0 {
                        if let Some(ref buf) = self.studio_line_buffer {
                            pass.set_pipeline(&self.line_pipeline);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..self.cached_studio_line_count, 0..1);
                            draw_calls += 1;
                        }
                    }
                    if dynamic_line_count > 0 {
                        if let Some(ref buf) = self.dynamic_line_buffer {
                            pass.set_pipeline(&self.line_pipeline);
                            pass.set_vertex_buffer(0, buf.slice(..));
                            pass.draw(0..dynamic_line_count, 0..1);
                            draw_calls += 1;
                        }
                    }
                }
            }
        }

        if use_scaled_target {
            if let Some(ref st) = self.scaled_target {
                let mut blit_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("3D Viewport Upscale Blit Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target_view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.10,
                                g: 0.11,
                                b: 0.13,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                blit_pass.set_viewport(vx as f32, vy as f32, vw as f32, vh as f32, 0.0, 1.0);
                blit_pass.set_scissor_rect(vx, vy, vw, vh);
                blit_pass.set_pipeline(&self.blit_pipeline);
                blit_pass.set_bind_group(0, &st.blit_bind_group, &[]);
                blit_pass.draw(0..3, 0..1);
            }
        }

        self.visible_chunk_sort_scratch = sort_scratch;
        self.visible_runs_scratch = runs_scratch;

        let primary_frustum = if !state.frustum_culling_enabled {
            None
        } else if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
            Some(&quad_frustums[3])
        } else {
            Some(&quad_frustums[0])
        };
        self.last_cull_stats = match primary_frustum {
            Some(f) if !self.cached_line_chunks.is_empty() => {
                let mut rb = 0usize;
                let mut ra = 0usize;
                for c in &self.cached_line_chunks {
                    if super::spatial_chunks::chunk_is_visible(c, Some(f)) {
                        rb += c.brush_count as usize;
                        ra += c.actor_count as usize;
                    }
                }
                let cb = self.cached_visible_brushes.saturating_sub(rb);
                let ca = self.cached_visible_actors.saturating_sub(ra);
                (rb, cb, ra, ca)
            }
            _ => (self.cached_visible_brushes, 0, self.cached_visible_actors, 0),
        };

        if let Some(timer) = &self.gpu_timer { timer.resolve(encoder); }
        let encode_submit_ms = encode_started.elapsed().as_secs_f64() * 1000.0;
        self.last_frame_stats = crate::graphics::RenderFrameStats {
            signature_ms,
            csg_ms,
            wireframe_ms,
            solids_ms,
            overlays_ms,
            ovl_movers_meshes_ms,
            ovl_actors_links_ms,
            ovl_trajectories_ms,
            ovl_radii_portals_sky_ms,
            ovl_zone_seals_ms,
            ovl_dispatchers_movtraj_ms,
            ovl_studios_gizmos_ms,
            upload_ms,
            encode_submit_ms,
            csg_rebuilt,
            csg_in_flight: self.csg_in_flight,
            csg_rebuild_reason,
            uploaded_line_bytes,
            uploaded_solid_bytes,
            line_vertex_count: self.cached_static_line_count as usize
                + self.cached_studio_line_count as usize
                + dynamic_line_count as usize,
            solid_vertex_count: self.cached_solid_vertex_count as usize,
            draw_calls,
            texture_layers_used: self.uploaded_texture_layers.len(),
            texture_fallback_surfaces: self.last_texture_fallbacks,
            lightmap_layers_used: self.lightmap_atlas.len(),
            lightmap_fallback_surfaces: self.last_lightmap_fallbacks,
            solid_chunks_total,
            solid_chunks_drawn,
            solid_tris_total,
            solid_tris_drawn,
            line_chunks_total,
            line_chunks_drawn,
            line_segs_total,
            line_segs_drawn,
        };
        self.frame_dynamic_lines = retain_vertex_storage(line_verts, self.max_retained_vertex_bytes);
    }
}

fn add_octahedron(lines: &mut Vec<LineVertex>, center: glam::Vec3, radius: f32, color: [f32; 4]) {
    let top = center + glam::Vec3::new(0.0, 0.0, radius);
    let bot = center - glam::Vec3::new(0.0, 0.0, radius);
    let p0 = center + glam::Vec3::new(radius, 0.0, 0.0);
    let p1 = center + glam::Vec3::new(0.0, radius, 0.0);
    let p2 = center - glam::Vec3::new(radius, 0.0, 0.0);
    let p3 = center - glam::Vec3::new(0.0, radius, 0.0);

    // Top pyramid
    lines.push(LineVertex::new(top.to_array(), color)); lines.push(LineVertex::new(p0.to_array(), color));
    lines.push(LineVertex::new(top.to_array(), color)); lines.push(LineVertex::new(p1.to_array(), color));
    lines.push(LineVertex::new(top.to_array(), color)); lines.push(LineVertex::new(p2.to_array(), color));
    lines.push(LineVertex::new(top.to_array(), color)); lines.push(LineVertex::new(p3.to_array(), color));

    // Bottom pyramid
    lines.push(LineVertex::new(bot.to_array(), color)); lines.push(LineVertex::new(p0.to_array(), color));
    lines.push(LineVertex::new(bot.to_array(), color)); lines.push(LineVertex::new(p1.to_array(), color));
    lines.push(LineVertex::new(bot.to_array(), color)); lines.push(LineVertex::new(p2.to_array(), color));
    lines.push(LineVertex::new(bot.to_array(), color)); lines.push(LineVertex::new(p3.to_array(), color));

    // Equator
    lines.push(LineVertex::new(p0.to_array(), color)); lines.push(LineVertex::new(p1.to_array(), color));
    lines.push(LineVertex::new(p1.to_array(), color)); lines.push(LineVertex::new(p2.to_array(), color));
    lines.push(LineVertex::new(p2.to_array(), color)); lines.push(LineVertex::new(p3.to_array(), color));
    lines.push(LineVertex::new(p3.to_array(), color)); lines.push(LineVertex::new(p0.to_array(), color));
}

fn add_wire_cylinder(lines: &mut Vec<LineVertex>, center: glam::Vec3, radius: f32, height: f32, color: [f32; 4]) {
    let half_h = height * 0.5;
    let segments = 12;
    for i in 0..segments {
        let a0 = i as f32 * std::f32::consts::TAU / segments as f32;
        let a1 = (i + 1) as f32 * std::f32::consts::TAU / segments as f32;

        let p0_bot = center + glam::Vec3::new(radius * a0.cos(), radius * a0.sin(), -half_h);
        let p1_bot = center + glam::Vec3::new(radius * a1.cos(), radius * a1.sin(), -half_h);
        let p0_top = center + glam::Vec3::new(radius * a0.cos(), radius * a0.sin(), half_h);
        let p1_top = center + glam::Vec3::new(radius * a1.cos(), radius * a1.sin(), half_h);

        lines.push(LineVertex::new(p0_bot.to_array(), color));
        lines.push(LineVertex::new(p1_bot.to_array(), color));
        lines.push(LineVertex::new(p0_top.to_array(), color));
        lines.push(LineVertex::new(p1_top.to_array(), color));
        lines.push(LineVertex::new(p0_bot.to_array(), color));
        lines.push(LineVertex::new(p0_top.to_array(), color));
    }
}

fn add_wire_circle(lines: &mut Vec<LineVertex>, center: glam::Vec3, radius: f32, color: [f32; 4]) {
    let segments = 24;
    for i in 0..segments {
        let a0 = i as f32 * std::f32::consts::TAU / segments as f32;
        let a1 = (i + 1) as f32 * std::f32::consts::TAU / segments as f32;

        let p0 = center + glam::Vec3::new(radius * a0.cos(), radius * a0.sin(), 0.0);
        let p1 = center + glam::Vec3::new(radius * a1.cos(), radius * a1.sin(), 0.0);

        lines.push(LineVertex::new(p0.to_array(), color));
        lines.push(LineVertex::new(p1.to_array(), color));
    }
}

fn add_wire_box(lines: &mut Vec<LineVertex>, center: glam::Vec3, half_extent: glam::Vec3, color: [f32; 4]) {
    let min = center - half_extent;
    let max = center + half_extent;

    let corners = [
        glam::Vec3::new(min.x, min.y, min.z),
        glam::Vec3::new(max.x, min.y, min.z),
        glam::Vec3::new(max.x, max.y, min.z),
        glam::Vec3::new(min.x, max.y, min.z),
        glam::Vec3::new(min.x, min.y, max.z),
        glam::Vec3::new(max.x, min.y, max.z),
        glam::Vec3::new(max.x, max.y, max.z),
        glam::Vec3::new(min.x, max.y, max.z),
    ];

    let edges = [
        (0, 1), (1, 2), (2, 3), (3, 0),
        (4, 5), (5, 6), (6, 7), (7, 4),
        (0, 4), (1, 5), (2, 6), (3, 7),
    ];

    for &(i, j) in &edges {
        lines.push(LineVertex::new(corners[i].to_array(), color));
        lines.push(LineVertex::new(corners[j].to_array(), color));
    }
}

fn add_transform_gizmo(
    lines: &mut Vec<LineVertex>,
    center: glam::Vec3,
    axis_length: f32,
    active_axis: crate::raycast::GizmoAxis,
    hovered_axis: crate::raycast::GizmoAxis,
    rot: glam::Mat3,
) {
    let tip_len = 16.0;
    let tip_w = 4.0;

    let handle_color = |target_axis: crate::raycast::GizmoAxis, default_col: [f32; 4]| -> [f32; 4] {
        if active_axis == target_axis {
            [1.0, 1.0, 0.2, 1.0]
        } else if hovered_axis == target_axis {
            [1.0, 0.85, 0.25, 0.95]
        } else {
            default_col
        }
    };

    // Central Screen-Plane Handle (White / Yellow on active/hover)
    let center_color = handle_color(crate::raycast::GizmoAxis::ScreenPlane, [0.95, 0.95, 0.95, 0.9]);
    let center_sz = (axis_length * 0.10).clamp(3.5, 8.0);
    add_wire_box(lines, center, glam::Vec3::splat(center_sz), center_color);

    let x_dir = rot * glam::Vec3::X;
    let y_dir = rot * glam::Vec3::Y;
    let z_dir = rot * glam::Vec3::Z;

    // X Axis: Red
    let x_color = handle_color(crate::raycast::GizmoAxis::X, [0.95, 0.2, 0.2, 1.0]);
    let x_end = center + x_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), x_color));
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    let x_base = center + x_dir * (axis_length - tip_len);
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    lines.push(LineVertex::new((x_base + y_dir * tip_w).to_array(), x_color));
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    lines.push(LineVertex::new((x_base - y_dir * tip_w).to_array(), x_color));
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    lines.push(LineVertex::new((x_base + z_dir * tip_w).to_array(), x_color));
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    lines.push(LineVertex::new((x_base - z_dir * tip_w).to_array(), x_color));

    // Y Axis: Green
    let y_color = handle_color(crate::raycast::GizmoAxis::Y, [0.2, 0.95, 0.2, 1.0]);
    let y_end = center + y_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), y_color));
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    let y_base = center + y_dir * (axis_length - tip_len);
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    lines.push(LineVertex::new((y_base + x_dir * tip_w).to_array(), y_color));
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    lines.push(LineVertex::new((y_base - x_dir * tip_w).to_array(), y_color));
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    lines.push(LineVertex::new((y_base + z_dir * tip_w).to_array(), y_color));
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    lines.push(LineVertex::new((y_base - z_dir * tip_w).to_array(), y_color));

    // Z Axis: Blue
    let z_color = handle_color(crate::raycast::GizmoAxis::Z, [0.25, 0.5, 1.0, 1.0]);
    let z_end = center + z_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), z_color));
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    let z_base = center + z_dir * (axis_length - tip_len);
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    lines.push(LineVertex::new((z_base + x_dir * tip_w).to_array(), z_color));
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    lines.push(LineVertex::new((z_base - x_dir * tip_w).to_array(), z_color));
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    lines.push(LineVertex::new((z_base + y_dir * tip_w).to_array(), z_color));
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    lines.push(LineVertex::new((z_base - y_dir * tip_w).to_array(), z_color));

    // Planar Handles (XY ground, XZ front, YZ side)
    let p_off = axis_length * 0.20;
    let p_sz = axis_length * 0.28;

    // XY Plane (Cyan)
    let xy_color = handle_color(crate::raycast::GizmoAxis::PlaneXY, [0.2, 0.85, 0.95, 0.85]);
    let xy0 = center + x_dir * p_off + y_dir * p_off;
    let xy1 = center + x_dir * (p_off + p_sz) + y_dir * p_off;
    let xy2 = center + x_dir * (p_off + p_sz) + y_dir * (p_off + p_sz);
    let xy3 = center + x_dir * p_off + y_dir * (p_off + p_sz);
    lines.push(LineVertex::new(xy0.to_array(), xy_color));
    lines.push(LineVertex::new(xy1.to_array(), xy_color));
    lines.push(LineVertex::new(xy1.to_array(), xy_color));
    lines.push(LineVertex::new(xy2.to_array(), xy_color));
    lines.push(LineVertex::new(xy2.to_array(), xy_color));
    lines.push(LineVertex::new(xy3.to_array(), xy_color));
    lines.push(LineVertex::new(xy3.to_array(), xy_color));
    lines.push(LineVertex::new(xy0.to_array(), xy_color));

    // XZ Plane (Magenta)
    let xz_color = handle_color(crate::raycast::GizmoAxis::PlaneXZ, [0.95, 0.35, 0.85, 0.85]);
    let xz0 = center + x_dir * p_off + z_dir * p_off;
    let xz1 = center + x_dir * (p_off + p_sz) + z_dir * p_off;
    let xz2 = center + x_dir * (p_off + p_sz) + z_dir * (p_off + p_sz);
    let xz3 = center + x_dir * p_off + z_dir * (p_off + p_sz);
    lines.push(LineVertex::new(xz0.to_array(), xz_color));
    lines.push(LineVertex::new(xz1.to_array(), xz_color));
    lines.push(LineVertex::new(xz1.to_array(), xz_color));
    lines.push(LineVertex::new(xz2.to_array(), xz_color));
    lines.push(LineVertex::new(xz2.to_array(), xz_color));
    lines.push(LineVertex::new(xz3.to_array(), xz_color));
    lines.push(LineVertex::new(xz3.to_array(), xz_color));
    lines.push(LineVertex::new(xz0.to_array(), xz_color));

    // YZ Plane (Lime)
    let yz_color = handle_color(crate::raycast::GizmoAxis::PlaneYZ, [0.85, 0.95, 0.25, 0.85]);
    let yz0 = center + y_dir * p_off + z_dir * p_off;
    let yz1 = center + y_dir * (p_off + p_sz) + z_dir * p_off;
    let yz2 = center + y_dir * (p_off + p_sz) + z_dir * (p_off + p_sz);
    let yz3 = center + y_dir * p_off + z_dir * (p_off + p_sz);
    lines.push(LineVertex::new(yz0.to_array(), yz_color));
    lines.push(LineVertex::new(yz1.to_array(), yz_color));
    lines.push(LineVertex::new(yz1.to_array(), yz_color));
    lines.push(LineVertex::new(yz2.to_array(), yz_color));
    lines.push(LineVertex::new(yz2.to_array(), yz_color));
    lines.push(LineVertex::new(yz3.to_array(), yz_color));
    lines.push(LineVertex::new(yz3.to_array(), yz_color));
    lines.push(LineVertex::new(yz0.to_array(), yz_color));

    // Active Axis Extended Guide Lines (Phase 5 Visual Feedback)
    let guide_len = 8000.0;
    match active_axis {
        crate::raycast::GizmoAxis::X => {
            let col = [1.0, 0.35, 0.35, 0.40];
            lines.push(LineVertex::new((center - x_dir * guide_len).to_array(), col));
            lines.push(LineVertex::new((center + x_dir * guide_len).to_array(), col));
        }
        crate::raycast::GizmoAxis::Y => {
            let col = [0.35, 1.0, 0.35, 0.40];
            lines.push(LineVertex::new((center - y_dir * guide_len).to_array(), col));
            lines.push(LineVertex::new((center + y_dir * guide_len).to_array(), col));
        }
        crate::raycast::GizmoAxis::Z => {
            let col = [0.40, 0.65, 1.0, 0.40];
            lines.push(LineVertex::new((center - z_dir * guide_len).to_array(), col));
            lines.push(LineVertex::new((center + z_dir * guide_len).to_array(), col));
        }
        _ => {}
    }
}

fn add_rotation_gizmo(
    lines: &mut Vec<LineVertex>,
    center: glam::Vec3,
    radius: f32,
    active_axis: crate::raycast::GizmoAxis,
    hovered_axis: crate::raycast::GizmoAxis,
    rot: glam::Mat3,
) {
    let segments = 36;
    let step = std::f32::consts::TAU / segments as f32;

    let handle_color = |target_axis: crate::raycast::GizmoAxis, default_col: [f32; 4]| -> [f32; 4] {
        if active_axis == target_axis {
            [1.0, 1.0, 0.2, 1.0]
        } else if hovered_axis == target_axis {
            [1.0, 0.85, 0.25, 0.95]
        } else {
            default_col
        }
    };

    let x_dir = rot * glam::Vec3::X;
    let y_dir = rot * glam::Vec3::Y;
    let z_dir = rot * glam::Vec3::Z;

    // Pitch: X Axis (YZ Plane) - Red
    let x_color = handle_color(crate::raycast::GizmoAxis::X, [0.95, 0.2, 0.2, 1.0]);
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + y_dir * (radius * a0.cos()) + z_dir * (radius * a0.sin());
        let p1 = center + y_dir * (radius * a1.cos()) + z_dir * (radius * a1.sin());
        lines.push(LineVertex::new(p0.to_array(), x_color));
        lines.push(LineVertex::new(p1.to_array(), x_color));
    }

    // Roll: Y Axis (XZ Plane) - Green
    let y_color = handle_color(crate::raycast::GizmoAxis::Y, [0.2, 0.95, 0.2, 1.0]);
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + x_dir * (radius * a0.cos()) + z_dir * (radius * a0.sin());
        let p1 = center + x_dir * (radius * a1.cos()) + z_dir * (radius * a1.sin());
        lines.push(LineVertex::new(p0.to_array(), y_color));
        lines.push(LineVertex::new(p1.to_array(), y_color));
    }

    // Yaw: Z Axis (XY Plane) - Blue
    let z_color = handle_color(crate::raycast::GizmoAxis::Z, [0.25, 0.5, 1.0, 1.0]);
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + x_dir * (radius * a0.cos()) + y_dir * (radius * a0.sin());
        let p1 = center + x_dir * (radius * a1.cos()) + y_dir * (radius * a1.sin());
        lines.push(LineVertex::new(p0.to_array(), z_color));
        lines.push(LineVertex::new(p1.to_array(), z_color));
    }
}

fn add_scale_gizmo(
    lines: &mut Vec<LineVertex>,
    center: glam::Vec3,
    axis_length: f32,
    active_axis: crate::raycast::GizmoAxis,
    hovered_axis: crate::raycast::GizmoAxis,
    rot: glam::Mat3,
) {
    let box_size = glam::Vec3::splat(6.0);

    let handle_color = |target_axis: crate::raycast::GizmoAxis, default_col: [f32; 4]| -> [f32; 4] {
        if active_axis == target_axis {
            [1.0, 1.0, 0.2, 1.0]
        } else if hovered_axis == target_axis {
            [1.0, 0.85, 0.25, 0.95]
        } else {
            default_col
        }
    };

    // Center handle (uniform scale / ScreenPlane)
    let center_color = handle_color(crate::raycast::GizmoAxis::ScreenPlane, [1.0, 1.0, 1.0, 0.9]);
    add_wire_box(lines, center, glam::Vec3::splat(4.0), center_color);

    let x_dir = rot * glam::Vec3::X;
    let y_dir = rot * glam::Vec3::Y;
    let z_dir = rot * glam::Vec3::Z;

    // X Axis: Red
    let x_color = handle_color(crate::raycast::GizmoAxis::X, [0.95, 0.2, 0.2, 1.0]);
    let x_end = center + x_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), x_color));
    lines.push(LineVertex::new(x_end.to_array(), x_color));
    add_wire_box(lines, x_end, box_size, x_color);

    // Y Axis: Green
    let y_color = handle_color(crate::raycast::GizmoAxis::Y, [0.2, 0.95, 0.2, 1.0]);
    let y_end = center + y_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), y_color));
    lines.push(LineVertex::new(y_end.to_array(), y_color));
    add_wire_box(lines, y_end, box_size, y_color);

    // Z Axis: Blue
    let z_color = handle_color(crate::raycast::GizmoAxis::Z, [0.25, 0.5, 1.0, 1.0]);
    let z_end = center + z_dir * axis_length;
    lines.push(LineVertex::new(center.to_array(), z_color));
    lines.push(LineVertex::new(z_end.to_array(), z_color));
    add_wire_box(lines, z_end, box_size, z_color);
}

fn ut99_light_color_rgba(hue: u8, sat: u8, brightness: u8) -> [f32; 4] {
    let h = (hue as f32) / 255.0 * 6.0;
    let s = 1.0 - ((sat as f32) / 255.0);
    let v = ((brightness as f32) / 255.0).clamp(0.4, 1.0);

    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    [r + m, g + m, b + m, 0.75]
}

fn add_wire_sphere(lines: &mut Vec<LineVertex>, center: glam::Vec3, radius: f32, color: [f32; 4]) {
    let segments = 36;
    let step = std::f32::consts::TAU / segments as f32;

    // XY Ring (horizontal equator)
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + glam::Vec3::new(radius * a0.cos(), radius * a0.sin(), 0.0);
        let p1 = center + glam::Vec3::new(radius * a1.cos(), radius * a1.sin(), 0.0);
        lines.push(LineVertex::new(p0.to_array(), color));
        lines.push(LineVertex::new(p1.to_array(), color));
    }

    // XZ Ring (vertical prime meridian)
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + glam::Vec3::new(radius * a0.cos(), 0.0, radius * a0.sin());
        let p1 = center + glam::Vec3::new(radius * a1.cos(), 0.0, radius * a1.sin());
        lines.push(LineVertex::new(p0.to_array(), color));
        lines.push(LineVertex::new(p1.to_array(), color));
    }

    // YZ Ring (vertical perpendicular meridian)
    for i in 0..segments {
        let a0 = i as f32 * step;
        let a1 = (i + 1) as f32 * step;
        let p0 = center + glam::Vec3::new(0.0, radius * a0.cos(), radius * a0.sin());
        let p1 = center + glam::Vec3::new(0.0, radius * a1.cos(), radius * a1.sin());
        lines.push(LineVertex::new(p0.to_array(), color));
        lines.push(LineVertex::new(p1.to_array(), color));
    }
}

/// Renders a continuous 3D ribbon trajectory with dual rails, centerline, and directional flow arrows
fn add_3d_ribbon_trajectory(
    line_verts: &mut Vec<LineVertex>,
    points: &[glam::Vec3],
    half_width: f32,
    rail_color: [f32; 4],
    arrow_color: [f32; 4],
) {
    if points.len() < 2 {
        return;
    }

    let mut dist_accum = 0.0f32;
    let arrow_spacing = 64.0f32;

    for i in 0..points.len() - 1 {
        let p0 = points[i];
        let p1 = points[i + 1];
        let delta = p1 - p0;
        let seg_len = delta.length();
        if seg_len < 1e-4 {
            continue;
        }
        let tangent = delta / seg_len;

        // Compute binormal perpendicular to tangent and world up (or forward if tangent is vertical)
        let up = if tangent.z.abs() > 0.92 {
            glam::Vec3::Y
        } else {
            glam::Vec3::Z
        };
        let binormal = tangent.cross(up).normalize();

        // Left and Right rails
        let l0 = p0 - binormal * half_width;
        let r0 = p0 + binormal * half_width;
        let l1 = p1 - binormal * half_width;
        let r1 = p1 + binormal * half_width;

        // Outer rails
        line_verts.push(LineVertex::new(l0.to_array(), rail_color));
        line_verts.push(LineVertex::new(l1.to_array(), rail_color));

        line_verts.push(LineVertex::new(r0.to_array(), rail_color));
        line_verts.push(LineVertex::new(r1.to_array(), rail_color));

        // Center centerline track (fainter track)
        let center_color = [rail_color[0], rail_color[1], rail_color[2], rail_color[3] * 0.45];
        line_verts.push(LineVertex::new(p0.to_array(), center_color));
        line_verts.push(LineVertex::new(p1.to_array(), center_color));

        // Directional Chevron Arrows
        dist_accum += seg_len;
        if dist_accum >= arrow_spacing {
            dist_accum = 0.0;
            let mid = p0.lerp(p1, 0.5);
            let arrow_tip = mid + tangent * (half_width * 1.2);
            let arrow_left = mid - tangent * (half_width * 0.5) - binormal * (half_width * 0.85);
            let arrow_right = mid - tangent * (half_width * 0.5) + binormal * (half_width * 0.85);

            // Left wing of chevron
            line_verts.push(LineVertex::new(arrow_tip.to_array(), arrow_color));
            line_verts.push(LineVertex::new(arrow_left.to_array(), arrow_color));

            // Right wing of chevron
            line_verts.push(LineVertex::new(arrow_tip.to_array(), arrow_color));
            line_verts.push(LineVertex::new(arrow_right.to_array(), arrow_color));

            // Cross tie
            line_verts.push(LineVertex::new(l0.to_array(), [rail_color[0], rail_color[1], rail_color[2], 0.35]));
            line_verts.push(LineVertex::new(r0.to_array(), [rail_color[0], rail_color[1], rail_color[2], 0.35]));
        }
    }
}




#[cfg(test)]
mod mipmap_tests {
    use super::*;

    #[test]
    fn mipmap_chain_dimensions_reach_1x1() {
        let mut w = TEXTURE_ARRAY_DIM;
        let mut h = TEXTURE_ARRAY_DIM;
        let mut data = vec![128u8; (w * h * 4) as usize];
        let mut dims = vec![(w, h)];
        for _ in 1..TEXTURE_ARRAY_MIP_LEVELS {
            let (nw, nh, next) = downsample_rgba8_srgb_2x2(&data, w, h);
            dims.push((nw, nh));
            w = nw;
            h = nh;
            data = next;
        }
        assert_eq!(dims.len(), TEXTURE_ARRAY_MIP_LEVELS as usize);
        assert_eq!(dims[0], (256, 256));
        assert_eq!(*dims.last().unwrap(), (1, 1));
    }

    #[test]
    fn solid_srgb_downsample_preserves_constant_color() {
        let mut src = vec![0u8; 4 * 4 * 4];
        for px in src.chunks_exact_mut(4) {
            px.copy_from_slice(&[180, 90, 45, 255]);
        }
        let (nw, nh, dst) = downsample_rgba8_srgb_2x2(&src, 4, 4);
        assert_eq!((nw, nh), (2, 2));
        for px in dst.chunks_exact(4) {
            assert!((px[0] as i32 - 180).abs() <= 1);
            assert!((px[1] as i32 - 90).abs() <= 1);
            assert!((px[2] as i32 - 45).abs() <= 1);
            assert_eq!(px[3], 255);
        }
    }

    #[test]
    fn masked_downsample_avoids_black_alpha_bleeding() {
        // One opaque pure red texel + three transparent black texels in a 2x2 quad
        let src = vec![
            255, 0, 0, 255, // opaque red
            0, 0, 0, 0,     // transparent black
            0, 0, 0, 0,     // transparent black
            0, 0, 0, 0,     // transparent black
        ];
        let (nw, nh, dst) = downsample_rgba8_srgb_2x2(&src, 2, 2);
        assert_eq!((nw, nh), (1, 1));
        assert!(dst[0] >= 254, "opaque-weighted red channel must stay bright, got {}", dst[0]);
        assert_eq!(dst[1], 0);
        assert_eq!(dst[2], 0);
        assert_eq!(dst[3], 64);
    }
}

#[cfg(test)]
#[path = "frame_vertices_tests.rs"]
mod frame_vertices_tests;

#[cfg(test)]
#[path = "lightmap_budget_tests.rs"]
mod lightmap_budget_tests;

#[cfg(test)]
#[path = "zone_cache_tests.rs"]
mod zone_cache_tests;
