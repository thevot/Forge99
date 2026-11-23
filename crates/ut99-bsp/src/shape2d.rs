use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Vec2, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub enum Segment2DType {
    #[default]
    Linear,
    Bezier {
        ctrl1: Vec2,
        ctrl2: Vec2,
    },
}

fn default_bezier_detail() -> usize {
    8
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Shape2D {
    pub points: Vec<Vec2>,
    pub closed: bool,
    #[serde(default)]
    pub segments: Vec<Segment2DType>,
    #[serde(default = "default_bezier_detail")]
    pub bezier_detail: usize,
}

impl Default for Shape2D {
    fn default() -> Self {
        Self::new_box(256.0, 256.0)
    }
}

impl Shape2D {
    pub fn new() -> Self {
        Self {
            points: Vec::new(),
            closed: true,
            segments: Vec::new(),
            bezier_detail: 8,
        }
    }

    pub fn new_box(width: f32, height: f32) -> Self {
        let hw = width * 0.5;
        let hh = height * 0.5;
        Self {
            points: vec![
                Vec2::new(-hw, -hh),
                Vec2::new(hw, -hh),
                Vec2::new(hw, hh),
                Vec2::new(-hw, hh),
            ],
            closed: true,
            segments: vec![Segment2DType::Linear; 4],
            bezier_detail: 8,
        }
    }

    pub fn new_circle(radius: f32, segments: usize) -> Self {
        let segs = segments.max(3);
        let mut points = Vec::with_capacity(segs);
        for i in 0..segs {
            let angle = i as f32 * std::f32::consts::TAU / segs as f32;
            points.push(Vec2::new(radius * angle.cos(), radius * angle.sin()));
        }
        Self {
            points,
            closed: true,
            segments: vec![Segment2DType::Linear; segs],
            bezier_detail: 8,
        }
    }

    pub fn new_arch(width: f32, height: f32, sides: usize) -> Self {
        let segs = sides.max(4);
        let hw = width * 0.5;
        let mut points = Vec::new();
        // Base corners
        points.push(Vec2::new(-hw, 0.0));
        points.push(Vec2::new(hw, 0.0));

        // Upper arch semi-circle
        for i in 0..=segs {
            let t = i as f32 / segs as f32;
            let angle = t * std::f32::consts::PI;
            let x = hw * angle.cos();
            let y = height * angle.sin();
            points.push(Vec2::new(-x, y));
        }
        let n = points.len();
        Self {
            points,
            closed: true,
            segments: vec![Segment2DType::Linear; n],
            bezier_detail: 8,
        }
    }

    pub fn new_cross(size: f32, thickness: f32) -> Self {
        let hs = size * 0.5;
        let ht = thickness * 0.5;
        Self {
            points: vec![
                Vec2::new(-ht, -hs),
                Vec2::new(ht, -hs),
                Vec2::new(ht, -ht),
                Vec2::new(hs, -ht),
                Vec2::new(hs, ht),
                Vec2::new(ht, ht),
                Vec2::new(ht, hs),
                Vec2::new(-ht, hs),
                Vec2::new(-ht, ht),
                Vec2::new(-hs, ht),
                Vec2::new(-hs, -ht),
                Vec2::new(-ht, -ht),
            ],
            closed: true,
            segments: vec![Segment2DType::Linear; 12],
            bezier_detail: 8,
        }
    }

    pub fn sync_segments(&mut self) {
        if self.segments.len() < self.points.len() {
            self.segments.resize(self.points.len(), Segment2DType::Linear);
        } else if self.segments.len() > self.points.len() {
            self.segments.truncate(self.points.len());
        }
    }

    pub fn segment_type(&self, seg_idx: usize) -> Segment2DType {
        self.segments.get(seg_idx).copied().unwrap_or(Segment2DType::Linear)
    }

    pub fn set_segment_bezier(&mut self, seg_idx: usize) {
        self.sync_segments();
        let n = self.points.len();
        if seg_idx >= n || n < 2 {
            return;
        }
        let p0 = self.points[seg_idx];
        let p1 = self.points[(seg_idx + 1) % n];
        let dir = p1 - p0;
        let normal = Vec2::new(-dir.y, dir.x).normalize_or_zero() * (dir.length() * 0.2);
        let ctrl1 = p0.lerp(p1, 1.0 / 3.0) + normal;
        let ctrl2 = p0.lerp(p1, 2.0 / 3.0) + normal;
        self.segments[seg_idx] = Segment2DType::Bezier { ctrl1, ctrl2 };
    }

    pub fn set_segment_linear(&mut self, seg_idx: usize) {
        self.sync_segments();
        if seg_idx < self.segments.len() {
            self.segments[seg_idx] = Segment2DType::Linear;
        }
    }

    /// Splits segment `seg_idx` at its midpoint (`t = 0.5`), inserting a new vertex (UnrealEd 2 `Split Side`).
    pub fn split_segment(&mut self, seg_idx: usize) {
        self.sync_segments();
        let n = self.points.len();
        if seg_idx >= n || n < 2 {
            return;
        }
        let p0 = self.points[seg_idx];
        let p1 = self.points[(seg_idx + 1) % n];
        let mid = match self.segment_type(seg_idx) {
            Segment2DType::Linear => (p0 + p1) * 0.5,
            Segment2DType::Bezier { ctrl1, ctrl2 } => {
                eval_cubic_bezier(p0, ctrl1, ctrl2, p1, 0.5)
            }
        };
        self.points.insert(seg_idx + 1, mid);
        self.segments[seg_idx] = Segment2DType::Linear;
        self.segments.insert(seg_idx + 1, Segment2DType::Linear);
    }

    /// Evaluates all Linear and Bezier segments into a sequence of 2D contour points.
    pub fn tessellated_points(&self) -> Vec<Vec2> {
        let n = self.points.len();
        if n < 2 {
            return self.points.clone();
        }
        let detail = self.bezier_detail.clamp(2, 32);
        let mut out = Vec::with_capacity(n * 2);
        let edge_count = if self.closed { n } else { n - 1 };

        for i in 0..edge_count {
            let p0 = self.points[i];
            let p1 = self.points[(i + 1) % n];
            match self.segment_type(i) {
                Segment2DType::Linear => {
                    out.push(p0);
                }
                Segment2DType::Bezier { ctrl1, ctrl2 } => {
                    for s in 0..detail {
                        let t = s as f32 / detail as f32;
                        out.push(eval_cubic_bezier(p0, ctrl1, ctrl2, p1, t));
                    }
                }
            }
        }
        if !self.closed {
            if let Some(&last) = self.points.last() {
                out.push(last);
            }
        }
        out
    }

    pub fn scale_shape(&mut self, factor: f32) {
        let c = self.center();
        for p in &mut self.points {
            *p = c + (*p - c) * factor;
        }
        for seg in &mut self.segments {
            if let Segment2DType::Bezier { ctrl1, ctrl2 } = seg {
                *ctrl1 = c + (*ctrl1 - c) * factor;
                *ctrl2 = c + (*ctrl2 - c) * factor;
            }
        }
    }

    pub fn rotate_shape(&mut self, angle_deg: f32) {
        let c = self.center();
        let rad = angle_deg.to_radians();
        let (sin_a, cos_a) = rad.sin_cos();
        let rot = |pt: Vec2| -> Vec2 {
            let d = pt - c;
            c + Vec2::new(d.x * cos_a - d.y * sin_a, d.x * sin_a + d.y * cos_a)
        };
        for p in &mut self.points {
            *p = rot(*p);
        }
        for seg in &mut self.segments {
            if let Segment2DType::Bezier { ctrl1, ctrl2 } = seg {
                *ctrl1 = rot(*ctrl1);
                *ctrl2 = rot(*ctrl2);
            }
        }
    }

    pub fn flip(&mut self, horizontal: bool) {
        let c = self.center();
        for p in &mut self.points {
            if horizontal {
                p.x = c.x - (p.x - c.x);
            } else {
                p.y = c.y - (p.y - c.y);
            }
        }
        for seg in &mut self.segments {
            if let Segment2DType::Bezier { ctrl1, ctrl2 } = seg {
                if horizontal {
                    ctrl1.x = c.x - (ctrl1.x - c.x);
                    ctrl2.x = c.x - (ctrl2.x - c.x);
                } else {
                    ctrl1.y = c.y - (ctrl1.y - c.y);
                    ctrl2.y = c.y - (ctrl2.y - c.y);
                }
            }
        }
    }

    pub fn center(&self) -> Vec2 {
        if self.points.is_empty() {
            return Vec2::ZERO;
        }
        let sum: Vec2 = self.points.iter().copied().sum();
        sum / (self.points.len() as f32)
    }

    /// Extrudes the 2D shape along the Z axis by `depth` to create a 3D Brush.
    /// Concave 2D profiles are automatically partitioned into convex cap triangles via ear-clipping.
    pub fn extrude(&self, depth: f32) -> Brush {
        let pts = self.tessellated_points();
        if pts.len() < 3 {
            return Brush::new("2DShape_Extrude", CsgOperation::Active);
        }

        let half_d = depth * 0.5;
        let n = pts.len();
        let mut polys = Vec::new();

        // 1. Bottom Face (Z = -half_d, normal points down -Z)
        polys.extend(build_cap_polygons(&pts, -half_d, true));

        // 2. Top Face (Z = +half_d, normal points up +Z)
        polys.extend(build_cap_polygons(&pts, half_d, false));

        // 3. Side Quads
        for i in 0..n {
            let next = (i + 1) % n;
            let p0 = pts[i];
            let p1 = pts[next];

            let quad = vec![
                Vec3::new(p0.x, p0.y, -half_d),
                Vec3::new(p1.x, p1.y, -half_d),
                Vec3::new(p1.x, p1.y, half_d),
                Vec3::new(p0.x, p0.y, half_d),
            ];
            if let Some(poly) = Polygon::new(quad) {
                polys.push(poly);
            }
        }

        let mut brush = Brush::new("2DShape_Extruded", CsgOperation::Active);
        brush.polys = polys;
        brush
    }

    /// Extrudes the 2D shape to an apex point (Pyramid/Cone) at `height`.
    pub fn extrude_to_point(&self, height: f32) -> Brush {
        let pts = self.tessellated_points();
        if pts.len() < 3 {
            return Brush::new("2DShape_Pyramid", CsgOperation::Active);
        }

        let n = pts.len();
        let mut polys = Vec::new();
        let c = self.center();
        let apex = Vec3::new(c.x, c.y, height);

        // Base
        polys.extend(build_cap_polygons(&pts, 0.0, true));

        // Sides
        for i in 0..n {
            let next = (i + 1) % n;
            let p0 = pts[i];
            let p1 = pts[next];

            let tri = vec![
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
                apex,
            ];
            if let Some(poly) = Polygon::new(tri) {
                polys.push(poly);
            }
        }

        let mut brush = Brush::new("2DShape_Pyramid", CsgOperation::Active);
        brush.polys = polys;
        brush
    }

    /// Extrudes the 2D shape with beveled edges (bottom bevel, main body, top bevel).
    pub fn extrude_to_bevel(&self, depth: f32, bevel_depth: f32, bevel_width: f32) -> Brush {
        let pts = self.tessellated_points();
        if pts.len() < 3 {
            return self.extrude(depth);
        }

        let c = self.center();
        let half_d = depth * 0.5;
        let b_depth = bevel_depth.min(half_d * 0.8);
        let n = pts.len();
        let mut polys = Vec::new();

        // Inset points for bevel
        let inset_points: Vec<Vec2> = pts
            .iter()
            .map(|p| {
                let dir = (*p - c).normalize_or_zero();
                *p - dir * bevel_width
            })
            .collect();

        // 1. Bottom Cap (inset, Z = -half_d)
        polys.extend(build_cap_polygons(&inset_points, -half_d, true));

        // 2. Lower Bevel Quads
        let z_lower = -half_d + b_depth;
        for i in 0..n {
            let next = (i + 1) % n;
            let quad = vec![
                Vec3::new(inset_points[i].x, inset_points[i].y, -half_d),
                Vec3::new(inset_points[next].x, inset_points[next].y, -half_d),
                Vec3::new(pts[next].x, pts[next].y, z_lower),
                Vec3::new(pts[i].x, pts[i].y, z_lower),
            ];
            if let Some(poly) = Polygon::new(quad) {
                polys.push(poly);
            }
        }

        // 3. Middle Straight Quads
        let z_upper = half_d - b_depth;
        for i in 0..n {
            let next = (i + 1) % n;
            let quad = vec![
                Vec3::new(pts[i].x, pts[i].y, z_lower),
                Vec3::new(pts[next].x, pts[next].y, z_lower),
                Vec3::new(pts[next].x, pts[next].y, z_upper),
                Vec3::new(pts[i].x, pts[i].y, z_upper),
            ];
            if let Some(poly) = Polygon::new(quad) {
                polys.push(poly);
            }
        }

        // 4. Upper Bevel Quads
        for i in 0..n {
            let next = (i + 1) % n;
            let quad = vec![
                Vec3::new(pts[i].x, pts[i].y, z_upper),
                Vec3::new(pts[next].x, pts[next].y, z_upper),
                Vec3::new(inset_points[next].x, inset_points[next].y, half_d),
                Vec3::new(inset_points[i].x, inset_points[i].y, half_d),
            ];
            if let Some(poly) = Polygon::new(quad) {
                polys.push(poly);
            }
        }

        // 5. Top Cap (inset, Z = +half_d)
        polys.extend(build_cap_polygons(&inset_points, half_d, false));

        let mut brush = Brush::new("2DShape_Beveled", CsgOperation::Active);
        brush.polys = polys;
        brush
    }

    /// Revolves (lathes) the 2D contour around the Y axis by `angle_degrees`.
    /// Points: X = radius from axis, Y = height along axis.
    pub fn revolve(&self, angle_degrees: f32, segments: usize, axis_offset: f32) -> Brush {
        let pts = self.tessellated_points();
        if pts.len() < 2 {
            return Brush::new("2DShape_Revolve", CsgOperation::Active);
        }

        let segs = segments.max(3);
        let max_rad = (angle_degrees.abs() % 360.001).to_radians();
        let full_circle = (angle_degrees.abs() - 360.0).abs() < 0.1;
        let num_steps = if full_circle { segs } else { segs + 1 };
        let n_pts = pts.len();

        let mut grid: Vec<Vec<Vec3>> = Vec::with_capacity(num_steps);

        for step in 0..num_steps {
            let t = if full_circle {
                step as f32 / segs as f32
            } else {
                step as f32 / (segs as f32)
            };
            let theta = t * max_rad;
            let cos_t = theta.cos();
            let sin_t = theta.sin();

            let ring: Vec<Vec3> = pts
                .iter()
                .map(|p| {
                    let r = p.x + axis_offset;
                    let z = p.y;
                    // Revolve around Z axis in XY plane
                    Vec3::new(r * cos_t, r * sin_t, z)
                })
                .collect();
            grid.push(ring);
        }

        let mut polys = Vec::new();

        let step_count = if full_circle { segs } else { segs };
        for step in 0..step_count {
            let next_step = (step + 1) % num_steps;
            for i in 0..(n_pts - 1) {
                let v00 = grid[step][i];
                let v10 = grid[next_step][i];
                let v11 = grid[next_step][i + 1];
                let v01 = grid[step][i + 1];

                let quad = vec![v00, v10, v11, v01];
                if let Some(poly) = Polygon::new(quad) {
                    polys.push(poly);
                }
            }
        }

        // If not full circle, add end caps
        if !full_circle && grid.len() >= 2 {
            let start_cap: Vec<Vec3> = grid[0].clone();
            if let Some(poly) = Polygon::new(start_cap) {
                polys.push(poly);
            }
            let mut end_cap: Vec<Vec3> = grid[grid.len() - 1].clone();
            end_cap.reverse();
            if let Some(poly) = Polygon::new(end_cap) {
                polys.push(poly);
            }
        }

        let mut brush = Brush::new("2DShape_Revolve", CsgOperation::Active);
        brush.polys = polys;
        brush
    }

    /// Converts 2D shape into a flat two-sided 2D sheet brush.
    pub fn to_sheet(&self) -> Brush {
        let pts = self.tessellated_points();
        if pts.len() < 3 {
            return Brush::new("2DShape_Sheet", CsgOperation::Active);
        }

        let mut polys = Vec::new();
        polys.extend(build_cap_polygons(&pts, 0.0, false));
        polys.extend(build_cap_polygons(&pts, 0.0, true));

        let mut brush = Brush::new("2DShape_Sheet", CsgOperation::Active);
        brush.polys = polys;
        brush
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_json(json_str: &str) -> Result<Self, serde_json::Error> {
        let mut s: Self = serde_json::from_str(json_str)?;
        s.sync_segments();
        Ok(s)
    }

    /// Exports the 2D shape to UnrealEd 2 `.2ds` text format.
    pub fn to_2ds(&self) -> String {
        let mut out = String::from("// UnrealEd 2.0 2D Shape File (.2ds)\n");
        out.push_str(&format!("Closed {}\n", if self.closed { 1 } else { 0 }));
        out.push_str(&format!("Detail {}\n", self.bezier_detail));
        out.push_str(&format!("NumVertices {}\n", self.points.len()));
        for (i, pt) in self.points.iter().enumerate() {
            match self.segment_type(i) {
                Segment2DType::Linear => {
                    out.push_str(&format!("Vertex {:.4} {:.4} Linear\n", pt.x, pt.y));
                }
                Segment2DType::Bezier { ctrl1, ctrl2 } => {
                    out.push_str(&format!(
                        "Vertex {:.4} {:.4} Bezier {:.4} {:.4} {:.4} {:.4}\n",
                        pt.x, pt.y, ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y
                    ));
                }
            }
        }
        out
    }

    /// Parses a `.2ds` file (supporting both native `.2ds` syntax and JSON `.2ds` files).
    pub fn from_2ds(content: &str) -> Result<Self, String> {
        let trimmed = content.trim();
        if trimmed.starts_with('{') {
            return Self::from_json(trimmed).map_err(|e| e.to_string());
        }
        let mut shape = Shape2D::new();
        for line in trimmed.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }
            match parts[0].to_ascii_lowercase().as_str() {
                "closed" if parts.len() >= 2 => {
                    shape.closed = parts[1] != "0" && !parts[1].eq_ignore_ascii_case("false");
                }
                "detail" if parts.len() >= 2 => {
                    if let Ok(d) = parts[1].parse::<usize>() {
                        shape.bezier_detail = d.clamp(2, 32);
                    }
                }
                "vertex" if parts.len() >= 3 => {
                    let x = parts[1].parse::<f32>().unwrap_or(0.0);
                    let y = parts[2].parse::<f32>().unwrap_or(0.0);
                    shape.points.push(Vec2::new(x, y));
                    if parts.len() >= 8 && parts[3].eq_ignore_ascii_case("bezier") {
                        let c1x = parts[4].parse::<f32>().unwrap_or(x);
                        let c1y = parts[5].parse::<f32>().unwrap_or(y);
                        let c2x = parts[6].parse::<f32>().unwrap_or(x);
                        let c2y = parts[7].parse::<f32>().unwrap_or(y);
                        shape.segments.push(Segment2DType::Bezier {
                            ctrl1: Vec2::new(c1x, c1y),
                            ctrl2: Vec2::new(c2x, c2y),
                        });
                    } else {
                        shape.segments.push(Segment2DType::Linear);
                    }
                }
                _ => {}
            }
        }
        if shape.points.is_empty() {
            return Err("No vertices found in .2ds content".into());
        }
        shape.sync_segments();
        Ok(shape)
    }
}

fn signed_area_2d(pts: &[Vec2]) -> f32 {
    let n = pts.len();
    if n < 3 {
        return 0.0;
    }
    let mut sum = 0.0f32;
    for i in 0..n {
        let p0 = pts[i];
        let p1 = pts[(i + 1) % n];
        sum += p0.x * p1.y - p1.x * p0.y;
    }
    0.5 * sum
}

/// Returns true if the 2D polygon is strictly or weakly convex (no reflex vertices).
pub fn is_convex_2d(pts: &[Vec2]) -> bool {
    let n = pts.len();
    if n <= 3 {
        return true;
    }
    let sign = if signed_area_2d(pts) >= 0.0 { 1.0f32 } else { -1.0f32 };
    for i in 0..n {
        let a = pts[i];
        let b = pts[(i + 1) % n];
        let c = pts[(i + 2) % n];
        let cross = (b - a).perp_dot(c - b) * sign;
        if cross < -1e-3 {
            return false;
        }
    }
    true
}

fn point_in_triangle_2d(p: Vec2, a: Vec2, b: Vec2, c: Vec2) -> bool {
    let d1 = (b - a).perp_dot(p - a);
    let d2 = (c - b).perp_dot(p - b);
    let d3 = (a - c).perp_dot(p - c);
    let has_neg = (d1 < -1e-4) || (d2 < -1e-4) || (d3 < -1e-4);
    let has_pos = (d1 > 1e-4) || (d2 > 1e-4) || (d3 > 1e-4);
    !(has_neg && has_pos)
}

/// Emits a single cap polygon if `pts` is already convex, or ear-clipped convex triangles if `pts` is concave.
fn build_cap_polygons(pts: &[Vec2], z: f32, flip_winding: bool) -> Vec<Polygon> {
    if pts.len() < 3 {
        return Vec::new();
    }
    if is_convex_2d(pts) {
        let mut verts: Vec<Vec3> = pts.iter().map(|p| Vec3::new(p.x, p.y, z)).collect();
        if flip_winding {
            verts.reverse();
        }
        return Polygon::new(verts).into_iter().collect();
    }

    // Ensure CCW order for ear-clipping
    let mut indices: Vec<usize> = (0..pts.len()).collect();
    let is_ccw = signed_area_2d(pts) >= 0.0;
    if !is_ccw {
        indices.reverse();
    }

    let mut polys = Vec::with_capacity(pts.len().saturating_sub(2));
    let mut guard = 0usize;
    let max_iters = pts.len() * pts.len() + 8;

    while indices.len() > 3 && guard < max_iters {
        guard += 1;
        let m = indices.len();
        let mut ear_pos: Option<usize> = None;

        for idx in 0..m {
            let i_prev = indices[(idx + m - 1) % m];
            let i_curr = indices[idx];
            let i_next = indices[(idx + 1) % m];

            let a = pts[i_prev];
            let b = pts[i_curr];
            let c = pts[i_next];

            let cross = (b - a).perp_dot(c - a);
            if cross <= 1e-4 {
                continue;
            }

            let mut contains_other = false;
            for &other_i in &indices {
                if other_i == i_prev || other_i == i_curr || other_i == i_next {
                    continue;
                }
                if point_in_triangle_2d(pts[other_i], a, b, c) {
                    contains_other = true;
                    break;
                }
            }

            if !contains_other {
                ear_pos = Some(idx);
                break;
            }
        }

        let cut = ear_pos.unwrap_or(0);
        let i_prev = indices[(cut + m - 1) % m];
        let i_curr = indices[cut];
        let i_next = indices[(cut + 1) % m];

        let mut tri = vec![
            Vec3::new(pts[i_prev].x, pts[i_prev].y, z),
            Vec3::new(pts[i_curr].x, pts[i_curr].y, z),
            Vec3::new(pts[i_next].x, pts[i_next].y, z),
        ];
        if flip_winding == is_ccw {
            tri.reverse();
        }
        if let Some(poly) = Polygon::new(tri) {
            polys.push(poly);
        }
        indices.remove(cut);
    }

    if indices.len() == 3 {
        let mut tri = vec![
            Vec3::new(pts[indices[0]].x, pts[indices[0]].y, z),
            Vec3::new(pts[indices[1]].x, pts[indices[1]].y, z),
            Vec3::new(pts[indices[2]].x, pts[indices[2]].y, z),
        ];
        if flip_winding == is_ccw {
            tri.reverse();
        }
        if let Some(poly) = Polygon::new(tri) {
            polys.push(poly);
        }
    }

    polys
}

#[inline]
pub fn eval_cubic_bezier(p0: Vec2, c1: Vec2, c2: Vec2, p1: Vec2, t: f32) -> Vec2 {
    let u = 1.0 - t;
    let tt = t * t;
    let uu = u * u;
    let uuu = uu * u;
    let ttt = tt * t;
    p0 * uuu + c1 * (3.0 * uu * t) + c2 * (3.0 * u * tt) + p1 * ttt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shape2d_extrude() {
        let shape = Shape2D::new_box(128.0, 128.0);
        let brush = shape.extrude(64.0);
        assert_eq!(brush.polys.len(), 6, "Extruded box must have 6 faces");
    }

    #[test]
    fn test_shape2d_concave_l_extrude_produces_convex_caps() {
        // Concave L-shape with 6 vertices (1 reflex interior corner at (0,0))
        let l_pts = vec![
            Vec2::new(-64.0, -64.0),
            Vec2::new(64.0, -64.0),
            Vec2::new(64.0, 0.0),
            Vec2::new(0.0, 0.0),
            Vec2::new(0.0, 64.0),
            Vec2::new(-64.0, 64.0),
        ];
        assert!(!is_convex_2d(&l_pts));
        let shape = Shape2D {
            points: l_pts,
            closed: true,
            segments: vec![Segment2DType::Linear; 6],
            bezier_detail: 8,
        };
        let brush = shape.extrude(64.0);
        // 4 bottom cap triangles + 4 top cap triangles + 6 side quads = 14 convex polygons
        assert_eq!(brush.polys.len(), 14);
        for poly in &brush.polys {
            assert!(poly.vertices.len() <= 4);
        }
    }

    #[test]
    fn test_shape2d_revolve() {
        // Line profile with 3 points
        let shape = Shape2D {
            points: vec![
                Vec2::new(32.0, -64.0),
                Vec2::new(64.0, 0.0),
                Vec2::new(32.0, 64.0),
            ],
            closed: false,
            segments: vec![Segment2DType::Linear; 3],
            bezier_detail: 8,
        };
        let brush = shape.revolve(360.0, 8, 0.0);
        assert!(!brush.polys.is_empty(), "Revolved brush must produce faces");
    }

    #[test]
    fn test_shape2d_pyramid() {
        let shape = Shape2D::new_box(100.0, 100.0);
        let brush = shape.extrude_to_point(150.0);
        assert_eq!(brush.polys.len(), 5, "Pyramid has 1 base + 4 side faces");
    }

    #[test]
    fn test_shape2d_bevel() {
        let shape = Shape2D::new_box(100.0, 100.0);
        let brush = shape.extrude_to_bevel(100.0, 20.0, 10.0);
        assert!(brush.polys.len() > 6, "Beveled shape has top, bottom and bevel quads");
    }

    #[test]
    fn test_shape2d_bezier_and_2ds_roundtrip() {
        let mut shape = Shape2D::new_box(256.0, 256.0);
        shape.set_segment_bezier(0);
        shape.bezier_detail = 6;
        let tess = shape.tessellated_points();
        // 1 Bezier segment (6 steps) + 3 Linear segments (1 step each) = 9 vertices
        assert_eq!(tess.len(), 9);

        let text = shape.to_2ds();
        let loaded = Shape2D::from_2ds(&text).expect("Failed to parse .2ds");
        assert_eq!(loaded.points.len(), 4);
        assert_eq!(loaded.bezier_detail, 6);
        assert!(matches!(loaded.segment_type(0), Segment2DType::Bezier { .. }));
    }
}


