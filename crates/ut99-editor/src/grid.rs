use super::renderer::vertex::LineVertex;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Grid {
    pub step: f32,
    pub extent: f32,
}

impl Default for Grid {
    fn default() -> Self {
        Self {
            step: 64.0,
            extent: 2048.0,
        }
    }
}

#[allow(dead_code)]
impl Grid {
    pub fn new(step: f32, extent: f32) -> Self {
        Self { step, extent }
    }

    pub fn generate_vertices(&self) -> Vec<LineVertex> {
        self.generate_vertices_for_viewport(crate::camera::ViewportType::Perspective)
    }

    pub fn generate_vertices_for_viewport(
        &self,
        viewport_type: crate::camera::ViewportType,
    ) -> Vec<LineVertex> {
        let mut vertices = Vec::new();
        let num_lines = (self.extent / self.step) as i32;

        let col_minor = [0.25, 0.25, 0.28, 1.0];
        let col_major = [0.45, 0.45, 0.50, 1.0];
        let col_x_axis = [0.85, 0.20, 0.20, 1.0]; // Red (X)
        let col_y_axis = [0.20, 0.85, 0.20, 1.0]; // Green (Y)
        let col_z_axis = [0.20, 0.40, 0.95, 1.0]; // Blue (Z)

        match viewport_type {
            crate::camera::ViewportType::Top | crate::camera::ViewportType::Perspective => {
                for i in -num_lines..=num_lines {
                    let pos = i as f32 * self.step;
                    let is_major = (i % 8) == 0;

                    // X-parallel lines on XY plane (y = pos, z = 0)
                    let color_x = if i == 0 {
                        col_x_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([-self.extent, pos, 0.0], color_x));
                    vertices.push(LineVertex::new([self.extent, pos, 0.0], color_x));

                    // Y-parallel lines on XY plane (x = pos, z = 0)
                    let color_y = if i == 0 {
                        col_y_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([pos, -self.extent, 0.0], color_y));
                    vertices.push(LineVertex::new([pos, self.extent, 0.0], color_y));
                }

                // Z axis indicator at origin
                vertices.push(LineVertex::new([0.0, 0.0, 0.0], col_z_axis));
                vertices.push(LineVertex::new([0.0, 0.0, 128.0], col_z_axis));
            }
            crate::camera::ViewportType::Front => {
                // XZ plane (y = 0)
                for i in -num_lines..=num_lines {
                    let pos = i as f32 * self.step;
                    let is_major = (i % 8) == 0;

                    // X-parallel horizontal lines (z = pos, y = 0)
                    let color_x = if i == 0 {
                        col_x_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([-self.extent, 0.0, pos], color_x));
                    vertices.push(LineVertex::new([self.extent, 0.0, pos], color_x));

                    // Z-parallel vertical lines (x = pos, y = 0)
                    let color_z = if i == 0 {
                        col_z_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([pos, 0.0, -self.extent], color_z));
                    vertices.push(LineVertex::new([pos, 0.0, self.extent], color_z));
                }
            }
            crate::camera::ViewportType::Side => {
                // YZ plane (x = 0)
                for i in -num_lines..=num_lines {
                    let pos = i as f32 * self.step;
                    let is_major = (i % 8) == 0;

                    // Y-parallel horizontal lines (z = pos, x = 0)
                    let color_y = if i == 0 {
                        col_y_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([0.0, -self.extent, pos], color_y));
                    vertices.push(LineVertex::new([0.0, self.extent, pos], color_y));

                    // Z-parallel vertical lines (y = pos, x = 0)
                    let color_z = if i == 0 {
                        col_z_axis
                    } else if is_major {
                        col_major
                    } else {
                        col_minor
                    };
                    vertices.push(LineVertex::new([0.0, pos, -self.extent], color_z));
                    vertices.push(LineVertex::new([0.0, pos, self.extent], color_z));
                }
            }
        }

        vertices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::camera::ViewportType;

    #[test]
    fn test_grid_generate_vertices_for_viewport_orientations() {
        let grid = Grid::new(64.0, 256.0);

        let top_verts = grid.generate_vertices_for_viewport(ViewportType::Top);
        assert!(!top_verts.is_empty());
        // All XY grid lines (except the last 2 vertices for Z axis indicator) lie on Z == 0.0
        for v in &top_verts[..top_verts.len() - 2] {
            assert_eq!(v.position[2], 0.0, "Top viewport grid must lie on XY plane (Z=0)");
        }

        let front_verts = grid.generate_vertices_for_viewport(ViewportType::Front);
        assert!(!front_verts.is_empty());
        for v in &front_verts {
            assert_eq!(v.position[1], 0.0, "Front viewport grid must lie on XZ plane (Y=0)");
        }
        assert!(
            front_verts.iter().any(|v| v.position[2].abs() > 0.0),
            "Front viewport grid must span Z axis"
        );

        let side_verts = grid.generate_vertices_for_viewport(ViewportType::Side);
        assert!(!side_verts.is_empty());
        for v in &side_verts {
            assert_eq!(v.position[0], 0.0, "Side viewport grid must lie on YZ plane (X=0)");
        }
        assert!(
            side_verts.iter().any(|v| v.position[2].abs() > 0.0),
            "Side viewport grid must span Z axis"
        );
    }
}

