use glam::Vec3;

#[derive(Debug, Clone, Copy)]
pub(crate) struct BvhEntry {
    pub pos: u32,
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Copy, Default)]
struct BvhNode {
    min: Vec3,
    max: Vec3,
    min_pos: u32,
    max_pos: u32,
    first_or_left: u32,
    count: u32,
}

/// Flat AABB BVH with per-node source index ranges `[min_pos, max_pos]` for zero-allocation queries.
pub(crate) struct AabbBvh {
    nodes: Vec<BvhNode>,
    indices: Vec<u32>,
}

const LEAF_MAX: usize = 6;
const MAX_SPATIAL_DEPTH: usize = 24;

#[inline]
fn aabb_overlaps(a_min: Vec3, a_max: Vec3, b_min: Vec3, b_max: Vec3, pad: f32) -> bool {
    !(a_max.x < b_min.x - pad
        || a_min.x > b_max.x + pad
        || a_max.y < b_min.y - pad
        || a_min.y > b_max.y + pad
        || a_max.z < b_min.z - pad
        || a_min.z > b_max.z + pad)
}

#[inline]
fn point_in_aabb(pt: Vec3, b_min: Vec3, b_max: Vec3, pad: f32) -> bool {
    !(pt.x < b_min.x - pad
        || pt.x > b_max.x + pad
        || pt.y < b_min.y - pad
        || pt.y > b_max.y + pad
        || pt.z < b_min.z - pad
        || pt.z > b_max.z + pad)
}

#[inline]
fn segment_intersects_aabb(
    from: Vec3,
    dir: Vec3,
    dist: f32,
    b_min: Vec3,
    b_max: Vec3,
    pad: f32,
) -> bool {
    let mut t_min = 0.0f32;
    let mut t_max = dist;
    for axis in 0..3 {
        let lo = b_min[axis] - pad;
        let hi = b_max[axis] + pad;
        let o = from[axis];
        let d = dir[axis];
        if d.abs() < 1e-12 {
            if o < lo || o > hi {
                return false;
            }
        } else {
            let inv = 1.0 / d;
            let mut t0 = (lo - o) * inv;
            let mut t1 = (hi - o) * inv;
            if t0 > t1 {
                std::mem::swap(&mut t0, &mut t1);
            }
            t_min = t_min.max(t0);
            t_max = t_max.min(t1);
            if t_max < t_min {
                return false;
            }
        }
    }
    true
}

impl AabbBvh {
    pub(crate) fn build(entries: &[BvhEntry]) -> Self {
        if entries.is_empty() {
            return Self {
                nodes: Vec::new(),
                indices: Vec::new(),
            };
        }
        let mut work = entries.to_vec();
        let mut nodes = Vec::with_capacity(entries.len().max(1) * 2);
        let mut indices = Vec::with_capacity(entries.len());
        nodes.push(BvhNode::default());
        Self::build_node(0, &mut work, 0, &mut nodes, &mut indices);
        Self { nodes, indices }
    }

    fn build_node(
        node_idx: usize,
        slice: &mut [BvhEntry],
        depth: usize,
        nodes: &mut Vec<BvhNode>,
        indices: &mut Vec<u32>,
    ) {
        let mut b_min = Vec3::splat(f32::MAX);
        let mut b_max = Vec3::splat(f32::MIN);
        let mut c_min = Vec3::splat(f32::MAX);
        let mut c_max = Vec3::splat(f32::MIN);
        let mut min_pos = u32::MAX;
        let mut max_pos = 0u32;

        for e in slice.iter() {
            b_min = b_min.min(e.min);
            b_max = b_max.max(e.max);
            let c = (e.min + e.max) * 0.5;
            c_min = c_min.min(c);
            c_max = c_max.max(c);
            min_pos = min_pos.min(e.pos);
            max_pos = max_pos.max(e.pos);
        }

        if slice.len() <= LEAF_MAX {
            // Descending source order allows early break in highest-index point queries.
            slice.sort_unstable_by(|a, b| b.pos.cmp(&a.pos));
            let first = indices.len() as u32;
            indices.extend(slice.iter().map(|e| e.pos));
            nodes[node_idx] = BvhNode {
                min: b_min,
                max: b_max,
                min_pos,
                max_pos,
                first_or_left: first,
                count: slice.len() as u32,
            };
            return;
        }

        let extent = c_max - c_min;
        let axis = if extent.x >= extent.y && extent.x >= extent.z {
            0
        } else if extent.y >= extent.z {
            1
        } else {
            2
        };

        let split = if depth >= MAX_SPATIAL_DEPTH {
            let mid = slice.len() / 2;
            slice.select_nth_unstable_by(mid, |a, b| {
                let ca = 0.5 * (a.min[axis] + a.max[axis]);
                let cb = 0.5 * (b.min[axis] + b.max[axis]);
                ca.total_cmp(&cb)
            });
            mid
        } else {
            let mid_coord = 0.5 * (c_min[axis] + c_max[axis]);
            let mut i = 0;
            let mut j = slice.len();
            while i < j {
                let c = 0.5 * (slice[i].min[axis] + slice[i].max[axis]);
                if c <= mid_coord {
                    i += 1;
                } else {
                    j -= 1;
                    slice.swap(i, j);
                }
            }
            if i == 0 || i == slice.len() {
                let mid = slice.len() / 2;
                slice.select_nth_unstable_by(mid, |a, b| {
                    let ca = 0.5 * (a.min[axis] + a.max[axis]);
                    let cb = 0.5 * (b.min[axis] + b.max[axis]);
                    ca.total_cmp(&cb)
                });
                mid
            } else {
                i
            }
        };

        let left_idx = nodes.len() as u32;
        nodes.push(BvhNode::default());
        nodes.push(BvhNode::default());
        nodes[node_idx] = BvhNode {
            min: b_min,
            max: b_max,
            min_pos,
            max_pos,
            first_or_left: left_idx,
            count: 0,
        };

        let (left_slice, right_slice) = slice.split_at_mut(split);
        Self::build_node(left_idx as usize, left_slice, depth + 1, nodes, indices);
        Self::build_node((left_idx + 1) as usize, right_slice, depth + 1, nodes, indices);
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub(crate) fn query_aabb_into(
        &self,
        q_min: Vec3,
        q_max: Vec3,
        pad: f32,
        max_pos_exclusive: u32,
        out: &mut Vec<u32>,
    ) {
        if self.nodes.is_empty() || max_pos_exclusive == 0 {
            return;
        }
        let root = &self.nodes[0];
        if root.min_pos >= max_pos_exclusive || !aabb_overlaps(root.min, root.max, q_min, q_max, pad) {
            return;
        }

        let mut stack = [0u32; 64];
        let mut sp = 1usize;
        stack[0] = 0;

        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if node.count > 0 {
                let start = node.first_or_left as usize;
                let end = start + node.count as usize;
                for &pos in &self.indices[start..end] {
                    if pos < max_pos_exclusive {
                        out.push(pos);
                    }
                }
            } else {
                let left_idx = node.first_or_left;
                let right_idx = left_idx + 1;
                let left = &self.nodes[left_idx as usize];
                if left.min_pos < max_pos_exclusive
                    && aabb_overlaps(left.min, left.max, q_min, q_max, pad)
                {
                    stack[sp] = left_idx;
                    sp += 1;
                }
                let right = &self.nodes[right_idx as usize];
                if right.min_pos < max_pos_exclusive
                    && aabb_overlaps(right.min, right.max, q_min, q_max, pad)
                {
                    stack[sp] = right_idx;
                    sp += 1;
                }
            }
        }
    }

    pub(crate) fn find_highest_pos_containing_point<F>(
        &self,
        pt: Vec3,
        pad: f32,
        max_pos_exclusive: u32,
        mut pred: F,
    ) -> Option<u32>
    where
        F: FnMut(u32) -> bool,
    {
        if self.nodes.is_empty() || max_pos_exclusive == 0 {
            return None;
        }
        let root = &self.nodes[0];
        if root.min_pos >= max_pos_exclusive || !point_in_aabb(pt, root.min, root.max, pad) {
            return None;
        }

        let mut best: Option<u32> = None;
        let mut stack = [0u32; 64];
        let mut sp = 1usize;
        stack[0] = 0;

        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if best.is_some_and(|bp| node.max_pos <= bp) {
                continue;
            }
            if node.count > 0 {
                let start = node.first_or_left as usize;
                let end = start + node.count as usize;
                for &pos in &self.indices[start..end] {
                    if pos >= max_pos_exclusive {
                        continue;
                    }
                    if best.is_some_and(|bp| pos <= bp) {
                        break;
                    }
                    if pred(pos) {
                        best = Some(pos);
                        break;
                    }
                }
            } else {
                let left_idx = node.first_or_left;
                let right_idx = left_idx + 1;
                let left = &self.nodes[left_idx as usize];
                let right = &self.nodes[right_idx as usize];
                let l_ok = left.min_pos < max_pos_exclusive
                    && best.map_or(true, |bp| left.max_pos > bp)
                    && point_in_aabb(pt, left.min, left.max, pad);
                let r_ok = right.min_pos < max_pos_exclusive
                    && best.map_or(true, |bp| right.max_pos > bp)
                    && point_in_aabb(pt, right.min, right.max, pad);
                match (l_ok, r_ok) {
                    (true, true) => {
                        if left.max_pos >= right.max_pos {
                            stack[sp] = right_idx;
                            stack[sp + 1] = left_idx;
                        } else {
                            stack[sp] = left_idx;
                            stack[sp + 1] = right_idx;
                        }
                        sp += 2;
                    }
                    (true, false) => {
                        stack[sp] = left_idx;
                        sp += 1;
                    }
                    (false, true) => {
                        stack[sp] = right_idx;
                        sp += 1;
                    }
                    (false, false) => {}
                }
            }
        }

        best
    }

    pub(crate) fn any_pos_above_containing_point<F>(
        &self,
        pt: Vec3,
        pad: f32,
        min_pos_exclusive: u32,
        mut pred: F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if self.nodes.is_empty() {
            return false;
        }
        let root = &self.nodes[0];
        if root.max_pos <= min_pos_exclusive || !point_in_aabb(pt, root.min, root.max, pad) {
            return false;
        }

        let mut stack = [0u32; 64];
        let mut sp = 1usize;
        stack[0] = 0;

        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if node.count > 0 {
                let start = node.first_or_left as usize;
                let end = start + node.count as usize;
                for &pos in &self.indices[start..end] {
                    if pos <= min_pos_exclusive {
                        break;
                    }
                    if pred(pos) {
                        return true;
                    }
                }
            } else {
                let left_idx = node.first_or_left;
                let right_idx = left_idx + 1;
                let left = &self.nodes[left_idx as usize];
                if left.max_pos > min_pos_exclusive && point_in_aabb(pt, left.min, left.max, pad) {
                    stack[sp] = left_idx;
                    sp += 1;
                }
                let right = &self.nodes[right_idx as usize];
                if right.max_pos > min_pos_exclusive && point_in_aabb(pt, right.min, right.max, pad)
                {
                    stack[sp] = right_idx;
                    sp += 1;
                }
            }
        }

        false
    }

    pub(crate) fn any_segment_hit<F>(
        &self,
        from: Vec3,
        dir: Vec3,
        dist: f32,
        pad: f32,
        mut pred: F,
    ) -> bool
    where
        F: FnMut(u32) -> bool,
    {
        if self.nodes.is_empty() {
            return false;
        }
        let root = &self.nodes[0];
        if !segment_intersects_aabb(from, dir, dist, root.min, root.max, pad) {
            return false;
        }

        let mut stack = [0u32; 64];
        let mut sp = 1usize;
        stack[0] = 0;

        while sp > 0 {
            sp -= 1;
            let node = &self.nodes[stack[sp] as usize];
            if node.count > 0 {
                let start = node.first_or_left as usize;
                let end = start + node.count as usize;
                for &pos in &self.indices[start..end] {
                    if pred(pos) {
                        return true;
                    }
                }
            } else {
                let left_idx = node.first_or_left;
                let right_idx = left_idx + 1;
                let left = &self.nodes[left_idx as usize];
                if segment_intersects_aabb(from, dir, dist, left.min, left.max, pad) {
                    stack[sp] = left_idx;
                    sp += 1;
                }
                let right = &self.nodes[right_idx as usize];
                if segment_intersects_aabb(from, dir, dist, right.min, right.max, pad) {
                    stack[sp] = right_idx;
                    sp += 1;
                }
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<BvhEntry> {
        let mut out = Vec::new();
        for i in 0..80u32 {
            let gx = (i % 10) as f32 * 100.0;
            let gy = ((i / 10) % 4) as f32 * 100.0;
            let gz = if i >= 40 { 25.0 } else { 0.0 };
            let min = Vec3::new(gx - 30.0, gy - 30.0, gz - 30.0);
            let max = Vec3::new(gx + 30.0, gy + 30.0, gz + 30.0);
            out.push(BvhEntry { pos: i * 2 + 1, min, max });
        }
        // Coincident entries at origin to test degenerate spatial split
        for k in 0..16u32 {
            out.push(BvhEntry {
                pos: 500 + k * 3,
                min: Vec3::splat(-40.0),
                max: Vec3::splat(40.0),
            });
        }
        out
    }

    #[test]
    fn query_aabb_and_point_queries_match_linear_scan() {
        let entries = sample_entries();
        let bvh = AabbBvh::build(&entries);

        let q_min = Vec3::new(-10.0, -10.0, -10.0);
        let q_max = Vec3::new(220.0, 120.0, 40.0);
        for &limit in &[1u32, 10, 50, 520, u32::MAX] {
            let mut got = Vec::new();
            bvh.query_aabb_into(q_min, q_max, 0.5, limit, &mut got);
            got.sort_unstable();
            let mut expected: Vec<u32> = entries
                .iter()
                .filter(|e| e.pos < limit && aabb_overlaps(e.min, e.max, q_min, q_max, 0.5))
                .map(|e| e.pos)
                .collect();
            expected.sort_unstable();
            assert_eq!(got, expected, "query_aabb mismatch for limit={limit}");
        }

        let probe_pts = [
            Vec3::ZERO,
            Vec3::new(200.0, 100.0, 10.0),
            Vec3::new(900.0, 300.0, 25.0),
            Vec3::new(9999.0, 9999.0, 9999.0),
        ];
        for &pt in &probe_pts {
            for &limit in &[1u32, 40, 520, u32::MAX] {
                let got = bvh.find_highest_pos_containing_point(pt, 0.05, limit, |pos| {
                    let e = entries.iter().find(|x| x.pos == pos).unwrap();
                    point_in_aabb(pt, e.min, e.max, 0.04) && pos % 2 == 1
                });
                let expected = entries
                    .iter()
                    .filter(|e| e.pos < limit && point_in_aabb(pt, e.min, e.max, 0.04) && e.pos % 2 == 1)
                    .map(|e| e.pos)
                    .max();
                assert_eq!(got, expected, "highest query mismatch at {pt:?} limit={limit}");
            }

            for &floor_pos in &[0u32, 20, 500, 540, 600] {
                let got = bvh.any_pos_above_containing_point(pt, 0.05, floor_pos, |pos| {
                    let e = entries.iter().find(|x| x.pos == pos).unwrap();
                    point_in_aabb(pt, e.min, e.max, 0.04) && pos % 3 == 0
                });
                let expected = entries.iter().any(|e| {
                    e.pos > floor_pos && point_in_aabb(pt, e.min, e.max, 0.04) && e.pos % 3 == 0
                });
                assert_eq!(got, expected, "any_above mismatch at {pt:?} floor={floor_pos}");
            }
        }
    }

    #[test]
    fn segment_queries_including_axis_aligned_rays_match_linear_scan() {
        let entries = sample_entries();
        let bvh = AabbBvh::build(&entries);

        let segments = [
            (Vec3::new(0.0, 0.0, -100.0), Vec3::new(0.0, 0.0, 200.0)),
            (Vec3::new(-50.0, 0.0, 0.0), Vec3::new(500.0, 0.0, 0.0)),
            (Vec3::new(-100.0, -100.0, -50.0), Vec3::new(950.0, 350.0, 60.0)),
            (Vec3::new(2000.0, 2000.0, 0.0), Vec3::new(2000.0, 2500.0, 0.0)),
        ];

        for (from, to) in segments {
            let delta = to - from;
            let dist = delta.length();
            let dir = delta / dist;
            let mut hit_positions = Vec::new();
            let hit = bvh.any_segment_hit(from, dir, dist, 0.01, |pos| {
                hit_positions.push(pos);
                pos == 509
            });
            let expected = entries.iter().any(|e| {
                segment_intersects_aabb(from, dir, dist, e.min, e.max, 0.01) && e.pos == 509
            });
            assert_eq!(hit, expected, "segment hit mismatch for {from:?} -> {to:?}");
        }
    }
}
