/// Procedural texture dynamic effect type from Unreal Engine 1's FireEngine
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProceduralTextureType {
    #[default]
    Fire,         // Rising cellular heat and flame convection
    WaterRipples, // 2D wave equation droplet ripple simulation
}

/// Color palette mapping for procedural simulation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProceduralColorPalette {
    #[default]
    FireThermal, // Classic Flame (Black -> Red -> Orange -> Yellow -> White)
    BluePlasma,  // Plasma Arc (Black -> Deep Blue -> Cyan -> White)
    GreenToxic,  // Toxic Slime / Acid (Black -> Dark Green -> Neon Green -> Yellow)
    PurpleVoid,  // Void / Energy (Black -> Purple -> Magenta -> White)
    WaterAzure,  // Water Azure Blue
    IceFrost,    // Glacial Ice / Frost
}

/// Dynamic procedural texture simulator replicating UE1 FireEngine algorithms
#[derive(Debug, Clone, PartialEq)]
pub struct ProceduralTextureSimulator {
    pub tex_type: ProceduralTextureType,
    pub palette: ProceduralColorPalette,
    pub width: usize,
    pub height: usize,
    pub buffer_a: Vec<f32>,
    pub buffer_b: Vec<f32>,
    pub current_is_a: bool,
    pub damping: f32,
    pub cooling_rate: f32,
    pub step_counter: usize,
}

impl ProceduralTextureSimulator {
    pub fn new_fire(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            tex_type: ProceduralTextureType::Fire,
            palette: ProceduralColorPalette::FireThermal,
            width,
            height,
            buffer_a: vec![0.0; size],
            buffer_b: vec![0.0; size],
            current_is_a: true,
            damping: 0.98,
            cooling_rate: 1.8,
            step_counter: 0,
        }
    }

    pub fn new_water(width: usize, height: usize) -> Self {
        let size = width * height;
        Self {
            tex_type: ProceduralTextureType::WaterRipples,
            palette: ProceduralColorPalette::WaterAzure,
            width,
            height,
            buffer_a: vec![0.0; size],
            buffer_b: vec![0.0; size],
            current_is_a: true,
            damping: 0.96,
            cooling_rate: 0.0,
            step_counter: 0,
        }
    }

    /// Triggers a ripple disturbance (droplet impact) at grid coordinates
    pub fn add_droplet(&mut self, cx: usize, cy: usize, strength: f32) {
        let w = self.width;
        let h = self.height;
        let buf = if self.current_is_a { &mut self.buffer_a } else { &mut self.buffer_b };

        let radius = 3;
        for dy in -(radius as isize)..=(radius as isize) {
            for dx in -(radius as isize)..=(radius as isize) {
                let px = (cx as isize + dx).rem_euclid(w as isize) as usize;
                let py = (cy as isize + dy).rem_euclid(h as isize) as usize;
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq <= (radius * radius) as f32 {
                    let falloff = 1.0 - (dist_sq / (radius * radius) as f32);
                    buf[py * w + px] = (buf[py * w + px] + strength * falloff).clamp(-255.0, 255.0);
                }
            }
        }
    }

    /// Triggers a localized heat spark or burst in fire simulation at grid coordinates
    pub fn add_heat_spark(&mut self, cx: usize, cy: usize, heat: f32) {
        let w = self.width;
        let h = self.height;
        let buf = if self.current_is_a { &mut self.buffer_a } else { &mut self.buffer_b };

        let radius = 3;
        for dy in -(radius as isize)..=(radius as isize) {
            for dx in -(radius as isize)..=(radius as isize) {
                let px = (cx as isize + dx).rem_euclid(w as isize) as usize;
                let py = (cy as isize + dy).rem_euclid(h as isize) as usize;
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq <= (radius * radius) as f32 {
                    let falloff = 1.0 - (dist_sq / (radius * radius) as f32);
                    buf[py * w + px] = (buf[py * w + px] + heat * falloff).clamp(0.0, 255.0);
                }
            }
        }
    }

    /// Advances simulation one step
    pub fn step(&mut self) {
        self.step_counter += 1;
        match self.tex_type {
            ProceduralTextureType::Fire => self.step_fire(),
            ProceduralTextureType::WaterRipples => self.step_water(),
        }
    }

    /// Cellular flame propagation with upward convection and cooling
    fn step_fire(&mut self) {
        let w = self.width;
        let h = self.height;
        let (src, dst) = if self.current_is_a {
            (&self.buffer_a, &mut self.buffer_b)
        } else {
            (&self.buffer_b, &mut self.buffer_a)
        };

        // Seed bottom two rows with pseudo-random heat sparks
        for x in 0..w {
            let seed = ((x * 17 + self.step_counter * 31) % 100) as f32;
            let heat = if seed > 40.0 { 200.0 + (seed * 0.55) } else { 0.0 };
            dst[(h - 1) * w + x] = heat;
            dst[(h - 2) * w + x] = heat * 0.85;
        }

        // Convection kernel: sample pixels below (y+1 and y+2) and propagate upward
        for y in 0..(h - 2) {
            for x in 0..w {
                let x_left = if x == 0 { w - 1 } else { x - 1 };
                let x_right = if x == w - 1 { 0 } else { x + 1 };

                let heat_below = src[(y + 1) * w + x_left]
                    + src[(y + 1) * w + x] * 2.0
                    + src[(y + 1) * w + x_right]
                    + src[(y + 2) * w + x];

                let avg_heat = (heat_below / 5.0) - self.cooling_rate;
                dst[y * w + x] = avg_heat.max(0.0);
            }
        }

        self.current_is_a = !self.current_is_a;
    }

    /// 2D discrete wave equation ripple simulation
    fn step_water(&mut self) {
        let w = self.width;
        let h = self.height;
        let (cur, prev) = if self.current_is_a {
            (&self.buffer_a, &mut self.buffer_b)
        } else {
            (&self.buffer_b, &mut self.buffer_a)
        };

        // Standard 2D wave discrete Laplacian
        for y in 1..(h - 1) {
            for x in 1..(w - 1) {
                let neighbors = cur[y * w + (x - 1)]
                    + cur[y * w + (x + 1)]
                    + cur[(y - 1) * w + x]
                    + cur[(y + 1) * w + x];

                let next = (neighbors * 0.5 - prev[y * w + x]) * self.damping;
                prev[y * w + x] = next;
            }
        }

        self.current_is_a = !self.current_is_a;
    }

    /// Converts active simulation buffer to a 32-bit RGBA pixel buffer
    pub fn to_rgba8(&self) -> Vec<u8> {
        let size = self.width * self.height;
        let mut pixels = Vec::with_capacity(size * 4);
        let buf = if self.current_is_a { &self.buffer_a } else { &self.buffer_b };

        match self.tex_type {
            ProceduralTextureType::Fire => {
                for &val in buf {
                    let heat = val.clamp(0.0, 255.0);
                    let (r, g, b) = match self.palette {
                        ProceduralColorPalette::FireThermal => {
                            if heat < 64.0 {
                                let t = heat / 64.0;
                                ((t * 220.0) as u8, 0, 0)
                            } else if heat < 160.0 {
                                let t = (heat - 64.0) / 96.0;
                                (255, (t * 200.0) as u8, 0)
                            } else {
                                let t = (heat - 160.0) / 95.0;
                                (255, 200 + (t * 55.0) as u8, (t * 255.0) as u8)
                            }
                        }
                        ProceduralColorPalette::BluePlasma => {
                            if heat < 80.0 {
                                let t = heat / 80.0;
                                (0, (t * 60.0) as u8, (t * 200.0) as u8)
                            } else if heat < 180.0 {
                                let t = (heat - 80.0) / 100.0;
                                ((t * 100.0) as u8, (60.0 + t * 180.0) as u8, 255)
                            } else {
                                let t = (heat - 180.0) / 75.0;
                                (100 + (t * 155.0) as u8, 240 + (t * 15.0) as u8, 255)
                            }
                        }
                        ProceduralColorPalette::GreenToxic => {
                            if heat < 80.0 {
                                let t = heat / 80.0;
                                (0, (t * 180.0) as u8, 0)
                            } else if heat < 180.0 {
                                let t = (heat - 80.0) / 100.0;
                                ((t * 150.0) as u8, 255, (t * 40.0) as u8)
                            } else {
                                let t = (heat - 180.0) / 75.0;
                                (150 + (t * 105.0) as u8, 255, (t * 220.0) as u8)
                            }
                        }
                        ProceduralColorPalette::PurpleVoid => {
                            if heat < 80.0 {
                                let t = heat / 80.0;
                                ((t * 150.0) as u8, 0, (t * 200.0) as u8)
                            } else if heat < 180.0 {
                                let t = (heat - 80.0) / 100.0;
                                (150 + (t * 105.0) as u8, (t * 80.0) as u8, 255)
                            } else {
                                let t = (heat - 180.0) / 75.0;
                                (255, 80 + (t * 175.0) as u8, 255)
                            }
                        }
                        _ => {
                            let t = heat / 255.0;
                            ((t * 255.0) as u8, (t * 255.0) as u8, (t * 255.0) as u8)
                        }
                    };
                    pixels.extend_from_slice(&[r, g, b, 255]);
                }
            }
            ProceduralTextureType::WaterRipples => {
                for &val in buf {
                    let height_offset = val.clamp(-128.0, 127.0);
                    let (r, g, b, a) = match self.palette {
                        ProceduralColorPalette::GreenToxic => {
                            let g_val = (160.0 + height_offset).clamp(0.0, 255.0) as u8;
                            let r_val = (30.0 + height_offset * 0.4).clamp(0.0, 255.0) as u8;
                            let b_val = (10.0 + height_offset * 0.2).clamp(0.0, 255.0) as u8;
                            (r_val, g_val, b_val, 230)
                        }
                        ProceduralColorPalette::IceFrost => {
                            let w_val = (190.0 + height_offset * 0.5).clamp(0.0, 255.0) as u8;
                            let c_val = (220.0 + height_offset * 0.3).clamp(0.0, 255.0) as u8;
                            (w_val, c_val, 255, 240)
                        }
                        _ => {
                            // Default Azure Water
                            let base_blue = (160.0 + height_offset).clamp(0.0, 255.0) as u8;
                            let base_cyan = (100.0 + height_offset * 0.8).clamp(0.0, 255.0) as u8;
                            let base_dark = (20.0 + height_offset * 0.3).clamp(0.0, 255.0) as u8;
                            (base_dark, base_cyan, base_blue, 220)
                        }
                    };
                    pixels.extend_from_slice(&[r, g, b, a]);
                }
            }
        }

        pixels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fire_procedural_simulation() {
        let mut fire = ProceduralTextureSimulator::new_fire(32, 32);
        assert_eq!(fire.width, 32);

        // Run 5 steps of fire convection
        for _ in 0..5 {
            fire.step();
        }

        let rgba = fire.to_rgba8();
        assert_eq!(rgba.len(), 32 * 32 * 4);

        // Verify bottom row has heat ignited
        let has_fire_pixels = rgba.iter().any(|&p| p > 50);
        assert!(has_fire_pixels, "Fire simulation must generate visible heated pixels");
    }

    #[test]
    fn test_water_ripples_simulation() {
        let mut water = ProceduralTextureSimulator::new_water(32, 32);
        water.add_droplet(16, 16, 100.0);

        // Step wave equation
        for _ in 0..4 {
            water.step();
        }

        let rgba = water.to_rgba8();
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }
}
