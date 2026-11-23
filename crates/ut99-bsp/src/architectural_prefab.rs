use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use std::f32::consts::PI;

/// Style of vaulted arch curvature
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchStyle {
    RoundRoman,
    GothicPointed,
    SegmentalFlat,
}

/// Architectural style of classical pillar
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillarStyle {
    CylindricalFluted,
    SquareChamfered,
    Hexagonal,
}

/// Type of architectural procedural prefab
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitecturalPrefabType {
    LinearStairs,
    SpiralStairs,
    VaultedArch,
    ClassicPillar,
    IndustrialConduit,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearStairsConfig {
    pub step_count: usize,
    pub step_width: f32,
    pub step_height: f32,
    pub step_depth: f32,
    pub solid_underside: bool,
}

impl Default for LinearStairsConfig {
    fn default() -> Self {
        Self {
            step_count: 8,
            step_width: 256.0,
            step_height: 16.0,
            step_depth: 32.0,
            solid_underside: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpiralStairsConfig {
    pub step_count: usize,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub step_height: f32,
    pub total_angle_degrees: f32,
    pub has_central_pillar: bool,
}

impl Default for SpiralStairsConfig {
    fn default() -> Self {
        Self {
            step_count: 16,
            inner_radius: 32.0,
            outer_radius: 192.0,
            step_height: 16.0,
            total_angle_degrees: 360.0,
            has_central_pillar: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VaultedArchConfig {
    pub arch_style: ArchStyle,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub thickness_depth: f32,
    pub segments: usize,
}

impl Default for VaultedArchConfig {
    fn default() -> Self {
        Self {
            arch_style: ArchStyle::RoundRoman,
            outer_radius: 256.0,
            inner_radius: 192.0,
            thickness_depth: 64.0,
            segments: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassicPillarConfig {
    pub pillar_style: PillarStyle,
    pub height: f32,
    pub radius: f32,
    pub base_height: f32,
    pub capital_height: f32,
    pub entasis_factor: f32,
    pub sides: usize,
}

impl Default for ClassicPillarConfig {
    fn default() -> Self {
        Self {
            pillar_style: PillarStyle::CylindricalFluted,
            height: 384.0,
            radius: 48.0,
            base_height: 32.0,
            capital_height: 32.0,
            entasis_factor: 1.15,
            sides: 12,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IndustrialConduitConfig {
    pub pipe_radius: f32,
    pub length: f32,
    pub bend_angle_degrees: f32,
    pub bend_radius: f32,
    pub has_flanges: bool,
    pub sides: usize,
}

impl Default for IndustrialConduitConfig {
    fn default() -> Self {
        Self {
            pipe_radius: 32.0,
            length: 256.0,
            bend_angle_degrees: 90.0,
            bend_radius: 96.0,
            has_flanges: true,
            sides: 10,
        }
    }
}

/// Unified configuration container for architectural procedural synthesis
#[derive(Debug, Clone, PartialEq)]
pub struct ArchitecturalPrefabConfig {
    pub prefab_type: ArchitecturalPrefabType,
    pub linear_stairs: LinearStairsConfig,
    pub spiral_stairs: SpiralStairsConfig,
    pub vaulted_arch: VaultedArchConfig,
    pub classic_pillar: ClassicPillarConfig,
    pub industrial_conduit: IndustrialConduitConfig,
}

impl Default for ArchitecturalPrefabConfig {
    fn default() -> Self {
        Self {
            prefab_type: ArchitecturalPrefabType::LinearStairs,
            linear_stairs: LinearStairsConfig::default(),
            spiral_stairs: SpiralStairsConfig::default(),
            vaulted_arch: VaultedArchConfig::default(),
            classic_pillar: ClassicPillarConfig::default(),
            industrial_conduit: IndustrialConduitConfig::default(),
        }
    }
}

/// Canonical architectural presets from legendary Unreal Tournament arenas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchitecturalPrefabPreset {
    TournamentArenaStairs,
    Deck16ToxicVatRing,
    FaceTowerSpiral,
    GothicCathedralArch,
    FortressKeepPillar,
    IndustrialSteamConduit,
}

impl ArchitecturalPrefabPreset {
    pub fn all() -> &'static [ArchitecturalPrefabPreset] {
        &[
            ArchitecturalPrefabPreset::TournamentArenaStairs,
            ArchitecturalPrefabPreset::Deck16ToxicVatRing,
            ArchitecturalPrefabPreset::FaceTowerSpiral,
            ArchitecturalPrefabPreset::GothicCathedralArch,
            ArchitecturalPrefabPreset::FortressKeepPillar,
            ArchitecturalPrefabPreset::IndustrialSteamConduit,
        ]
    }

    pub fn title(&self) -> &'static str {
        match self {
            ArchitecturalPrefabPreset::TournamentArenaStairs => "Tournament Arena Grand Access (8 Steps)",
            ArchitecturalPrefabPreset::Deck16ToxicVatRing => "Deck16 Slime Vat Semicircular Ring Arch",
            ArchitecturalPrefabPreset::FaceTowerSpiral => "CTF-Face Sniper Tower Spiral Staircase",
            ArchitecturalPrefabPreset::GothicCathedralArch => "Gothic Cathedral Pointed Vault Arch",
            ArchitecturalPrefabPreset::FortressKeepPillar => "Classic Citadel Fluted Pillar with Entasis",
            ArchitecturalPrefabPreset::IndustrialSteamConduit => "Industrial Heavy Steam 90° Conduit Elbow",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            ArchitecturalPrefabPreset::TournamentArenaStairs => "Standard competition stair flight (16 uu rise, 32 uu run, 256 uu width) preventing bot movement stumbles.",
            ArchitecturalPrefabPreset::Deck16ToxicVatRing => "Curved elevated arching rim designed to frame circular acid vats and lava pits.",
            ArchitecturalPrefabPreset::FaceTowerSpiral => "Compact 360° helical staircase with central support core for narrow sniper towers.",
            ArchitecturalPrefabPreset::GothicCathedralArch => "Soaring pointed arch structure inspired by medieval crypts and cathedral naves.",
            ArchitecturalPrefabPreset::FortressKeepPillar => "Monumental cylindrical column with swelling entasis, square plinth and molded capital.",
            ArchitecturalPrefabPreset::IndustrialSteamConduit => "Heavy pressure pipeline bend with bolted structural connection flanges.",
        }
    }

    pub fn create_config(&self) -> ArchitecturalPrefabConfig {
        let mut cfg = ArchitecturalPrefabConfig::default();
        match self {
            ArchitecturalPrefabPreset::TournamentArenaStairs => {
                cfg.prefab_type = ArchitecturalPrefabType::LinearStairs;
                cfg.linear_stairs = LinearStairsConfig {
                    step_count: 8,
                    step_width: 256.0,
                    step_height: 16.0,
                    step_depth: 32.0,
                    solid_underside: true,
                };
            }
            ArchitecturalPrefabPreset::Deck16ToxicVatRing => {
                cfg.prefab_type = ArchitecturalPrefabType::VaultedArch;
                cfg.vaulted_arch = VaultedArchConfig {
                    arch_style: ArchStyle::RoundRoman,
                    outer_radius: 320.0,
                    inner_radius: 256.0,
                    thickness_depth: 96.0,
                    segments: 16,
                };
            }
            ArchitecturalPrefabPreset::FaceTowerSpiral => {
                cfg.prefab_type = ArchitecturalPrefabType::SpiralStairs;
                cfg.spiral_stairs = SpiralStairsConfig {
                    step_count: 16,
                    inner_radius: 32.0,
                    outer_radius: 160.0,
                    step_height: 16.0,
                    total_angle_degrees: 360.0,
                    has_central_pillar: true,
                };
            }
            ArchitecturalPrefabPreset::GothicCathedralArch => {
                cfg.prefab_type = ArchitecturalPrefabType::VaultedArch;
                cfg.vaulted_arch = VaultedArchConfig {
                    arch_style: ArchStyle::GothicPointed,
                    outer_radius: 288.0,
                    inner_radius: 224.0,
                    thickness_depth: 64.0,
                    segments: 14,
                };
            }
            ArchitecturalPrefabPreset::FortressKeepPillar => {
                cfg.prefab_type = ArchitecturalPrefabType::ClassicPillar;
                cfg.classic_pillar = ClassicPillarConfig {
                    pillar_style: PillarStyle::CylindricalFluted,
                    height: 384.0,
                    radius: 54.0,
                    base_height: 40.0,
                    capital_height: 40.0,
                    entasis_factor: 1.18,
                    sides: 16,
                };
            }
            ArchitecturalPrefabPreset::IndustrialSteamConduit => {
                cfg.prefab_type = ArchitecturalPrefabType::IndustrialConduit;
                cfg.industrial_conduit = IndustrialConduitConfig {
                    pipe_radius: 36.0,
                    length: 256.0,
                    bend_angle_degrees: 90.0,
                    bend_radius: 112.0,
                    has_flanges: true,
                    sides: 12,
                };
            }
        }
        cfg
    }
}

/// Helper building a 6-sided convex box prism between min and max bounds
fn build_box_polys(min: Vec3, max: Vec3) -> Vec<Polygon> {
    let mut polys = Vec::with_capacity(6);

    // Bottom (-Z)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(min.x, max.y, min.z),
    ]) { polys.push(p); }

    // Top (+Z)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(max.x, min.y, max.z),
    ]) { polys.push(p); }

    // Front (+Y)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(max.x, max.y, min.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(min.x, max.y, max.z),
    ]) { polys.push(p); }

    // Back (-Y)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, min.y, max.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, min.y, min.z),
    ]) { polys.push(p); }

    // Left (-X)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(min.x, min.y, min.z),
        Vec3::new(min.x, max.y, min.z),
        Vec3::new(min.x, max.y, max.z),
        Vec3::new(min.x, min.y, max.z),
    ]) { polys.push(p); }

    // Right (+X)
    if let Some(p) = Polygon::new(vec![
        Vec3::new(max.x, min.y, min.z),
        Vec3::new(max.x, min.y, max.z),
        Vec3::new(max.x, max.y, max.z),
        Vec3::new(max.x, max.y, min.z),
    ]) { polys.push(p); }

    polys
}

/// Helper building a 6-sided wedge prism connecting 4 bottom vertices to 4 top vertices
fn build_wedge_polys(b0: Vec3, b1: Vec3, b2: Vec3, b3: Vec3, t0: Vec3, t1: Vec3, t2: Vec3, t3: Vec3) -> Vec<Polygon> {
    let mut polys = Vec::with_capacity(6);

    // Bottom
    if let Some(p) = Polygon::new(vec![b0, b1, b2, b3]) { polys.push(p); }
    // Top
    if let Some(p) = Polygon::new(vec![t0, t3, t2, t1]) { polys.push(p); }
    // Sides
    if let Some(p) = Polygon::new(vec![b0, t0, t1, b1]) { polys.push(p); }
    if let Some(p) = Polygon::new(vec![b1, t1, t2, b2]) { polys.push(p); }
    if let Some(p) = Polygon::new(vec![b2, t2, t3, b3]) { polys.push(p); }
    if let Some(p) = Polygon::new(vec![b3, t3, t0, b0]) { polys.push(p); }

    polys
}

/// Generates polygons for linear stairs
fn generate_linear_stairs(cfg: &LinearStairsConfig) -> Vec<Polygon> {
    let mut polys = Vec::new();
    let half_w = cfg.step_width * 0.5;

    for i in 0..cfg.step_count {
        let y_start = (i as f32) * cfg.step_depth;
        let y_end = y_start + cfg.step_depth;
        let z_base = if cfg.solid_underside { 0.0 } else { (i as f32) * cfg.step_height };
        let z_top = ((i + 1) as f32) * cfg.step_height;

        let min = Vec3::new(-half_w, y_start, z_base);
        let max = Vec3::new(half_w, y_end, z_top);
        polys.extend(build_box_polys(min, max));
    }
    polys
}

/// Generates polygons for spiral helical stairs
fn generate_spiral_stairs(cfg: &SpiralStairsConfig) -> Vec<Polygon> {
    let mut polys = Vec::new();
    let total_rad = cfg.total_angle_degrees.to_radians();
    let angle_step = total_rad / (cfg.step_count as f32);

    for i in 0..cfg.step_count {
        let a0 = (i as f32) * angle_step;
        let a1 = ((i + 1) as f32) * angle_step;
        let z0 = (i as f32) * cfg.step_height;
        let z1 = z0 + cfg.step_height;

        let b0 = Vec3::new(cfg.inner_radius * a0.cos(), cfg.inner_radius * a0.sin(), z0);
        let b1 = Vec3::new(cfg.outer_radius * a0.cos(), cfg.outer_radius * a0.sin(), z0);
        let b2 = Vec3::new(cfg.outer_radius * a1.cos(), cfg.outer_radius * a1.sin(), z0);
        let b3 = Vec3::new(cfg.inner_radius * a1.cos(), cfg.inner_radius * a1.sin(), z0);

        let t0 = Vec3::new(b0.x, b0.y, z1);
        let t1 = Vec3::new(b1.x, b1.y, z1);
        let t2 = Vec3::new(b2.x, b2.y, z1);
        let t3 = Vec3::new(b3.x, b3.y, z1);

        polys.extend(build_wedge_polys(b0, b1, b2, b3, t0, t1, t2, t3));
    }

    if cfg.has_central_pillar {
        let total_h = (cfg.step_count as f32) * cfg.step_height;
        let pillar_r = cfg.inner_radius * 0.95;
        let sides = 8;
        for s in 0..sides {
            let a0 = (s as f32 / sides as f32) * 2.0 * PI;
            let a1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

            let b0 = Vec3::new(0.0, 0.0, 0.0);
            let b1 = Vec3::new(pillar_r * a0.cos(), pillar_r * a0.sin(), 0.0);
            let b2 = Vec3::new(pillar_r * a1.cos(), pillar_r * a1.sin(), 0.0);

            let t0 = Vec3::new(0.0, 0.0, total_h);
            let t1 = Vec3::new(pillar_r * a0.cos(), pillar_r * a0.sin(), total_h);
            let t2 = Vec3::new(pillar_r * a1.cos(), pillar_r * a1.sin(), total_h);

            if let Some(p) = Polygon::new(vec![b0, b1, b2]) { polys.push(p); }
            if let Some(p) = Polygon::new(vec![t0, t2, t1]) { polys.push(p); }
            if let Some(p) = Polygon::new(vec![b1, t1, t2, b2]) { polys.push(p); }
        }
    }

    polys
}

/// Generates polygons for vaulted arches
fn generate_vaulted_arch(cfg: &VaultedArchConfig) -> Vec<Polygon> {
    let mut polys = Vec::new();
    let half_d = cfg.thickness_depth * 0.5;
    let segs = cfg.segments.max(4);

    for s in 0..segs {
        let f0 = s as f32 / segs as f32;
        let f1 = (s + 1) as f32 / segs as f32;

        let (a0, a1) = (f0 * PI, f1 * PI);

        let (r_in_0, r_in_1, r_out_0, r_out_1) = match cfg.arch_style {
            ArchStyle::RoundRoman => (cfg.inner_radius, cfg.inner_radius, cfg.outer_radius, cfg.outer_radius),
            ArchStyle::GothicPointed => {
                // Pointed peak: scale vertical component by sharper factor
                let p_mult0 = 1.0 + 0.35 * (1.0 - (f0 - 0.5).abs() * 2.0);
                let p_mult1 = 1.0 + 0.35 * (1.0 - (f1 - 0.5).abs() * 2.0);
                (cfg.inner_radius * p_mult0, cfg.inner_radius * p_mult1, cfg.outer_radius * p_mult0, cfg.outer_radius * p_mult1)
            }
            ArchStyle::SegmentalFlat => (cfg.inner_radius * 0.7, cfg.inner_radius * 0.7, cfg.outer_radius * 0.8, cfg.outer_radius * 0.8),
        };

        // Front face (+Y)
        let f_b0 = Vec3::new(r_in_0 * a0.cos(), half_d, r_in_0 * a0.sin());
        let f_b1 = Vec3::new(r_out_0 * a0.cos(), half_d, r_out_0 * a0.sin());
        let f_b2 = Vec3::new(r_out_1 * a1.cos(), half_d, r_out_1 * a1.sin());
        let f_b3 = Vec3::new(r_in_1 * a1.cos(), half_d, r_in_1 * a1.sin());

        // Back face (-Y)
        let b_b0 = Vec3::new(f_b0.x, -half_d, f_b0.z);
        let b_b1 = Vec3::new(f_b1.x, -half_d, f_b1.z);
        let b_b2 = Vec3::new(f_b2.x, -half_d, f_b2.z);
        let b_b3 = Vec3::new(f_b3.x, -half_d, f_b3.z);

        polys.extend(build_wedge_polys(b_b0, b_b1, b_b2, b_b3, f_b0, f_b1, f_b2, f_b3));
    }
    polys
}

/// Generates polygons for classical columns and pillars
fn generate_classic_pillar(cfg: &ClassicPillarConfig) -> Vec<Polygon> {
    let mut polys = Vec::new();
    let sides = cfg.sides.max(4);
    let shaft_height = cfg.height - cfg.base_height - cfg.capital_height;

    // 1. Plinth Base
    let base_r = cfg.radius * 1.3;
    for s in 0..sides {
        let a0 = (s as f32 / sides as f32) * 2.0 * PI;
        let a1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

        let b1 = Vec3::new(base_r * a0.cos(), base_r * a0.sin(), 0.0);
        let b2 = Vec3::new(base_r * a1.cos(), base_r * a1.sin(), 0.0);
        let t1 = Vec3::new(base_r * a0.cos(), base_r * a0.sin(), cfg.base_height);
        let t2 = Vec3::new(base_r * a1.cos(), base_r * a1.sin(), cfg.base_height);

        if let Some(p) = Polygon::new(vec![Vec3::ZERO, b1, b2]) { polys.push(p); }
        if let Some(p) = Polygon::new(vec![Vec3::new(0.0, 0.0, cfg.base_height), t2, t1]) { polys.push(p); }
        if let Some(p) = Polygon::new(vec![b1, t1, t2, b2]) { polys.push(p); }
    }

    // 2. Shaft with Entasis
    let shaft_base_z = cfg.base_height;
    let shaft_mid_z = shaft_base_z + shaft_height * 0.5;
    let shaft_top_z = shaft_base_z + shaft_height;
    let r_mid = cfg.radius * cfg.entasis_factor;

    for s in 0..sides {
        let a0 = (s as f32 / sides as f32) * 2.0 * PI;
        let a1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

        // Lower half
        let b1 = Vec3::new(cfg.radius * a0.cos(), cfg.radius * a0.sin(), shaft_base_z);
        let b2 = Vec3::new(cfg.radius * a1.cos(), cfg.radius * a1.sin(), shaft_base_z);
        let m1 = Vec3::new(r_mid * a0.cos(), r_mid * a0.sin(), shaft_mid_z);
        let m2 = Vec3::new(r_mid * a1.cos(), r_mid * a1.sin(), shaft_mid_z);
        if let Some(p) = Polygon::new(vec![b1, m1, m2, b2]) { polys.push(p); }

        // Upper half
        let t1 = Vec3::new(cfg.radius * a0.cos(), cfg.radius * a0.sin(), shaft_top_z);
        let t2 = Vec3::new(cfg.radius * a1.cos(), cfg.radius * a1.sin(), shaft_top_z);
        if let Some(p) = Polygon::new(vec![m1, t1, t2, m2]) { polys.push(p); }
    }

    // 3. Molded Capital
    let cap_r = cfg.radius * 1.35;
    for s in 0..sides {
        let a0 = (s as f32 / sides as f32) * 2.0 * PI;
        let a1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

        let b1 = Vec3::new(cfg.radius * a0.cos(), cfg.radius * a0.sin(), shaft_top_z);
        let b2 = Vec3::new(cfg.radius * a1.cos(), cfg.radius * a1.sin(), shaft_top_z);
        let t1 = Vec3::new(cap_r * a0.cos(), cap_r * a0.sin(), cfg.height);
        let t2 = Vec3::new(cap_r * a1.cos(), cap_r * a1.sin(), cfg.height);

        if let Some(p) = Polygon::new(vec![Vec3::new(0.0, 0.0, cfg.height), t2, t1]) { polys.push(p); }
        if let Some(p) = Polygon::new(vec![b1, t1, t2, b2]) { polys.push(p); }
    }

    polys
}

/// Generates polygons for industrial conduit pipe bends
fn generate_industrial_conduit(cfg: &IndustrialConduitConfig) -> Vec<Polygon> {
    let mut polys = Vec::new();
    let sides = cfg.sides.max(6);

    if cfg.bend_angle_degrees.abs() < 1e-2 {
        // Straight pipe along Z
        for s in 0..sides {
            let a0 = (s as f32 / sides as f32) * 2.0 * PI;
            let a1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

            let b1 = Vec3::new(cfg.pipe_radius * a0.cos(), cfg.pipe_radius * a0.sin(), 0.0);
            let b2 = Vec3::new(cfg.pipe_radius * a1.cos(), cfg.pipe_radius * a1.sin(), 0.0);
            let t1 = Vec3::new(b1.x, b1.y, cfg.length);
            let t2 = Vec3::new(b2.x, b2.y, cfg.length);

            if let Some(p) = Polygon::new(vec![Vec3::ZERO, b1, b2]) { polys.push(p); }
            if let Some(p) = Polygon::new(vec![Vec3::new(0.0, 0.0, cfg.length), t2, t1]) { polys.push(p); }
            if let Some(p) = Polygon::new(vec![b1, t1, t2, b2]) { polys.push(p); }
        }
    } else {
        // Toroidal elbow bend
        let segs = 8;
        let total_rad = cfg.bend_angle_degrees.to_radians();

        for i in 0..segs {
            let theta0 = (i as f32 / segs as f32) * total_rad;
            let theta1 = ((i + 1) as f32 / segs as f32) * total_rad;

            for s in 0..sides {
                let phi0 = (s as f32 / sides as f32) * 2.0 * PI;
                let phi1 = ((s + 1) as f32 / sides as f32) * 2.0 * PI;

                // Center of toroidal ring at (cfg.bend_radius, 0, 0)
                let sample_torus = |th: f32, ph: f32| -> Vec3 {
                    let r_torus = cfg.bend_radius + cfg.pipe_radius * ph.cos();
                    Vec3::new(
                        r_torus * th.cos(),
                        cfg.pipe_radius * ph.sin(),
                        r_torus * th.sin(),
                    )
                };

                let v0 = sample_torus(theta0, phi0);
                let v1 = sample_torus(theta0, phi1);
                let v2 = sample_torus(theta1, phi1);
                let v3 = sample_torus(theta1, phi0);

                if let Some(p) = Polygon::new(vec![v0, v1, v2, v3]) {
                    polys.push(p);
                }
            }
        }
    }
    polys
}

/// Generates all polygons for the specified architectural configuration
pub fn build_prefab_polygons(config: &ArchitecturalPrefabConfig) -> Vec<Polygon> {
    match config.prefab_type {
        ArchitecturalPrefabType::LinearStairs => generate_linear_stairs(&config.linear_stairs),
        ArchitecturalPrefabType::SpiralStairs => generate_spiral_stairs(&config.spiral_stairs),
        ArchitecturalPrefabType::VaultedArch => generate_vaulted_arch(&config.vaulted_arch),
        ArchitecturalPrefabType::ClassicPillar => generate_classic_pillar(&config.classic_pillar),
        ArchitecturalPrefabType::IndustrialConduit => generate_industrial_conduit(&config.industrial_conduit),
    }
}

/// Constructs a complete Brush ready for the level or builder brush
pub fn build_prefab_brush(name: &str, csg_oper: CsgOperation, config: &ArchitecturalPrefabConfig) -> Brush {
    let mut brush = Brush::new(name, csg_oper);
    brush.polys = build_prefab_polygons(config);
    brush
}
