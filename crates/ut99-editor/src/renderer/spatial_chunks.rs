use crate::camera::ViewportType;
use crate::culling::Frustum;
use crate::renderer::vertex::{LineVertex, SolidVertex};
use glam::Vec3;

pub(crate) const SOLID_BUCKET_COUNT: usize = 5;
const MAX_GRID_CELLS: usize = 64;
const MAX_TOTAL_SLOTS: usize = MAX_GRID_CELLS + 1;
const SOLID_CHUNK_THRESHOLD_TRIS: usize = 512;
const LINE_CHUNK_THRESHOLD_SEGS: usize = 2048;
const CHUNK_AABB_EPS: f32 = 0.05;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SpatialChunk {
    pub start: u32,
    pub end: u32,
    pub aabb_min: Vec3,
    pub aabb_max: Vec3,
    pub prim_count: u32,
    pub brush_count: u32,
    pub actor_count: u32,
    pub always_visible: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct AdaptiveGrid {
    origin: Vec3,
    inv_extent: Vec3,
    dims: [usize; 3],
    pub cell_count: usize,
    pub max_cell_dim: f32,
    morton_rank: [u8; MAX_GRID_CELLS],
}

impl AdaptiveGrid {
    pub(crate) fn new(min: Vec3, max: Vec3) -> Self {
        let extent = (max - min).max(Vec3::splat(1.0));
        let mut dims = [1usize; 3];
        let ext = extent.to_array();
        loop {
            let mut best_axis = None;
            let mut best_cell_size = 0.0f32;
            for a in 0..3 {
                if dims[a] >= 8 {
                    continue;
                }
                let next_cells = (dims[a] + 1) * dims[(a + 1) % 3] * dims[(a + 2) % 3];
                if next_cells <= MAX_GRID_CELLS {
                    let cell_size = ext[a] / dims[a] as f32;
                    if cell_size > best_cell_size {
                        best_cell_size = cell_size;
                        best_axis = Some(a);
                    }
                }
            }
            match best_axis {
                Some(a) => dims[a] += 1,
                None => break,
            }
        }

        let cell_count = dims[0] * dims[1] * dims[2];
        let max_cell_dim = (ext[0] / dims[0] as f32)
            .max(ext[1] / dims[1] as f32)
            .max(ext[2] / dims[2] as f32);

        let mut order: [(u32, usize); MAX_GRID_CELLS] = [(u32::MAX, 0); MAX_GRID_CELLS];
        for iz in 0..dims[2] {
            for iy in 0..dims[1] {
                for ix in 0..dims[0] {
                    let linear = (iz * dims[1] + iy) * dims[0] + ix;
                    order[linear] = (morton_3d(ix as u32, iy as u32, iz as u32), linear);
                }
            }
        }
        order[..cell_count].sort_unstable_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));

        let mut morton_rank = [0u8; MAX_GRID_CELLS];
        for (rank, &(_, linear)) in order[..cell_count].iter().enumerate() {
            morton_rank[linear] = rank as u8;
        }

        Self {
            origin: min,
            inv_extent: Vec3::new(1.0 / ext[0], 1.0 / ext[1], 1.0 / ext[2]),
            dims,
            cell_count,
            max_cell_dim,
            morton_rank,
        }
    }

    #[inline]
    pub(crate) fn classify_point(&self, p: Vec3) -> usize {
        if !p.is_finite() {
            return self.cell_count;
        }
        let u = ((p.x - self.origin.x) * self.inv_extent.x).clamp(0.0, 0.999_999);
        let v = ((p.y - self.origin.y) * self.inv_extent.y).clamp(0.0, 0.999_999);
        let w = ((p.z - self.origin.z) * self.inv_extent.z).clamp(0.0, 0.999_999);
        let ix = (u * self.dims[0] as f32) as usize;
        let iy = (v * self.dims[1] as f32) as usize;
        let iz = (w * self.dims[2] as f32) as usize;
        let linear = (iz * self.dims[1] + iy) * self.dims[0] + ix;
        self.morton_rank[linear] as usize
    }
}

#[inline]
fn part1by2(mut n: u32) -> u32 {
    n &= 0x03FF;
    n = (n ^ (n << 16)) & 0x0300_00FF;
    n = (n ^ (n << 8)) & 0x0300_F00F;
    n = (n ^ (n << 4)) & 0x030C_30C3;
    n = (n ^ (n << 2)) & 0x0924_9249;
    n
}

#[inline]
fn morton_3d(x: u32, y: u32, z: u32) -> u32 {
    part1by2(x) | (part1by2(y) << 1) | (part1by2(z) << 2)
}

#[inline]
fn pad_aabb(min: Vec3, max: Vec3) -> (Vec3, Vec3) {
    let pad = Vec3::splat(CHUNK_AABB_EPS);
    (min - pad, max + pad)
}

pub(crate) fn partition_solid_bucket_into_chunks(
    src_indices: &[u32],
    verts: &[SolidVertex],
    base_offset: u32,
    dst_indices: &mut Vec<u32>,
    out_chunks: &mut Vec<SpatialChunk>,
    slot_scratch: &mut Vec<u8>,
) {
    out_chunks.clear();
    let tri_count = src_indices.len() / 3;
    if tri_count == 0 {
        return;
    }
    let used_len = tri_count * 3;
    let src = &src_indices[..used_len];

    if tri_count < SOLID_CHUNK_THRESHOLD_TRIS {
        let mut bmin = Vec3::splat(f32::MAX);
        let mut bmax = Vec3::splat(f32::MIN);
        let mut any_nonfinite = false;
        for &idx in src {
            let p = Vec3::from_array(verts[idx as usize].position);
            if p.is_finite() {
                bmin = bmin.min(p);
                bmax = bmax.max(p);
            } else {
                any_nonfinite = true;
            }
        }
        let (aabb_min, aabb_max, always_visible) = if any_nonfinite || bmin.x > bmax.x {
            (Vec3::ZERO, Vec3::ZERO, true)
        } else {
            let (pmin, pmax) = pad_aabb(bmin, bmax);
            (pmin, pmax, false)
        };
        dst_indices.extend_from_slice(src);
        out_chunks.push(SpatialChunk {
            start: base_offset,
            end: base_offset + used_len as u32,
            aabb_min,
            aabb_max,
            prim_count: tri_count as u32,
            brush_count: 0,
            actor_count: 0,
            always_visible,
        });
        return;
    }

    let mut cmin = Vec3::splat(f32::MAX);
    let mut cmax = Vec3::splat(f32::MIN);
    for t in 0..tri_count {
        let b = t * 3;
        let p0 = Vec3::from_array(verts[src[b] as usize].position);
        let p1 = Vec3::from_array(verts[src[b + 1] as usize].position);
        let p2 = Vec3::from_array(verts[src[b + 2] as usize].position);
        if p0.is_finite() && p1.is_finite() && p2.is_finite() {
            let c = (p0 + p1 + p2) * (1.0 / 3.0);
            cmin = cmin.min(c);
            cmax = cmax.max(c);
        }
    }

    if cmin.x > cmax.x || !cmin.is_finite() || !cmax.is_finite() {
        dst_indices.extend_from_slice(src);
        out_chunks.push(SpatialChunk {
            start: base_offset,
            end: base_offset + used_len as u32,
            aabb_min: Vec3::ZERO,
            aabb_max: Vec3::ZERO,
            prim_count: tri_count as u32,
            brush_count: 0,
            actor_count: 0,
            always_visible: true,
        });
        return;
    }

    let grid = AdaptiveGrid::new(cmin, cmax);
    let large_slot = grid.cell_count;
    let total_slots = large_slot + 1;
    let large_thresh = 2.0 * grid.max_cell_dim;

    slot_scratch.clear();
    slot_scratch.resize(tri_count, 0);
    let mut counts = [0u32; MAX_TOTAL_SLOTS];
    let mut slot_min = [Vec3::splat(f32::MAX); MAX_TOTAL_SLOTS];
    let mut slot_max = [Vec3::splat(f32::MIN); MAX_TOTAL_SLOTS];
    let mut slot_nonfinite = [false; MAX_TOTAL_SLOTS];

    for t in 0..tri_count {
        let b = t * 3;
        let p0 = Vec3::from_array(verts[src[b] as usize].position);
        let p1 = Vec3::from_array(verts[src[b + 1] as usize].position);
        let p2 = Vec3::from_array(verts[src[b + 2] as usize].position);
        let tmin = p0.min(p1).min(p2);
        let tmax = p0.max(p1).max(p2);
        let finite = tmin.is_finite() && tmax.is_finite();
        let slot = if !finite || (tmax - tmin).max_element() > large_thresh {
            large_slot
        } else {
            grid.classify_point((p0 + p1 + p2) * (1.0 / 3.0))
        };
        slot_scratch[t] = slot as u8;
        counts[slot] += 1;
        if finite {
            slot_min[slot] = slot_min[slot].min(tmin);
            slot_max[slot] = slot_max[slot].max(tmax);
        } else {
            slot_nonfinite[slot] = true;
        }
    }

    let mut write_tri_offsets = [0usize; MAX_TOTAL_SLOTS];
    let mut running_tris = 0u32;
    for s in 0..total_slots {
        let count = counts[s];
        if count == 0 {
            continue;
        }
        write_tri_offsets[s] = running_tris as usize;
        let start = base_offset + running_tris * 3;
        let end = start + count * 3;
        let (aabb_min, aabb_max, always_visible) = if slot_nonfinite[s] || slot_min[s].x > slot_max[s].x {
            (Vec3::ZERO, Vec3::ZERO, true)
        } else {
            let (pmin, pmax) = pad_aabb(slot_min[s], slot_max[s]);
            (pmin, pmax, false)
        };
        out_chunks.push(SpatialChunk {
            start,
            end,
            aabb_min,
            aabb_max,
            prim_count: count,
            brush_count: 0,
            actor_count: 0,
            always_visible,
        });
        running_tris += count;
    }

    let dst_start = dst_indices.len();
    dst_indices.resize(dst_start + used_len, 0);
    let dst = &mut dst_indices[dst_start..];
    for t in 0..tri_count {
        let s = slot_scratch[t] as usize;
        let dst_idx = write_tri_offsets[s] * 3;
        write_tri_offsets[s] += 1;
        let src_idx = t * 3;
        dst[dst_idx] = src[src_idx];
        dst[dst_idx + 1] = src[src_idx + 1];
        dst[dst_idx + 2] = src[src_idx + 2];
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LineChunkPartition {
    pub(crate) grid: Option<AdaptiveGrid>,
    slot_to_chunk: [u8; MAX_TOTAL_SLOTS],
}

impl LineChunkPartition {
    #[inline]
    pub(crate) fn chunk_for_point(&self, p: Vec3, extent: f32) -> usize {
        let Some(ref grid) = self.grid else {
            return 0;
        };
        let slot = if !p.is_finite() || extent > 2.0 * grid.max_cell_dim {
            grid.cell_count
        } else {
            grid.classify_point(p)
        };
        let mapped = self.slot_to_chunk[slot];
        if mapped != u8::MAX {
            mapped as usize
        } else {
            0
        }
    }
}

pub(crate) fn partition_world_lines_into_chunks(
    world_slice: &mut [LineVertex],
    base_offset: u32,
    out_chunks: &mut Vec<SpatialChunk>,
    line_scratch: &mut Vec<LineVertex>,
    slot_scratch: &mut Vec<u8>,
) -> LineChunkPartition {
    out_chunks.clear();
    let empty_map = [u8::MAX; MAX_TOTAL_SLOTS];
    let seg_count = world_slice.len() / 2;
    if seg_count == 0 {
        return LineChunkPartition {
            grid: None,
            slot_to_chunk: empty_map,
        };
    }

    let used_len = seg_count * 2;
    let slice = &mut world_slice[..used_len];

    if seg_count < LINE_CHUNK_THRESHOLD_SEGS {
        let mut bmin = Vec3::splat(f32::MAX);
        let mut bmax = Vec3::splat(f32::MIN);
        let mut any_nonfinite = false;
        for v in slice.iter() {
            let p = Vec3::from_array(v.position);
            if p.is_finite() {
                bmin = bmin.min(p);
                bmax = bmax.max(p);
            } else {
                any_nonfinite = true;
            }
        }
        let (aabb_min, aabb_max, always_visible) = if any_nonfinite || bmin.x > bmax.x {
            (Vec3::ZERO, Vec3::ZERO, true)
        } else {
            let (pmin, pmax) = pad_aabb(bmin, bmax);
            (pmin, pmax, false)
        };
        out_chunks.push(SpatialChunk {
            start: base_offset,
            end: base_offset + used_len as u32,
            aabb_min,
            aabb_max,
            prim_count: seg_count as u32,
            brush_count: 0,
            actor_count: 0,
            always_visible,
        });
        return LineChunkPartition {
            grid: None,
            slot_to_chunk: empty_map,
        };
    }

    let mut cmin = Vec3::splat(f32::MAX);
    let mut cmax = Vec3::splat(f32::MIN);
    for seg in 0..seg_count {
        let p0 = Vec3::from_array(slice[seg * 2].position);
        let p1 = Vec3::from_array(slice[seg * 2 + 1].position);
        if p0.is_finite() && p1.is_finite() {
            let mid = (p0 + p1) * 0.5;
            cmin = cmin.min(mid);
            cmax = cmax.max(mid);
        }
    }

    if cmin.x > cmax.x || !cmin.is_finite() || !cmax.is_finite() {
        out_chunks.push(SpatialChunk {
            start: base_offset,
            end: base_offset + used_len as u32,
            aabb_min: Vec3::ZERO,
            aabb_max: Vec3::ZERO,
            prim_count: seg_count as u32,
            brush_count: 0,
            actor_count: 0,
            always_visible: true,
        });
        return LineChunkPartition {
            grid: None,
            slot_to_chunk: empty_map,
        };
    }

    let grid = AdaptiveGrid::new(cmin, cmax);
    let large_slot = grid.cell_count;
    let total_slots = large_slot + 1;
    let large_thresh = 2.0 * grid.max_cell_dim;

    slot_scratch.clear();
    slot_scratch.resize(seg_count, 0);
    let mut counts = [0u32; MAX_TOTAL_SLOTS];
    let mut slot_min = [Vec3::splat(f32::MAX); MAX_TOTAL_SLOTS];
    let mut slot_max = [Vec3::splat(f32::MIN); MAX_TOTAL_SLOTS];
    let mut slot_nonfinite = [false; MAX_TOTAL_SLOTS];

    for seg in 0..seg_count {
        let p0 = Vec3::from_array(slice[seg * 2].position);
        let p1 = Vec3::from_array(slice[seg * 2 + 1].position);
        let smin = p0.min(p1);
        let smax = p0.max(p1);
        let finite = smin.is_finite() && smax.is_finite();
        let slot = if !finite || (smax - smin).max_element() > large_thresh {
            large_slot
        } else {
            grid.classify_point((p0 + p1) * 0.5)
        };
        slot_scratch[seg] = slot as u8;
        counts[slot] += 1;
        if finite {
            slot_min[slot] = slot_min[slot].min(smin);
            slot_max[slot] = slot_max[slot].max(smax);
        } else {
            slot_nonfinite[slot] = true;
        }
    }

    let mut write_seg_offsets = [0usize; MAX_TOTAL_SLOTS];
    let mut slot_to_chunk = [u8::MAX; MAX_TOTAL_SLOTS];
    let mut running_segs = 0u32;
    for s in 0..total_slots {
        let count = counts[s];
        if count == 0 {
            continue;
        }
        slot_to_chunk[s] = out_chunks.len() as u8;
        write_seg_offsets[s] = running_segs as usize;
        let start = base_offset + running_segs * 2;
        let end = start + count * 2;
        let (aabb_min, aabb_max, always_visible) = if slot_nonfinite[s] || slot_min[s].x > slot_max[s].x {
            (Vec3::ZERO, Vec3::ZERO, true)
        } else {
            let (pmin, pmax) = pad_aabb(slot_min[s], slot_max[s]);
            (pmin, pmax, false)
        };
        out_chunks.push(SpatialChunk {
            start,
            end,
            aabb_min,
            aabb_max,
            prim_count: count,
            brush_count: 0,
            actor_count: 0,
            always_visible,
        });
        running_segs += count;
    }

    line_scratch.clear();
    line_scratch.extend_from_slice(slice);
    for seg in 0..seg_count {
        let s = slot_scratch[seg] as usize;
        let dst_idx = write_seg_offsets[s] * 2;
        write_seg_offsets[s] += 1;
        let src_idx = seg * 2;
        slice[dst_idx] = line_scratch[src_idx];
        slice[dst_idx + 1] = line_scratch[src_idx + 1];
    }

    LineChunkPartition {
        grid: Some(grid),
        slot_to_chunk,
    }
}

#[inline]
pub(crate) fn chunk_is_visible(chunk: &SpatialChunk, frustum: Option<&Frustum>) -> bool {
    match frustum {
        None => true,
        Some(_) if chunk.always_visible => true,
        Some(f) => f.intersects_aabb(chunk.aabb_min, chunk.aabb_max),
    }
}

#[inline]
fn push_coalesced_run(out_runs: &mut Vec<(u32, u32)>, start: u32, end: u32) {
    if end <= start {
        return;
    }
    if let Some(last) = out_runs.last_mut() {
        if last.1 == start {
            last.1 = end;
            return;
        }
    }
    out_runs.push((start, end));
}

pub(crate) fn collect_visible_coalesced_runs(
    chunks: &[SpatialChunk],
    frustum: Option<&Frustum>,
    out_runs: &mut Vec<(u32, u32)>,
) -> (u32, u32) {
    out_runs.clear();
    let mut chunks_drawn = 0u32;
    let mut prims_drawn = 0u32;
    for c in chunks {
        if chunk_is_visible(c, frustum) {
            chunks_drawn += 1;
            prims_drawn += c.prim_count;
            push_coalesced_run(out_runs, c.start, c.end);
        }
    }
    (chunks_drawn, prims_drawn)
}

#[inline]
pub(crate) fn chunk_sort_key(
    chunk: &SpatialChunk,
    vp_type: ViewportType,
    cam_pos: Vec3,
    cam_fwd: Vec3,
) -> (f32, f32) {
    if chunk.always_visible {
        return (f32::MAX, 0.0);
    }
    let center = (chunk.aabb_min + chunk.aabb_max) * 0.5;
    match vp_type {
        ViewportType::Perspective => {
            let closest = cam_pos.clamp(chunk.aabb_min, chunk.aabb_max);
            ((closest - cam_pos).length_squared(), (center - cam_pos).dot(cam_fwd))
        }
        ViewportType::Top => (-chunk.aabb_max.z, -center.z),
        ViewportType::Front => (chunk.aabb_min.y, center.y),
        ViewportType::Side => (chunk.aabb_min.x, center.x),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn collect_visible_opaque_runs(
    chunk_slices: &[&[SpatialChunk]],
    frustum: Option<&Frustum>,
    sort_front_to_back: bool,
    vp_type: ViewportType,
    cam_pos: Vec3,
    cam_fwd: Vec3,
    sort_scratch: &mut Vec<(f32, f32, u32, u32)>,
    out_runs: &mut Vec<(u32, u32)>,
) -> (u32, u32) {
    out_runs.clear();
    let mut chunks_drawn = 0u32;
    let mut prims_drawn = 0u32;

    if !sort_front_to_back {
        for &slice in chunk_slices {
            for c in slice {
                if chunk_is_visible(c, frustum) {
                    chunks_drawn += 1;
                    prims_drawn += c.prim_count;
                    push_coalesced_run(out_runs, c.start, c.end);
                }
            }
        }
        return (chunks_drawn, prims_drawn);
    }

    sort_scratch.clear();
    for &slice in chunk_slices {
        for c in slice {
            if chunk_is_visible(c, frustum) {
                chunks_drawn += 1;
                prims_drawn += c.prim_count;
                let (k0, k1) = chunk_sort_key(c, vp_type, cam_pos, cam_fwd);
                sort_scratch.push((k0, k1, c.start, c.end));
            }
        }
    }

    sort_scratch.sort_by(|a, b| {
        a.0.total_cmp(&b.0)
            .then_with(|| a.1.total_cmp(&b.1))
            .then_with(|| a.2.cmp(&b.2))
    });

    for &(_, _, start, end) in sort_scratch.iter() {
        push_coalesced_run(out_runs, start, end);
    }
    (chunks_drawn, prims_drawn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::Camera;

    fn dummy_solid_vert(pos: Vec3) -> SolidVertex {
        SolidVertex::new(pos.to_array(), [0.0, 0.0, 1.0], [1.0, 1.0, 1.0, 1.0], [0.0, 0.0])
    }

    fn dummy_line_vert(pos: Vec3) -> LineVertex {
        LineVertex::new(pos.to_array(), [1.0, 1.0, 1.0, 1.0])
    }

    #[test]
    fn small_solid_bucket_keeps_single_chunk_and_exact_index_order() {
        let verts: Vec<SolidVertex> = (0..12)
            .map(|i| dummy_solid_vert(Vec3::new(i as f32 * 10.0, 0.0, 0.0)))
            .collect();
        let src: Vec<u32> = vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let mut dst = vec![99, 100, 101];
        let mut chunks = Vec::new();
        let mut slots = Vec::new();

        partition_solid_bucket_into_chunks(&src, &verts, 3, &mut dst, &mut chunks, &mut slots);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 3);
        assert_eq!(chunks[0].end, 15);
        assert_eq!(chunks[0].prim_count, 4);
        assert!(!chunks[0].always_visible);
        assert_eq!(&dst[3..], src.as_slice());
        assert!(chunks[0].aabb_min.x <= -CHUNK_AABB_EPS + 1e-4);
        assert!(chunks[0].aabb_max.x >= 110.0 + CHUNK_AABB_EPS - 1e-4);
    }

    #[test]
    fn large_solid_bucket_is_bijective_and_isolates_oversized_triangles() {
        let mut verts = Vec::new();
        let mut src = Vec::new();
        let mut expected_tris = Vec::new();

        for i in 0..2000usize {
            let x = ((i % 20) as f32 - 10.0) * 128.0;
            let y = (((i / 20) % 10) as f32 - 5.0) * 128.0;
            let z = ((i / 200) as f32 - 5.0) * 128.0;
            let base = verts.len() as u32;
            let span = if i == 42 { 10_000.0 } else { 16.0 };
            verts.push(dummy_solid_vert(Vec3::new(x, y, z)));
            verts.push(dummy_solid_vert(Vec3::new(x + span, y, z)));
            verts.push(dummy_solid_vert(Vec3::new(x, y + span, z)));
            src.extend_from_slice(&[base, base + 1, base + 2]);
            expected_tris.push([base, base + 1, base + 2]);
        }

        let mut dst = Vec::new();
        let mut chunks = Vec::new();
        let mut slots = Vec::new();
        partition_solid_bucket_into_chunks(&src, &verts, 0, &mut dst, &mut chunks, &mut slots);

        assert!(chunks.len() > 8, "expected spatial subdivision into multiple chunks");
        assert_eq!(dst.len(), src.len());

        let total_prims: u32 = chunks.iter().map(|c| c.prim_count).sum();
        assert_eq!(total_prims, 2000);

        let mut seen_tris = Vec::with_capacity(2000);
        for c in &chunks {
            assert!(!c.always_visible);
            assert_eq!(c.end - c.start, c.prim_count * 3);
            for tri in dst[c.start as usize..c.end as usize].chunks_exact(3) {
                let t = [tri[0], tri[1], tri[2]];
                seen_tris.push(t);
                for &v_idx in &t {
                    let p = Vec3::from_array(verts[v_idx as usize].position);
                    assert!(
                        p.x >= c.aabb_min.x && p.x <= c.aabb_max.x
                            && p.y >= c.aabb_min.y && p.y <= c.aabb_max.y
                            && p.z >= c.aabb_min.z && p.z <= c.aabb_max.z,
                        "vertex {p:?} outside chunk AABB {:?}..{:?}",
                        c.aabb_min,
                        c.aabb_max
                    );
                }
            }
        }

        // Oversized triangle 42*3 isolated into last (oversized) chunk without polluting other cells.
        let oversized_idx = (42 * 3) as u32;
        let last_chunk = chunks.last().unwrap();
        assert!(dst[last_chunk.start as usize..last_chunk.end as usize].contains(&oversized_idx));
        let max_local_span = chunks[..chunks.len() - 1]
            .iter()
            .map(|c| (c.aabb_max - c.aabb_min).max_element())
            .fold(0.0f32, f32::max);
        assert!(
            max_local_span < 2500.0,
            "local cell span {max_local_span} was bloated by oversized triangle"
        );

        seen_tris.sort_unstable();
        expected_tris.sort_unstable();
        assert_eq!(seen_tris, expected_tris);
    }

    #[test]
    fn large_line_bucket_is_bijective_and_attributes_counts() {
        let mut lines = Vec::new();
        let mut expected_segs = Vec::new();
        for i in 0..3000usize {
            let x = ((i % 30) as f32 - 15.0) * 100.0;
            let y = ((i / 30) as f32 - 50.0) * 100.0;
            let p0 = Vec3::new(x, y, 0.0);
            let p1 = Vec3::new(x + 20.0, y + 10.0, 15.0);
            lines.push(dummy_line_vert(p0));
            lines.push(dummy_line_vert(p1));
            expected_segs.push((p0.to_array(), p1.to_array()));
        }

        let mut chunks = Vec::new();
        let mut line_scratch = Vec::new();
        let mut slot_scratch = Vec::new();
        let part = partition_world_lines_into_chunks(
            &mut lines,
            100,
            &mut chunks,
            &mut line_scratch,
            &mut slot_scratch,
        );
        assert!(chunks.len() > 8);
        assert_eq!(chunks.first().unwrap().start, 100);
        assert_eq!(chunks.last().unwrap().end, 100 + 6000);

        for c in &chunks {
            for seg in lines[(c.start - 100) as usize..(c.end - 100) as usize].chunks_exact(2) {
                for v in seg {
                    let p = Vec3::from_array(v.position);
                    assert!(
                        p.x >= c.aabb_min.x && p.x <= c.aabb_max.x
                            && p.y >= c.aabb_min.y && p.y <= c.aabb_max.y
                            && p.z >= c.aabb_min.z && p.z <= c.aabb_max.z
                    );
                }
            }
        }

        let target_chunk = part.chunk_for_point(Vec3::new(0.0, 0.0, 0.0), 20.0);
        assert!(target_chunk < chunks.len());

        let mut actual_segs: Vec<_> = lines
            .chunks_exact(2)
            .map(|s| (s[0].position, s[1].position))
            .collect();
        actual_segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then_with(|| a.1.partial_cmp(&b.1).unwrap()));
        expected_segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then_with(|| a.1.partial_cmp(&b.1).unwrap()));
        assert_eq!(actual_segs, expected_segs);
    }

    #[test]
    fn coalescing_and_front_to_back_sorting_across_viewport_types() {
        let c_far = SpatialChunk {
            start: 0,
            end: 30,
            aabb_min: Vec3::new(900.0, -50.0, -50.0),
            aabb_max: Vec3::new(1000.0, 50.0, 50.0),
            prim_count: 10,
            brush_count: 0,
            actor_count: 0,
            always_visible: false,
        };
        let c_mid = SpatialChunk {
            start: 30,
            end: 60,
            aabb_min: Vec3::new(400.0, 200.0, 200.0),
            aabb_max: Vec3::new(500.0, 300.0, 300.0),
            prim_count: 10,
            brush_count: 0,
            actor_count: 0,
            always_visible: false,
        };
        let c_near = SpatialChunk {
            start: 60,
            end: 90,
            aabb_min: Vec3::new(100.0, -50.0, 400.0),
            aabb_max: Vec3::new(200.0, 50.0, 500.0),
            prim_count: 10,
            brush_count: 0,
            actor_count: 0,
            always_visible: false,
        };
        let chunks = [c_far, c_mid, c_near];
        let mut sort_scratch = Vec::new();
        let mut runs = Vec::new();

        // When culling is OFF and F2B is OFF, contiguous chunks merge into 1 single draw run.
        let (drawn, prims) = collect_visible_opaque_runs(
            &[&chunks],
            None,
            false,
            ViewportType::Perspective,
            Vec3::ZERO,
            Vec3::X,
            &mut sort_scratch,
            &mut runs,
        );
        assert_eq!((drawn, prims), (3, 30));
        assert_eq!(runs, vec![(0, 90)]);

        // Perspective from origin looking +X orders c_near (d^2=170k), c_mid (d^2=240k), c_far (d^2=810k).
        collect_visible_opaque_runs(
            &[&chunks],
            None,
            true,
            ViewportType::Perspective,
            Vec3::ZERO,
            Vec3::X,
            &mut sort_scratch,
            &mut runs,
        );
        assert_eq!(runs, vec![(60, 90), (30, 60), (0, 30)]);

        // Top (-Z) orders highest Z first: c_near (max.z=500), c_mid (300), c_far (50).
        collect_visible_opaque_runs(
            &[&chunks],
            None,
            true,
            ViewportType::Top,
            Vec3::ZERO,
            Vec3::NEG_Z,
            &mut sort_scratch,
            &mut runs,
        );
        assert_eq!(runs, vec![(60, 90), (30, 60), (0, 30)]);

        // Side (+X) orders lowest X first: c_near (min.x=100), c_mid (400), c_far (900).
        collect_visible_opaque_runs(
            &[&chunks],
            None,
            true,
            ViewportType::Side,
            Vec3::ZERO,
            Vec3::X,
            &mut sort_scratch,
            &mut runs,
        );
        assert_eq!(runs, vec![(60, 90), (30, 60), (0, 30)]);

        // Front (+Y) orders lowest Y first: c_far & c_near tie at min.y=-50, c_mid at +200.
        collect_visible_opaque_runs(
            &[&chunks],
            None,
            true,
            ViewportType::Front,
            Vec3::ZERO,
            Vec3::Y,
            &mut sort_scratch,
            &mut runs,
        );
        assert_eq!(runs.last().copied().unwrap(), (30, 60));
    }

    #[test]
    fn nonfinite_vertices_produce_always_visible_chunk_and_never_nan_sort() {
        let verts = vec![
            dummy_solid_vert(Vec3::new(f32::NAN, 0.0, 0.0)),
            dummy_solid_vert(Vec3::ZERO),
            dummy_solid_vert(Vec3::ONE),
        ];
        let src = vec![0, 1, 2];
        let mut dst = Vec::new();
        let mut chunks = Vec::new();
        let mut slots = Vec::new();
        partition_solid_bucket_into_chunks(&src, &verts, 0, &mut dst, &mut chunks, &mut slots);
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].always_visible);

        let cam_vp = Camera::default().view_projection_matrix_for_type(ViewportType::Perspective, 1.5);
        let frustum = Frustum::from_view_projection(&cam_vp);
        let mut runs = Vec::new();
        let (drawn, _) = collect_visible_coalesced_runs(&chunks, Some(&frustum), &mut runs);
        assert_eq!(drawn, 1);
        assert_eq!(runs, vec![(0, 3)]);
    }
}
