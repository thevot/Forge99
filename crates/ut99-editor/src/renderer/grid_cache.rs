use crate::{camera::ViewportType, grid::Grid};
use super::vertex::LineVertex;

const MAX_PLANE_BYTES: usize = 1024 * 1024;

/// Three grid planes for the current parameters, retaining at most 3 MiB total.
#[derive(Default)]
pub(super) struct GridCache {
    key: Option<(u32, u32)>,
    planes: [Option<Vec<LineVertex>>; 3],
}
impl GridCache {
    pub fn append(&mut self, grid: &Grid, viewport: ViewportType, output: &mut Vec<LineVertex>) {
        let key = (grid.step.to_bits(), grid.extent.to_bits());
        if self.key != Some(key) {
            self.planes = Default::default();
            self.key = Some(key);
        }
        let index = match viewport {
            ViewportType::Top | ViewportType::Perspective => 0,
            ViewportType::Front => 1,
            ViewportType::Side => 2,
        };
        if let Some(vertices) = &self.planes[index] {
            output.extend_from_slice(vertices);
            return;
        }
        let vertices = grid.generate_vertices_for_viewport(viewport);
        let bytes = vertices.capacity().saturating_mul(std::mem::size_of::<LineVertex>());
        output.extend_from_slice(&vertices);
        // Oversized grids retain the existing generation path without growing the cache.
        if grid.step.is_finite() && grid.step > 0.0 && grid.extent.is_finite()
            && grid.extent >= 0.0 && bytes <= MAX_PLANE_BYTES
        {
            self.planes[index] = Some(vertices);
        }
    }
    pub fn bytes(&self) -> usize {
        self.planes.iter().flatten().map(|vertices| vertices.capacity() * std::mem::size_of::<LineVertex>()).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn assert_matches(cache: &mut GridCache, grid: &Grid, viewport: ViewportType) {
        let prefix = LineVertex::new([1.0, 2.0, 3.0], [0.25; 4]);
        let mut expected = vec![prefix]; expected.extend(grid.generate_vertices_for_viewport(viewport));
        let mut actual = vec![prefix]; cache.append(grid, viewport, &mut actual);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&actual), bytemuck::cast_slice::<LineVertex, u8>(&expected));
    }
    #[test]
    fn vertices_and_axis_colors_match_after_parameter_and_plane_changes() {
        let mut cache = GridCache::default();
        for grid in [Grid::default(), Grid::new(16.0, 512.0), Grid::new(32.0, 128.0), Grid::default(), Grid::new(64.0, 0.0)] {
            for _ in 0..2 {
                for viewport in [ViewportType::Top, ViewportType::Side, ViewportType::Front, ViewportType::Perspective] {
                    assert_matches(&mut cache, &grid, viewport);
                }
            }
            assert!(cache.bytes() <= 3 * MAX_PLANE_BYTES);
        }
    }
    #[test]
    fn top_and_perspective_share_storage_and_oversized_grids_are_not_retained() {
        let mut cache = GridCache::default();
        let grid = Grid::default();
        assert_matches(&mut cache, &grid, ViewportType::Top);
        let pointer = cache.planes[0].as_ref().unwrap().as_ptr();
        let bytes = cache.bytes();
        assert_matches(&mut cache, &grid, ViewportType::Perspective);
        assert_eq!(cache.planes[0].as_ref().unwrap().as_ptr(), pointer);
        assert_eq!(cache.bytes(), bytes);
        assert_matches(&mut cache, &Grid::new(1.0, 8192.0), ViewportType::Top);
        assert_eq!(cache.bytes(), 0);
        assert_matches(&mut cache, &Grid::new(f32::NAN, 128.0), ViewportType::Top);
        assert_eq!(cache.bytes(), 0);
    }
    #[test]
    #[ignore = "manual CPU grid generation benchmark; run with --ignored --nocapture"]
    fn benchmark_three_plane_grid_generation_vs_cache() {
        use std::{hint::black_box, time::Instant};
        for grid in [Grid::default(), Grid::new(8.0, 8192.0)] {
            let planes = [ViewportType::Top, ViewportType::Side, ViewportType::Front];
            let mut cache = GridCache::default(); let mut output = Vec::new();
            for viewport in planes { cache.append(&grid, viewport, &mut output); }
            let mut old_times = Vec::new(); let mut new_times = Vec::new();
            for round in 0..5 {
                for old in if round % 2 == 0 { [true, false] } else { [false, true] } {
                    let start = Instant::now();
                    for _ in 0..1000 {
                        output.clear();
                        for viewport in planes {
                            if old { output.extend(black_box(&grid).generate_vertices_for_viewport(viewport)); }
                            else { cache.append(black_box(&grid), viewport, &mut output); }
                        }
                        black_box(&output);
                    }
                    let us = start.elapsed().as_secs_f64() * 1_000_000.0 / 1000.0;
                    if old { old_times.push(us); } else { new_times.push(us); }
                }
            }
            old_times.sort_by(f64::total_cmp); new_times.sort_by(f64::total_cmp);
            eprintln!("step={}, extent={}: three-plane generation={:.3} us, cached={:.3} us, retained={} bytes",
                grid.step, grid.extent, old_times[2], new_times[2], cache.bytes());
        }
    }
}
