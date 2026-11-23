use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use glam::{Mat4, Quat};
use std::io::{self, Cursor};
use std::path::Path;

use super::actor::ActorRecord;
use super::polys::{RawPoly, RawPolys};
use super::tagged_property::{
    read_tagged_properties, write_tagged_properties, write_tagged_property_with_meta,
    TaggedProperty, TaggedValue,
};
use crate::package::compact_index::{read_compact_index, write_compact_index};
use crate::package::{Package, PACKAGE_FILE_TAG};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq)]
pub struct UnrBrush {
    pub actor: ActorRecord,
    pub local_polys: RawPolys,
    pub world_polys: Vec<RawPoly>,
}

/// Serialized UT99 Navigation ReachSpec (logical edge in `ULevel::ReachSpecs`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReachSpecRecord {
    pub distance: i32,
    pub start_actor_name: String,
    pub end_actor_name: String,
    pub collision_radius: i32,
    pub collision_height: i32,
    pub reach_flags: i32,
    pub b_pruned: u8,
}

/// Canonical UE1 `FBspNode` serialized in `UModel::Nodes`.
#[derive(Debug, Clone, PartialEq)]
pub struct UModelBspNode {
    pub plane: glam::Vec4,
    pub zone_mask: u64,
    pub node_flags: u8,
    pub i_vert_pool: i32,
    pub i_surf: i32,
    pub i_back: i32,
    pub i_front: i32,
    pub i_plane: i32,
    pub i_collision_bound: i32,
    pub i_render_bound: i32,
    pub i_zone: [u8; 2],
    pub num_vertices: u8,
    pub i_leaf: [i32; 2],
}

/// Canonical UE1 `FBspSurf` serialized in `UModel::Surfs`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UModelBspSurf {
    pub texture_ref: i32,
    pub poly_flags: u32,
    pub p_base: i32,
    pub v_normal: i32,
    pub v_texture_u: i32,
    pub v_texture_v: i32,
    pub i_light_map: i32,
    pub i_brush_poly: i32,
    pub pan_u: i16,
    pub pan_v: i16,
    pub actor_ref: i32,
}

/// Canonical UE1 `FVert` serialized in `UModel::Verts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UModelVert {
    pub p_vertex: i32,
    pub i_side: i32,
}

/// Canonical UE1 `FZoneProperties` serialized in `UModel::Zones`.
#[derive(Debug, Clone, PartialEq)]
pub struct UModelZoneProperties {
    pub zone_actor_ref: i32,
    pub connectivity: u64,
    pub visibility: u64,
    pub last_render_time: f32,
}

/// Canonical UE1 `FLightMapIndex` serialized in `UModel::LightMap`.
#[derive(Debug, Clone, PartialEq)]
pub struct UModelLightMapIndex {
    pub data_offset: i32,
    pub i_light_actors: i32,
    pub pan: glam::Vec3,
    pub u_scale: f32,
    pub v_scale: f32,
    pub u_clamp: i32,
    pub v_clamp: i32,
    pub surf_idx: i32,
    pub light_actors: i32,
}

/// Canonical UE1 `FBox` serialized in `UModel::Bounds`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UModelBox {
    pub min: glam::Vec3,
    pub max: glam::Vec3,
    pub is_valid: u8,
}

/// Canonical UE1 `FLeaf` serialized in `UModel::Leaves`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UModelLeaf {
    pub i_zone: i32,
    pub i_permeating: i32,
    pub i_volumetric: i32,
    pub visible_zones: u64,
}

/// Complete compiled UE1 `UModel` BSP & Zone payload (`Model_World`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CompiledUModelBsp {
    pub vectors: Vec<glam::Vec3>,
    pub points: Vec<glam::Vec3>,
    pub nodes: Vec<UModelBspNode>,
    pub surfs: Vec<UModelBspSurf>,
    pub verts: Vec<UModelVert>,
    pub num_shared_sides: i32,
    pub zones: Vec<UModelZoneProperties>,
    pub light_maps: Vec<UModelLightMapIndex>,
    pub light_bits: Vec<u8>,
    pub bounds: Vec<UModelBox>,
    pub leaf_hulls: Vec<i32>,
    pub leaves: Vec<UModelLeaf>,
    pub lights: Vec<i32>,
    pub root_outside: i32,
    pub linked: i32,
}

impl CompiledUModelBsp {
    /// Polygon flags that never decide world cavity solidity: `PF_NotSolid` (0x08), `PF_Semisolid` (0x20) and
    /// `PF_Portal` (0x04000000) surfaces are not CSG world splitters.
    const NON_SOLID_FLAGS: u32 = 0x08 | 0x20 | 0x0400_0000;

    /// Classifies a world position against the compiled BSP: returns `(is_solid, zone_index)`.
    /// The front side of a terminal node is open space, the back side is solid. Calibrated on the stock
    /// maps (PlayerStarts/pickups are never solid and always have a floor below them).
    pub fn classify_point(&self, p: glam::Vec3) -> (bool, i32) {
        if self.nodes.is_empty() {
            return (true, -1);
        }
        let mut idx = 0i32;
        let mut solid = true;
        let mut zone = 0i32;
        for _ in 0..self.nodes.len() + 1 {
            let Some(n) = self.nodes.get(idx as usize) else {
                break;
            };
            let d = n.plane.x * p.x + n.plane.y * p.y + n.plane.z * p.z - n.plane.w;
            let front = d > 0.0;
            zone = n.i_zone[usize::from(front)] as i32;
            let flags = self
                .surfs
                .get(n.i_surf.max(0) as usize)
                .filter(|_| n.i_surf >= 0)
                .map_or(0, |s| s.poly_flags);
            if flags & Self::NON_SOLID_FLAGS == 0 {
                solid = !front;
            }
            let child = if front { n.i_front } else { n.i_back };
            if child < 0 {
                let leaf_idx = n.i_leaf[usize::from(front)];
                if leaf_idx > 0 {
                    if let Some(leaf) = self.leaves.get(leaf_idx as usize) {
                        if leaf.i_zone > 0 {
                            solid = false;
                            zone = leaf.i_zone;
                        }
                    } else {
                        solid = false;
                    }
                }
                return (solid, zone);
            }
            idx = child;
        }
        (solid, zone)
    }

    pub fn point_is_solid(&self, p: glam::Vec3) -> bool {
        self.classify_point(p).0
    }

    /// Distance from `p` straight down to the nearest upward-facing semi-solid surface (`PF_Semisolid` = 0x20).
    pub fn semisolid_floor_distance(&self, p: glam::Vec3, max: f32) -> Option<f32> {
        let mut closest_d: Option<f32> = None;
        for node in &self.nodes {
            if node.i_surf < 0 {
                continue;
            }
            let Some(surf) = self.surfs.get(node.i_surf as usize) else {
                continue;
            };
            if surf.poly_flags & 0x0000_0020 == 0 || surf.poly_flags & (0x08 | 0x0400_0000) != 0 {
                continue;
            }
            if node.plane.z < 0.70 {
                continue;
            }
            let z_hit = (node.plane.w - node.plane.x * p.x - node.plane.y * p.y) / node.plane.z;
            let d = p.z - z_hit;
            if d < -0.5 || d > max {
                continue;
            }
            if node.num_vertices < 3 {
                continue;
            }
            let pool_start = node.i_vert_pool.max(0) as usize;
            let n_verts = node.num_vertices as usize;
            if pool_start + n_verts > self.verts.len() {
                continue;
            }
            let mut min_x = f32::MAX;
            let mut max_x = f32::MIN;
            let mut min_y = f32::MAX;
            let mut max_y = f32::MIN;
            let mut pts = Vec::with_capacity(n_verts);

            for k in 0..n_verts {
                let v_idx = self.verts[pool_start + k].p_vertex.max(0) as usize;
                let Some(&v) = self.points.get(v_idx) else { continue; };
                min_x = min_x.min(v.x);
                max_x = max_x.max(v.x);
                min_y = min_y.min(v.y);
                max_y = max_y.max(v.y);
                pts.push(v);
            }

            if pts.len() < 3 {
                continue;
            }

            if p.x < min_x - 2.0 || p.x > max_x + 2.0 || p.y < min_y - 2.0 || p.y > max_y + 2.0 {
                continue;
            }

            // Standard Jordan Curve (Even-Odd) point-in-polygon test
            let mut inside = false;
            let mut j = pts.len() - 1;
            for i in 0..pts.len() {
                let vi = pts[i];
                let vj = pts[j];
                if ((vi.y > p.y) != (vj.y > p.y))
                    && (p.x < (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y) + vi.x)
                {
                    inside = !inside;
                }
                j = i;
            }

            // Also accept if point is within the bounding box of an axis-aligned pad (pad edges)
            if !inside && p.x >= min_x - 1.0 && p.x <= max_x + 1.0 && p.y >= min_y - 1.0 && p.y <= max_y + 1.0 {
                inside = true;
            }

            if inside {
                let d_clamped = d.max(0.0);
                closest_d = Some(closest_d.map_or(d_clamped, |c| c.min(d_clamped)));
            }
        }
        closest_d
    }

    /// Distance from `p` straight down to the first solid or semi-solid floor, probing every `step` units up to `max`.
    pub fn floor_distance(&self, p: glam::Vec3, max: f32, step: f32) -> Option<f32> {
        let solid_d = {
            let mut d = 0.0;
            let mut found = None;
            while d <= max {
                if self.point_is_solid(p - glam::Vec3::new(0.0, 0.0, d)) {
                    found = Some(d);
                    break;
                }
                d += step;
            }
            found
        };

        let semi_d = self.semisolid_floor_distance(p, max);

        match (solid_d, semi_d) {
            (Some(s), Some(sm)) => Some(s.min(sm)),
            (Some(s), None) => Some(s),
            (None, Some(sm)) => Some(sm),
            (None, None) => None,
        }
    }

    /// Distance from `p` straight up to the first solid.
    pub fn ceiling_distance(&self, p: glam::Vec3, max: f32, step: f32) -> Option<f32> {
        let mut d = 0.0;
        while d <= max {
            if self.point_is_solid(p + glam::Vec3::new(0.0, 0.0, d)) {
                return Some(d);
            }
            d += step;
        }
        None
    }

    /// True when a vertical cylinder (axis + 8 rim points at 3 heights) is entirely in open space.
    pub fn cylinder_is_free(&self, center: glam::Vec3, radius: f32, half_height: f32) -> bool {
        let h = (half_height - 1.0).max(1.0);
        for dz in [-h, 0.0, h] {
            if self.point_is_solid(center + glam::Vec3::new(0.0, 0.0, dz)) {
                return false;
            }
            for k in 0..8 {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                let q = center + glam::Vec3::new(a.cos() * radius, a.sin() * radius, dz);
                if self.point_is_solid(q) {
                    return false;
                }
            }
        }
        true
    }
}

/// Structural validation report for a UT99 `.unr` package.
#[derive(Debug, Clone, Default)]
pub struct UnrValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub actor_count: usize,
    pub brush_count: usize,
    pub model_count: usize,
    pub reach_spec_count: usize,
    pub bsp_node_count: usize,
    pub bsp_surf_count: usize,
    pub bsp_vert_count: usize,
    pub zone_count: usize,
}

/// A recoverable problem encountered while decoding the level. The list is not
/// exhaustive: an empty list does not certify the entire package was decoded.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UnrReadIssue {
    pub kind: String,
    pub actor_name: Option<String>,
    pub export_index: Option<usize>,
    pub detail: String,
}

#[derive(Debug, Clone)]
pub struct UnrLevel {
    pub map_name: String,
    pub actors: crate::TrackedVec<ActorRecord>,
    pub brushes: Vec<UnrBrush>,
    pub lights: Vec<ActorRecord>,
    pub player_starts: Vec<ActorRecord>,
    pub path_nodes: Vec<ActorRecord>,
    pub weapons_and_pickups: Vec<ActorRecord>,
    pub other_actors: Vec<ActorRecord>,
    pub embedded_textures: Vec<crate::utx::UtxTexture>,
    pub embedded_sounds: Vec<crate::package::SoundResource>,
    /// Optional per-brush, per-polygon symbolic texture names (`Package.Group.Texture` or `MyLevel.Texture`).
    pub brush_poly_textures: Vec<Vec<String>>,
    /// AI navigation graph edges serialized into `ULevel::ReachSpecs` and `NavigationPoint::Paths`.
    pub reach_specs: Vec<ReachSpecRecord>,
    /// Compiled `Model_World` BSP & Zone structures serialized into / parsed from `ULevel::Model`.
    pub world_bsp: Option<CompiledUModelBsp>,
    pub read_issues: Vec<UnrReadIssue>,
    pub package: Package,
}

impl Default for UnrLevel {
    fn default() -> Self {
        Self {
            map_name: "Untitled".to_string(),
            actors: crate::TrackedVec::new(),
            brushes: Vec::new(),
            lights: Vec::new(),
            player_starts: Vec::new(),
            path_nodes: Vec::new(),
            weapons_and_pickups: Vec::new(),
            other_actors: Vec::new(),
            embedded_textures: Vec::new(),
            embedded_sounds: Vec::new(),
            brush_poly_textures: Vec::new(),
            reach_specs: Vec::new(),
            world_bsp: None,
            read_issues: Vec::new(),
            package: Package::default(),
        }
    }
}

fn find_or_add_import(
    pkg: &mut Package,
    class_package: i32,
    class_name: i32,
    package_index: i32,
    object_name: i32,
) -> i32 {
    for (idx, imp) in pkg.imports.entries.iter().enumerate() {
        if imp.class_package == class_package
            && imp.class_name == class_name
            && imp.package_index == package_index
            && imp.object_name == object_name
        {
            return -((idx as i32) + 1);
        }
    }
    pkg.add_import(class_package, class_name, package_index, object_name)
}

fn import_or_resolve_object_path(
    pkg: &mut Package,
    raw_path: &str,
    default_class: &str,
    _imp_core: i32,
    imp_engine: i32,
    n_package: i32,
    n_class: i32,
) -> i32 {
    let trimmed = raw_path.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("None")
        || trimmed.eq_ignore_ascii_case("Default")
    {
        return 0;
    }

    let n_core = pkg.names.add("Core", 0) as i32;
    let n_engine = pkg.names.add("Engine", 0) as i32;

    let (cls_str, inner) = if let Some(open_q) = trimmed.find('\'') {
        if let Some(close_q) = trimmed.rfind('\'') {
            if close_q > open_q {
                (&trimmed[..open_q], &trimmed[open_q + 1..close_q])
            } else {
                (default_class, trimmed)
            }
        } else {
            (default_class, trimmed)
        }
    } else {
        (default_class, trimmed)
    };

    let inner = inner.trim();
    if inner.is_empty()
        || inner.eq_ignore_ascii_case("None")
        || inner.eq_ignore_ascii_case("Default")
    {
        return 0;
    }

    let normalized_inner: String;
    let inner = match inner.to_ascii_lowercase().as_str() {
        "genfluid.slime.slim2" | "slime.slime01" | "indus.slime.greenglowpipe01" => {
            "GenFluid.Water.dgoop3"
        }
        "genfluid.water.wetwater1" => "GenFluid.Water.Water1",
        "hubeffects.lavax2" => "GenFluid.lava.Lava2",
        "greatfire.ancpurp" => "GenFluid.Water.Water1",
        "egypt.gyptrm1" => "Egypt.GypWal1",
        "genearth.rock.icer" => "Ancient.Base.STONTSTC",
        "doorsmod.ls1" | "doorsmod.ds1" => "DoorsMod.General.mlift2sta",
        "doorsmod.le1" | "doorsmod.de1" => "DoorsMod.General.mlift2end",
        _ => {
            if inner.len() > 15 && inner[..15].eq_ignore_ascii_case("GenFluid.Slime.") {
                normalized_inner = "GenFluid.Water.dgoop3".to_string();
                &normalized_inner
            } else {
                inner
            }
        }
    };

    let parts: Vec<&str> = inner.split('.').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return 0;
    }

    // Check if it refers to an existing export in MyLevel
    if parts[0].eq_ignore_ascii_case("MyLevel") || parts.len() == 1 {
        let target_obj = parts.last().copied().unwrap_or(inner);
        for (exp_idx, exp) in pkg.exports.entries.iter().enumerate() {
            if let Some(en) = pkg.get_name(exp.object_name.max(0) as usize) {
                if en.eq_ignore_ascii_case(target_obj) {
                    return (exp_idx + 1) as i32;
                }
            }
        }
        if parts[0].eq_ignore_ascii_case("MyLevel") && parts.len() >= 2 {
            // If MyLevel prefix was used for an external texture or unexported asset, strip MyLevel
            if parts.len() == 2 {
                let n_obj = pkg.names.add(parts[1], 0) as i32;
                let n_cls = pkg.names.add(cls_str, 0) as i32;
                return find_or_add_import(pkg, n_engine, n_cls, imp_engine, n_obj);
            }
        }
    }

    let (class_pkg, class_nm) = if cls_str.eq_ignore_ascii_case("Class") {
        (n_core, n_class)
    } else if cls_str.eq_ignore_ascii_case("Package") {
        (n_core, n_package)
    } else {
        let n_cls = pkg.names.add(cls_str, 0) as i32;
        (n_engine, n_cls)
    };

    if parts.len() == 1 {
        let n_obj = pkg.names.add(parts[0], 0) as i32;
        return find_or_add_import(pkg, class_pkg, class_nm, imp_engine, n_obj);
    }

    // Multi-part path: Package.[Group.]Object
    let n_outer = pkg.names.add(parts[0], 0) as i32;
    let mut parent_ref = find_or_add_import(pkg, n_core, n_package, 0, n_outer);
    for &grp in &parts[1..parts.len() - 1] {
        let n_grp = pkg.names.add(grp, 0) as i32;
        parent_ref = find_or_add_import(pkg, n_core, n_package, parent_ref, n_grp);
    }
    let n_leaf = pkg.names.add(*parts.last().unwrap(), 0) as i32;
    find_or_add_import(pkg, class_pkg, class_nm, parent_ref, n_leaf)
}

fn build_umodel_bytes(
    name_none: i32,
    polys: &[RawPoly],
    polys_ref: i32,
    root_outside: i32,
) -> io::Result<Vec<u8>> {
    let mut min_v = glam::Vec3::splat(f32::MAX);
    let mut max_v = glam::Vec3::splat(f32::MIN);
    let mut has_verts = false;
    for p in polys {
        for &v in &p.vertices {
            min_v = min_v.min(v);
            max_v = max_v.max(v);
            has_verts = true;
        }
    }
    if !has_verts {
        min_v = glam::Vec3::splat(-128.0);
        max_v = glam::Vec3::splat(128.0);
    }
    let center = (min_v + max_v) * 0.5;
    let radius = ((max_v - min_v) * 0.5).length();

    let mut model_bytes = Vec::with_capacity(64);
    // 1. UObject tagged properties terminator (None)
    write_compact_index(&mut model_bytes, name_none)?;
    // 2. UPrimitive BoundingBox (25 bytes: min Vec3, max Vec3, is_valid u8)
    model_bytes.write_f32::<LittleEndian>(min_v.x)?;
    model_bytes.write_f32::<LittleEndian>(min_v.y)?;
    model_bytes.write_f32::<LittleEndian>(min_v.z)?;
    model_bytes.write_f32::<LittleEndian>(max_v.x)?;
    model_bytes.write_f32::<LittleEndian>(max_v.y)?;
    model_bytes.write_f32::<LittleEndian>(max_v.z)?;
    model_bytes.write_u8(if has_verts { 1 } else { 0 })?;
    // 3. UPrimitive BoundingSphere (16 bytes: center Vec3, radius f32)
    model_bytes.write_f32::<LittleEndian>(center.x)?;
    model_bytes.write_f32::<LittleEndian>(center.y)?;
    model_bytes.write_f32::<LittleEndian>(center.z)?;
    model_bytes.write_f32::<LittleEndian>(radius)?;
    // 4. 5 empty TTransArrays: Vectors, Points, Nodes, Surfs, Verts
    for _ in 0..5 {
        write_compact_index(&mut model_bytes, 0)?;
    }
    // 5. NumSharedSides = 4, NumZones = 0
    model_bytes.write_i32::<LittleEndian>(4)?;
    model_bytes.write_i32::<LittleEndian>(0)?;
    // 6. Polys reference
    write_compact_index(&mut model_bytes, polys_ref)?;
    // 7. LightMap, LightBits, Bounds, LeafHulls, Leaves, Lights (6 empty arrays)
    for _ in 0..6 {
        write_compact_index(&mut model_bytes, 0)?;
    }
    // 8. RootOutside & Linked
    model_bytes.write_i32::<LittleEndian>(root_outside)?;
    model_bytes.write_i32::<LittleEndian>(0)?;

    Ok(model_bytes)
}

/// Compiles world-space `RawPoly` surfaces into a full UE1 `CompiledUModelBsp` (`Vectors`, `Points`,
/// `Nodes` `FBspNode`, `Surfs` `FBspSurf`, `Verts` `FVert`, `Zones`, `LightMap`, `LightBits`, `Bounds`, `Leaves`).
pub fn compile_bsp_from_world_polys(polys: &[RawPoly]) -> CompiledUModelBsp {
    let mut bsp = CompiledUModelBsp {
        num_shared_sides: 4,
        zones: vec![
            UModelZoneProperties {
                zone_actor_ref: 0,
                connectivity: 0x3,
                visibility: 0x3,
                last_render_time: 0.0,
            },
            UModelZoneProperties {
                zone_actor_ref: 0,
                connectivity: 0x3,
                visibility: 0x3,
                last_render_time: 0.0,
            },
        ],
        root_outside: 0,
        linked: if polys.is_empty() { 0 } else { 1 },
        ..Default::default()
    };

    if polys.is_empty() {
        return bsp;
    }

    let add_unique_vec = |list: &mut Vec<glam::Vec3>, v: glam::Vec3| -> i32 {
        if let Some(pos) = list.iter().position(|x| (*x - v).length_squared() < 1e-4) {
            pos as i32
        } else {
            let idx = list.len() as i32;
            list.push(v);
            idx
        }
    };

    // Leaves: 0 = outside/solid leaf, 1 = playable interior leaf (Zone 1)
    bsp.leaves.push(UModelLeaf {
        i_zone: 0,
        i_permeating: -1,
        i_volumetric: -1,
        visible_zones: 0x3,
    });
    bsp.leaves.push(UModelLeaf {
        i_zone: 1,
        i_permeating: -1,
        i_volumetric: -1,
        visible_zones: 0x3,
    });

    // Populate Surfaces, Vectors, Points, LightMaps, LightBits
    let mut poly_surf_indices = Vec::with_capacity(polys.len());
    for (surf_idx, p) in polys.iter().enumerate() {
        let p_base = add_unique_vec(&mut bsp.points, p.base);
        let normal = p.normal.normalize_or_zero();
        let v_normal = add_unique_vec(&mut bsp.vectors, normal);
        let v_tex_u = add_unique_vec(&mut bsp.vectors, p.texture_u);
        let v_tex_v = add_unique_vec(&mut bsp.vectors, p.texture_v);

        let lm_idx = bsp.light_maps.len() as i32;
        let data_offset = bsp.light_bits.len() as i32;
        // 8x8 default fully-lit lightmap bits (8 bytes)
        bsp.light_bits.extend_from_slice(&[0xFF; 8]);
        bsp.light_maps.push(UModelLightMapIndex {
            data_offset,
            i_light_actors: -1,
            pan: p.base,
            u_scale: 16.0,
            v_scale: 16.0,
            u_clamp: 8,
            v_clamp: 8,
            surf_idx: surf_idx as i32,
            light_actors: 0,
        });

        bsp.surfs.push(UModelBspSurf {
            texture_ref: p.texture_ref,
            poly_flags: p.poly_flags,
            p_base,
            v_normal,
            v_texture_u: v_tex_u,
            v_texture_v: v_tex_v,
            i_light_map: lm_idx,
            i_brush_poly: p.brush_poly_index,
            pan_u: p.pan_u,
            pan_v: p.pan_v,
            actor_ref: p.actor_ref,
        });
        poly_surf_indices.push((surf_idx as i32, p.clone()));
    }

    // Build recursive BSP nodes
    fn build_nodes_recursive(
        items: &[(i32, RawPoly)],
        bsp: &mut CompiledUModelBsp,
    ) -> i32 {
        if items.is_empty() {
            return -1;
        }
        let (surf_idx, ref root_poly) = items[0];
        let n = root_poly.normal.normalize_or_zero();
        let plane_w = n.dot(root_poly.base);

        // Compute bounding box for all items in this subtree
        let mut b_min = glam::Vec3::splat(f32::MAX);
        let mut b_max = glam::Vec3::splat(f32::MIN);
        for (_, poly) in items {
            for &v in &poly.vertices {
                b_min = b_min.min(v);
                b_max = b_max.max(v);
            }
        }
        let bound_idx = bsp.bounds.len() as i32;
        bsp.bounds.push(UModelBox {
            min: b_min,
            max: b_max,
            is_valid: 1,
        });

        // Add vertices of root_poly to Verts pool
        let i_vert_pool = bsp.verts.len() as i32;
        let num_verts = root_poly.vertices.len().min(255);
        for &v in root_poly.vertices.iter().take(num_verts) {
            let p_vertex = if let Some(pos) = bsp
                .points
                .iter()
                .position(|x| (*x - v).length_squared() < 1e-4)
            {
                pos as i32
            } else {
                let idx = bsp.points.len() as i32;
                bsp.points.push(v);
                idx
            };
            bsp.verts.push(UModelVert { p_vertex, i_side: -1 });
        }

        let node_idx = bsp.nodes.len() as i32;
        bsp.nodes.push(UModelBspNode {
            plane: glam::Vec4::new(n.x, n.y, n.z, plane_w),
            zone_mask: 0x3,
            node_flags: 0,
            i_vert_pool,
            i_surf: surf_idx,
            i_back: -1,
            i_front: -1,
            i_plane: -1,
            i_collision_bound: -1,
            i_render_bound: bound_idx,
            i_zone: [1, 1],
            num_vertices: num_verts as u8,
            i_leaf: [0, 1],
        });

        let mut front_items = Vec::new();
        let mut back_items = Vec::new();
        let mut coplanar_items = Vec::new();

        for item in &items[1..] {
            let mut max_d = f32::MIN;
            let mut min_d = f32::MAX;
            for &v in &item.1.vertices {
                let d = n.dot(v) - plane_w;
                max_d = max_d.max(d);
                min_d = min_d.min(d);
            }
            if min_d >= -0.25 && max_d <= 0.25 {
                coplanar_items.push(item.clone());
            } else if min_d >= -0.25 {
                front_items.push(item.clone());
            } else if max_d <= 0.25 {
                back_items.push(item.clone());
            } else {
                // Spanning polygon: place in front or back by centroid
                let center = item
                    .1
                    .vertices
                    .iter()
                    .copied()
                    .fold(glam::Vec3::ZERO, |a, b| a + b)
                    / (item.1.vertices.len().max(1) as f32);
                if n.dot(center) - plane_w >= 0.0 {
                    front_items.push(item.clone());
                } else {
                    back_items.push(item.clone());
                }
            }
        }

        let i_plane = build_nodes_recursive(&coplanar_items, bsp);
        let i_front = build_nodes_recursive(&front_items, bsp);
        let i_back = build_nodes_recursive(&back_items, bsp);

        bsp.nodes[node_idx as usize].i_plane = i_plane;
        bsp.nodes[node_idx as usize].i_front = i_front;
        bsp.nodes[node_idx as usize].i_back = i_back;

        node_idx
    }

    build_nodes_recursive(&poly_surf_indices, &mut bsp);
    bsp
}

fn build_world_umodel_bytes(
    name_none: i32,
    polys: &[RawPoly],
    polys_ref: i32,
    bsp: &CompiledUModelBsp,
) -> io::Result<Vec<u8>> {
    let mut min_v = glam::Vec3::splat(f32::MAX);
    let mut max_v = glam::Vec3::splat(f32::MIN);
    let mut has_verts = false;
    for p in polys {
        for &v in &p.vertices {
            min_v = min_v.min(v);
            max_v = max_v.max(v);
            has_verts = true;
        }
    }
    if !has_verts {
        min_v = glam::Vec3::splat(-128.0);
        max_v = glam::Vec3::splat(128.0);
    }
    let center = (min_v + max_v) * 0.5;
    let radius = ((max_v - min_v) * 0.5).length();

    let mut buf = Vec::with_capacity(512);
    // 1. UObject tagged properties terminator (None)
    write_compact_index(&mut buf, name_none)?;
    // 2. UPrimitive BoundingBox (25 bytes) + BoundingSphere (16 bytes)
    buf.write_f32::<LittleEndian>(min_v.x)?;
    buf.write_f32::<LittleEndian>(min_v.y)?;
    buf.write_f32::<LittleEndian>(min_v.z)?;
    buf.write_f32::<LittleEndian>(max_v.x)?;
    buf.write_f32::<LittleEndian>(max_v.y)?;
    buf.write_f32::<LittleEndian>(max_v.z)?;
    buf.write_u8(if has_verts { 1 } else { 0 })?;
    buf.write_f32::<LittleEndian>(center.x)?;
    buf.write_f32::<LittleEndian>(center.y)?;
    buf.write_f32::<LittleEndian>(center.z)?;
    buf.write_f32::<LittleEndian>(radius)?;

    // 3. Vectors
    write_compact_index(&mut buf, bsp.vectors.len() as i32)?;
    for v in &bsp.vectors {
        buf.write_f32::<LittleEndian>(v.x)?;
        buf.write_f32::<LittleEndian>(v.y)?;
        buf.write_f32::<LittleEndian>(v.z)?;
    }

    // 4. Points
    write_compact_index(&mut buf, bsp.points.len() as i32)?;
    for p in &bsp.points {
        buf.write_f32::<LittleEndian>(p.x)?;
        buf.write_f32::<LittleEndian>(p.y)?;
        buf.write_f32::<LittleEndian>(p.z)?;
    }

    // 5. Nodes (FBspNode)
    write_compact_index(&mut buf, bsp.nodes.len() as i32)?;
    for n in &bsp.nodes {
        buf.write_f32::<LittleEndian>(n.plane.x)?;
        buf.write_f32::<LittleEndian>(n.plane.y)?;
        buf.write_f32::<LittleEndian>(n.plane.z)?;
        buf.write_f32::<LittleEndian>(n.plane.w)?;
        buf.write_u64::<LittleEndian>(n.zone_mask)?;
        buf.write_u8(n.node_flags)?;
        write_compact_index(&mut buf, n.i_vert_pool)?;
        write_compact_index(&mut buf, n.i_surf)?;
        write_compact_index(&mut buf, n.i_back)?;
        write_compact_index(&mut buf, n.i_front)?;
        write_compact_index(&mut buf, n.i_plane)?;
        write_compact_index(&mut buf, n.i_collision_bound)?;
        write_compact_index(&mut buf, n.i_render_bound)?;
        buf.write_u8(n.i_zone[0])?;
        buf.write_u8(n.i_zone[1])?;
        buf.write_u8(n.num_vertices)?;
        buf.write_i32::<LittleEndian>(n.i_leaf[0])?;
        buf.write_i32::<LittleEndian>(n.i_leaf[1])?;
    }

    // 6. Surfs (FBspSurf)
    write_compact_index(&mut buf, bsp.surfs.len() as i32)?;
    for s in &bsp.surfs {
        write_compact_index(&mut buf, s.texture_ref)?;
        buf.write_u32::<LittleEndian>(s.poly_flags)?;
        write_compact_index(&mut buf, s.p_base)?;
        write_compact_index(&mut buf, s.v_normal)?;
        write_compact_index(&mut buf, s.v_texture_u)?;
        write_compact_index(&mut buf, s.v_texture_v)?;
        write_compact_index(&mut buf, s.i_light_map)?;
        write_compact_index(&mut buf, s.i_brush_poly)?;
        buf.write_i16::<LittleEndian>(s.pan_u)?;
        buf.write_i16::<LittleEndian>(s.pan_v)?;
        write_compact_index(&mut buf, s.actor_ref)?;
    }

    // 7. Verts (FVert)
    write_compact_index(&mut buf, bsp.verts.len() as i32)?;
    for v in &bsp.verts {
        write_compact_index(&mut buf, v.p_vertex)?;
        write_compact_index(&mut buf, v.i_side)?;
    }

    // 8. NumSharedSides, NumZones, Zones[NumZones] (17 bytes per zone in UE1)
    buf.write_i32::<LittleEndian>(bsp.num_shared_sides)?;
    buf.write_i32::<LittleEndian>(bsp.zones.len() as i32)?;
    for z in &bsp.zones {
        write_compact_index(&mut buf, z.zone_actor_ref)?;
        buf.write_u64::<LittleEndian>(z.connectivity)?;
        buf.write_u64::<LittleEndian>(z.visibility)?;
    }

    // 9. Polys reference
    write_compact_index(&mut buf, polys_ref)?;

    // 10. LightMap (TArray<FLightMapIndex>: DataOffset, Pan, UClamp, VClamp, UScale, VScale, iLightActors)
    write_compact_index(&mut buf, bsp.light_maps.len() as i32)?;
    for lm in &bsp.light_maps {
        buf.write_i32::<LittleEndian>(lm.data_offset)?;
        buf.write_f32::<LittleEndian>(lm.pan.x)?;
        buf.write_f32::<LittleEndian>(lm.pan.y)?;
        buf.write_f32::<LittleEndian>(lm.pan.z)?;
        write_compact_index(&mut buf, lm.u_clamp)?;
        write_compact_index(&mut buf, lm.v_clamp)?;
        buf.write_f32::<LittleEndian>(lm.u_scale)?;
        buf.write_f32::<LittleEndian>(lm.v_scale)?;
        buf.write_i32::<LittleEndian>(lm.i_light_actors)?;
    }

    // 11. LightBits (TArray<BYTE>)
    write_compact_index(&mut buf, bsp.light_bits.len() as i32)?;
    buf.extend_from_slice(&bsp.light_bits);

    // 12. Bounds (TArray<FBox>)
    write_compact_index(&mut buf, bsp.bounds.len() as i32)?;
    for b in &bsp.bounds {
        buf.write_f32::<LittleEndian>(b.min.x)?;
        buf.write_f32::<LittleEndian>(b.min.y)?;
        buf.write_f32::<LittleEndian>(b.min.z)?;
        buf.write_f32::<LittleEndian>(b.max.x)?;
        buf.write_f32::<LittleEndian>(b.max.y)?;
        buf.write_f32::<LittleEndian>(b.max.z)?;
        buf.write_u8(b.is_valid)?;
    }

    // 13. LeafHulls (TArray<INT>)
    write_compact_index(&mut buf, bsp.leaf_hulls.len() as i32)?;
    for &lh in &bsp.leaf_hulls {
        buf.write_i32::<LittleEndian>(lh)?;
    }

    // 14. Leaves (TArray<FLeaf>)
    write_compact_index(&mut buf, bsp.leaves.len() as i32)?;
    for lf in &bsp.leaves {
        write_compact_index(&mut buf, lf.i_zone)?;
        write_compact_index(&mut buf, lf.i_permeating)?;
        write_compact_index(&mut buf, lf.i_volumetric)?;
        buf.write_u64::<LittleEndian>(lf.visible_zones)?;
    }

    // 15. Lights (TArray<AActor*>)
    write_compact_index(&mut buf, bsp.lights.len() as i32)?;
    for &l_ref in &bsp.lights {
        write_compact_index(&mut buf, l_ref)?;
    }

    // 16. RootOutside & Linked
    buf.write_i32::<LittleEndian>(bsp.root_outside)?;
    buf.write_i32::<LittleEndian>(bsp.linked)?;

    Ok(buf)
}

/// Parses a compiled `UModel` export binary payload into `CompiledUModelBsp` and optional `Polys` reference.
pub fn parse_compiled_umodel_bsp(
    model_bytes: &[u8],
    object_flags: u32,
    package: &Package,
) -> Option<(CompiledUModelBsp, i32)> {
    if model_bytes.len() < 42 {
        return None;
    }
    let has_stack = (object_flags & 0x02000000) != 0;
    let mut cur = Cursor::new(model_bytes);
    if has_stack {
        if let Ok(node) = read_compact_index(&mut cur) {
            let _ = read_compact_index(&mut cur);
            let _ = cur.read_u64::<LittleEndian>();
            let _ = cur.read_i32::<LittleEndian>();
            if node != 0 {
                let _ = read_compact_index(&mut cur);
            }
        }
    }
    let after_props_pos = if read_tagged_properties(&mut cur, package).is_ok() {
        cur.position() as usize
    } else {
        1usize
    };
    if after_props_pos + 41 > model_bytes.len() {
        return None;
    }
    cur.set_position((after_props_pos + 41) as u64);

    let mut bsp = CompiledUModelBsp::default();

    let vec_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&vec_cnt) {
        return None;
    }
    for _ in 0..vec_cnt {
        let x = cur.read_f32::<LittleEndian>().ok()?;
        let y = cur.read_f32::<LittleEndian>().ok()?;
        let z = cur.read_f32::<LittleEndian>().ok()?;
        bsp.vectors.push(glam::Vec3::new(x, y, z));
    }

    let pt_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&pt_cnt) {
        return None;
    }
    for _ in 0..pt_cnt {
        let x = cur.read_f32::<LittleEndian>().ok()?;
        let y = cur.read_f32::<LittleEndian>().ok()?;
        let z = cur.read_f32::<LittleEndian>().ok()?;
        bsp.points.push(glam::Vec3::new(x, y, z));
    }

    let node_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&node_cnt) {
        return None;
    }
    for _ in 0..node_cnt {
        let px = cur.read_f32::<LittleEndian>().ok()?;
        let py = cur.read_f32::<LittleEndian>().ok()?;
        let pz = cur.read_f32::<LittleEndian>().ok()?;
        let pw = cur.read_f32::<LittleEndian>().ok()?;
        let zone_mask = cur.read_u64::<LittleEndian>().ok()?;
        let node_flags = cur.read_u8().ok()?;
        let i_vert_pool = read_compact_index(&mut cur).ok()?;
        let i_surf = read_compact_index(&mut cur).ok()?;
        let i_back = read_compact_index(&mut cur).ok()?;
        let i_front = read_compact_index(&mut cur).ok()?;
        let i_plane = read_compact_index(&mut cur).ok()?;
        let i_collision_bound = read_compact_index(&mut cur).ok()?;
        let i_render_bound = read_compact_index(&mut cur).ok()?;
        let z0 = cur.read_u8().ok()?;
        let z1 = cur.read_u8().ok()?;
        let num_vertices = cur.read_u8().ok()?;
        let l0 = cur.read_i32::<LittleEndian>().ok()?;
        let l1 = cur.read_i32::<LittleEndian>().ok()?;
        bsp.nodes.push(UModelBspNode {
            plane: glam::Vec4::new(px, py, pz, pw),
            zone_mask,
            node_flags,
            i_vert_pool,
            i_surf,
            i_back,
            i_front,
            i_plane,
            i_collision_bound,
            i_render_bound,
            i_zone: [z0, z1],
            num_vertices,
            i_leaf: [l0, l1],
        });
    }

    let surf_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&surf_cnt) {
        return None;
    }
    for _ in 0..surf_cnt {
        let texture_ref = read_compact_index(&mut cur).ok()?;
        let poly_flags = cur.read_u32::<LittleEndian>().ok()?;
        let p_base = read_compact_index(&mut cur).ok()?;
        let v_normal = read_compact_index(&mut cur).ok()?;
        let v_texture_u = read_compact_index(&mut cur).ok()?;
        let v_texture_v = read_compact_index(&mut cur).ok()?;
        let i_light_map = read_compact_index(&mut cur).ok()?;
        let i_brush_poly = read_compact_index(&mut cur).ok()?;
        let pan_u = cur.read_i16::<LittleEndian>().ok()?;
        let pan_v = cur.read_i16::<LittleEndian>().ok()?;
        let actor_ref = read_compact_index(&mut cur).ok()?;
        bsp.surfs.push(UModelBspSurf {
            texture_ref,
            poly_flags,
            p_base,
            v_normal,
            v_texture_u,
            v_texture_v,
            i_light_map,
            i_brush_poly,
            pan_u,
            pan_v,
            actor_ref,
        });
    }

    let vert_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=500_000).contains(&vert_cnt) {
        return None;
    }
    for _ in 0..vert_cnt {
        let p_vertex = read_compact_index(&mut cur).ok()?;
        let i_side = read_compact_index(&mut cur).ok()?;
        bsp.verts.push(UModelVert { p_vertex, i_side });
    }

    bsp.num_shared_sides = cur.read_i32::<LittleEndian>().ok()?;
    let num_zones = cur.read_i32::<LittleEndian>().ok()?;
    if !(0..=64).contains(&num_zones) {
        return None;
    }
    for _ in 0..num_zones {
        let zone_actor_ref = read_compact_index(&mut cur).ok()?;
        let connectivity = cur.read_u64::<LittleEndian>().ok()?;
        let visibility = cur.read_u64::<LittleEndian>().ok()?;
        bsp.zones.push(UModelZoneProperties {
            zone_actor_ref,
            connectivity,
            visibility,
            last_render_time: 0.0,
        });
    }

    let polys_ref = read_compact_index(&mut cur).ok()?;

    let lm_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&lm_cnt) {
        return None;
    }
    for _ in 0..lm_cnt {
        let data_offset = cur.read_i32::<LittleEndian>().ok()?;
        let px = cur.read_f32::<LittleEndian>().ok()?;
        let py = cur.read_f32::<LittleEndian>().ok()?;
        let pz = cur.read_f32::<LittleEndian>().ok()?;
        let u_clamp = read_compact_index(&mut cur).ok()?;
        let v_clamp = read_compact_index(&mut cur).ok()?;
        let u_scale = cur.read_f32::<LittleEndian>().ok()?;
        let v_scale = cur.read_f32::<LittleEndian>().ok()?;
        let i_light_actors = cur.read_i32::<LittleEndian>().ok()?;
        bsp.light_maps.push(UModelLightMapIndex {
            data_offset,
            i_light_actors,
            pan: glam::Vec3::new(px, py, pz),
            u_scale,
            v_scale,
            u_clamp,
            v_clamp,
            surf_idx: 0,
            light_actors: 0,
        });
    }

    let lb_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=10_000_000).contains(&lb_cnt) {
        return None;
    }
    let pos = cur.position() as usize;
    if pos + (lb_cnt as usize) > model_bytes.len() {
        return None;
    }
    bsp.light_bits
        .extend_from_slice(&model_bytes[pos..pos + (lb_cnt as usize)]);
    cur.set_position((pos + (lb_cnt as usize)) as u64);

    let bounds_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&bounds_cnt) {
        return None;
    }
    for _ in 0..bounds_cnt {
        let min_x = cur.read_f32::<LittleEndian>().ok()?;
        let min_y = cur.read_f32::<LittleEndian>().ok()?;
        let min_z = cur.read_f32::<LittleEndian>().ok()?;
        let max_x = cur.read_f32::<LittleEndian>().ok()?;
        let max_y = cur.read_f32::<LittleEndian>().ok()?;
        let max_z = cur.read_f32::<LittleEndian>().ok()?;
        let is_valid = cur.read_u8().ok()?;
        bsp.bounds.push(UModelBox {
            min: glam::Vec3::new(min_x, min_y, min_z),
            max: glam::Vec3::new(max_x, max_y, max_z),
            is_valid,
        });
    }

    let lh_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&lh_cnt) {
        return None;
    }
    for _ in 0..lh_cnt {
        bsp.leaf_hulls.push(cur.read_i32::<LittleEndian>().ok()?);
    }

    let leaves_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=200_000).contains(&leaves_cnt) {
        return None;
    }
    for _ in 0..leaves_cnt {
        let i_zone = read_compact_index(&mut cur).ok()?;
        let i_permeating = read_compact_index(&mut cur).ok()?;
        let i_volumetric = read_compact_index(&mut cur).ok()?;
        let visible_zones = cur.read_u64::<LittleEndian>().ok()?;
        bsp.leaves.push(UModelLeaf {
            i_zone,
            i_permeating,
            i_volumetric,
            visible_zones,
        });
    }

    let lights_cnt = read_compact_index(&mut cur).ok()?;
    if !(0..=50_000).contains(&lights_cnt) {
        return None;
    }
    for _ in 0..lights_cnt {
        bsp.lights.push(read_compact_index(&mut cur).ok()?);
    }

    bsp.root_outside = cur.read_i32::<LittleEndian>().ok()?;
    bsp.linked = cur.read_i32::<LittleEndian>().ok()?;

    Some((bsp, polys_ref))
}

impl UnrLevel {

    /// Distance from `p` straight down to the nearest solid BSP floor or semi-solid brush floor.
    pub fn floor_distance(&self, p: glam::Vec3, max: f32, step: f32) -> Option<f32> {
        let bsp_d = self.world_bsp.as_ref().and_then(|b| b.floor_distance(p, max, step));

        let mut closest_d = bsp_d;
        for brush in &self.brushes {
            for poly in &brush.world_polys {
                if poly.normal.z < 0.70 {
                    continue;
                }
                if poly.poly_flags & (0x08 | 0x0400_0000) != 0 {
                    continue;
                }
                let d_plane = poly.normal.dot(poly.base);
                let z_hit = (d_plane - poly.normal.x * p.x - poly.normal.y * p.y) / poly.normal.z;
                let d = p.z - z_hit;
                if d < -0.5 || d > max {
                    continue;
                }
                let pts = &poly.vertices;
                if pts.len() < 3 {
                    continue;
                }
                let mut min_x = f32::MAX;
                let mut max_x = f32::MIN;
                let mut min_y = f32::MAX;
                let mut max_y = f32::MIN;
                for v in pts {
                    min_x = min_x.min(v.x);
                    max_x = max_x.max(v.x);
                    min_y = min_y.min(v.y);
                    max_y = max_y.max(v.y);
                }
                // Walkable floors must have a minimum width and length of 24uu (excludes thin baseboards/trims)
                if (max_x - min_x) < 24.0 || (max_y - min_y) < 24.0 {
                    continue;
                }
                if p.x < min_x - 1.0 || p.x > max_x + 1.0 || p.y < min_y - 1.0 || p.y > max_y + 1.0 {
                    continue;
                }
                let mut inside = false;
                let mut j = pts.len() - 1;
                for i in 0..pts.len() {
                    let vi = pts[i];
                    let vj = pts[j];
                    if ((vi.y > p.y) != (vj.y > p.y))
                        && (p.x < (vj.x - vi.x) * (p.y - vi.y) / (vj.y - vi.y) + vi.x)
                    {
                        inside = !inside;
                    }
                    j = i;
                }
                if !inside && p.x >= min_x - 0.5 && p.x <= max_x + 0.5 && p.y >= min_y - 0.5 && p.y <= max_y + 0.5 {
                    inside = true;
                }
                if inside {
                    let d_clamped = d.max(0.0);
                    closest_d = Some(closest_d.map_or(d_clamped, |c| c.min(d_clamped)));
                }
            }
        }
        closest_d
    }

    pub fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let package = Package::open(&path)?;
        let map_name = path
            .as_ref()
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_string());

        Self::from_package(package, map_name)
    }

    /// Opens a `.unr` map in Safe Recovery Mode (`REQ-J06`):
    /// Ignores any potentially corrupted compiled `Model_World` BSP (`world_bsp`) or stale `ReachSpecs`,
    /// recovering strictly the source CSG `Brush` list, `Actor` records, and `MyLevel` embedded assets.
    pub fn open_safe_recovery<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let mut level = Self::open(path)?;
        level.world_bsp = None;
        level.reach_specs.clear();
        Ok(level)
    }

    /// Synchronizes the categorized actor views (`lights`, `player_starts`, `path_nodes`,
    /// `weapons_and_pickups`, `other_actors`) from the master `actors` list so that newly placed,
    /// edited, or deleted actors in `actors` are immediately reflected in viewport rendering and `.unr` saving.
    pub fn sync_actor_views(&mut self) {
        self.lights.clear();
        self.player_starts.clear();
        self.path_nodes.clear();
        self.weapons_and_pickups.clear();
        self.other_actors.clear();

        for actor in &self.actors {
            let cls_lower = actor.class_name.to_ascii_lowercase();
            if cls_lower.contains("brush") || cls_lower.contains("mover") {
                continue;
            } else if cls_lower.contains("light") {
                self.lights.push(actor.clone());
            } else if cls_lower.contains("playerstart") {
                self.player_starts.push(actor.clone());
            } else if cls_lower.contains("pathnode")
                || cls_lower.contains("lift")
                || cls_lower.contains("jump")
            {
                self.path_nodes.push(actor.clone());
            } else if is_weapon_or_pickup(&cls_lower) {
                self.weapons_and_pickups.push(actor.clone());
            } else {
                self.other_actors.push(actor.clone());
            }
        }
    }

    pub fn build_package(&self) -> io::Result<Package> {
        let mut pkg = Package::default();
        pkg.summary.file_version = 68;
        pkg.summary.pkg_flags = if self.package.summary.pkg_flags != 0 {
            self.package.summary.pkg_flags
        } else {
            1
        };
        pkg.summary.guid = if self.package.summary.guid != [0u8; 16] {
            self.package.summary.guid
        } else {
            [0x55, 0x4E, 0x52, 0x39, 0x39, 0, 0, 0, 1, 2, 3, 4, 5, 6, 7, 8]
        };

        // Ensure None is index 0
        let name_none = pkg.names.add("None", 0) as i32;
        let n_core = pkg.names.add("Core", 0) as i32;
        let n_engine = pkg.names.add("Engine", 0) as i32;
        let n_botpack = pkg.names.add("BotPack", 0) as i32;
        let n_unrealshare = pkg.names.add("UnrealShare", 0) as i32;
        let n_class = pkg.names.add("Class", 0) as i32;
        let n_package = pkg.names.add("Package", 0) as i32;
        let n_level = pkg.names.add("Level", 0) as i32;
        let n_mylevel = pkg.names.add("MyLevel", 0) as i32;
        let n_levelinfo = pkg.names.add("LevelInfo", 0) as i32;
        let n_levelsummary = pkg.names.add("LevelSummary", 0) as i32;
        let n_model = pkg.names.add("Model", 0) as i32;
        let n_polys = pkg.names.add("Polys", 0) as i32;
        let n_brush = pkg.names.add("Brush", 0) as i32;
        let n_mover = pkg.names.add("Mover", 0) as i32;
        let n_light = pkg.names.add("Light", 0) as i32;
        let n_player_start = pkg.names.add("PlayerStart", 0) as i32;
        let n_path_node = pkg.names.add("PathNode", 0) as i32;

        let imp_core = pkg.add_import(n_core, n_package, 0, n_core);
        let imp_engine = pkg.add_import(n_core, n_package, 0, n_engine);
        let imp_botpack = pkg.add_import(n_core, n_package, 0, n_botpack);
        let imp_unrealshare = pkg.add_import(n_core, n_package, 0, n_unrealshare);
        let _imp_class = pkg.add_import(n_core, n_class, imp_core, n_class);
        let imp_level = pkg.add_import(n_core, n_class, imp_engine, n_level);
        let imp_levelinfo = pkg.add_import(n_core, n_class, imp_engine, n_levelinfo);
        let imp_levelsummary = pkg.add_import(n_core, n_class, imp_engine, n_levelsummary);
        let imp_model = pkg.add_import(n_core, n_class, imp_engine, n_model);
        let imp_polys = pkg.add_import(n_core, n_class, imp_engine, n_polys);
        let _imp_brush = pkg.add_import(n_core, n_class, imp_engine, n_brush);
        let _imp_mover = pkg.add_import(n_core, n_class, imp_engine, n_mover);
        let _imp_light = pkg.add_import(n_core, n_class, imp_engine, n_light);
        let _imp_player_start = pkg.add_import(n_core, n_class, imp_engine, n_player_start);
        let _imp_path_node = pkg.add_import(n_core, n_class, imp_engine, n_path_node);

        // Pre-reserve MyLevel as export 1 (index 0 in exports array)
        let mylevel_exp_idx = pkg.add_export(n_mylevel, imp_level, 0, 0, 0x0007_0001, Vec::new());
        assert_eq!(mylevel_exp_idx, 1);

        // Export embedded MyLevel textures and sounds early so polygons and actors can reference them directly
        if !self.embedded_textures.is_empty() {
            crate::utx::UtxPackage::write_textures_into_package(
                &self.embedded_textures,
                &mut pkg,
                -1,
            )?;
        }
        if !self.embedded_sounds.is_empty() {
            crate::package::write_sounds_into_package(
                &self.embedded_sounds,
                &mut pkg,
                0,
            )?;
        }

        let mut actor_refs = Vec::new();
        let mut actor_name_to_ref: HashMap<String, i32> = HashMap::new();

        // Helper to write an actor's state frame + tagged properties (including raw metadata)
        let write_actor_payload = |cls_ref: i32,
                                   li_ref: i32,
                                   props: &[(TaggedProperty, Option<(u8, Option<String>)>)],
                                   pkg: &mut Package|
         -> io::Result<Vec<u8>> {
            let mut buf = Vec::new();
            // Canonical UE1 FStateFrame: Node = cls_ref, StateNode = cls_ref, ProbeMask = ~0u64, LatentAction = 0i32, Offset = -1
            write_compact_index(&mut buf, cls_ref)?;
            write_compact_index(&mut buf, cls_ref)?;
            buf.write_u64::<LittleEndian>(u64::MAX)?;
            buf.write_i32::<LittleEndian>(0)?;
            write_compact_index(&mut buf, -1)?;
            if li_ref > 0 {
                let level_prop = TaggedProperty {
                    name: "Level".to_string(),
                    array_index: 0,
                    value: TaggedValue::Object(li_ref),
                };
                write_tagged_property_with_meta(&mut buf, &level_prop, None, pkg)?;
                let mut reg_bytes = Vec::with_capacity(8);
                write_compact_index(&mut reg_bytes, li_ref)?;
                reg_bytes.write_i32::<LittleEndian>(0)?;
                reg_bytes.write_u8(1)?;
                let reg_prop = TaggedProperty {
                    name: "Region".to_string(),
                    array_index: 0,
                    value: TaggedValue::Raw(reg_bytes),
                };
                let reg_meta = (10u8, Some("PointRegion".to_string()));
                write_tagged_property_with_meta(&mut buf, &reg_prop, Some(&reg_meta), pkg)?;
            }
            for (prop, meta) in props {
                write_tagged_property_with_meta(&mut buf, prop, meta.as_ref(), pkg)?;
            }
            write_compact_index(&mut buf, 0)?; // Terminal None
            Ok(buf)
        };

        // Helper to append remaining non-transient properties from ActorRecord::properties
        let append_extra_props =
            |props: &mut Vec<(TaggedProperty, Option<(u8, Option<String>)>)>,
             actor: &ActorRecord,
             handled: &[&str],
             pkg: &mut Package,
             local_actors: &HashMap<String, i32>| {
                let is_level_info = actor.class_name.eq_ignore_ascii_case("LevelInfo");
                let mut sorted_keys: Vec<&String> = actor.properties.keys().collect();
                sorted_keys.sort();
                for key in sorted_keys {
                    let prop = &actor.properties[key];
                    let base_lower = prop.name.to_ascii_lowercase();
                    if handled
                        .iter()
                        .any(|h| h.eq_ignore_ascii_case(&prop.name) || h.eq_ignore_ascii_case(key))
                    {
                        continue;
                    }
                    // Skip internal engine transient/pointer properties that reference runtime structures
                    if matches!(
                        base_lower.as_str(),
                        "region"
                            | "level"
                            | "xlevel"
                            | "oldlocation"
                            | "collocation"
                            | "deleted"
                            | "brush"
                            | "nextinventory"
                            | "summary"
                            | "navigationpointlist"
                            | "nextnavigationpoint"
                    ) {
                        continue;
                    }
                    // These are object properties in UE1. Wizard and T3D actors
                    // may hold a symbolic name before export; resolve only known
                    // local actors and preserve unresolved values for inspection.
                    if matches!(base_lower.as_str(), "mymarker" | "markeditem") {
                        if let TaggedValue::Name(value) | TaggedValue::Str(value) = &prop.value {
                            let raw = actor.object_paths.get(key).map(String::as_str).unwrap_or(value).trim().trim_matches('"');
                            let path = raw.split_once('\'').map(|(_, rest)| rest.trim_end_matches('\'')).unwrap_or(raw);
                            let parts: Vec<_> = path.split('.').collect();
                            let name = match parts.as_slice() {
                                [name] => Some(*name),
                                [outer, name] if outer.eq_ignore_ascii_case("MyLevel") => Some(*name),
                                _ => None,
                            };
                            if let Some(reference) = name.and_then(|name| local_actors.get(&name.to_ascii_lowercase())).copied() {
                                props.push((TaggedProperty { name: prop.name.clone(), array_index: prop.array_index, value: TaggedValue::Object(reference) }, None));
                                continue;
                            }
                        }
                    }
                    if is_level_info && base_lower == "defaultgametype" {
                        let raw_gt = match &prop.value {
                            TaggedValue::Str(s) | TaggedValue::Name(s) => Some(s.clone()),
                            TaggedValue::Object(old_ref) => actor
                                .object_paths
                                .get(key)
                                .cloned()
                                .or_else(|| {
                                    if *old_ref != 0 {
                                        Some(self.package.resolve_full_object_path(*old_ref))
                                    } else {
                                        None
                                    }
                                }),
                            _ => None,
                        };
                        if let Some(gt_str) = raw_gt {
                            let inner = gt_str
                                .split('\'')
                                .nth(1)
                                .unwrap_or(&gt_str)
                                .trim_matches('"')
                                .trim();
                            if !inner.is_empty() && !inner.eq_ignore_ascii_case("None") {
                                let qual = if inner.contains('.') {
                                    format!("Class'{inner}'")
                                } else {
                                    format!("Class'BotPack.{inner}'")
                                };
                                let cls_ref = import_or_resolve_object_path(
                                    pkg,
                                    &qual,
                                    "Class",
                                    imp_core,
                                    imp_engine,
                                    n_package,
                                    n_class,
                                );
                                if cls_ref != 0 {
                                    props.push((
                                        TaggedProperty {
                                            name: "DefaultGameType".to_string(),
                                            array_index: 0,
                                            value: TaggedValue::Object(cls_ref),
                                        },
                                        None,
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                    if is_level_info && base_lower == "song" {
                        let raw_song = match &prop.value {
                            TaggedValue::Str(s) | TaggedValue::Name(s) => Some(s.clone()),
                            TaggedValue::Object(old_ref) => actor
                                .object_paths
                                .get(key)
                                .cloned()
                                .or_else(|| {
                                    if *old_ref != 0 {
                                        Some(self.package.resolve_full_object_path(*old_ref))
                                    } else {
                                        None
                                    }
                                }),
                            _ => None,
                        };
                        if let Some(song_str) = raw_song {
                            let inner = song_str
                                .split('\'')
                                .nth(1)
                                .unwrap_or(&song_str)
                                .trim_matches('"')
                                .trim();
                            let inner = inner.strip_suffix(".umx").unwrap_or(inner);
                            let inner = inner.strip_suffix(".UMX").unwrap_or(inner);
                            if !inner.is_empty() && !inner.eq_ignore_ascii_case("None") {
                                let norm = if inner.eq_ignore_ascii_case("Superfist") {
                                    "Suprfist"
                                } else {
                                    inner
                                };
                                let qual = if norm.contains('.') {
                                    format!("Music'{norm}'")
                                } else if norm.eq_ignore_ascii_case("Credits") {
                                    "Music'Credits.Trophy'".to_string()
                                } else {
                                    format!("Music'{norm}.{norm}'")
                                };
                                let music_ref = import_or_resolve_object_path(
                                    pkg,
                                    &qual,
                                    "Music",
                                    imp_core,
                                    imp_engine,
                                    n_package,
                                    n_class,
                                );
                                if music_ref != 0 {
                                    props.push((
                                        TaggedProperty {
                                            name: "Song".to_string(),
                                            array_index: 0,
                                            value: TaggedValue::Object(music_ref),
                                        },
                                        None,
                                    ));
                                }
                            }
                        }
                        continue;
                    }
                    if matches!(
                        base_lower.as_str(),
                        "ambientsound"
                            | "openingsound"
                            | "openedsound"
                            | "closingsound"
                            | "closedsound"
                            | "moveambientsound"
                    ) {
                        if let TaggedValue::Str(s) | TaggedValue::Name(s) = &prop.value {
                            let snd_ref = import_or_resolve_object_path(
                                pkg,
                                s,
                                "Sound",
                                imp_core,
                                imp_engine,
                                n_package,
                                n_class,
                            );
                            if snd_ref != 0 {
                                props.push((
                                    TaggedProperty {
                                        name: prop.name.clone(),
                                        array_index: prop.array_index,
                                        value: TaggedValue::Object(snd_ref),
                                    },
                                    None,
                                ));
                            }
                            continue;
                        }
                    }
                    match &prop.value {
                        TaggedValue::Object(old_ref) => {
                            let sym_path = actor
                                .object_paths
                                .get(key)
                                .cloned()
                                .or_else(|| {
                                    if *old_ref != 0 {
                                        let p = self.package.resolve_full_object_path(*old_ref);
                                        if p != "None" && !p.starts_with("Export(") && !p.starts_with("Import(") {
                                            Some(p)
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    }
                                });
                            if let Some(path_str) = sym_path {
                                let new_ref = import_or_resolve_object_path(
                                    pkg,
                                    &path_str,
                                    "Object",
                                    imp_core,
                                    imp_engine,
                                    n_package,
                                    n_class,
                                );
                                if new_ref != 0 {
                                    props.push((
                                        TaggedProperty {
                                            name: prop.name.clone(),
                                            array_index: prop.array_index,
                                            value: TaggedValue::Object(new_ref),
                                        },
                                        None,
                                    ));
                                }
                            }
                        }
                        TaggedValue::Raw(_) => {
                            let meta = actor.raw_property_meta.get(key).cloned();
                            props.push((prop.clone(), meta));
                        }
                        _ => {
                            props.push((prop.clone(), None));
                        }
                    }
                }
            };

        // Collect effective categorized actor lists, including any actors added directly to `self.actors`
        let mut known_names: std::collections::HashSet<String> = std::collections::HashSet::new();
        for b in &self.brushes {
            if !b.actor.name.is_empty() {
                known_names.insert(b.actor.name.to_ascii_lowercase());
            }
        }
        let mut lights_list: Vec<&ActorRecord> = self.lights.iter().collect();
        let mut pstarts_list: Vec<&ActorRecord> = self.player_starts.iter().collect();
        let mut pnodes_list: Vec<&ActorRecord> = self.path_nodes.iter().collect();
        let mut pickups_list: Vec<&ActorRecord> = self.weapons_and_pickups.iter().collect();
        let mut others_list: Vec<&ActorRecord> = self.other_actors.iter().collect();
        for a in lights_list
            .iter()
            .chain(pstarts_list.iter())
            .chain(pnodes_list.iter())
            .chain(pickups_list.iter())
            .chain(others_list.iter())
        {
            if !a.name.is_empty() {
                known_names.insert(a.name.to_ascii_lowercase());
            }
        }
        for a in &self.actors {
            let cls_lower = a.class_name.to_ascii_lowercase();
            if cls_lower.contains("brush") || cls_lower.contains("mover") {
                continue;
            }
            if !a.name.is_empty() && !known_names.insert(a.name.to_ascii_lowercase()) {
                continue;
            }
            if cls_lower.contains("light") {
                lights_list.push(a);
            } else if cls_lower.contains("playerstart") {
                pstarts_list.push(a);
            } else if cls_lower.contains("pathnode")
                || cls_lower.contains("lift")
                || cls_lower.contains("jump")
            {
                pnodes_list.push(a);
            } else if is_weapon_or_pickup(&cls_lower) {
                pickups_list.push(a);
            } else {
                others_list.push(a);
            }
        }

        // 0. Guarantee Actors[0] is ALWAYS LevelInfo0 (UE1 ULevel invariant)
        let existing_level_info = others_list
            .iter()
            .copied()
            .find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo"))
            .or_else(|| {
                self.actors
                    .iter()
                    .find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo"))
            });

        let default_li = ActorRecord {
            name: "LevelInfo0".to_string(),
            class_name: "LevelInfo".to_string(),
            flags: 0x0207_0001,
            ..Default::default()
        };
        let li_actor = existing_level_info.unwrap_or(&default_li);
        let li_name = if li_actor.name.is_empty() {
            "LevelInfo0".to_string()
        } else {
            li_actor.name.clone()
        };
        let n_li = pkg.names.add(&li_name, 0) as i32;
        let li_ref = pkg.add_export(n_li, imp_levelinfo, 0, 0, 0x0207_0001, Vec::new());
        actor_refs.push(li_ref);
        actor_name_to_ref.insert(li_name.to_ascii_lowercase(), li_ref);

        // Emit LevelSummary export for UT99 map browser metadata
        let mut summary_props = Vec::new();
        for sum_key in ["Title", "Author", "IdealPlayerCount", "LevelEnterText"] {
            if let Some(p) = li_actor
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(sum_key))
                .map(|(_, v)| v.clone())
            {
                summary_props.push(p);
            }
        }
        let mut summary_buf = Vec::new();
        write_tagged_properties(&mut summary_buf, &summary_props, &mut pkg)?;
        let level_summary_ref = pkg.add_export(
            n_levelsummary,
            imp_levelsummary,
            0,
            0,
            0x0007_0004,
            summary_buf,
        );

        // Pass 1: Pre-register all Brushes/Movers (with their Polys & Model exports) and all other Actors
        // so that intra-level MyLevel object references (forward or backward) resolve to valid export indices.
        let mut all_world_polys = Vec::new();
        let mut brush_export_entries: Vec<(i32, i32, i32)> = Vec::with_capacity(self.brushes.len());

        for (i, b) in self.brushes.iter().enumerate() {
            let is_mover = b.actor.class_name.to_ascii_lowercase().contains("mover");
            let mut resolved_polys = b.local_polys.clone();
            for (j, poly) in resolved_polys.polys.iter_mut().enumerate() {
                let explicit_tex = self
                    .brush_poly_textures
                    .get(i)
                    .and_then(|v| v.get(j))
                    .cloned()
                    .unwrap_or_default();
                let tex_path = if !explicit_tex.is_empty() {
                    explicit_tex
                } else if poly.texture_ref != 0 {
                    self.package.resolve_full_object_path(poly.texture_ref)
                } else {
                    String::new()
                };
                if !tex_path.is_empty() && tex_path != "None" {
                    poly.texture_ref = import_or_resolve_object_path(
                        &mut pkg,
                        &tex_path,
                        "Texture",
                        imp_core,
                        imp_engine,
                        n_package,
                        n_class,
                    );
                }
            }

            // 1a. Polys export
            let polys_name = pkg.names.add(format!("Polys_{i}"), 0) as i32;
            let polys_bytes = resolved_polys.to_bytes()?;
            let polys_ref = pkg.add_export(polys_name, imp_polys, 0, 0, 0x0007_0001, polys_bytes);

            // 1b. Model export (Movers get compiled BSP nodes/leaves for collision & raytrace; static CSG brushes get UModel + Polys)
            let model_name = pkg.names.add(format!("Model_{i}"), 0) as i32;
            let root_outside = if b.actor.csg_oper == 2 { 0 } else { 1 };
            let model_bytes = if is_mover {
                let mover_bsp = compile_bsp_from_world_polys(&resolved_polys.polys);
                build_world_umodel_bytes(name_none, &resolved_polys.polys, polys_ref, &mover_bsp)?
            } else {
                build_umodel_bytes(name_none, &resolved_polys.polys, polys_ref, root_outside)?
            };
            let model_ref = pkg.add_export(model_name, imp_model, 0, 0, 0x0007_0001, model_bytes);

            // 1c. Brush / Mover actor pre-registration
            let default_brush_cls = if is_mover { "Mover" } else { "Brush" };
            let cls_ref = resolve_actor_class_import(
                &mut pkg,
                &b.actor,
                default_brush_cls,
                imp_core,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
                n_package,
                n_class,
            );
            let brush_actor_name = if b.actor.name.is_empty() {
                if is_mover {
                    format!("Mover_{i}")
                } else {
                    format!("Brush_{i}")
                }
            } else {
                b.actor.name.clone()
            };
            let n_brush_actor = pkg.names.add(&brush_actor_name, 0) as i32;
            let brush_ref = pkg.add_export(n_brush_actor, cls_ref, 0, 0, 0x0207_0001, Vec::new());
            actor_refs.push(brush_ref);
            actor_name_to_ref.insert(brush_actor_name.to_ascii_lowercase(), brush_ref);
            brush_export_entries.push((brush_ref, model_ref, cls_ref));

            if !is_mover {
                for (j, mut wp) in b.world_polys.iter().cloned().enumerate() {
                    if let Some(rp) = resolved_polys.polys.get(j) {
                        wp.texture_ref = rp.texture_ref;
                    }
                    wp.actor_ref = brush_ref;
                    if b.actor.csg_oper == 2 {
                        wp.normal = -wp.normal;
                        wp.vertices.reverse();
                    }
                    all_world_polys.push(wp);
                }
            }
        }

        let mut light_export_refs = Vec::with_capacity(lights_list.len());
        for (i, light) in lights_list.iter().enumerate() {
            let cls_ref = resolve_actor_class_import(
                &mut pkg,
                light,
                "Light",
                imp_core,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
                n_package,
                n_class,
            );
            let light_name = if light.name.is_empty() {
                format!("Light_{i}")
            } else {
                light.name.clone()
            };
            let n_light_act = pkg.names.add(&light_name, 0) as i32;
            let light_ref = pkg.add_export(n_light_act, cls_ref, 0, 0, 0x0207_0001, Vec::new());
            actor_refs.push(light_ref);
            actor_name_to_ref.insert(light_name.to_ascii_lowercase(), light_ref);
            light_export_refs.push((light_ref, cls_ref));
        }

        let mut pstart_export_refs = Vec::with_capacity(pstarts_list.len());
        for (i, pstart) in pstarts_list.iter().enumerate() {
            let cls_ref = resolve_actor_class_import(
                &mut pkg,
                pstart,
                "PlayerStart",
                imp_core,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
                n_package,
                n_class,
            );
            let pstart_name = if pstart.name.is_empty() {
                format!("PlayerStart_{i}")
            } else {
                pstart.name.clone()
            };
            let n_pstart_act = pkg.names.add(&pstart_name, 0) as i32;
            let pstart_ref = pkg.add_export(n_pstart_act, cls_ref, 0, 0, 0x0207_0001, Vec::new());
            actor_refs.push(pstart_ref);
            actor_name_to_ref.insert(pstart_name.to_ascii_lowercase(), pstart_ref);
            pstart_export_refs.push((pstart_ref, cls_ref));
        }

        let mut pnode_export_refs = Vec::with_capacity(pnodes_list.len());
        for (i, pnode) in pnodes_list.iter().enumerate() {
            let cls_name = if pnode.class_name.is_empty() {
                "PathNode"
            } else {
                &pnode.class_name
            };
            let cls_ref = resolve_actor_class_import(
                &mut pkg,
                pnode,
                "PathNode",
                imp_core,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
                n_package,
                n_class,
            );
            let pnode_name = if pnode.name.is_empty() {
                format!("{cls_name}_{i}")
            } else {
                pnode.name.clone()
            };
            let n_pnode_act = pkg.names.add(&pnode_name, 0) as i32;
            let pnode_ref = pkg.add_export(n_pnode_act, cls_ref, 0, 0, 0x0207_0001, Vec::new());
            actor_refs.push(pnode_ref);
            actor_name_to_ref.insert(pnode_name.to_ascii_lowercase(), pnode_ref);
            pnode_export_refs.push((pnode_ref, cls_ref, pnode_name));
        }

        let mut misc_export_refs = Vec::new();
        for (i, actor) in pickups_list.iter().chain(others_list.iter()).enumerate() {
            if actor.class_name.eq_ignore_ascii_case("LevelInfo") {
                continue;
            }
            let cls_name = if actor.class_name.is_empty() {
                "Actor"
            } else {
                &actor.class_name
            };
            let short_cls = cls_name.rsplit('.').next().unwrap_or(cls_name);
            let imp_custom = resolve_actor_class_import(
                &mut pkg,
                actor,
                "Actor",
                imp_core,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
                n_package,
                n_class,
            );
            let act_name = if actor.name.is_empty() {
                format!("{short_cls}_{i}")
            } else {
                actor.name.clone()
            };
            let n_act = pkg.names.add(&act_name, 0) as i32;
            let act_ref = pkg.add_export(n_act, imp_custom, 0, 0, 0x0207_0001, Vec::new());
            actor_refs.push(act_ref);
            actor_name_to_ref.insert(act_name.to_ascii_lowercase(), act_ref);
            misc_export_refs.push((act_ref, imp_custom, *actor));
        }

        // Build NavigationPointList chain across all PlayerStarts, PathNodes, and NavigationPoint subclasses
        let is_extra_nav_point = |cls: &str| -> bool {
            let short = cls.rsplit('.').next().unwrap_or(cls).to_ascii_lowercase();
            short.contains("teleporter")
                || matches!(
                    short.as_str(),
                    "flagbase"
                        | "controlpoint"
                        | "domcontrolpoint"
                        | "defensepoint"
                        | "alternatepath"
                        | "fortstandard"
                        | "ambushpoint"
                        | "patrolpoint"
                        | "homebase"
                        | "spawnpoint"
                        | "translocdest"
                        | "navigationpoint"
                )
        };
        let nav_chain: Vec<i32> = pstart_export_refs
            .iter()
            .map(|(r, _)| *r)
            .chain(pnode_export_refs.iter().map(|(r, _, _)| *r))
            .chain(
                misc_export_refs
                    .iter()
                    .filter(|(_, _, a)| is_extra_nav_point(&a.class_name))
                    .map(|(r, _, _)| *r),
            )
            .collect();
        let mut next_nav_map: HashMap<i32, i32> = HashMap::new();
        for win in nav_chain.windows(2) {
            next_nav_map.insert(win[0], win[1]);
        }

        // Pass 2: Serialize actor payloads now that all MyLevel exports exist in `pkg.exports`
        // 2.0 LevelInfo0 payload
        let mut li_props = Vec::new();
        li_props.push((
            TaggedProperty {
                name: "Summary".to_string(),
                array_index: 0,
                value: TaggedValue::Object(level_summary_ref),
            },
            None,
        ));
        if let Some(&first_nav) = nav_chain.first() {
            li_props.push((
                TaggedProperty {
                    name: "NavigationPointList".to_string(),
                    array_index: 0,
                    value: TaggedValue::Object(first_nav),
                },
                None,
            ));
        }
        if li_actor.location != glam::Vec3::ZERO {
            li_props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(li_actor.location),
                },
                None,
            ));
        }
        if !li_actor
            .properties
            .keys()
            .any(|k| k.eq_ignore_ascii_case("Screenshot"))
            && self
                .embedded_textures
                .iter()
                .any(|t| t.name.eq_ignore_ascii_case("Screenshot"))
        {
            let screenshot_ref = import_or_resolve_object_path(
                &mut pkg,
                "MyLevel.Screenshot",
                "Texture",
                imp_core,
                imp_engine,
                n_package,
                n_class,
            );
            if screenshot_ref != 0 {
                li_props.push((
                    TaggedProperty {
                        name: "Screenshot".to_string(),
                        array_index: 0,
                        value: TaggedValue::Object(screenshot_ref),
                    },
                    None,
                ));
            }
        }
        append_extra_props(
            &mut li_props,
            li_actor,
            &["Location", "Rotation", "Tag", "Event", "Group"],
            &mut pkg,
            &actor_name_to_ref,
        );
        let li_payload = write_actor_payload(imp_levelinfo, li_ref, &li_props, &mut pkg)?;
        pkg.set_export_payload(li_ref, li_payload);

        // 2.1 Brushes & Movers payloads
        for (b, &(brush_ref, model_ref, cls_ref)) in self.brushes.iter().zip(brush_export_entries.iter()) {
            let mut props = Vec::new();
            props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(b.actor.location),
                },
                None,
            ));
            if b.actor.rotation != (0, 0, 0) {
                props.push((
                    TaggedProperty {
                        name: "Rotation".to_string(),
                        array_index: 0,
                        value: TaggedValue::Rotator {
                            pitch: b.actor.rotation.0,
                            yaw: b.actor.rotation.1,
                            roll: b.actor.rotation.2,
                        },
                    },
                    None,
                ));
            }
            if b.actor.prepivot != glam::Vec3::ZERO {
                props.push((
                    TaggedProperty {
                        name: "PrePivot".to_string(),
                        array_index: 0,
                        value: TaggedValue::Vector(b.actor.prepivot),
                    },
                    None,
                ));
            }
            let (main_sheer_rate, main_sheer_axis) = b
                .actor
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("MainScale"))
                .and_then(|(_, p)| match p.value {
                    TaggedValue::Scale {
                        sheer_rate,
                        sheer_axis,
                        ..
                    } => Some((sheer_rate, sheer_axis)),
                    _ => None,
                })
                .unwrap_or((0.0, 0));
            if b.actor.main_scale != glam::Vec3::ONE
                || main_sheer_rate != 0.0
                || main_sheer_axis != 0
            {
                props.push((
                    TaggedProperty {
                        name: "MainScale".to_string(),
                        array_index: 0,
                        value: TaggedValue::Scale {
                            scale: b.actor.main_scale,
                            sheer_rate: main_sheer_rate,
                            sheer_axis: main_sheer_axis,
                        },
                    },
                    None,
                ));
            }
            let (post_sheer_rate, post_sheer_axis) = b
                .actor
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("PostScale"))
                .and_then(|(_, p)| match p.value {
                    TaggedValue::Scale {
                        sheer_rate,
                        sheer_axis,
                        ..
                    } => Some((sheer_rate, sheer_axis)),
                    _ => None,
                })
                .unwrap_or((0.0, 0));
            if b.actor.post_scale != glam::Vec3::ONE
                || post_sheer_rate != 0.0
                || post_sheer_axis != 0
            {
                props.push((
                    TaggedProperty {
                        name: "PostScale".to_string(),
                        array_index: 0,
                        value: TaggedValue::Scale {
                            scale: b.actor.post_scale,
                            sheer_rate: post_sheer_rate,
                            sheer_axis: post_sheer_axis,
                        },
                    },
                    None,
                ));
            }
            props.push((
                TaggedProperty {
                    name: "Brush".to_string(),
                    array_index: 0,
                    value: TaggedValue::Object(model_ref),
                },
                None,
            ));
            props.push((
                TaggedProperty {
                    name: "CsgOper".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(b.actor.csg_oper),
                },
                None,
            ));
            if b.actor.poly_flags != 0 {
                props.push((
                    TaggedProperty {
                        name: "PolyFlags".to_string(),
                        array_index: 0,
                        value: TaggedValue::Int(b.actor.poly_flags as i32),
                    },
                    None,
                ));
            }
            if !b.actor.tag.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Tag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(b.actor.tag.clone()),
                    },
                    None,
                ));
            }
            if !b.actor.event.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Event".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(b.actor.event.clone()),
                    },
                    None,
                ));
            }
            if !b.actor.group.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Group".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(b.actor.group.clone()),
                    },
                    None,
                ));
            }

            append_extra_props(
                &mut props,
                &b.actor,
                &[
                    "Location",
                    "Rotation",
                    "PrePivot",
                    "MainScale",
                    "PostScale",
                    "Brush",
                    "CsgOper",
                    "PolyFlags",
                    "Tag",
                    "Event",
                    "Group",
                ],
                &mut pkg,
                &actor_name_to_ref,
            );

            let payload = write_actor_payload(cls_ref, li_ref, &props, &mut pkg)?;
            pkg.set_export_payload(brush_ref, payload);
        }

        // 2.2 Lights payloads
        for (light, &(light_ref, cls_ref)) in lights_list.iter().zip(light_export_refs.iter()) {
            let mut props = Vec::new();
            props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(light.location),
                },
                None,
            ));
            if light.rotation != (0, 0, 0) {
                props.push((
                    TaggedProperty {
                        name: "Rotation".to_string(),
                        array_index: 0,
                        value: TaggedValue::Rotator {
                            pitch: light.rotation.0,
                            yaw: light.rotation.1,
                            roll: light.rotation.2,
                        },
                    },
                    None,
                ));
            }
            props.push((
                TaggedProperty {
                    name: "LightBrightness".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(light.light_brightness),
                },
                None,
            ));
            props.push((
                TaggedProperty {
                    name: "LightRadius".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(light.light_radius),
                },
                None,
            ));
            props.push((
                TaggedProperty {
                    name: "LightHue".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(light.light_hue),
                },
                None,
            ));
            props.push((
                TaggedProperty {
                    name: "LightSaturation".to_string(),
                    array_index: 0,
                    value: TaggedValue::Byte(light.light_saturation),
                },
                None,
            ));
            if light.light_type != 0 {
                props.push((
                    TaggedProperty {
                        name: "LightType".to_string(),
                        array_index: 0,
                        value: TaggedValue::Byte(light.light_type),
                    },
                    None,
                ));
            }
            if light.light_effect != 0 {
                props.push((
                    TaggedProperty {
                        name: "LightEffect".to_string(),
                        array_index: 0,
                        value: TaggedValue::Byte(light.light_effect),
                    },
                    None,
                ));
            }
            if !light.tag.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Tag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(light.tag.clone()),
                    },
                    None,
                ));
            }
            if !light.event.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Event".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(light.event.clone()),
                    },
                    None,
                ));
            }
            if !light.group.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Group".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(light.group.clone()),
                    },
                    None,
                ));
            }

            append_extra_props(
                &mut props,
                light,
                &[
                    "Location",
                    "Rotation",
                    "LightBrightness",
                    "LightRadius",
                    "LightHue",
                    "LightSaturation",
                    "LightType",
                    "LightEffect",
                    "Tag",
                    "Event",
                    "Group",
                ],
                &mut pkg,
                &actor_name_to_ref,
            );

            let payload = write_actor_payload(cls_ref, li_ref, &props, &mut pkg)?;
            pkg.set_export_payload(light_ref, payload);
        }

        // 2.3 PlayerStarts payloads
        for (pstart, &(pstart_ref, cls_ref)) in pstarts_list.iter().zip(pstart_export_refs.iter()) {
            let mut props = Vec::new();
            props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(pstart.location),
                },
                None,
            ));
            if pstart.rotation != (0, 0, 0) {
                props.push((
                    TaggedProperty {
                        name: "Rotation".to_string(),
                        array_index: 0,
                        value: TaggedValue::Rotator {
                            pitch: pstart.rotation.0,
                            yaw: pstart.rotation.1,
                            roll: pstart.rotation.2,
                        },
                    },
                    None,
                ));
            }
            if let Some(&next_nav) = next_nav_map.get(&pstart_ref) {
                props.push((
                    TaggedProperty {
                        name: "nextNavigationPoint".to_string(),
                        array_index: 0,
                        value: TaggedValue::Object(next_nav),
                    },
                    None,
                ));
            }
            if !pstart.tag.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Tag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pstart.tag.clone()),
                    },
                    None,
                ));
            }
            if !pstart.event.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Event".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pstart.event.clone()),
                    },
                    None,
                ));
            }
            if !pstart.group.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Group".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pstart.group.clone()),
                    },
                    None,
                ));
            }

            append_extra_props(
                &mut props,
                pstart,
                &["Location", "Rotation", "Tag", "Event", "Group"],
                &mut pkg,
                &actor_name_to_ref,
            );

            let payload = write_actor_payload(cls_ref, li_ref, &props, &mut pkg)?;
            pkg.set_export_payload(pstart_ref, payload);
        }

        // 2.4 PathNodes payloads
        for (pnode, (pnode_ref, cls_ref, pnode_name)) in pnodes_list.iter().zip(pnode_export_refs.iter()) {
            let mut props = Vec::new();
            props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(pnode.location),
                },
                None,
            ));
            if pnode.rotation != (0, 0, 0) {
                props.push((
                    TaggedProperty {
                        name: "Rotation".to_string(),
                        array_index: 0,
                        value: TaggedValue::Rotator {
                            pitch: pnode.rotation.0,
                            yaw: pnode.rotation.1,
                            roll: pnode.rotation.2,
                        },
                    },
                    None,
                ));
            }
            if let Some(&next_nav) = next_nav_map.get(pnode_ref) {
                props.push((
                    TaggedProperty {
                        name: "nextNavigationPoint".to_string(),
                        array_index: 0,
                        value: TaggedValue::Object(next_nav),
                    },
                    None,
                ));
            }
            if !pnode.tag.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Tag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pnode.tag.clone()),
                    },
                    None,
                ));
            }
            if !pnode.event.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Event".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pnode.event.clone()),
                    },
                    None,
                ));
            }
            if !pnode.group.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Group".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(pnode.group.clone()),
                    },
                    None,
                ));
            }

            let mut out_slot = 0i32;
            let mut in_slot = 0i32;
            for (rs_idx, rs) in self.reach_specs.iter().enumerate() {
                if rs.start_actor_name.eq_ignore_ascii_case(pnode_name) && out_slot < 16 {
                    props.push((
                        TaggedProperty {
                            name: "Paths".to_string(),
                            array_index: out_slot,
                            value: TaggedValue::Int(rs_idx as i32),
                        },
                        None,
                    ));
                    out_slot += 1;
                }
                if rs.end_actor_name.eq_ignore_ascii_case(pnode_name) && in_slot < 16 {
                    props.push((
                        TaggedProperty {
                            name: "upstreamPaths".to_string(),
                            array_index: in_slot,
                            value: TaggedValue::Int(rs_idx as i32),
                        },
                        None,
                    ));
                    in_slot += 1;
                }
            }

            append_extra_props(
                &mut props,
                pnode,
                &[
                    "Location",
                    "Rotation",
                    "Tag",
                    "Event",
                    "Group",
                    "Paths",
                    "upstreamPaths",
                ],
                &mut pkg,
                &actor_name_to_ref,
            );

            let payload = write_actor_payload(*cls_ref, li_ref, &props, &mut pkg)?;
            pkg.set_export_payload(*pnode_ref, payload);
        }

        // 2.5 Weapons, Pickups & Other Actors payloads
        for (act_ref, cls_ref, actor) in misc_export_refs {
            let mut props = Vec::new();
            props.push((
                TaggedProperty {
                    name: "Location".to_string(),
                    array_index: 0,
                    value: TaggedValue::Vector(actor.location),
                },
                None,
            ));
            if actor.rotation != (0, 0, 0) {
                props.push((
                    TaggedProperty {
                        name: "Rotation".to_string(),
                        array_index: 0,
                        value: TaggedValue::Rotator {
                            pitch: actor.rotation.0,
                            yaw: actor.rotation.1,
                            roll: actor.rotation.2,
                        },
                    },
                    None,
                ));
            }
            if actor.prepivot != glam::Vec3::ZERO {
                props.push((
                    TaggedProperty {
                        name: "PrePivot".to_string(),
                        array_index: 0,
                        value: TaggedValue::Vector(actor.prepivot),
                    },
                    None,
                ));
            }
            if !actor.tag.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Tag".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(actor.tag.clone()),
                    },
                    None,
                ));
            }
            if !actor.event.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Event".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(actor.event.clone()),
                    },
                    None,
                ));
            }
            if !actor.group.is_empty() {
                props.push((
                    TaggedProperty {
                        name: "Group".to_string(),
                        array_index: 0,
                        value: TaggedValue::Name(actor.group.clone()),
                    },
                    None,
                ));
            }
            if let Some(&next_nav) = next_nav_map.get(&act_ref) {
                props.push((
                    TaggedProperty {
                        name: "nextNavigationPoint".to_string(),
                        array_index: 0,
                        value: TaggedValue::Object(next_nav),
                    },
                    None,
                ));
            }
            if is_extra_nav_point(&actor.class_name) && !actor.name.is_empty() {
                let mut out_slot = 0i32;
                let mut in_slot = 0i32;
                for (rs_idx, rs) in self.reach_specs.iter().enumerate() {
                    if rs.start_actor_name.eq_ignore_ascii_case(&actor.name) && out_slot < 16 {
                        props.push((
                            TaggedProperty {
                                name: "Paths".to_string(),
                                array_index: out_slot,
                                value: TaggedValue::Int(rs_idx as i32),
                            },
                            None,
                        ));
                        out_slot += 1;
                    }
                    if rs.end_actor_name.eq_ignore_ascii_case(&actor.name) && in_slot < 16 {
                        props.push((
                            TaggedProperty {
                                name: "upstreamPaths".to_string(),
                                array_index: in_slot,
                                value: TaggedValue::Int(rs_idx as i32),
                            },
                            None,
                        ));
                        in_slot += 1;
                    }
                }
            }

            append_extra_props(
                &mut props,
                actor,
                &[
                    "Location",
                    "Rotation",
                    "PrePivot",
                    "Tag",
                    "Event",
                    "Group",
                    "Paths",
                    "upstreamPaths",
                ],
                &mut pkg,
                &actor_name_to_ref,
            );

            let payload = write_actor_payload(cls_ref, li_ref, &props, &mut pkg)?;
            pkg.set_export_payload(act_ref, payload);
        }

        // Export World Model (Model_World) referenced by ULevel::Model
        let world_polys_name = pkg.names.add("Polys_World", 0) as i32;
        let world_polys_raw = RawPolys {
            polys: all_world_polys.clone(),
        };
        let world_polys_bytes = world_polys_raw.to_bytes()?;
        let world_polys_ref = pkg.add_export(world_polys_name, imp_polys, 0, 0, 0x0007_0001, world_polys_bytes);

        let world_model_name = pkg.names.add("Model_World", 0) as i32;
        let mut compiled_bsp = self
            .world_bsp
            .clone()
            .unwrap_or_else(|| compile_bsp_from_world_polys(&all_world_polys));
        for z in &mut compiled_bsp.zones {
            if z.zone_actor_ref == 0 {
                z.zone_actor_ref = li_ref;
            }
        }
        let world_model_bytes =
            build_world_umodel_bytes(name_none, &all_world_polys, world_polys_ref, &compiled_bsp)?;
        let world_model_ref =
            pkg.add_export(world_model_name, imp_model, 0, 0, 0x0007_0001, world_model_bytes);

        // Populate MyLevel (ULevel) payload
        let mut level_buf = Vec::new();
        level_buf.push(0u8); // UObject None property tag (name_none = 0)
        let count = actor_refs.len() as i32;
        level_buf.write_i32::<LittleEndian>(count)?;
        level_buf.write_i32::<LittleEndian>(count)?;
        for &act_ref in &actor_refs {
            write_compact_index(&mut level_buf, act_ref)?;
        }

        // FURL: Protocol ("unreal"), Host (""), Map ("Index.unr"), Portal (""), Op (0), Port (7777), Valid (1)
        write_compact_index(&mut level_buf, 7)?;
        level_buf.extend_from_slice(b"unreal\0");
        write_compact_index(&mut level_buf, 0)?;
        write_compact_index(&mut level_buf, 10)?;
        level_buf.extend_from_slice(b"Index.unr\0");
        write_compact_index(&mut level_buf, 0)?;
        write_compact_index(&mut level_buf, 0)?; // Op array count = 0
        level_buf.write_i32::<LittleEndian>(7777)?;
        level_buf.write_i32::<LittleEndian>(1)?;

        // ULevel::Model reference
        write_compact_index(&mut level_buf, world_model_ref)?;

        // ULevel::ReachSpecs array
        write_compact_index(&mut level_buf, self.reach_specs.len() as i32)?;
        for rs in &self.reach_specs {
            let start_ref = actor_name_to_ref
                .get(&rs.start_actor_name.to_ascii_lowercase())
                .copied()
                .unwrap_or(0);
            let end_ref = actor_name_to_ref
                .get(&rs.end_actor_name.to_ascii_lowercase())
                .copied()
                .unwrap_or(0);
            level_buf.write_i32::<LittleEndian>(rs.distance)?;
            write_compact_index(&mut level_buf, start_ref)?;
            write_compact_index(&mut level_buf, end_ref)?;
            level_buf.write_i32::<LittleEndian>(rs.collision_radius.max(34))?;
            level_buf.write_i32::<LittleEndian>(rs.collision_height.max(78))?;
            level_buf.write_i32::<LittleEndian>(rs.reach_flags.max(1))?;
            level_buf.write_u8(rs.b_pruned)?;
        }

        // ApproxTime (f32), FirstDeleted (CompactIndex), TextBlocks[16], TravelInfo
        level_buf.write_f32::<LittleEndian>(0.0)?;
        write_compact_index(&mut level_buf, 0)?;
        for _ in 0..16 {
            write_compact_index(&mut level_buf, 0)?;
        }
        write_compact_index(&mut level_buf, 0)?;

        let mylevel_offset = pkg.raw_data.len() as i32;
        let mylevel_size = level_buf.len() as i32;
        pkg.raw_data.extend_from_slice(&level_buf);
        pkg.exports.entries[0].serial_offset = mylevel_offset;
        pkg.exports.entries[0].serial_size = mylevel_size;

        Ok(pkg)
    }

    /// Validates a built or loaded `.unr` `Package` against Unreal Engine 1 v68 (`UT99`) structural invariants.
    pub fn validate_package(pkg: &Package) -> UnrValidationReport {
        let mut report = UnrValidationReport {
            is_valid: true,
            ..Default::default()
        };

        if pkg.summary.tag != PACKAGE_FILE_TAG {
            report.is_valid = false;
            report.errors.push(format!(
                "Invalid package tag: 0x{:08X} (expected 0x{:08X})",
                pkg.summary.tag, PACKAGE_FILE_TAG
            ));
        }
        if pkg.summary.file_version < 61 || pkg.summary.file_version > 69 {
            report.warnings.push(format!(
                "Non-standard UE1 file_version: {} (expected 68 for UT99)",
                pkg.summary.file_version
            ));
        }
        if !pkg
            .names
            .entries
            .iter()
            .any(|n| n.name.eq_ignore_ascii_case("None"))
        {
            report.is_valid = false;
            report
                .errors
                .push("NameTable is missing mandatory 'None' entry".to_string());
        }
        let name_count = pkg.names.entries.len() as i32;
        for (i, n) in pkg.names.entries.iter().enumerate() {
            if n.flags == 0 {
                report.is_valid = false;
                report.errors.push(format!(
                    "NameTable[{i}] ('{}') has flags == 0 (missing RF_LoadForClient/Server/Edit)",
                    n.name
                ));
            }
        }
        for (i, imp) in pkg.imports.entries.iter().enumerate() {
            if imp.class_package < 0 || imp.class_package >= name_count {
                report.is_valid = false;
                report.errors.push(format!(
                    "ImportTable[{i}] has out-of-range class_package FName index {}/{}",
                    imp.class_package, name_count
                ));
            }
            if imp.class_name < 0 || imp.class_name >= name_count {
                report.is_valid = false;
                report.errors.push(format!(
                    "ImportTable[{i}] has out-of-range class_name FName index {}/{}",
                    imp.class_name, name_count
                ));
            }
            if imp.object_name < 0 || imp.object_name >= name_count {
                report.is_valid = false;
                report.errors.push(format!(
                    "ImportTable[{i}] has out-of-range object_name FName index {}/{}",
                    imp.object_name, name_count
                ));
            }
        }
        for (i, exp) in pkg.exports.entries.iter().enumerate() {
            if exp.object_name < 0 || exp.object_name >= name_count {
                report.is_valid = false;
                report.errors.push(format!(
                    "ExportTable[{i}] has out-of-range object_name FName index {}/{}",
                    exp.object_name, name_count
                ));
            }
        }

        let my_level_idx = pkg.exports.entries.iter().position(|e| {
            pkg.get_name(e.object_name.max(0) as usize)
                .map(|n| n.eq_ignore_ascii_case("MyLevel"))
                .unwrap_or(false)
        });
        let Some(level_idx) = my_level_idx else {
            report.is_valid = false;
            report
                .errors
                .push("ExportTable is missing 'MyLevel' (ULevel) export".to_string());
            return report;
        };

        if let Some(level_bytes) = pkg.get_export_bytes(level_idx) {
            if let Ok((actor_refs, rs_count)) = parse_level_actors_and_reachspecs(level_bytes) {
                report.actor_count = actor_refs.len();
                report.reach_spec_count = rs_count;
                if let Some(&first_ref) = actor_refs.first() {
                    if first_ref > 0 {
                        let exp_idx = (first_ref - 1) as usize;
                        if let Some(exp) = pkg.exports.entries.get(exp_idx) {
                            let cls = pkg.get_export_class_name(exp);
                            if !cls.eq_ignore_ascii_case("LevelInfo") {
                                report.warnings.push(format!(
                                    "Actors[0] class is '{}' (expected 'LevelInfo')",
                                    cls
                                ));
                            }
                        }
                    }
                } else {
                    report.is_valid = false;
                    report
                        .errors
                        .push("MyLevel Actors array is empty".to_string());
                }
            }
        }

        for (i, exp) in pkg.exports.entries.iter().enumerate() {
            let cls = pkg.get_export_class_name(exp);
            if cls.eq_ignore_ascii_case("Brush") || cls.eq_ignore_ascii_case("Mover") {
                report.brush_count += 1;
            } else if cls.eq_ignore_ascii_case("Model") {
                report.model_count += 1;
                if let Some(m_bytes) = pkg.get_export_bytes(i) {
                    if m_bytes.len() < 42 {
                        report.warnings.push(format!(
                            "Model export #{i} is only {} bytes (< 42-byte UPrimitive header)",
                            m_bytes.len()
                        ));
                    } else if let Some((bsp, _)) =
                        parse_compiled_umodel_bsp(m_bytes, exp.object_flags, pkg)
                    {
                        if !bsp.nodes.is_empty() || !bsp.zones.is_empty() {
                            report.bsp_node_count += bsp.nodes.len();
                            report.bsp_surf_count += bsp.surfs.len();
                            report.bsp_vert_count += bsp.verts.len();
                            report.zone_count = report.zone_count.max(bsp.zones.len());
                        }
                    }
                }
            }
        }

        report
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let pkg = self.build_package()?;
        pkg.write_to_file(path)?;
        Ok(())
    }

    pub fn from_package(package: Package, map_name: String) -> anyhow::Result<Self> {
        // 1. Locate MyLevel export (Class: Level)
        let my_level_idx = package.exports.entries.iter().position(|e| {
            let name = package.get_name(e.object_name.max(0) as usize).unwrap_or("");
            name == "MyLevel"
        });

        let mut actor_indices = Vec::new();
        let mut raw_reach_specs = Vec::new();

        if let Some(level_exp_idx) = my_level_idx {
            if let Some(level_bytes) = package.get_export_bytes(level_exp_idx) {
                actor_indices = parse_level_actors(level_bytes)?;
                raw_reach_specs = parse_level_reach_specs(level_bytes, &package);
            }
        }

        // If MyLevel wasn't found or was empty, scan all exports with RF_HasStack
        if actor_indices.is_empty() {
            for (idx, exp) in package.exports.entries.iter().enumerate() {
                if (exp.object_flags & 0x02000000) != 0 && exp.serial_size > 0 {
                    actor_indices.push(idx as i32 + 1);
                }
            }
        }

        let mut actors = Vec::new();
        let mut brushes = Vec::new();
        let mut brush_poly_textures = Vec::new();
        let mut lights = Vec::new();
        let mut player_starts = Vec::new();
        let mut path_nodes = Vec::new();
        let mut weapons_and_pickups = Vec::new();
        let mut other_actors = Vec::new();
        let mut read_issues = Vec::new();

        // 2. Parse each actor
        for &act_ref in &actor_indices {
            if act_ref <= 0 {
                continue;
            }
            let exp_idx = (act_ref - 1) as usize;
            let actor = match ActorRecord::from_export(exp_idx, &package) {
                Ok(a) => a,
                Err(error) => {
                    read_issues.push(UnrReadIssue {
                        kind: "actor_decode".into(), actor_name: None,
                        export_index: Some(exp_idx), detail: error.to_string(),
                    });
                    continue;
                }
            };

            let cls_lower = actor.class_name.to_lowercase();

            if cls_lower.contains("brush") || cls_lower.contains("mover") {
                // Parse Model & Polys
                if actor.brush_ref > 0 {
                    let model_exp_idx = (actor.brush_ref - 1) as usize;
                    if let Some(local_polys) = parse_model_polys(model_exp_idx, &package) {
                        let poly_tex_names: Vec<String> = local_polys
                            .polys
                            .iter()
                            .map(|p| {
                                if p.texture_ref != 0 {
                                    let full = package.resolve_full_object_path(p.texture_ref);
                                    if full != "None" {
                                        full
                                    } else {
                                        String::new()
                                    }
                                } else {
                                    String::new()
                                }
                            })
                            .collect();
                        brush_poly_textures.push(poly_tex_names);
                        let world_polys = transform_polys_to_world(&local_polys.polys, &actor);
                        brushes.push(UnrBrush {
                            actor: actor.clone(),
                            local_polys,
                            world_polys,
                        });
                    } else {
                        let empty_polys_ref = verified_empty_model_polys(model_exp_idx, &package);
                        read_issues.push(UnrReadIssue {
                            kind: if empty_polys_ref.is_some() { "empty_brush_polygons" } else { "brush_polygons" }.into(),
                            actor_name: Some(actor.name.clone()),
                            export_index: Some(model_exp_idx),
                            detail: match empty_polys_ref {
                                Some(reference) => format!("Structured model references a decoded empty Polys export (reference {reference}); other model data was not assessed"),
                                None => "Could not decode a non-empty polygon list for this model".into(),
                            },
                        });
                    }
                } else {
                    read_issues.push(UnrReadIssue {
                        kind: "brush_reference".into(), actor_name: Some(actor.name.clone()),
                        export_index: Some(exp_idx),
                        detail: format!("Brush model reference is {}", actor.brush_ref),
                    });
                }
            } else if cls_lower.contains("light") {
                lights.push(actor.clone());
            } else if cls_lower.contains("playerstart") {
                player_starts.push(actor.clone());
            } else if cls_lower.contains("pathnode") || cls_lower.contains("lift") || cls_lower.contains("jump") {
                path_nodes.push(actor.clone());
            } else if is_weapon_or_pickup(&cls_lower) {
                weapons_and_pickups.push(actor.clone());
            } else {
                other_actors.push(actor.clone());
            }

            actors.push(actor);
        }

        // 3. Extract any embedded MyLevel textures and sounds present in this .unr package
        let embedded_textures = crate::utx::UtxPackage::from_package(package.clone(), "MyLevel".to_string())
            .map(|utx| utx.textures)
            .unwrap_or_default();

        let mut embedded_sounds = Vec::new();
        for (i, exp) in package.exports.entries.iter().enumerate() {
            if package.get_export_class_name(exp).eq_ignore_ascii_case("Sound") {
                let snd_name = package.get_export_object_name(exp);
                if let Some(raw_bytes) = package.get_export_bytes(i) {
                    if let Some(snd) = crate::package::parse_sound_export(&snd_name, "MyLevel", raw_bytes) {
                        embedded_sounds.push(snd);
                    }
                }
            }
        }

        // 4. Parse compiled World Model (ULevel::Model / Model_World) BSP if present
        let brush_model_indices: std::collections::HashSet<usize> = actors
            .iter()
            .filter_map(|a| {
                if a.brush_ref > 0 {
                    Some((a.brush_ref - 1) as usize)
                } else {
                    None
                }
            })
            .collect();
        let mut world_bsp: Option<CompiledUModelBsp> = None;
        for (i, exp) in package.exports.entries.iter().enumerate() {
            if brush_model_indices.contains(&i) {
                continue;
            }
            let name = package.get_name(exp.object_name.max(0) as usize).unwrap_or("");
            if name.eq_ignore_ascii_case("Model_World")
                || package.get_export_class_name(exp).eq_ignore_ascii_case("Model")
            {
                if let Some(m_bytes) = package.get_export_bytes(i) {
                    if let Some((bsp, _)) =
                        parse_compiled_umodel_bsp(m_bytes, exp.object_flags, &package)
                    {
                        if !bsp.nodes.is_empty() || !bsp.zones.is_empty() {
                            if name.eq_ignore_ascii_case("Model_World") {
                                world_bsp = Some(bsp);
                                break;
                            }
                            let replace = match &world_bsp {
                                None => true,
                                Some(prev) => {
                                    (
                                        !bsp.light_maps.is_empty(),
                                        bsp.zones.len(),
                                        bsp.nodes.len(),
                                    ) > (
                                        !prev.light_maps.is_empty(),
                                        prev.zones.len(),
                                        prev.nodes.len(),
                                    )
                                }
                            };
                            if replace {
                                world_bsp = Some(bsp);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            map_name,
            actors: actors.into(),
            brushes,
            lights,
            player_starts,
            path_nodes,
            weapons_and_pickups,
            other_actors,
            embedded_textures,
            embedded_sounds,
            brush_poly_textures,
            reach_specs: raw_reach_specs,
            world_bsp,
            read_issues,
            package,
        })
    }

    /// Garbage-collects any `embedded_textures` or `embedded_sounds` in `MyLevel` that are not
    /// referenced by any brush polygon, actor property (`Skin`, `AmbientSound`, `OpeningSound`, etc.),
    /// or `Screenshot` (which is always preserved for the UT99 map browser preview).
    pub fn prune_unreferenced_mylevel_assets(&mut self) -> (usize, usize) {
        let mut referenced_names = std::collections::HashSet::new();
        referenced_names.insert("screenshot".to_string());

        for poly_list in &self.brush_poly_textures {
            for tex_name in poly_list {
                let short = tex_name
                    .trim_matches('\'')
                    .rsplit('.')
                    .next()
                    .unwrap_or(tex_name)
                    .to_ascii_lowercase();
                if !short.is_empty() {
                    referenced_names.insert(short);
                }
            }
        }

        let all_actor_groups = [
            self.actors.as_slice(),
            self.lights.as_slice(),
            self.player_starts.as_slice(),
            self.path_nodes.as_slice(),
            self.weapons_and_pickups.as_slice(),
            self.other_actors.as_slice(),
        ];
        for group in all_actor_groups {
            for act in group {
                for path in act.object_paths.values() {
                    let clean = path.split('\'').nth(1).unwrap_or(path);
                    let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                    if !short.is_empty() {
                        referenced_names.insert(short);
                    }
                }
                for prop in act.properties.values() {
                    if let TaggedValue::Str(s) | TaggedValue::Name(s) = &prop.value {
                        let clean = s.split('\'').nth(1).unwrap_or(s);
                        let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                        if !short.is_empty() {
                            referenced_names.insert(short);
                        }
                    }
                }
            }
        }
        for b in &self.brushes {
            for path in b.actor.object_paths.values() {
                let clean = path.split('\'').nth(1).unwrap_or(path);
                let short = clean.rsplit('.').next().unwrap_or(clean).to_ascii_lowercase();
                if !short.is_empty() {
                    referenced_names.insert(short);
                }
            }
        }

        let before_tex = self.embedded_textures.len();
        self.embedded_textures.retain(|t| {
            let short = t.name.rsplit('.').next().unwrap_or(&t.name).to_ascii_lowercase();
            referenced_names.contains(&short)
        });
        let pruned_tex = before_tex.saturating_sub(self.embedded_textures.len());

        let before_snd = self.embedded_sounds.len();
        self.embedded_sounds.retain(|s| {
            let short = s.name.rsplit('.').next().unwrap_or(&s.name).to_ascii_lowercase();
            referenced_names.contains(&short)
        });
        let pruned_snd = before_snd.saturating_sub(self.embedded_sounds.len());

        (pruned_tex, pruned_snd)
    }
}

/// Parses the actors array from ULevel binary payload.
fn parse_level_actors(data: &[u8]) -> io::Result<Vec<i32>> {
    let (actors, _) = parse_level_actors_and_reachspecs(data)?;
    Ok(actors)
}

fn skip_fstring(cur: &mut Cursor<&[u8]>) -> io::Result<()> {
    let len = read_compact_index(cur)?;
    if len > 0 {
        let pos = cur.position() as usize;
        let rem = cur.get_ref().len().saturating_sub(pos);
        cur.set_position((pos + (len as usize).min(rem)) as u64);
    } else if len < 0 {
        let pos = cur.position() as usize;
        let bytes = ((-len) as usize) * 2;
        let rem = cur.get_ref().len().saturating_sub(pos);
        cur.set_position((pos + bytes.min(rem)) as u64);
    }
    Ok(())
}

fn parse_level_actors_and_reachspecs(data: &[u8]) -> io::Result<(Vec<i32>, usize)> {
    if data.len() < 8 {
        return Ok((Vec::new(), 0));
    }

    let mut offset = 0;
    if data.len() >= 9 {
        let mut cur1 = Cursor::new(&data[1..9]);
        let c1 = cur1.read_i32::<LittleEndian>()?;
        let m1 = cur1.read_i32::<LittleEndian>()?;
        if c1 == m1 && c1 > 0 && c1 < 20_000 {
            offset = 1;
        }
    }
    if offset == 0 && data.len() >= 8 {
        let mut cur0 = Cursor::new(&data[0..8]);
        let c0 = cur0.read_i32::<LittleEndian>()?;
        let m0 = cur0.read_i32::<LittleEndian>()?;
        if !(c0 == m0 && c0 > 0 && c0 < 20_000) {
            offset = 1;
        }
    }

    let mut cur = Cursor::new(&data[offset..]);
    let count = cur.read_i32::<LittleEndian>()?;
    let _max_count = cur.read_i32::<LittleEndian>()?;

    let mut actors = Vec::with_capacity(count.max(0) as usize);
    for _ in 0..count {
        if let Ok(act_ref) = read_compact_index(&mut cur) {
            if act_ref > 0 {
                actors.push(act_ref);
            }
        } else {
            break;
        }
    }

    // Attempt to read FURL + Model + ReachSpecs count
    let mut rs_count = 0usize;
    if skip_fstring(&mut cur).is_ok()
        && skip_fstring(&mut cur).is_ok()
        && skip_fstring(&mut cur).is_ok()
        && skip_fstring(&mut cur).is_ok()
    {
        if let Ok(op_cnt) = read_compact_index(&mut cur) {
            if (0..=64).contains(&op_cnt) {
                for _ in 0..op_cnt {
                    let _ = skip_fstring(&mut cur);
                }
                let _ = cur.read_i32::<LittleEndian>(); // Port
                let _ = cur.read_i32::<LittleEndian>(); // Valid
                let _ = read_compact_index(&mut cur); // World Model ref
                if let Ok(rc) = read_compact_index(&mut cur) {
                    if (0..=100_000).contains(&rc) {
                        rs_count = rc as usize;
                    }
                }
            }
        }
    }

    Ok((actors, rs_count))
}

fn parse_level_reach_specs(data: &[u8], package: &Package) -> Vec<ReachSpecRecord> {
    if data.len() < 8 {
        return Vec::new();
    }
    let mut offset = 0;
    if data.len() >= 9 {
        if let (Ok(c1), Ok(m1)) = (
            Cursor::new(&data[1..5]).read_i32::<LittleEndian>(),
            Cursor::new(&data[5..9]).read_i32::<LittleEndian>(),
        ) {
            if c1 == m1 && c1 > 0 && c1 < 20_000 {
                offset = 1;
            }
        }
    }
    let mut cur = Cursor::new(&data[offset..]);
    let Ok(count) = cur.read_i32::<LittleEndian>() else {
        return Vec::new();
    };
    let Ok(_max_count) = cur.read_i32::<LittleEndian>() else {
        return Vec::new();
    };
    for _ in 0..count.max(0) {
        if read_compact_index(&mut cur).is_err() {
            return Vec::new();
        }
    }
    if skip_fstring(&mut cur).is_err()
        || skip_fstring(&mut cur).is_err()
        || skip_fstring(&mut cur).is_err()
        || skip_fstring(&mut cur).is_err()
    {
        return Vec::new();
    }
    let Ok(op_cnt) = read_compact_index(&mut cur) else {
        return Vec::new();
    };
    if !(0..=64).contains(&op_cnt) {
        return Vec::new();
    }
    for _ in 0..op_cnt {
        if skip_fstring(&mut cur).is_err() {
            return Vec::new();
        }
    }
    let _ = cur.read_i32::<LittleEndian>();
    let _ = cur.read_i32::<LittleEndian>();
    let _ = read_compact_index(&mut cur); // World Model ref
    let Ok(rc) = read_compact_index(&mut cur) else {
        return Vec::new();
    };
    if !(0..=50_000).contains(&rc) {
        return Vec::new();
    }
    let mut specs = Vec::with_capacity(rc as usize);
    for _ in 0..rc {
        let Ok(distance) = cur.read_i32::<LittleEndian>() else {
            break;
        };
        let Ok(start_ref) = read_compact_index(&mut cur) else {
            break;
        };
        let Ok(end_ref) = read_compact_index(&mut cur) else {
            break;
        };
        let Ok(collision_radius) = cur.read_i32::<LittleEndian>() else {
            break;
        };
        let Ok(collision_height) = cur.read_i32::<LittleEndian>() else {
            break;
        };
        let Ok(reach_flags) = cur.read_i32::<LittleEndian>() else {
            break;
        };
        let Ok(b_pruned) = cur.read_u8() else {
            break;
        };
        let start_actor_name = if start_ref > 0 {
            package.resolve_object_reference(start_ref)
        } else {
            String::new()
        };
        let end_actor_name = if end_ref > 0 {
            package.resolve_object_reference(end_ref)
        } else {
            String::new()
        };
        specs.push(ReachSpecRecord {
            distance,
            start_actor_name,
            end_actor_name,
            collision_radius,
            collision_height,
            reach_flags,
            b_pruned,
        });
    }
    specs
}

/// Only trust an empty list reached through the structured model parser. A
/// byte-scan can find unrelated references and must not suppress decode warnings.
fn verified_empty_model_polys(model_exp_idx: usize, package: &Package) -> Option<i32> {
    let exp = package.exports.entries.get(model_exp_idx)?;
    let bytes = package.get_export_bytes(model_exp_idx)?;
    let (_, reference) = parse_compiled_umodel_bsp(bytes, exp.object_flags, package)?;
    if reference <= 0 { return None; }
    let target_idx = (reference - 1) as usize;
    let target = package.exports.entries.get(target_idx)?;
    if !package.get_export_class_name(target).eq_ignore_ascii_case("Polys") { return None; }
    let data = package.get_export_bytes(target_idx)?;
    if data.len() < 8 { return None; }
    let polys = RawPolys::from_bytes_with_flags(data, target.object_flags).ok()?;
    if polys.polys.is_empty() { Some(reference) } else { None }
}

fn parse_model_polys(model_exp_idx: usize, package: &Package) -> Option<RawPolys> {
    let model_bytes = package.get_export_bytes(model_exp_idx)?;
    if model_bytes.is_empty() {
        return None;
    }

    // 1. Fast path for canonical UE1 UModel (tagged properties + 41-byte UPrimitive bounds + 5 empty arrays + Polys ref)
    let exp = package.exports.entries.get(model_exp_idx)?;
    let has_stack = (exp.object_flags & 0x02000000) != 0;
    let mut cur = Cursor::new(model_bytes);
    if has_stack {
        if let Ok(node) = read_compact_index(&mut cur) {
            let _ = read_compact_index(&mut cur);
            let _ = cur.read_u64::<LittleEndian>();
            let _ = cur.read_i32::<LittleEndian>();
            if node != 0 {
                let _ = read_compact_index(&mut cur);
            }
        }
    }
    let after_props_pos = if read_tagged_properties(&mut cur, package).is_ok() {
        cur.position() as usize
    } else {
        1usize
    };

    if after_props_pos + 41 < model_bytes.len() {
        let mut fast_cur = Cursor::new(&model_bytes[after_props_pos + 41..]);
        let mut five_zeros = true;
        for _ in 0..5 {
            if read_compact_index(&mut fast_cur).ok() != Some(0) {
                five_zeros = false;
                break;
            }
        }
        if five_zeros {
            let pos_after_zeros = fast_cur.position();
            let mut candidate_refs = Vec::with_capacity(2);
            if let (Ok(_num_shared_sides), Ok(0)) = (
                fast_cur.read_i32::<LittleEndian>(),
                fast_cur.read_i32::<LittleEndian>(),
            ) {
                if let Ok(r) = read_compact_index(&mut fast_cur) {
                    candidate_refs.push(r);
                }
            }
            fast_cur.set_position(pos_after_zeros);
            if let Ok(r) = read_compact_index(&mut fast_cur) {
                candidate_refs.push(r);
            }
            for ref_idx in candidate_refs {
                if ref_idx > 0 && (ref_idx as usize) <= package.exports.entries.len() {
                    let target_idx = (ref_idx - 1) as usize;
                    let target_exp = &package.exports.entries[target_idx];
                    let target_name = package
                        .get_name(target_exp.object_name.max(0) as usize)
                        .unwrap_or("");
                    if target_name.starts_with("Polys")
                        || package.get_export_class_name(target_exp) == "Polys"
                    {
                        if let Some(polys_bytes) = package.get_export_bytes(target_idx) {
                            if let Ok(polys) = RawPolys::from_bytes_with_flags(
                                polys_bytes,
                                target_exp.object_flags,
                            ) {
                                if !polys.polys.is_empty() {
                                    return Some(polys);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // 1b. Structured UModel BSP parser (handles non-empty Vectors, Points, Nodes, Surfs, Verts, Zones)
    if let Some((_, ref_idx)) = parse_compiled_umodel_bsp(model_bytes, exp.object_flags, package) {
        if ref_idx > 0 && (ref_idx as usize) <= package.exports.entries.len() {
            let target_idx = (ref_idx - 1) as usize;
            let target_exp = &package.exports.entries[target_idx];
            if let Some(polys_bytes) = package.get_export_bytes(target_idx) {
                if let Ok(polys) =
                    RawPolys::from_bytes_with_flags(polys_bytes, target_exp.object_flags)
                {
                    if !polys.polys.is_empty() {
                        return Some(polys);
                    }
                }
            }
        }
    }

    // 2. Scan after UPrimitive 41-byte header (or from 0 for compact synthetic models)
    let scan_start = if model_bytes.len() > after_props_pos + 41 {
        after_props_pos + 41
    } else {
        0
    };

    for offset in scan_start..model_bytes.len() {
        let mut cur = Cursor::new(&model_bytes[offset..]);
        if let Ok(ref_idx) = read_compact_index(&mut cur) {
            if ref_idx > 0 && (ref_idx as usize) <= package.exports.entries.len() {
                let target_idx = (ref_idx - 1) as usize;
                let target_exp = &package.exports.entries[target_idx];
                let target_name = package.get_name(target_exp.object_name.max(0) as usize).unwrap_or("");
                let is_polys = target_name.starts_with("Polys") || package.get_export_class_name(target_exp) == "Polys";
                if is_polys {
                    if let Some(polys_bytes) = package.get_export_bytes(target_idx) {
                        if let Ok(polys) = RawPolys::from_bytes_with_flags(polys_bytes, target_exp.object_flags) {
                            if !polys.polys.is_empty() {
                                return Some(polys);
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Transforms local brush polygons to world space using the Actor's transform.
fn transform_polys_to_world(polys: &[RawPoly], actor: &ActorRecord) -> Vec<RawPoly> {
    // Unreal Engine Rotator units: 65536 units = 360 degrees (2*PI radians)
    const ROT_TO_RAD: f32 = std::f32::consts::PI / 32768.0;

    let pitch_rad = actor.rotation.0 as f32 * ROT_TO_RAD;
    let yaw_rad = actor.rotation.1 as f32 * ROT_TO_RAD;
    let roll_rad = actor.rotation.2 as f32 * ROT_TO_RAD;

    // Unreal rotation order: Roll (X) * Pitch (Y) * Yaw (Z)
    let rotation_quat = Quat::from_rotation_z(yaw_rad)
        * Quat::from_rotation_y(pitch_rad)
        * Quat::from_rotation_x(roll_rad);

    let transform = Mat4::from_translation(actor.location)
        * Mat4::from_scale(actor.post_scale)
        * Mat4::from_quat(rotation_quat)
        * Mat4::from_scale(actor.main_scale)
        * Mat4::from_translation(-actor.prepivot);

    polys
        .iter()
        .map(|p| {
            let world_vertices = p
                .vertices
                .iter()
                .map(|&v| transform.transform_point3(v))
                .collect();

            let world_normal = rotation_quat * p.normal;
            let world_base = transform.transform_point3(p.base);

            RawPoly {
                base: world_base,
                normal: world_normal,
                texture_u: p.texture_u,
                texture_v: p.texture_v,
                vertices: world_vertices,
                poly_flags: p.poly_flags,
                actor_ref: p.actor_ref,
                texture_ref: p.texture_ref,
                item_name: p.item_name,
                link_index: p.link_index,
                brush_poly_index: p.brush_poly_index,
                pan_u: p.pan_u,
                pan_v: p.pan_v,
            }
        })
        .collect()
}

fn is_weapon_or_pickup(cls: &str) -> bool {
    cls.contains("rifle")
        || cls.contains("cannon")
        || cls.contains("gun")
        || cls.contains("eightball")
        || cls.contains("ammo")
        || cls.contains("armor")
        || cls.contains("health")
        || cls.contains("medbox")
        || cls.contains("damage")
        || cls.contains("shield")
        || cls.contains("pickup")
        || cls.contains("translocator")
}

/// Resolves or creates the UE1 `Class` import for an actor, preserving its exact subclass and outer package.
fn resolve_actor_class_import(
    pkg: &mut Package,
    actor: &ActorRecord,
    default_cls: &str,
    _imp_core: i32,
    imp_engine: i32,
    imp_botpack: i32,
    imp_unrealshare: i32,
    n_package: i32,
    n_class: i32,
) -> i32 {
    let n_core = pkg.names.add("Core", 0) as i32;
    let raw_cls = if actor.class_name.trim().is_empty() {
        default_cls
    } else {
        actor.class_name.trim()
    };
    let raw_short_cls = raw_cls.rsplit('.').next().unwrap_or(raw_cls).trim();
    let short_cls = if raw_short_cls.eq_ignore_ascii_case("Banner1") {
        "Flag1"
    } else {
        raw_short_cls
    };
    let n_cls = pkg.names.add(short_cls, 0) as i32;
    let outer_pkg = if let Some((pkg_prefix, _)) = raw_cls.split_once('.') {
        let p = pkg_prefix.trim();
        if p.eq_ignore_ascii_case("Engine") {
            infer_class_package(
                pkg,
                short_cls,
                n_package,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
            )
        } else if p.eq_ignore_ascii_case("Botpack") {
            imp_botpack
        } else if p.eq_ignore_ascii_case("UnrealShare") {
            imp_unrealshare
        } else {
            let n_p = pkg.names.add(p, 0) as i32;
            find_or_add_import(pkg, n_core, n_package, 0, n_p)
        }
    } else if let Some(saved_pkg) = actor
        .object_paths
        .get("__class_package")
        .filter(|s| !s.trim().is_empty())
    {
        let p = saved_pkg.trim();
        if p.eq_ignore_ascii_case("Engine") {
            infer_class_package(
                pkg,
                short_cls,
                n_package,
                imp_engine,
                imp_botpack,
                imp_unrealshare,
            )
        } else if p.eq_ignore_ascii_case("Botpack") {
            imp_botpack
        } else if p.eq_ignore_ascii_case("UnrealShare") {
            imp_unrealshare
        } else {
            let n_p = pkg.names.add(p, 0) as i32;
            find_or_add_import(pkg, n_core, n_package, 0, n_p)
        }
    } else {
        infer_class_package(
            pkg,
            short_cls,
            n_package,
            imp_engine,
            imp_botpack,
            imp_unrealshare,
        )
    };
    find_or_add_import(pkg, n_core, n_class, outer_pkg, n_cls)
}

/// Maps an Unreal Tournament 99 actor class name to its canonical outer package import reference.
fn infer_class_package(
    pkg: &mut Package,
    cls_name: &str,
    n_package: i32,
    imp_engine: i32,
    imp_botpack: i32,
    imp_unrealshare: i32,
) -> i32 {
    let lower = cls_name.to_ascii_lowercase();
    if lower.starts_with("ut_")
        || matches!(
            lower.as_str(),
            "warheadlauncher"
                | "shockrifle"
                | "supershockrifle"
                | "pulsegun"
                | "ripper"
                | "minigun2"
                | "impacthammer"
                | "enforcer"
                | "doubleenforcer"
                | "sniperrifle"
                | "chainsaw"
                | "translocator"
                | "healthvial"
                | "healthpack"
                | "medbox"
                | "thighpads"
                | "armor2"
                | "udamage"
                | "flagbase"
                | "defensepoint"
                | "alternatepath"
                | "controlpoint"
                | "domcontrolpoint"
                | "fortstandard"
                | "assaultinfo"
                | "teamtrigger"
                | "timedtrigger"
                | "clientmover"
                | "shockcore"
                | "pammo"
                | "bladehopper"
                | "flakammo"
                | "bioammo"
                | "minihealthpack"
                | "miniammo"
                | "rifleammo"
                | "bulletbox"
                | "rocketpack"
                | "eclip"
                | "jumpspot"
                | "translocdest"
                | "blockedpath"
                | "onewaypath"
                | "rocketsplash"
                | "lightcone"
                | "lightbox"
                | "distanceviewtrigger"
                | "fadeviewtrigger"
                | "kicker"
                | "botpoint"
                | "spectatorcam"
                | "visibleteleporter"
                | "utteleeffect"
                | "utteleporteffect"
        )
    {
        imp_botpack
    } else if matches!(
        lower.as_str(),
        "dispersionpistol"
            | "favoritesteleporter"
            | "automag"
            | "stinger"
            | "asmd"
            | "eightball"
            | "flakcannon"
            | "razorjack"
            | "gesbiorifle"
            | "rifle"
            | "minigun"
            | "health"
            | "superhealth"
            | "bandage"
            | "kevlarsuit"
            | "armor"
            | "shieldbelt"
            | "jumpboots"
            | "amplifier"
            | "flashlight"
            | "searchlight"
            | "flare"
            | "scubagear"
            | "breakingglass"
            | "explodingwall"
            | "triggeredambientsound"
            | "stochastictrigger"
            | "triggerlight"
            | "torchflame"
            | "lantern"
            | "lantern2"
            | "candle"
            | "candle2"
            | "sconce"
            | "chargeLight"
            | "chargelight"
            | "sightlight"
            | "overheatlight"
            | "elevatormover"
            | "elevatortrigger"
            | "gradualmover"
            | "assertmover"
            | "attachmover"
            | "loopmover"
            | "mixmover"
            | "rotatingmover"
            | "waterzone"
            | "lavaZone"
            | "lavazone"
            | "slimezone"
            | "nitrogenzone"
            | "tarzone"
            | "vacuumzone"
            | "pressurezone"
            | "killz"
            | "musicEvent"
            | "musicevent"
            | "roundrobin"
            | "specialevent"
            | "translatorEvent"
            | "translatorevent"
            | "earthquake"
            | "thingfactory"
            | "creaturefactory"
            | "spawnpoint"
            | "nalifruit"
            | "nali"
            | "skaarjwarrior"
            | "brute"
            | "fly"
            | "manta"
            | "tentacle"
            | "slith"
            | "steelbarrel"
            | "barrel"
            | "steelbox"
            | "woodenbox"
            | "chest"
            | "fan2"
            | "urn"
            | "vase"
            | "pottery0"
            | "pottery1"
            | "pottery2"
            | "monkstatue"
            | "nalistatue"
            | "boulder"
            | "boulder1"
            | "boulder2"
            | "plant1"
            | "plant2"
            | "plant3"
            | "plant4"
            | "plant5"
            | "plant6"
            | "plant7"
            | "tree1"
            | "tree2"
            | "tree3"
            | "tree4"
            | "tree5"
            | "tree6"
            | "tree7"
            | "tree8"
            | "tree9"
            | "tree10"
            | "tree11"
            | "tree12"
            | "sign1"
            | "wire"
            | "knife"
            | "book"
            | "chair"
            | "table"
            | "escapepod"
    ) {
        imp_unrealshare
    } else if matches!(
        lower.as_str(),
        "queendest"
            | "queen"
            | "titan"
            | "warlord"
            | "krall"
            | "mercenary"
            | "gasbag"
            | "pupae"
            | "skaarjtrooper"
            | "skaarjgunner"
            | "skaarjsniper"
            | "skaarjofficer"
            | "skaarjinfantry"
            | "iceskaarj"
            | "darkmatchlight"
            | "robot"
            | "lamp1"
            | "lamp4"
            | "flag1"
            | "flag2"
            | "flag3"
            | "tapestry1"
            | "dice"
    ) {
        let n_core = pkg.names.add("Core", 0) as i32;
        let n_unreali = pkg.names.add("UnrealI", 0) as i32;
        find_or_add_import(pkg, n_core, n_package, 0, n_unreali)
    } else {
        imp_engine
    }
}


