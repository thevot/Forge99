use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::{Vec2, Vec3};
use std::f32::consts::PI;

/// Sculpting modes for interactive terrain modification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainSculptMode {
    Raise,
    Lower,
    Smooth,
    Flatten,
    Noise,
}

/// Slope classification of terrain surface triangles
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainSlopeCategory {
    FlatWalkable,  // 0 - 25 degrees
    ModerateSlope, // 25 - 45 degrees
    SteepCliff,    // > 45 degrees
}

/// Canonical topographic terrain presets for Unreal Tournament arenas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerrainPreset {
    RollingHillsMeadow,
    RockyCanyonGorge,
    ArenaCraterBasin,
    HighlandsFortressPlateau,
}

impl TerrainPreset {
    pub fn all() -> &'static [TerrainPreset] {
        &[
            TerrainPreset::RollingHillsMeadow,
            TerrainPreset::RockyCanyonGorge,
            TerrainPreset::ArenaCraterBasin,
            TerrainPreset::HighlandsFortressPlateau,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            TerrainPreset::RollingHillsMeadow => "Rolling Hills Meadow",
            TerrainPreset::RockyCanyonGorge => "Rocky Canyon Gorge",
            TerrainPreset::ArenaCraterBasin => "Arena Crater Basin",
            TerrainPreset::HighlandsFortressPlateau => "Highlands Fortress Plateau",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TerrainPreset::RollingHillsMeadow => "Gentle harmonic undulations creating natural open ground for DM/CTF.",
            TerrainPreset::RockyCanyonGorge => "Deep central trench corridor bordered by towering rocky cliff walls.",
            TerrainPreset::ArenaCraterBasin => "Depressed circular bowl arena focusing combat toward a central powerup.",
            TerrainPreset::HighlandsFortressPlateau => "Elevated flat mesa plateau surrounded by defensible slopes.",
        }
    }
}

/// Topographic heightmap terrain grid
#[derive(Debug, Clone, PartialEq)]
pub struct TerrainGrid {
    pub rows: usize,
    pub cols: usize,
    pub cell_size: f32,
    pub base_z: f32,
    pub heights: Vec<f32>, // (rows + 1) * (cols + 1) height values
    pub solid_base: bool,
}

impl Default for TerrainGrid {
    fn default() -> Self {
        let rows = 16;
        let cols = 16;
        let vert_count = (rows + 1) * (cols + 1);
        Self {
            rows,
            cols,
            cell_size: 128.0,
            base_z: 0.0,
            heights: vec![0.0; vert_count],
            solid_base: true,
        }
    }
}

impl TerrainGrid {
    pub fn new(rows: usize, cols: usize, cell_size: f32, base_z: f32) -> Self {
        let vert_count = (rows + 1) * (cols + 1);
        Self {
            rows,
            cols,
            cell_size,
            base_z,
            heights: vec![0.0; vert_count],
            solid_base: true,
        }
    }

    #[inline]
    pub fn vert_idx(&self, r: usize, c: usize) -> usize {
        r * (self.cols + 1) + c
    }

    pub fn get_height(&self, r: usize, c: usize) -> f32 {
        if r <= self.rows && c <= self.cols {
            self.heights[self.vert_idx(r, c)]
        } else {
            0.0
        }
    }

    pub fn set_height(&mut self, r: usize, c: usize, h: f32) {
        if r <= self.rows && c <= self.cols {
            let idx = self.vert_idx(r, c);
            self.heights[idx] = h;
        }
    }

    pub fn vertex_pos(&self, r: usize, c: usize) -> Vec3 {
        let x = (c as f32 - (self.cols as f32) * 0.5) * self.cell_size;
        let y = (r as f32 - (self.rows as f32) * 0.5) * self.cell_size;
        let z = self.base_z + self.get_height(r, c);
        Vec3::new(x, y, z)
    }

    /// Sculpt terrain around a 2D center point with radius and strength
    pub fn sculpt(&mut self, center: Vec2, radius: f32, strength: f32, mode: TerrainSculptMode, target_height: f32) {
        let rad_sq = radius * radius;

        for r in 0..=self.rows {
            for c in 0..=self.cols {
                let pos = self.vertex_pos(r, c);
                let dist_sq = (pos.x - center.x) * (pos.x - center.x) + (pos.y - center.y) * (pos.y - center.y);
                if dist_sq <= rad_sq {
                    let dist = dist_sq.sqrt();
                    // Smooth cosine falloff from 1.0 down to 0.0
                    let falloff = 0.5 * (1.0 + ((dist / radius) * PI).cos());
                    let cur_h = self.get_height(r, c);

                    let new_h = match mode {
                        TerrainSculptMode::Raise => cur_h + strength * falloff,
                        TerrainSculptMode::Lower => cur_h - strength * falloff,
                        TerrainSculptMode::Flatten => {
                            cur_h + (target_height - cur_h) * (strength.clamp(0.0, 1.0) * falloff)
                        }
                        TerrainSculptMode::Smooth => {
                            // Average height of 4-neighborhood
                            let mut sum = cur_h;
                            let mut count = 1.0;
                            if r > 0 { sum += self.get_height(r - 1, c); count += 1.0; }
                            if r < self.rows { sum += self.get_height(r + 1, c); count += 1.0; }
                            if c > 0 { sum += self.get_height(r, c - 1); count += 1.0; }
                            if c < self.cols { sum += self.get_height(r, c + 1); count += 1.0; }
                            let avg = sum / count;
                            cur_h + (avg - cur_h) * (strength.clamp(0.0, 1.0) * falloff)
                        }
                        TerrainSculptMode::Noise => {
                            let freq = 0.05;
                            let n = ((r as f32 * freq).sin() * (c as f32 * freq * 1.3).cos()) * strength;
                            cur_h + n * falloff
                        }
                    };
                    self.set_height(r, c, new_h);
                }
            }
        }
    }

    /// Applies procedural multi-octave harmonic noise to the entire heightmap
    pub fn generate_procedural_noise(&mut self, amplitude: f32, frequency: f32) {
        for r in 0..=self.rows {
            for c in 0..=self.cols {
                let fx = c as f32 * frequency;
                let fy = r as f32 * frequency;

                // 3 octaves of sine-cosine wave combinations
                let o1 = (fx * 1.0).sin() * (fy * 1.0).cos();
                let o2 = (fx * 2.1).cos() * (fy * 1.9).sin() * 0.5;
                let o3 = (fx * 4.3).sin() * (fy * 4.1).sin() * 0.25;

                let h = (o1 + o2 + o3) * amplitude;
                self.set_height(r, c, h);
            }
        }
    }

    /// Applies a canonical topographic preset
    pub fn apply_preset(&mut self, preset: TerrainPreset) {
        let width_span = (self.cols as f32) * self.cell_size;
        let height_span = (self.rows as f32) * self.cell_size;

        match preset {
            TerrainPreset::RollingHillsMeadow => {
                self.generate_procedural_noise(96.0, 0.25);
            }
            TerrainPreset::RockyCanyonGorge => {
                for r in 0..=self.rows {
                    let norm_y = (r as f32 / self.rows as f32) * 2.0 - 1.0;
                    let canyon_wall = norm_y.abs().powf(2.0) * 320.0;
                    for c in 0..=self.cols {
                        let jitter = (c as f32 * 0.8).sin() * 24.0;
                        self.set_height(r, c, canyon_wall + jitter);
                    }
                }
            }
            TerrainPreset::ArenaCraterBasin => {
                let half_span = (width_span.min(height_span)) * 0.45;
                for r in 0..=self.rows {
                    for c in 0..=self.cols {
                        let pos = self.vertex_pos(r, c);
                        let dist = (pos.x * pos.x + pos.y * pos.y).sqrt();
                        let crater = if dist < half_span {
                            -((1.0 - dist / half_span) * 160.0)
                        } else {
                            (dist - half_span).min(96.0) * 0.5
                        };
                        self.set_height(r, c, crater);
                    }
                }
            }
            TerrainPreset::HighlandsFortressPlateau => {
                let plateau_rad = (width_span.min(height_span)) * 0.28;
                for r in 0..=self.rows {
                    for c in 0..=self.cols {
                        let pos = self.vertex_pos(r, c);
                        let dist = (pos.x * pos.x + pos.y * pos.y).sqrt();
                        let h = if dist <= plateau_rad {
                            192.0
                        } else if dist <= plateau_rad + 200.0 {
                            let f = (dist - plateau_rad) / 200.0;
                            192.0 * (1.0 - f)
                        } else {
                            0.0
                        };
                        self.set_height(r, c, h);
                    }
                }
            }
        }
    }

    /// Evaluates the slope angle in degrees of a cell triangle
    pub fn evaluate_triangle_slope(&self, v0: Vec3, v1: Vec3, v2: Vec3) -> (f32, TerrainSlopeCategory) {
        let edge1 = v1 - v0;
        let edge2 = v2 - v0;
        let normal = edge1.cross(edge2).normalize_or_zero();

        // Cosine of angle with UP (Vec3::Z)
        let cos_angle = normal.dot(Vec3::Z).clamp(-1.0, 1.0);
        let slope_deg = cos_angle.acos().to_degrees();

        let category = if slope_deg <= 25.0 {
            TerrainSlopeCategory::FlatWalkable
        } else if slope_deg <= 45.0 {
            TerrainSlopeCategory::ModerateSlope
        } else {
            TerrainSlopeCategory::SteepCliff
        };

        (slope_deg, category)
    }

    /// Returns the minimum and maximum Z bounds of the terrain
    pub fn min_max_elevation(&self) -> (f32, f32) {
        let mut min_z = f32::MAX;
        let mut max_z = f32::MIN;
        for &h in &self.heights {
            let z = self.base_z + h;
            if z < min_z { min_z = z; }
            if z > max_z { max_z = z; }
        }
        if min_z > max_z { (0.0, 0.0) } else { (min_z, max_z) }
    }

    /// Generates all 3D polygons for the terrain surface and optional solid base enclosure
    pub fn to_polygons(&self) -> Vec<Polygon> {
        let mut polys = Vec::new();

        // 1. Surface Triangles (2 per cell)
        for r in 0..self.rows {
            for c in 0..self.cols {
                let v00 = self.vertex_pos(r, c);
                let v10 = self.vertex_pos(r + 1, c);
                let v11 = self.vertex_pos(r + 1, c + 1);
                let v01 = self.vertex_pos(r, c + 1);

                // Triangle 1: (v00, v10, v11)
                if let Some(p) = Polygon::new(vec![v00, v10, v11]) {
                    polys.push(p);
                }
                // Triangle 2: (v00, v11, v01)
                if let Some(p) = Polygon::new(vec![v00, v11, v01]) {
                    polys.push(p);
                }
            }
        }

        // 2. Solid Base Foundation (Skirt walls and bottom floor)
        if self.solid_base {
            let (min_elev, _) = self.min_max_elevation();
            let floor_z = min_elev - 96.0;

            // South skirt (-Y: r = 0)
            for c in 0..self.cols {
                let t0 = self.vertex_pos(0, c);
                let t1 = self.vertex_pos(0, c + 1);
                let b0 = Vec3::new(t0.x, t0.y, floor_z);
                let b1 = Vec3::new(t1.x, t1.y, floor_z);
                if let Some(p) = Polygon::new(vec![t0, b0, b1, t1]) { polys.push(p); }
            }

            // North skirt (+Y: r = rows)
            for c in 0..self.cols {
                let t0 = self.vertex_pos(self.rows, c);
                let t1 = self.vertex_pos(self.rows, c + 1);
                let b0 = Vec3::new(t0.x, t0.y, floor_z);
                let b1 = Vec3::new(t1.x, t1.y, floor_z);
                if let Some(p) = Polygon::new(vec![t1, b1, b0, t0]) { polys.push(p); }
            }

            // West skirt (-X: c = 0)
            for r in 0..self.rows {
                let t0 = self.vertex_pos(r, 0);
                let t1 = self.vertex_pos(r + 1, 0);
                let b0 = Vec3::new(t0.x, t0.y, floor_z);
                let b1 = Vec3::new(t1.x, t1.y, floor_z);
                if let Some(p) = Polygon::new(vec![t1, b1, b0, t0]) { polys.push(p); }
            }

            // East skirt (+X: c = cols)
            for r in 0..self.rows {
                let t0 = self.vertex_pos(r, self.cols);
                let t1 = self.vertex_pos(r + 1, self.cols);
                let b0 = Vec3::new(t0.x, t0.y, floor_z);
                let b1 = Vec3::new(t1.x, t1.y, floor_z);
                if let Some(p) = Polygon::new(vec![t0, b0, b1, t1]) { polys.push(p); }
            }

            // Bottom floor rectangle
            let sw = self.vertex_pos(0, 0);
            let se = self.vertex_pos(0, self.cols);
            let ne = self.vertex_pos(self.rows, self.cols);
            let nw = self.vertex_pos(self.rows, 0);
            let fb_sw = Vec3::new(sw.x, sw.y, floor_z);
            let fb_se = Vec3::new(se.x, se.y, floor_z);
            let fb_ne = Vec3::new(ne.x, ne.y, floor_z);
            let fb_nw = Vec3::new(nw.x, nw.y, floor_z);
            if let Some(p) = Polygon::new(vec![fb_sw, fb_se, fb_ne, fb_nw]) {
                polys.push(p);
            }
        }

        polys
    }

    /// Builds a ready-to-use CSG Brush from the terrain
    pub fn to_brush(&self, name: &str, csg_oper: CsgOperation) -> Brush {
        let mut brush = Brush::new(name, csg_oper);
        brush.polys = self.to_polygons();
        brush
    }
}
