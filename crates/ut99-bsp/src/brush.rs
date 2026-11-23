use super::poly::Polygon;
use super::CsgOperation;
use glam::Vec3;
use std::f32::consts::PI;

#[derive(Debug, Clone, PartialEq)]
pub struct Brush {
    pub name: String,
    pub csg_oper: CsgOperation,
    pub location: Vec3,
    pub rotation: Vec3, // Pitch, Yaw, Roll in degrees
    pub scale: Vec3,
    pub polys: Vec<Polygon>,
    pub group: Option<String>,
}

impl Brush {
    pub fn new(name: impl Into<String>, csg_oper: CsgOperation) -> Self {
        Self {
            name: name.into(),
            csg_oper,
            location: Vec3::ZERO,
            rotation: Vec3::ZERO,
            scale: Vec3::ONE,
            polys: Vec::new(),
            group: None,
        }
    }

    /// Returns the affine local-to-world transform matrix for this brush.
    #[inline]
    pub fn model_matrix(&self) -> glam::Mat4 {
        let rad_pitch = self.rotation.x.to_radians();
        let rad_yaw = self.rotation.y.to_radians();
        let rad_roll = self.rotation.z.to_radians();
        let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
        let scale_matrix = glam::Mat4::from_scale(self.scale);
        let trans_matrix = glam::Mat4::from_translation(self.location);
        trans_matrix * rot_matrix * scale_matrix
    }

    /// Transforms all vertices of this brush into world space using its location, rotation, and scale.
    pub fn to_world_polys(&self) -> Vec<Polygon> {
        let model_matrix = self.model_matrix();

        self.polys
            .iter()
            .filter_map(|poly| {
                let transformed_verts: Vec<Vec3> = poly
                    .vertices
                    .iter()
                    .map(|&v| model_matrix.transform_point3(v))
                    .collect();

                let mut new_poly = Polygon::new(transformed_verts)?;
                new_poly.texture = poly.texture.clone();
                new_poly.flags = poly.flags;
                new_poly.base = model_matrix.transform_point3(poly.base);
                new_poly.texture_u = model_matrix.transform_vector3(poly.texture_u);
                new_poly.texture_v = model_matrix.transform_vector3(poly.texture_v);
                new_poly.pan_u = poly.pan_u;
                new_poly.pan_v = poly.pan_v;
                new_poly.vertex_colors = poly.vertex_colors.clone();
                new_poly.lightmap = poly.lightmap.clone();
                Some(new_poly)
            })
            .collect()
    }

    /// Computes the Axis-Aligned Bounding Box (min, max) of this brush transformed into world space.
    /// Uses direct matrix transformations on local vertices to achieve sub-microsecond zero-allocation performance.
    pub fn world_bounds(&self) -> (Vec3, Vec3) {
        if self.polys.is_empty() {
            return (self.location, self.location);
        }
        let model_matrix = self.model_matrix();

        let mut min = Vec3::splat(f32::MAX);
        let mut max = Vec3::splat(f32::MIN);
        for poly in &self.polys {
            for &v in &poly.vertices {
                let tv = model_matrix.transform_point3(v);
                min = min.min(tv);
                max = max.max(tv);
            }
        }
        (min, max)
    }

    /// Alias for world_bounds() guaranteeing zero heap allocations for high-frequency frustum culling.
    pub fn fast_world_bounds(&self) -> (Vec3, Vec3) {
        self.world_bounds()
    }

    /// Checks if this brush's world AABB intersects another AABB with an optional tolerance/epsilon.
    pub fn intersects_bounds(&self, other_min: Vec3, other_max: Vec3, tolerance: f32) -> bool {
        let (my_min, my_max) = self.world_bounds();
        (my_min.x - tolerance) <= other_max.x && (my_max.x + tolerance) >= other_min.x
            && (my_min.y - tolerance) <= other_max.y && (my_max.y + tolerance) >= other_min.y
            && (my_min.z - tolerance) <= other_max.z && (my_max.z + tolerance) >= other_min.z
    }

    /// Generates a simplified invisible collision hull (AABB blocking volume with PF_INVISIBLE) enclosing this brush.
    pub fn build_collision_hull(&self, margin: f32) -> Brush {
        let (min, max) = self.world_bounds();
        let size = (max - min) + Vec3::splat(margin * 2.0);
        let center = (min + max) * 0.5;
        let mut hull = build_cube(format!("{}_CollisionHull", self.name), CsgOperation::Add, size);
        hull.location = center;
        for poly in &mut hull.polys {
            poly.flags |= crate::poly::PF_INVISIBLE | crate::poly::PF_SEMISOLID;
        }
        hull
    }

    /// Bakes the brush's current rotation and scale into its local polygon geometry,
    /// then resets rotation to (0, 0, 0) and scale to (1, 1, 1).
    /// The world space orientation and bounds remain unchanged.
    pub fn transform_permanently(&mut self) {
        if self.rotation == Vec3::ZERO && self.scale == Vec3::ONE {
            return;
        }

        let rad_pitch = self.rotation.x.to_radians();
        let rad_yaw = self.rotation.y.to_radians();
        let rad_roll = self.rotation.z.to_radians();

        let rot_matrix = glam::Mat4::from_euler(glam::EulerRot::YXZ, rad_yaw, rad_pitch, rad_roll);
        let scale_matrix = glam::Mat4::from_scale(self.scale);
        let local_matrix = rot_matrix * scale_matrix;

        for poly in &mut self.polys {
            for v in &mut poly.vertices {
                *v = local_matrix.transform_point3(*v);
            }
            poly.base = local_matrix.transform_point3(poly.base);
            poly.texture_u = local_matrix.transform_vector3(poly.texture_u);
            poly.texture_v = local_matrix.transform_vector3(poly.texture_v);

            if poly.vertices.len() >= 3 {
                if let Some(plane) = super::Plane::from_points(poly.vertices[0], poly.vertices[1], poly.vertices[2]) {
                    poly.plane = plane;
                }
            }
        }

        self.rotation = Vec3::ZERO;
        self.scale = Vec3::ONE;
    }

    /// Checks whether a world-space point lies inside this brush's volume (UnrealEd 2.0 `Select Inside`).
    pub fn contains_world_point(&self, world_pt: Vec3) -> bool {
        let (min, max) = self.world_bounds();
        if world_pt.x < min.x - 0.5
            || world_pt.x > max.x + 0.5
            || world_pt.y < min.y - 0.5
            || world_pt.y > max.y + 0.5
            || world_pt.z < min.z - 0.5
            || world_pt.z > max.z + 0.5
        {
            return false;
        }
        let world_polys = self.to_world_polys();
        if world_polys.is_empty() {
            return false;
        }
        for poly in &world_polys {
            if poly.plane.dist_to_point(world_pt) > 0.5 {
                return false;
            }
        }
        true
    }

    /// UnrealEd 2.0 `Polygons -> Merge`:
    /// Iteratively merges coplanar adjacent polygons sharing an edge if their union remains convex.
    /// Returns the number of polygon merges performed.
    pub fn merge_coplanar_polygons(&mut self) -> usize {
        fn is_convex_on_plane(verts: &[Vec3], normal: Vec3) -> bool {
            let n = verts.len();
            if n < 3 {
                return false;
            }
            for i in 0..n {
                let p0 = verts[i];
                let p1 = verts[(i + 1) % n];
                let p2 = verts[(i + 2) % n];
                let e1 = p1 - p0;
                let e2 = p2 - p1;
                if e1.length_squared() < 1e-6 || e2.length_squared() < 1e-6 {
                    continue;
                }
                let cross_dot = e1.cross(e2).dot(normal);
                if cross_dot < -1e-2 {
                    return false;
                }
            }
            true
        }

        fn try_merge_pair(a: &Polygon, b: &Polygon) -> Option<Polygon> {
            if a.plane.normal.dot(b.plane.normal) < 0.999
                || (a.plane.dist - b.plane.dist).abs() > 0.25
                || a.flags != b.flags
            {
                return None;
            }
            if !a.texture.is_empty()
                && !b.texture.is_empty()
                && !a.texture.eq_ignore_ascii_case(&b.texture)
            {
                return None;
            }
            let na = a.vertices.len();
            let nb = b.vertices.len();
            let eps = 0.1f32;
            for i in 0..na {
                let a0 = a.vertices[i];
                let a1 = a.vertices[(i + 1) % na];
                for j in 0..nb {
                    let b0 = b.vertices[j];
                    let b1 = b.vertices[(j + 1) % nb];
                    if a0.distance(b1) <= eps && a1.distance(b0) <= eps {
                        // Splice vertices: a[0..=i], then b[(j+2)..nb] + b[0..j], then a[(i+1)..na]
                        let mut merged = Vec::with_capacity(na + nb - 2);
                        for k in 0..=i {
                            merged.push(a.vertices[k]);
                        }
                        for step in 2..nb {
                            merged.push(b.vertices[(j + step) % nb]);
                        }
                        for k in (i + 1)..na {
                            merged.push(a.vertices[k]);
                        }
                        // Remove collinear vertices
                        let mut cleaned = Vec::with_capacity(merged.len());
                        let mlen = merged.len();
                        for k in 0..mlen {
                            let prev = merged[(k + mlen - 1) % mlen];
                            let curr = merged[k];
                            let next = merged[(k + 1) % mlen];
                            let d1 = (curr - prev).normalize_or_zero();
                            let d2 = (next - curr).normalize_or_zero();
                            if d1.dot(d2) < 0.9995 {
                                cleaned.push(curr);
                            }
                        }
                        if cleaned.len() >= 3 && is_convex_on_plane(&cleaned, a.plane.normal) {
                            let mut poly = Polygon::new(cleaned)?;
                            poly.plane = a.plane;
                            poly.texture = if !a.texture.is_empty() {
                                a.texture.clone()
                            } else {
                                b.texture.clone()
                            };
                            poly.flags = a.flags;
                            poly.base = a.base;
                            poly.texture_u = a.texture_u;
                            poly.texture_v = a.texture_v;
                            return Some(poly);
                        }
                    }
                }
            }
            None
        }

        let mut merged_count = 0usize;
        loop {
            let mut found = None;
            'outer: for i in 0..self.polys.len() {
                for j in (i + 1)..self.polys.len() {
                    if let Some(m) = try_merge_pair(&self.polys[i], &self.polys[j]) {
                        found = Some((i, j, m));
                        break 'outer;
                    }
                }
            }
            if let Some((i, j, merged_poly)) = found {
                self.polys[i] = merged_poly;
                self.polys.remove(j);
                merged_count += 1;
            } else {
                break;
            }
        }
        merged_count
    }

    /// UnrealEd 2.0 `Polygons -> Separate`:
    /// Fan-triangulates any polygon with more than 3 vertices into independent triangular polygons.
    /// Returns the number of newly created polygons.
    pub fn separate_polygons(&mut self) -> usize {
        let mut new_polys = Vec::with_capacity(self.polys.len() * 2);
        let mut added = 0usize;
        for poly in &self.polys {
            if poly.vertices.len() <= 3 {
                new_polys.push(poly.clone());
                continue;
            }
            let v0 = poly.vertices[0];
            for i in 1..(poly.vertices.len() - 1) {
                let v1 = poly.vertices[i];
                let v2 = poly.vertices[i + 1];
                if let Some(mut tri) = Polygon::new(vec![v0, v1, v2]) {
                    tri.plane = poly.plane;
                    tri.texture = poly.texture.clone();
                    tri.flags = poly.flags;
                    tri.base = poly.base;
                    tri.texture_u = poly.texture_u;
                    tri.texture_v = poly.texture_v;
                    new_polys.push(tri);
                    if i > 1 {
                        added += 1;
                    }
                }
            }
        }
        self.polys = new_polys;
        added
    }

    /// Recomputes clean, orthonormal texture UV axes (`texture_u`, `texture_v`) and `base`
    /// for every polygon in the brush according to its face normal.
    /// Prevents stretched textures on ramps, 45-degree chamfered walls, and octagonal chambers.
    pub fn recompute_uvs(&mut self) {
        for poly in &mut self.polys {
            let n = poly.plane.normal;
            if let Some(&v0) = poly.vertices.first() {
                poly.base = v0;
            }
            if n.z.abs() > 0.70 {
                // Floor, ceiling, or walkable ramp slope: planar XY projection
                poly.texture_u = Vec3::X;
                poly.texture_v = Vec3::Y;
            } else {
                // Vertical or angled wall: horizontal tangent U and vertical V
                let horiz = Vec3::new(-n.y, n.x, 0.0).normalize_or_zero();
                poly.texture_u = if horiz.length_squared() > 0.5 {
                    horiz
                } else {
                    Vec3::X
                };
                poly.texture_v = Vec3::new(0.0, 0.0, -1.0);
            }
        }
    }
}

/// Cardinal ascent direction for a convex ramp wedge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RampDirection {
    /// Ascends toward +Y (low edge at -Y, high back at +Y).
    North,
    /// Ascends toward -Y (low edge at +Y, high back at -Y).
    South,
    /// Ascends toward +X (low edge at -X, high back at +X).
    East,
    /// Ascends toward -X (low edge at +X, high back at -X).
    West,
}

/// Procedural builder for an axis-aligned box / cube
pub fn build_cube(name: impl Into<String>, csg_oper: CsgOperation, size: Vec3) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    // 8 vertices
    let p0 = Vec3::new(-hx, -hy, -hz);
    let p1 = Vec3::new( hx, -hy, -hz);
    let p2 = Vec3::new( hx,  hy, -hz);
    let p3 = Vec3::new(-hx,  hy, -hz);
    let p4 = Vec3::new(-hx, -hy,  hz);
    let p5 = Vec3::new( hx, -hy,  hz);
    let p6 = Vec3::new( hx,  hy,  hz);
    let p7 = Vec3::new(-hx,  hy,  hz);

    // 6 faces (counter-clockwise winding for outward-pointing normals)
    // Bottom (-Z)
    if let Some(poly) = Polygon::new(vec![p0, p3, p2, p1]) {
        brush.polys.push(poly);
    }
    // Top (+Z)
    if let Some(poly) = Polygon::new(vec![p4, p5, p6, p7]) {
        brush.polys.push(poly);
    }
    // Front (-Y)
    if let Some(poly) = Polygon::new(vec![p0, p1, p5, p4]) {
        brush.polys.push(poly);
    }
    // Back (+Y)
    if let Some(poly) = Polygon::new(vec![p3, p7, p6, p2]) {
        brush.polys.push(poly);
    }
    // Left (-X)
    if let Some(poly) = Polygon::new(vec![p0, p4, p7, p3]) {
        brush.polys.push(poly);
    }
    // Right (+X)
    if let Some(poly) = Polygon::new(vec![p1, p2, p6, p5]) {
        brush.polys.push(poly);
    }

    brush
}

/// Procedural builder for an axis-aligned convex right-triangular wedge / ramp.
///
/// The ramp slopes upward along +Y (`RampDirection::North`), with its low base at -Y and high edge at +Y.
/// All 5 polygonal faces form a strictly convex polyhedron (triangular prism),
/// guaranteed watertight, convex and free of internal coplanar split faces.
pub fn build_ramp(name: impl Into<String>, csg_oper: CsgOperation, size: Vec3) -> Brush {
    build_ramp_oriented(name, csg_oper, size, RampDirection::North)
}

/// Procedural builder for a cardinal-oriented convex right-triangular wedge / ramp.
///
/// Generates exact grid-aligned vertices for all 4 cardinal directions (`North`, `South`, `East`, `West`)
/// without floating-point Euler rotation drift, with outward normals and clean UV vectors.
pub fn build_ramp_oriented(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    size: Vec3,
    dir: RampDirection,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    let b00 = Vec3::new(-hx, -hy, -hz); // SW bottom
    let b10 = Vec3::new(hx, -hy, -hz);  // SE bottom
    let b11 = Vec3::new(hx, hy, -hz);   // NE bottom
    let b01 = Vec3::new(-hx, hy, -hz);  // NW bottom

    let t00 = Vec3::new(-hx, -hy, hz);  // SW top
    let t10 = Vec3::new(hx, -hy, hz);   // SE top
    let t11 = Vec3::new(hx, hy, hz);    // NE top
    let t01 = Vec3::new(-hx, hy, hz);   // NW top

    // 1. Bottom face (-Z): normal (0, 0, -1)
    if let Some(poly) = Polygon::new(vec![b00, b01, b11, b10]) {
        brush.polys.push(poly);
    }

    match dir {
        RampDirection::North => {
            // High back at +Y, low edge at -Y
            if let Some(poly) = Polygon::new(vec![b01, t01, t11, b11]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b00, b10, t11, t01]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b00, t01, b01]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b10, b11, t11]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::South => {
            // High back at -Y, low edge at +Y
            if let Some(poly) = Polygon::new(vec![b00, b10, t10, t00]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b11, b01, t00, t10]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b00, t00, b01]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b10, b11, t10]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::East => {
            // High back at +X, low edge at -X
            if let Some(poly) = Polygon::new(vec![b10, b11, t11, t10]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b01, b00, t10, t11]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b00, b10, t10]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b01, t11, b11]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::West => {
            // High back at -X, low edge at +X
            if let Some(poly) = Polygon::new(vec![b00, t00, t01, b01]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b10, b11, t01, t00]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b00, b10, t00]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![b01, t01, b11]) {
                brush.polys.push(poly);
            }
        }
    }

    brush.recompute_uvs();
    brush
}

/// Procedural builder for a cardinal-oriented stepped architectural flight of stairs.
///
/// Generates `step_count` clean stepped tread and riser blocks ascending along `dir`.
/// Step 0 begins at `-size.z/2` and the top of the last step meets `+size.z/2`.
/// All faces have outward-pointing normals and clean UVs.
pub fn build_stepped_stairs_oriented(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    size: Vec3,
    dir: RampDirection,
    step_count: usize,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let n = step_count.max(1);
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;
    let z_floor = -hz;
    let h_step = size.z / n as f32;

    // Bottom face (-Z, normal [0, 0, -1])
    let b00 = Vec3::new(-hx, -hy, z_floor);
    let b10 = Vec3::new(hx, -hy, z_floor);
    let b11 = Vec3::new(hx, hy, z_floor);
    let b01 = Vec3::new(-hx, hy, z_floor);
    if let Some(poly) = Polygon::new(vec![b00, b01, b11, b10]) {
        brush.polys.push(poly);
    }

    match dir {
        RampDirection::North => {
            // High back at +Y, low foot at -Y
            let l_step = size.y / n as f32;
            for i in 0..n {
                let y0 = -hy + (i as f32) * l_step;
                let y1 = -hy + ((i + 1) as f32) * l_step;
                let z_prev = z_floor + (i as f32) * h_step;
                let z_top = z_floor + ((i + 1) as f32) * h_step;

                // Top tread (+Z)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(-hx, y0, z_top),
                    Vec3::new(hx, y0, z_top),
                    Vec3::new(hx, y1, z_top),
                    Vec3::new(-hx, y1, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // Front riser (-Y, facing -Y towards foot)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(-hx, y0, z_prev),
                    Vec3::new(hx, y0, z_prev),
                    Vec3::new(hx, y0, z_top),
                    Vec3::new(-hx, y0, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // West side (-X)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(-hx, y0, z_floor),
                    Vec3::new(-hx, y0, z_top),
                    Vec3::new(-hx, y1, z_top),
                    Vec3::new(-hx, y1, z_floor),
                ]) {
                    brush.polys.push(poly);
                }
                // East side (+X)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(hx, y0, z_floor),
                    Vec3::new(hx, y1, z_floor),
                    Vec3::new(hx, y1, z_top),
                    Vec3::new(hx, y0, z_top),
                ]) {
                    brush.polys.push(poly);
                }
            }
            // Back wall (+Y, facing +Y)
            if let Some(poly) = Polygon::new(vec![
                Vec3::new(-hx, hy, z_floor),
                Vec3::new(-hx, hy, hz),
                Vec3::new(hx, hy, hz),
                Vec3::new(hx, hy, z_floor),
            ]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::South => {
            // High back at -Y, low foot at +Y
            let l_step = size.y / n as f32;
            for i in 0..n {
                let y0 = hy - (i as f32) * l_step;
                let y1 = hy - ((i + 1) as f32) * l_step;
                let z_prev = z_floor + (i as f32) * h_step;
                let z_top = z_floor + ((i + 1) as f32) * h_step;

                // Top tread (+Z)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(-hx, y1, z_top),
                    Vec3::new(hx, y1, z_top),
                    Vec3::new(hx, y0, z_top),
                    Vec3::new(-hx, y0, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // Front riser (+Y, facing +Y towards foot)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(hx, y0, z_prev),
                    Vec3::new(-hx, y0, z_prev),
                    Vec3::new(-hx, y0, z_top),
                    Vec3::new(hx, y0, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // West side (-X)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(-hx, y1, z_floor),
                    Vec3::new(-hx, y1, z_top),
                    Vec3::new(-hx, y0, z_top),
                    Vec3::new(-hx, y0, z_floor),
                ]) {
                    brush.polys.push(poly);
                }
                // East side (+X)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(hx, y1, z_floor),
                    Vec3::new(hx, y0, z_floor),
                    Vec3::new(hx, y0, z_top),
                    Vec3::new(hx, y1, z_top),
                ]) {
                    brush.polys.push(poly);
                }
            }
            // Back wall (-Y, facing -Y)
            if let Some(poly) = Polygon::new(vec![
                Vec3::new(-hx, -hy, hz),
                Vec3::new(-hx, -hy, z_floor),
                Vec3::new(hx, -hy, z_floor),
                Vec3::new(hx, -hy, hz),
            ]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::East => {
            // High back at +X, low foot at -X
            let l_step = size.x / n as f32;
            for i in 0..n {
                let x0 = -hx + (i as f32) * l_step;
                let x1 = -hx + ((i + 1) as f32) * l_step;
                let z_prev = z_floor + (i as f32) * h_step;
                let z_top = z_floor + ((i + 1) as f32) * h_step;

                // Top tread (+Z)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x0, -hy, z_top),
                    Vec3::new(x1, -hy, z_top),
                    Vec3::new(x1, hy, z_top),
                    Vec3::new(x0, hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // Front riser (-X, facing -X towards foot)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x0, hy, z_prev),
                    Vec3::new(x0, -hy, z_prev),
                    Vec3::new(x0, -hy, z_top),
                    Vec3::new(x0, hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // South side (-Y)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x0, -hy, z_floor),
                    Vec3::new(x1, -hy, z_floor),
                    Vec3::new(x1, -hy, z_top),
                    Vec3::new(x0, -hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // North side (+Y)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x0, hy, z_floor),
                    Vec3::new(x0, hy, z_top),
                    Vec3::new(x1, hy, z_top),
                    Vec3::new(x1, hy, z_floor),
                ]) {
                    brush.polys.push(poly);
                }
            }
            // Back wall (+X, facing +X)
            if let Some(poly) = Polygon::new(vec![
                Vec3::new(hx, -hy, z_floor),
                Vec3::new(hx, hy, z_floor),
                Vec3::new(hx, hy, hz),
                Vec3::new(hx, -hy, hz),
            ]) {
                brush.polys.push(poly);
            }
        }
        RampDirection::West => {
            // High back at -X, low foot at +X
            let l_step = size.x / n as f32;
            for i in 0..n {
                let x0 = hx - (i as f32) * l_step;
                let x1 = hx - ((i + 1) as f32) * l_step;
                let z_prev = z_floor + (i as f32) * h_step;
                let z_top = z_floor + ((i + 1) as f32) * h_step;

                // Top tread (+Z)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x1, -hy, z_top),
                    Vec3::new(x0, -hy, z_top),
                    Vec3::new(x0, hy, z_top),
                    Vec3::new(x1, hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // Front riser (+X, facing +X towards foot)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x0, -hy, z_prev),
                    Vec3::new(x0, hy, z_prev),
                    Vec3::new(x0, hy, z_top),
                    Vec3::new(x0, -hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // South side (-Y)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x1, -hy, z_floor),
                    Vec3::new(x0, -hy, z_floor),
                    Vec3::new(x0, -hy, z_top),
                    Vec3::new(x1, -hy, z_top),
                ]) {
                    brush.polys.push(poly);
                }
                // North side (+Y)
                if let Some(poly) = Polygon::new(vec![
                    Vec3::new(x1, hy, z_floor),
                    Vec3::new(x1, hy, z_top),
                    Vec3::new(x0, hy, z_top),
                    Vec3::new(x0, hy, z_floor),
                ]) {
                    brush.polys.push(poly);
                }
            }
            // Back wall (-X, facing -X)
            if let Some(poly) = Polygon::new(vec![
                Vec3::new(-hx, hy, z_floor),
                Vec3::new(-hx, -hy, z_floor),
                Vec3::new(-hx, -hy, hz),
                Vec3::new(-hx, hy, hz),
            ]) {
                brush.polys.push(poly);
            }
        }
    }

    brush.recompute_uvs();
    brush
}

/// Procedural builder for a grid-aligned octagonal prism (`size.x`, `size.y`, `size.z`).
///
/// Unlike a raw trigonometric cylinder whose vertices land on irrational coordinates,
/// this octagon has flat cardinal walls at `X = ±size.x/2` and `Y = ±size.y/2` spanning
/// `7/16` (`43.75%` per half-axis = `87.5%` total flat width, or `7/16` corner chamfer)
/// snapped to the 16-UU grid so orthogonal corridors join cleanly with zero BSP cracks.
pub fn build_aligned_octagon(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    size: Vec3,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let hx = size.x * 0.5;
    let hy = size.y * 0.5;
    let hz = size.z * 0.5;

    // Flat cardinal half-span = 7/16 of half-extent (exact binary fraction, snaps to 16 UU for 256+ rooms)
    let cx = ((hx * 0.4375) / 16.0).round().max(1.0) * 16.0;
    let cy = ((hy * 0.4375) / 16.0).round().max(1.0) * 16.0;
    let cx = cx.min(hx - 16.0);
    let cy = cy.min(hy - 16.0);

    // 8 counter-clockwise vertices in XY plane
    let ring_xy = [
        (hx, -cy),  // 0: SE East wall start
        (hx, cy),   // 1: NE East wall end
        (cx, hy),   // 2: NE North wall start
        (-cx, hy),  // 3: NW North wall end
        (-hx, cy),  // 4: NW West wall start
        (-hx, -cy), // 5: SW West wall end
        (-cx, -hy), // 6: SW South wall start
        (cx, -hy),  // 7: SE South wall end
    ];

    let bottom_verts: Vec<Vec3> = ring_xy.iter().map(|&(x, y)| Vec3::new(x, y, -hz)).collect();
    let top_verts: Vec<Vec3> = ring_xy.iter().map(|&(x, y)| Vec3::new(x, y, hz)).collect();

    // Bottom cap (-Z outward normal -> clockwise in XY)
    let mut bot_cap = bottom_verts.clone();
    bot_cap.reverse();
    if let Some(poly) = Polygon::new(bot_cap) {
        brush.polys.push(poly);
    }

    // Top cap (+Z outward normal -> counter-clockwise in XY)
    if let Some(poly) = Polygon::new(top_verts.clone()) {
        brush.polys.push(poly);
    }

    // 8 vertical side quads (outward normals)
    for i in 0..8 {
        let next = (i + 1) % 8;
        let p0 = bottom_verts[i];
        let p1 = bottom_verts[next];
        let p2 = top_verts[next];
        let p3 = top_verts[i];
        if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            brush.polys.push(poly);
        }
    }

    brush.recompute_uvs();
    brush
}

/// Procedural builder for a cylinder
pub fn build_cylinder(name: impl Into<String>, csg_oper: CsgOperation, height: f32, radius: f32, sides: usize) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let sides = sides.max(3);
    let hz = height * 0.5;

    let mut bottom_verts = Vec::with_capacity(sides);
    let mut top_verts = Vec::with_capacity(sides);

    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * 2.0 * PI;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        bottom_verts.push(Vec3::new(x, y, -hz));
        top_verts.push(Vec3::new(x, y, hz));
    }

    // Bottom Cap
    let mut bot_cap = bottom_verts.clone();
    bot_cap.reverse();
    if let Some(poly) = Polygon::new(bot_cap) {
        brush.polys.push(poly);
    }

    // Top Cap
    if let Some(poly) = Polygon::new(top_verts.clone()) {
        brush.polys.push(poly);
    }

    // Side Quads
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = bottom_verts[i];
        let p1 = bottom_verts[next];
        let p2 = top_verts[next];
        let p3 = top_verts[i];

        if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            brush.polys.push(poly);
        }
    }

    brush
}

/// Procedural builder for a cone
pub fn build_cone(name: impl Into<String>, csg_oper: CsgOperation, height: f32, radius: f32, sides: usize) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let sides = sides.max(3);
    let hz = height * 0.5;

    let mut bottom_verts = Vec::with_capacity(sides);
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * 2.0 * PI;
        let x = radius * angle.cos();
        let y = radius * angle.sin();
        bottom_verts.push(Vec3::new(x, y, -hz));
    }

    // Bottom Cap
    let mut bot_cap = bottom_verts.clone();
    bot_cap.reverse();
    if let Some(poly) = Polygon::new(bot_cap) {
        brush.polys.push(poly);
    }

    // Apex
    let apex = Vec3::new(0.0, 0.0, hz);

    // Side Triangles
    for i in 0..sides {
        let next = (i + 1) % sides;
        let p0 = bottom_verts[i];
        let p1 = bottom_verts[next];
        if let Some(poly) = Polygon::new(vec![p0, p1, apex]) {
            brush.polys.push(poly);
        }
    }

    brush
}

/// Orientation axis for 2D planar sheet brushes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetAxis {
    #[default]
    XY, // Horizontal (water/acid surfaces, ceiling/floor barriers)
    XZ, // Vertical Front (banners, tapestries, waterfalls)
    YZ, // Vertical Side (corridor partitions, door portals)
}

/// Surface polyflags preset for sheet brushes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SheetPreset {
    #[default]
    Normal,
    Water,      // Translucent, two-sided, modulated
    AcidSlime,  // Translucent, two-sided, unlit
    Banner,     // Masked, two-sided
    ZonePortal, // Portal, invisible
}

/// Enhanced procedural builder for a 2D planar sheet with configurable orientation axis and material preset.
pub fn build_sheet_advanced(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    width: f32,
    height: f32,
    axis: SheetAxis,
    preset: SheetPreset,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let hw = width * 0.5;
    let hh = height * 0.5;

    let (p0, p1, p2, p3) = match axis {
        SheetAxis::XY => (
            Vec3::new(-hw, -hh, 0.0),
            Vec3::new( hw, -hh, 0.0),
            Vec3::new( hw,  hh, 0.0),
            Vec3::new(-hw,  hh, 0.0),
        ),
        SheetAxis::XZ => (
            Vec3::new(-hw, 0.0, -hh),
            Vec3::new( hw, 0.0, -hh),
            Vec3::new( hw, 0.0,  hh),
            Vec3::new(-hw, 0.0,  hh),
        ),
        SheetAxis::YZ => (
            Vec3::new(0.0, -hw, -hh),
            Vec3::new(0.0,  hw, -hh),
            Vec3::new(0.0,  hw,  hh),
            Vec3::new(0.0, -hw,  hh),
        ),
    };

    let flags = match preset {
        SheetPreset::Normal => 0,
        SheetPreset::Water => crate::poly::PF_TRANSLUCENT | crate::poly::PF_TWO_SIDED | crate::poly::PF_MODULATED,
        SheetPreset::AcidSlime => crate::poly::PF_TRANSLUCENT | crate::poly::PF_TWO_SIDED | crate::poly::PF_UNLIT,
        SheetPreset::Banner => crate::poly::PF_MASKED | crate::poly::PF_TWO_SIDED,
        SheetPreset::ZonePortal => crate::poly::PF_PORTAL | crate::poly::PF_INVISIBLE,
    };

    if let Some(mut poly) = Polygon::new(vec![p0, p1, p2, p3]) {
        poly.flags = flags;
        brush.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(vec![p3, p2, p1, p0]) {
        poly.flags = flags;
        brush.polys.push(poly);
    }

    brush
}

/// Procedural builder for a 2D planar sheet (portal, water, barrier)
pub fn build_sheet(name: impl Into<String>, csg_oper: CsgOperation, width: f32, height: f32) -> Brush {
    build_sheet_advanced(name, csg_oper, width, height, SheetAxis::XY, SheetPreset::Normal)
}

/// Procedural builder for a linear staircase
pub fn build_staircase(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    num_steps: usize,
    step_height: f32,
    step_width: f32,
    step_length: f32,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let num_steps = num_steps.max(1);
    let hw = step_width * 0.5;
    let total_length = num_steps as f32 * step_length;
    let _total_height = num_steps as f32 * step_height;

    // Start centered at Y=0, Z=0
    let start_y = -total_length * 0.5;

    for i in 0..num_steps {
        let y0 = start_y + i as f32 * step_length;
        let y1 = y0 + step_length;
        let z_base = 0.0;
        let z_step = (i + 1) as f32 * step_height;

        let p0 = Vec3::new(-hw, y0, z_base);
        let p1 = Vec3::new( hw, y0, z_base);
        let p2 = Vec3::new( hw, y1, z_base);
        let p3 = Vec3::new(-hw, y1, z_base);
        let p4 = Vec3::new(-hw, y0, z_step);
        let p5 = Vec3::new( hw, y0, z_step);
        let p6 = Vec3::new( hw, y1, z_step);
        let p7 = Vec3::new(-hw, y1, z_step);

        // Top tread
        if let Some(poly) = Polygon::new(vec![p7, p6, p5, p4]) {
            brush.polys.push(poly);
        }
        // Front riser
        if let Some(poly) = Polygon::new(vec![p4, p5, p1, p0]) {
            brush.polys.push(poly);
        }
        // Left side
        if let Some(poly) = Polygon::new(vec![p3, p7, p4, p0]) {
            brush.polys.push(poly);
        }
        // Right side
        if let Some(poly) = Polygon::new(vec![p1, p5, p6, p2]) {
            brush.polys.push(poly);
        }
        // Back side (for the last step)
        if i == num_steps - 1 {
            if let Some(poly) = Polygon::new(vec![p2, p6, p7, p3]) {
                brush.polys.push(poly);
            }
        }
        // Bottom
        if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            brush.polys.push(poly);
        }
    }

    brush
}

/// Procedural builder for a UV sphere
pub fn build_sphere(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    radius: f32,
    rings: usize,
    segments: usize,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let rings = rings.max(2);
    let segments = segments.max(3);

    let mut grid = Vec::new();
    for r in 0..=rings {
        let phi = (r as f32 / rings as f32) * PI;
        let mut row = Vec::new();
        for s in 0..=segments {
            let theta = (s as f32 / segments as f32) * 2.0 * PI;
            let x = radius * phi.sin() * theta.cos();
            let y = radius * phi.sin() * theta.sin();
            let z = radius * phi.cos();
            row.push(Vec3::new(x, y, z));
        }
        grid.push(row);
    }

    for r in 0..rings {
        for s in 0..segments {
            let p0 = grid[r][s];
            let p1 = grid[r][s + 1];
            let p2 = grid[r + 1][s + 1];
            let p3 = grid[r + 1][s];

            if r == 0 {
                // Top polar triangle
                if let Some(poly) = Polygon::new(vec![p0, p2, p3]) {
                    brush.polys.push(poly);
                }
            } else if r == rings - 1 {
                // Bottom polar triangle
                if let Some(poly) = Polygon::new(vec![p0, p1, p2]) {
                    brush.polys.push(poly);
                }
            } else {
                // Quad
                if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
                    brush.polys.push(poly);
                }
            }
        }
    }

    brush
}

/// Procedural builder for a curved staircase
pub fn build_curved_staircase(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    num_steps: usize,
    step_height: f32,
    inner_radius: f32,
    step_width: f32,
    angle_degrees: f32,
    counter_clockwise: bool,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let num_steps = num_steps.max(1);
    let outer_radius = inner_radius + step_width;
    let total_rad = angle_degrees.to_radians();
    let step_rad = total_rad / num_steps as f32;
    let dir = if counter_clockwise { 1.0 } else { -1.0 };

    for i in 0..num_steps {
        let a0 = dir * (i as f32 * step_rad);
        let a1 = dir * ((i + 1) as f32 * step_rad);

        let z_base = 0.0;
        let z_step = (i + 1) as f32 * step_height;

        let (sin0, cos0) = a0.sin_cos();
        let (sin1, cos1) = a1.sin_cos();

        // 8 points for step wedge
        let p_in_b0 = Vec3::new(inner_radius * cos0, inner_radius * sin0, z_base);
        let p_out_b0 = Vec3::new(outer_radius * cos0, outer_radius * sin0, z_base);
        let p_out_b1 = Vec3::new(outer_radius * cos1, outer_radius * sin1, z_base);
        let p_in_b1 = Vec3::new(inner_radius * cos1, inner_radius * sin1, z_base);

        let p_in_t0 = Vec3::new(inner_radius * cos0, inner_radius * sin0, z_step);
        let p_out_t0 = Vec3::new(outer_radius * cos0, outer_radius * sin0, z_step);
        let p_out_t1 = Vec3::new(outer_radius * cos1, outer_radius * sin1, z_step);
        let p_in_t1 = Vec3::new(inner_radius * cos1, inner_radius * sin1, z_step);

        if counter_clockwise {
            // Top tread
            if let Some(poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_t1, p_in_t1]) {
                brush.polys.push(poly);
            }
            // Front riser
            if let Some(poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_t0, p_in_t0]) {
                brush.polys.push(poly);
            }
            // Outer wall
            if let Some(poly) = Polygon::new(vec![p_out_b0, p_out_b1, p_out_t1, p_out_t0]) {
                brush.polys.push(poly);
            }
            // Inner wall
            if let Some(poly) = Polygon::new(vec![p_in_b1, p_in_b0, p_in_t0, p_in_t1]) {
                brush.polys.push(poly);
            }
            // Bottom
            if let Some(poly) = Polygon::new(vec![p_in_b1, p_out_b1, p_out_b0, p_in_b0]) {
                brush.polys.push(poly);
            }
            // Back riser (last step)
            if i == num_steps - 1 {
                if let Some(poly) = Polygon::new(vec![p_out_b1, p_in_b1, p_in_t1, p_out_t1]) {
                    brush.polys.push(poly);
                }
            }
        } else {
            // Clockwise winding
            if let Some(poly) = Polygon::new(vec![p_in_t1, p_out_t1, p_out_t0, p_in_t0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_b0, p_in_b0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_out_t0, p_out_t1, p_out_b1, p_out_b0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_in_t1, p_in_t0, p_in_b0, p_in_b1]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_b1, p_in_b1]) {
                brush.polys.push(poly);
            }
            if i == num_steps - 1 {
                if let Some(poly) = Polygon::new(vec![p_out_t1, p_in_t1, p_in_b1, p_out_b1]) {
                    brush.polys.push(poly);
                }
            }
        }
    }

    brush
}

/// Procedural builder for a spiral staircase (tower stairs) with configurable step thickness,
/// sloped ceiling (underside), and sloped floor (ramp top) matching UnrealEd 2.0 `SpiralStairBuilder`.
#[allow(clippy::too_many_arguments)]
pub fn build_spiral_staircase_ex(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    num_steps: usize,
    step_height: f32,
    step_thickness: f32,
    inner_radius: f32,
    step_width: f32,
    total_angle_degrees: f32,
    sloped_ceiling: bool,
    sloped_floor: bool,
    counter_clockwise: bool,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let num_steps = num_steps.max(1);
    let thickness = step_thickness.max(4.0);
    let outer_radius = inner_radius + step_width;
    let total_rad = total_angle_degrees.to_radians();
    let step_rad = total_rad / num_steps as f32;
    let dir = if counter_clockwise { 1.0 } else { -1.0 };

    for i in 0..num_steps {
        let a0 = dir * (i as f32 * step_rad);
        let a1 = dir * ((i + 1) as f32 * step_rad);

        let z_top0 = if sloped_floor {
            i as f32 * step_height
        } else {
            (i + 1) as f32 * step_height
        };
        let z_top1 = (i + 1) as f32 * step_height;

        let z_bot0 = (i as f32 * step_height) - thickness;
        let z_bot1 = if sloped_ceiling {
            ((i + 1) as f32 * step_height) - thickness
        } else {
            z_bot0
        };

        let (sin0, cos0) = a0.sin_cos();
        let (sin1, cos1) = a1.sin_cos();

        let p_in_b0 = Vec3::new(inner_radius * cos0, inner_radius * sin0, z_bot0);
        let p_out_b0 = Vec3::new(outer_radius * cos0, outer_radius * sin0, z_bot0);
        let p_out_b1 = Vec3::new(outer_radius * cos1, outer_radius * sin1, z_bot1);
        let p_in_b1 = Vec3::new(inner_radius * cos1, inner_radius * sin1, z_bot1);

        let p_in_t0 = Vec3::new(inner_radius * cos0, inner_radius * sin0, z_top0);
        let p_out_t0 = Vec3::new(outer_radius * cos0, outer_radius * sin0, z_top0);
        let p_out_t1 = Vec3::new(outer_radius * cos1, outer_radius * sin1, z_top1);
        let p_in_t1 = Vec3::new(inner_radius * cos1, inner_radius * sin1, z_top1);

        if counter_clockwise {
            // Top tread (triangulated if sloped_floor creates non-planar helix)
            if sloped_floor {
                if let Some(p1) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_t1]) {
                    brush.polys.push(p1);
                }
                if let Some(p2) = Polygon::new(vec![p_in_t0, p_out_t1, p_in_t1]) {
                    brush.polys.push(p2);
                }
            } else if let Some(poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_t1, p_in_t1]) {
                brush.polys.push(poly);
            }
            // Front riser
            if let Some(poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_t0, p_in_t0]) {
                brush.polys.push(poly);
            }
            // Outer wall
            if let Some(poly) = Polygon::new(vec![p_out_b0, p_out_b1, p_out_t1, p_out_t0]) {
                brush.polys.push(poly);
            }
            // Inner wall
            if let Some(poly) = Polygon::new(vec![p_in_b1, p_in_b0, p_in_t0, p_in_t1]) {
                brush.polys.push(poly);
            }
            // Bottom underside (triangulated if sloped_ceiling)
            if sloped_ceiling {
                if let Some(p1) = Polygon::new(vec![p_in_b1, p_out_b1, p_out_b0]) {
                    brush.polys.push(p1);
                }
                if let Some(p2) = Polygon::new(vec![p_in_b1, p_out_b0, p_in_b0]) {
                    brush.polys.push(p2);
                }
            } else if let Some(poly) = Polygon::new(vec![p_in_b1, p_out_b1, p_out_b0, p_in_b0]) {
                brush.polys.push(poly);
            }
            // Back riser
            if let Some(poly) = Polygon::new(vec![p_out_b1, p_in_b1, p_in_t1, p_out_t1]) {
                brush.polys.push(poly);
            }
        } else {
            if sloped_floor {
                if let Some(p1) = Polygon::new(vec![p_in_t1, p_out_t1, p_out_t0]) {
                    brush.polys.push(p1);
                }
                if let Some(p2) = Polygon::new(vec![p_in_t1, p_out_t0, p_in_t0]) {
                    brush.polys.push(p2);
                }
            } else if let Some(poly) = Polygon::new(vec![p_in_t1, p_out_t1, p_out_t0, p_in_t0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_in_t0, p_out_t0, p_out_b0, p_in_b0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_out_t0, p_out_t1, p_out_b1, p_out_b0]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_in_t1, p_in_t0, p_in_b0, p_in_b1]) {
                brush.polys.push(poly);
            }
            if sloped_ceiling {
                if let Some(p1) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_b1]) {
                    brush.polys.push(p1);
                }
                if let Some(p2) = Polygon::new(vec![p_in_b0, p_out_b1, p_in_b1]) {
                    brush.polys.push(p2);
                }
            } else if let Some(poly) = Polygon::new(vec![p_in_b0, p_out_b0, p_out_b1, p_in_b1]) {
                brush.polys.push(poly);
            }
            if let Some(poly) = Polygon::new(vec![p_out_t1, p_in_t1, p_in_b1, p_out_b1]) {
                brush.polys.push(poly);
            }
        }
    }

    brush
}

/// Procedural builder for a spiral staircase (tower stairs)
pub fn build_spiral_staircase(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    num_steps: usize,
    step_height: f32,
    inner_radius: f32,
    step_width: f32,
    total_angle_degrees: f32,
    counter_clockwise: bool,
) -> Brush {
    build_spiral_staircase_ex(
        name,
        csg_oper,
        num_steps,
        step_height,
        step_height.max(16.0),
        inner_radius,
        step_width,
        total_angle_degrees,
        false,
        false,
        counter_clockwise,
    )
}

/// Procedural builder for a geodesic sphere / tetrahedron (`TetrahedronBuilder` with `SphereExtrapolation` 1..=5).
pub fn build_tetrahedron_extrapolated(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    radius: f32,
    sphere_extrapolation: usize,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let r = radius.max(1.0);
    let sqrt2 = 2.0f32.sqrt();
    let sqrt6 = 6.0f32.sqrt();

    let p0 = Vec3::new(0.0, 0.0, r);
    let p1 = Vec3::new((2.0 * sqrt2 / 3.0) * r, 0.0, -r / 3.0);
    let p2 = Vec3::new((-sqrt2 / 3.0) * r, (sqrt6 / 3.0) * r, -r / 3.0);
    let p3 = Vec3::new((-sqrt2 / 3.0) * r, (-sqrt6 / 3.0) * r, -r / 3.0);

    let mut tris = vec![[p0, p1, p2], [p0, p2, p3], [p0, p3, p1], [p1, p3, p2]];
    let levels = sphere_extrapolation.clamp(1, 5);
    for _ in 1..levels {
        let mut next_tris = Vec::with_capacity(tris.len() * 4);
        for [v0, v1, v2] in tris {
            let m01 = ((v0 + v1) * 0.5).normalize_or_zero() * r;
            let m12 = ((v1 + v2) * 0.5).normalize_or_zero() * r;
            let m20 = ((v2 + v0) * 0.5).normalize_or_zero() * r;
            next_tris.push([v0, m01, m20]);
            next_tris.push([m01, v1, m12]);
            next_tris.push([m20, m12, v2]);
            next_tris.push([m01, m12, m20]);
        }
        tris = next_tris;
    }

    for [a, b, c] in tris {
        if let Some(poly) = Polygon::new(vec![a, b, c]) {
            brush.polys.push(poly);
        }
    }

    brush
}

/// Procedural builder for a tetrahedron (TetrahedronBuilder)
pub fn build_tetrahedron(name: impl Into<String>, csg_oper: CsgOperation, radius: f32) -> Brush {
    build_tetrahedron_extrapolated(name, csg_oper, radius, 1)
}

/// Procedural builder for a torus (Tarquin Torus builder)
pub fn build_torus(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    ring_radius: f32,
    tube_radius: f32,
    ring_sides: usize,
    tube_sides: usize,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let ring_sides = ring_sides.max(3);
    let tube_sides = tube_sides.max(3);

    let mut grid = Vec::with_capacity(ring_sides);
    for r in 0..ring_sides {
        let theta = (r as f32 / ring_sides as f32) * 2.0 * PI;
        let (sin_t, cos_t) = theta.sin_cos();

        let mut row = Vec::with_capacity(tube_sides);
        for t in 0..tube_sides {
            let phi = (t as f32 / tube_sides as f32) * 2.0 * PI;
            let (sin_p, cos_p) = phi.sin_cos();

            let r_dist = ring_radius + tube_radius * cos_p;
            let x = r_dist * cos_t;
            let y = r_dist * sin_t;
            let z = tube_radius * sin_p;
            row.push(Vec3::new(x, y, z));
        }
        grid.push(row);
    }

    for r in 0..ring_sides {
        let next_r = (r + 1) % ring_sides;
        for t in 0..tube_sides {
            let next_t = (t + 1) % tube_sides;

            let p0 = grid[r][t];
            let p1 = grid[r][next_t];
            let p2 = grid[next_r][next_t];
            let p3 = grid[next_r][t];

            if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
                brush.polys.push(poly);
            }
        }
    }

    brush
}

/// Procedural builder for volumetric brush (crossing 2D sheets for fire/fog/torches)
pub fn build_volumetric(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    width: f32,
    height: f32,
    num_sheets: usize,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let num_sheets = num_sheets.max(1);
    let hw = width * 0.5;
    let hz = height * 0.5;

    for i in 0..num_sheets {
        let angle = (i as f32 / num_sheets as f32) * PI;
        let (sin_a, cos_a) = angle.sin_cos();

        let dx = hw * cos_a;
        let dy = hw * sin_a;

        let p0 = Vec3::new(-dx, -dy, -hz);
        let p1 = Vec3::new( dx,  dy, -hz);
        let p2 = Vec3::new( dx,  dy,  hz);
        let p3 = Vec3::new(-dx, -dy,  hz);

        // Front face
        if let Some(poly) = Polygon::new(vec![p0, p1, p2, p3]) {
            brush.polys.push(poly);
        }
        // Back face
        if let Some(poly) = Polygon::new(vec![p3, p2, p1, p0]) {
            brush.polys.push(poly);
        }
    }

    brush
}

/// Procedural builder for terrain mesh (TerrainBuilder)
pub fn build_terrain(
    name: impl Into<String>,
    csg_oper: CsgOperation,
    width_steps: usize,
    length_steps: usize,
    step_size: f32,
    height_amplitude: f32,
    seed: u32,
) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    let w_steps = width_steps.max(1);
    let l_steps = length_steps.max(1);

    let origin_x = -(w_steps as f32 * step_size) * 0.5;
    let origin_y = -(l_steps as f32 * step_size) * 0.5;

    let mut heights = Vec::with_capacity(w_steps + 1);
    for x in 0..=w_steps {
        let mut row = Vec::with_capacity(l_steps + 1);
        for y in 0..=l_steps {
            let px = x as f32;
            let py = y as f32;
            // Simple deterministic noise
            let wave1 = (px * 0.45 + (seed as f32 * 0.1)).sin();
            let wave2 = (py * 0.35 + (seed as f32 * 0.2)).cos();
            let wave3 = ((px + py) * 0.25).sin();
            let h = (wave1 + wave2 + wave3) / 3.0 * height_amplitude;
            row.push(h);
        }
        heights.push(row);
    }

    for x in 0..w_steps {
        for y in 0..l_steps {
            let x0 = origin_x + x as f32 * step_size;
            let x1 = x0 + step_size;
            let y0 = origin_y + y as f32 * step_size;
            let y1 = y0 + step_size;

            let z00 = heights[x][y];
            let z10 = heights[x + 1][y];
            let z11 = heights[x + 1][y + 1];
            let z01 = heights[x][y + 1];

            let p00 = Vec3::new(x0, y0, z00);
            let p10 = Vec3::new(x1, y0, z10);
            let p11 = Vec3::new(x1, y1, z11);
            let p01 = Vec3::new(x0, y1, z01);

            // Two triangles for terrain quad
            if let Some(t1) = Polygon::new(vec![p00, p10, p11]) {
                brush.polys.push(t1);
            }
            if let Some(t2) = Polygon::new(vec![p00, p11, p01]) {
                brush.polys.push(t2);
            }
        }
    }

    brush
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ramp_convexity_and_normals() {
        let size = Vec3::new(256.0, 512.0, 128.0);
        let ramp = build_ramp("TestRamp", CsgOperation::Add, size);

        // A right triangular prism has exactly 5 faces
        assert_eq!(ramp.polys.len(), 5);

        // Check normal orientations
        let bottom = ramp.polys.iter().find(|p| p.plane.normal.z < -0.9).expect("bottom face (-Z)");
        assert_eq!(bottom.vertices.len(), 4);

        let back = ramp.polys.iter().find(|p| p.plane.normal.y > 0.9).expect("back face (+Y)");
        assert_eq!(back.vertices.len(), 4);

        let left = ramp.polys.iter().find(|p| p.plane.normal.x < -0.9).expect("left face (-X)");
        assert_eq!(left.vertices.len(), 3);

        let right = ramp.polys.iter().find(|p| p.plane.normal.x > 0.9).expect("right face (+X)");
        assert_eq!(right.vertices.len(), 3);

        let slope = ramp.polys.iter().find(|p| p.plane.normal.z > 0.1 && p.plane.normal.y < -0.1).expect("slope face (+Z, -Y)");
        assert_eq!(slope.vertices.len(), 4);

        // Watertight vertex bound check
        let (bmin, bmax) = ramp.world_bounds();
        let bsize = bmax - bmin;
        assert!((bsize.x - size.x).abs() < 1e-3);
        assert!((bsize.y - size.y).abs() < 1e-3);
        assert!((bsize.z - size.z).abs() < 1e-3);

        // Verify all 4 cardinal directions are strictly convex (center is on the inside of all 5 planes)
        for dir in [
            RampDirection::North,
            RampDirection::South,
            RampDirection::East,
            RampDirection::West,
        ] {
            let r = build_ramp_oriented("OrientedRamp", CsgOperation::Add, size, dir);
            assert_eq!(r.polys.len(), 5);
            let centroid = Vec3::new(0.0, 0.0, -size.z * 0.2);
            for poly in &r.polys {
                assert!(
                    poly.plane.dist_to_point(centroid) < -1.0,
                    "Centroid must be inside all faces for {:?}, got dist={}",
                    dir,
                    poly.plane.dist_to_point(centroid)
                );
            }
        }
    }

    #[test]
    fn test_build_aligned_octagon_convexity_and_grid_snap() {
        let size = Vec3::new(1024.0, 1024.0, 512.0);
        let oct = build_aligned_octagon("OctRoom", CsgOperation::Subtract, size);
        // 2 caps + 8 side quads = 10 faces
        assert_eq!(oct.polys.len(), 10);
        for poly in &oct.polys {
            assert!(
                poly.plane.dist_to_point(Vec3::ZERO) < -100.0,
                "Origin must be strictly inside octagon"
            );
            for v in &poly.vertices {
                assert!((v.x % 16.0).abs() < 1e-4, "Vertex X not snapped to 16 UU: {}", v.x);
                assert!((v.y % 16.0).abs() < 1e-4, "Vertex Y not snapped to 16 UU: {}", v.y);
                assert!((v.z % 16.0).abs() < 1e-4, "Vertex Z not snapped to 16 UU: {}", v.z);
            }
        }
        // Corner outside chamfer must be rejected by contains_world_point
        assert!(oct.contains_world_point(Vec3::ZERO));
        assert!(!oct.contains_world_point(Vec3::new(490.0, 490.0, 0.0)));
    }

    #[test]
    fn test_build_stepped_stairs_oriented_watertight_and_cardinal() {
        let size = Vec3::new(256.0, 192.0, 128.0);
        let step_count = 8;

        for dir in [
            RampDirection::North,
            RampDirection::South,
            RampDirection::East,
            RampDirection::West,
        ] {
            let stairs = build_stepped_stairs_oriented(
                "TestStairs",
                CsgOperation::Add,
                size,
                dir,
                step_count,
            );
            // Bottom (1) + Back (1) + 8 steps * 4 faces = 34 polygons
            assert_eq!(stairs.polys.len(), 1 + 1 + step_count * 4);

            let (bmin, bmax) = stairs.world_bounds();
            let bsize = bmax - bmin;
            assert!((bsize.x - size.x).abs() < 1e-3, "dir {:?} bsize.x: {}", dir, bsize.x);
            assert!((bsize.y - size.y).abs() < 1e-3, "dir {:?} bsize.y: {}", dir, bsize.y);
            assert!((bsize.z - size.z).abs() < 1e-3, "dir {:?} bsize.z: {}", dir, bsize.z);

            // All polygons must have non-zero area and valid outward normals
            let (expected_riser_normal, expected_back_normal) = match dir {
                RampDirection::North => (Vec3::new(0.0, -1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)),
                RampDirection::South => (Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, -1.0, 0.0)),
                RampDirection::East => (Vec3::new(-1.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)),
                RampDirection::West => (Vec3::new(1.0, 0.0, 0.0), Vec3::new(-1.0, 0.0, 0.0)),
            };

            for (p_idx, poly) in stairs.polys.iter().enumerate() {
                assert!(poly.vertices.len() >= 3, "Poly #{} has < 3 verts", p_idx);
                assert!(poly.plane.normal.length() > 0.99, "Poly #{} has bad normal", p_idx);

                if p_idx == 0 {
                    assert!((poly.plane.normal - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-3, "bottom normal");
                } else if p_idx == stairs.polys.len() - 1 {
                    assert!((poly.plane.normal - expected_back_normal).length() < 1e-3, "back normal for {:?}", dir);
                } else {
                    let step_face = (p_idx - 1) % 4;
                    if step_face == 0 {
                        assert!((poly.plane.normal - Vec3::new(0.0, 0.0, 1.0)).length() < 1e-3, "tread normal");
                    } else if step_face == 1 {
                        assert!((poly.plane.normal - expected_riser_normal).length() < 1e-3, "riser normal for {:?}", dir);
                    }
                }
            }
        }
    }
}



