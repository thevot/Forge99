use glam::Vec3;

/// Frame-local lazy counts: polygons of the same brush share an overlap result.
/// Keeps the legacy inclusive AABB test, including touching faces and non-finite data.
pub(super) struct FrameOverlapCounts<'a> {
    bounds: &'a [(Vec3, Vec3)],
    counts: Vec<Option<usize>>,
}
impl<'a> FrameOverlapCounts<'a> {
    pub fn new(bounds: &'a [(Vec3, Vec3)]) -> Self {
        Self { bounds, counts: vec![None; bounds.len()] }
    }
    pub fn count(&mut self, index: usize) -> usize {
        if let Some(count) = self.counts[index] { return count; }
        let (min, max) = self.bounds[index];
        let count = self.bounds.iter().enumerate().filter(|&(other, &(other_min, other_max))| {
            other != index
                && min.x <= other_max.x && max.x >= other_min.x
                && min.y <= other_max.y && max.y >= other_min.y
                && min.z <= other_max.z && max.z >= other_min.z
        }).count();
        self.counts[index] = Some(count);
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn legacy(bounds: &[(Vec3, Vec3)], index: usize) -> usize {
        let (my_min, my_max) = bounds[index];
        bounds.iter().enumerate().filter(|&(other_i, &(o_min, o_max))| {
            other_i != index && my_min.x <= o_max.x && my_max.x >= o_min.x
                && my_min.y <= o_max.y && my_max.y >= o_min.y
                && my_min.z <= o_max.z && my_max.z >= o_min.z
        }).count()
    }
    #[test]
    fn matches_legacy_for_touching_degenerate_and_non_finite_bounds() {
        let bounds = vec![
            (Vec3::ZERO, Vec3::ONE), (Vec3::ONE, Vec3::splat(2.0)),
            (Vec3::ZERO, Vec3::ZERO), (Vec3::splat(-5.0), Vec3::splat(-3.0)),
            (Vec3::splat(f32::NEG_INFINITY), Vec3::splat(f32::INFINITY)),
            (Vec3::splat(f32::NAN), Vec3::ONE), (Vec3::ONE, Vec3::ZERO),
        ];
        let mut cache = FrameOverlapCounts::new(&bounds);
        for index in 0..bounds.len() {
            assert_eq!(cache.count(index), legacy(&bounds, index));
            assert_eq!(cache.count(index), legacy(&bounds, index));
        }
    }
    #[test]
    fn counts_are_lazy_and_new_frame_uses_changed_bounds() {
        let mut bounds = vec![(Vec3::ZERO, Vec3::ONE), (Vec3::ZERO, Vec3::ONE)];
        {
            let mut cache = FrameOverlapCounts::new(&bounds);
            for _ in 0..24 { assert_eq!(cache.count(0), 1); }
            assert_eq!(cache.counts, [Some(1), None]);
        }
        bounds[1] = (Vec3::splat(10.0), Vec3::splat(11.0));
        assert_eq!(FrameOverlapCounts::new(&bounds).count(0), 0);
        assert!(FrameOverlapCounts::new(&[]).counts.is_empty());
    }
    fn fixture(count: usize) -> Vec<(Vec3, Vec3)> {
        (0..count).map(|i| {
            let min = Vec3::new((i % 31) as f32 * 16.0, ((i / 31) % 31) as f32 * 16.0, (i % 7) as f32 * 8.0);
            (min, min + Vec3::splat(96.0))
        }).collect()
    }
    #[test]
    fn matches_legacy_across_large_scene_and_subset_queries() {
        let bounds = fixture(500);
        let mut cache = FrameOverlapCounts::new(&bounds);
        for index in (0..500).rev().step_by(3) { assert_eq!(cache.count(index), legacy(&bounds, index)); }
        for index in 0..500 { assert_eq!(cache.count(index), legacy(&bounds, index)); }
    }
    #[test]
    #[ignore = "manual CPU overlap benchmark; run with --ignored --nocapture"]
    fn benchmark_per_polygon_vs_per_brush_overlap_counts() {
        use std::{hint::black_box, time::Instant};
        for count in [500, 1500] {
            let bounds = fixture(count);
            for polygons in [6, 24] {
                let mut old_times = Vec::new(); let mut new_times = Vec::new();
                for round in 0..5 {
                    for old in if round % 2 == 0 { [true, false] } else { [false, true] } {
                        let start = Instant::now();
                        let mut checksum = 0usize;
                        let mut cache = if old { None } else { Some(FrameOverlapCounts::new(&bounds)) };
                        for index in 0..count {
                            for _ in 0..polygons {
                                checksum += if old { legacy(black_box(&bounds), index) }
                                    else { cache.as_mut().unwrap().count(black_box(index)) };
                            }
                        }
                        black_box(checksum);
                        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                        if old { old_times.push(elapsed); } else { new_times.push(elapsed); }
                    }
                }
                old_times.sort_by(f64::total_cmp); new_times.sort_by(f64::total_cmp);
                eprintln!("brushes={count}, polygons/brush={polygons}: legacy={:.3} ms, cached={:.3} ms", old_times[2], new_times[2]);
            }
        }
    }
}
