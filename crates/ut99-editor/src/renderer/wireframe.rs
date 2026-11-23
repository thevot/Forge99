use glam::{EulerRot, Mat4, Vec3};
use ut99_bsp::{Brush, Plane};
use super::vertex::LineVertex;

/// Reuses only transformed positions for world-brush outlines. Material, vertex
/// color and lightmap data stay borrowed in the source brush.
#[derive(Default)]
pub(super) struct WireframeScratch {
    vertices: Vec<Vec3>,
}

impl WireframeScratch {
    pub(super) fn append(
        &mut self,
        brush: &Brush,
        output: &mut Vec<LineVertex>,
        mut color_for_polygon: impl FnMut(usize) -> [f32; 4],
    ) {
        let rotation = Mat4::from_euler(
            EulerRot::YXZ, brush.rotation.y.to_radians(),
            brush.rotation.x.to_radians(), brush.rotation.z.to_radians(),
        );
        let model = Mat4::from_translation(brush.location) * rotation * Mat4::from_scale(brush.scale);
        let mut world_polygon_index = 0;
        for polygon in &brush.polys {
            self.vertices.clear();
            self.vertices.extend(polygon.vertices.iter().map(|&vertex| model.transform_point3(vertex)));
            // Match Polygon::new/to_world_polys, including collapsed transforms.
            if self.vertices.len() < 3
                || Plane::from_points(self.vertices[0], self.vertices[1], self.vertices[2]).is_none() {
                continue;
            }
            // The former path enumerates polygons after filtering invalid ones.
            let color = color_for_polygon(world_polygon_index);
            world_polygon_index += 1;
            for index in 0..self.vertices.len() {
                let next = (index + 1) % self.vertices.len();
                output.push(LineVertex::new(self.vertices[index].to_array(), color));
                output.push(LineVertex::new(self.vertices[next].to_array(), color));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ut99_bsp::{build_cube, CsgOperation, LightmapData};

    fn color(index: usize) -> [f32; 4] {
        if index == 1 { [1.0, 0.75, 0.1, 1.0] } else { [0.2, 0.4, 0.8, 1.0] }
    }
    fn legacy(brush: &Brush, output: &mut Vec<LineVertex>) {
        for (index, polygon) in brush.to_world_polys().iter().enumerate() {
            for vertex in 0..polygon.vertices.len() {
                output.push(LineVertex::new(polygon.vertices[vertex].to_array(), color(index)));
                output.push(LineVertex::new(polygon.vertices[(vertex + 1) % polygon.vertices.len()].to_array(), color(index)));
            }
        }
    }
    fn assert_equivalent(brush: &Brush) {
        let mut expected = Vec::new(); legacy(brush, &mut expected);
        let mut actual = Vec::new(); WireframeScratch::default().append(brush, &mut actual, color);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&expected), bytemuck::cast_slice::<LineVertex, u8>(&actual));
    }
    fn cube() -> Brush { build_cube("Wireframe", CsgOperation::Add, Vec3::splat(256.0)) }

    #[test]
    fn transforms_preserve_exact_segments_and_selection_colors() {
        for rotation in [Vec3::ZERO, Vec3::new(17.0, 53.0, -31.0), Vec3::new(90.0, 180.0, 270.0)] {
            for scale in [Vec3::ONE, Vec3::new(0.5, 2.0, 3.0), Vec3::new(-1.0, 2.0, 0.5), -Vec3::ONE,
                Vec3::new(0.0, 1.0, 1.0), Vec3::new(1.0, 0.0, 1.0), Vec3::new(1.0, 1.0, 0.0), Vec3::ZERO] {
                let mut brush = cube(); brush.location = Vec3::new(123.0, -456.0, 789.0);
                brush.rotation = rotation; brush.scale = scale; assert_equivalent(&brush);
            }
        }
    }
    #[test]
    fn invalid_polygons_keep_legacy_filtering_and_polygon_indices() {
        let mut brush = cube();
        brush.polys[0].vertices.clear();
        brush.polys[2].vertices = vec![Vec3::ZERO, Vec3::X];
        brush.polys[3].vertices = vec![Vec3::ZERO, Vec3::X, Vec3::X * 2.0];
        brush.polys[4].vertices[0] = Vec3::splat(f32::NAN);
        assert_equivalent(&brush);
        brush.polys.clear(); assert_equivalent(&brush);
    }
    #[test]
    fn lighting_and_materials_do_not_change_outlines_or_source_data() {
        let mut brush = cube();
        for polygon in &mut brush.polys {
            polygon.texture = "Audit.Walls.Panel".into(); polygon.flags = ut99_bsp::PF_UNLIT;
            polygon.vertex_colors = vec![[0.3, 0.5, 0.8]; polygon.vertices.len()];
            polygon.lightmap = Some(LightmapData { width: 32, height: 32, luxels: vec![[0.7; 3]; 1024] });
        }
        let before = brush.clone(); assert_equivalent(&brush); assert_eq!(brush, before);
    }
    #[test]
    fn scratch_is_reused_and_existing_output_is_preserved() {
        let brush = cube(); let mut scratch = WireframeScratch::default();
        let prefix = LineVertex::new([9.0, 8.0, 7.0], [1.0; 4]);
        let mut output = vec![prefix]; scratch.append(&brush, &mut output, color);
        let capacity = scratch.vertices.capacity(); let first = bytemuck::cast_slice::<LineVertex, u8>(&output).to_vec();
        output.clear(); output.push(prefix); scratch.append(&brush, &mut output, color);
        assert_eq!(scratch.vertices.capacity(), capacity);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&output), first.as_slice());
        let mut combined = vec![prefix]; legacy(&brush, &mut combined); legacy(&brush, &mut combined);
        scratch.append(&brush, &mut output, color);
        assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&output), bytemuck::cast_slice::<LineVertex, u8>(&combined));
    }

    #[test]
    #[ignore = "manual CPU benchmark; run with --ignored --nocapture"]
    fn benchmark_world_wireframe() {
        use std::{hint::black_box, time::Instant};
        for count in [250, 1000] {
            for lit in [false, true] {
                let brushes: Vec<_> = (0..count).map(|index| {
                    let mut brush = cube();
                    brush.location = Vec3::new((index % 25) as f32 * 512.0, (index / 25) as f32 * 512.0, 0.0);
                    brush.rotation = Vec3::new(10.0, (index % 33) as f32, 5.0); brush.scale = Vec3::new(1.0, 0.75, 1.25);
                    for polygon in &mut brush.polys {
                        polygon.texture = "Audit.Walls.Panel".into(); polygon.vertex_colors = vec![[0.7; 3]; polygon.vertices.len()];
                        if lit { polygon.lightmap = Some(LightmapData { width: 32, height: 32, luxels: vec![[0.7; 3]; 1024] }); }
                    }
                    brush
                }).collect();
                let mut expected = Vec::new(); for brush in &brushes { legacy(brush, &mut expected); }
                let mut output = Vec::new(); let mut scratch = WireframeScratch::default();
                for brush in &brushes { scratch.append(brush, &mut output, color); }
                assert_eq!(bytemuck::cast_slice::<LineVertex, u8>(&expected), bytemuck::cast_slice::<LineVertex, u8>(&output));
                let mut old_times = Vec::new(); let mut new_times = Vec::new();
                for round in 0..5 {
                    for old in if round % 2 == 0 { [true, false] } else { [false, true] } {
                        let start = Instant::now();
                        for _ in 0..20 {
                            output.clear();
                            for brush in black_box(&brushes) {
                                if old { legacy(brush, &mut output); }
                                else { scratch.append(brush, &mut output, color); }
                            }
                            black_box(&output);
                        }
                        let millis = start.elapsed().as_secs_f64() * 1000.0 / 20.0;
                        if old { old_times.push(millis); } else { new_times.push(millis); }
                    }
                }
                old_times.sort_by(f64::total_cmp); new_times.sort_by(f64::total_cmp);
                println!("brushes={count}, lightmaps={lit}, old={:.3}ms, new={:.3}ms, speedup={:.2}x", old_times[2], new_times[2], old_times[2] / new_times[2]);
            }
        }
    }
}
