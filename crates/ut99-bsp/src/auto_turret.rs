// crates/ut99-bsp/src/auto_turret.rs
//
// Interactive Automated Defense Turret, Sentry Cannon & Emplacement Studio
//
// Mathematical, geometric, and kinematic model for automated stationary defense pawns
// (TeamCannon, MinigunTurret, CeilingSentry), swivel tracking cones, elevation arcs,
// structural mount brushes, projectile ballistics, and bot evasion waypoints.

use crate::brush::Brush;
use crate::poly::Polygon;
use crate::CsgOperation;
use glam::Vec3;
use serde::{Deserialize, Serialize};

/// Canonical Unreal Tournament & Unreal 1 defense turret archetypes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurretArchetype {
    CoastalDefenseBattery,
    MinigunCeilingSentry,
    PerimeterRocketPod,
    ShockBeamTurret,
    SniperFlakBunker,
}

impl TurretArchetype {
    pub fn all() -> &'static [TurretArchetype] {
        &[
            TurretArchetype::CoastalDefenseBattery,
            TurretArchetype::MinigunCeilingSentry,
            TurretArchetype::PerimeterRocketPod,
            TurretArchetype::ShockBeamTurret,
            TurretArchetype::SniperFlakBunker,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TurretArchetype::CoastalDefenseBattery => "Coastal Defense Heavy Battery (3000 uu Range)",
            TurretArchetype::MinigunCeilingSentry => "Minigun Ceiling Sentry Turret (10 rps)",
            TurretArchetype::PerimeterRocketPod => "Perimeter Dual Rocket Pod Emplacement",
            TurretArchetype::ShockBeamTurret => "Shock Beam Energy Sentry (Hitscan)",
            TurretArchetype::SniperFlakBunker => "Sniper Flak Watchtower Bunker",
        }
    }

    pub fn default_config(&self) -> TurretConfig {
        match self {
            TurretArchetype::CoastalDefenseBattery => TurretConfig {
                archetype: *self,
                position: Vec3::ZERO,
                base_rotation_yaw_deg: 0.0,
                mount: TurretMountConfig {
                    mount_type: TurretMountType::FloorPedestal,
                    pedestal_radius: 48.0,
                    pedestal_height: 32.0,
                    barrel_length: 64.0,
                    barrel_radius: 6.0,
                    barrel_count: 1,
                    has_shield_plate: true,
                    shield_width: 80.0,
                    shield_height: 48.0,
                },
                tracking: TurretTrackingConfig {
                    yaw_min_deg: -90.0,
                    yaw_max_deg: 90.0,
                    pitch_min_deg: -10.0,
                    pitch_max_deg: 45.0,
                    tracking_range: 3000.0,
                    tracking_speed_deg_sec: 45.0,
                },
                combat: TurretCombatConfig {
                    weapon_type: TurretWeaponType::PlasmaBolt,
                    fire_rate_hz: 1.5,
                    damage_per_shot: 85.0,
                    health: 500.0,
                    projectile_speed: 1200.0,
                },
                team: TurretTeamOwnership::RedTeam,
                tag: "BeachCannon01".to_string(),
                mount_texture: "Indus.Metal.RustyIron01".to_string(),
                armor_texture: "Indus.Metal.DarkArmor01".to_string(),
            },
            TurretArchetype::MinigunCeilingSentry => TurretConfig {
                archetype: *self,
                position: Vec3::ZERO,
                base_rotation_yaw_deg: 0.0,
                mount: TurretMountConfig {
                    mount_type: TurretMountType::CeilingSuspended,
                    pedestal_radius: 32.0,
                    pedestal_height: 16.0,
                    barrel_length: 40.0,
                    barrel_radius: 3.0,
                    barrel_count: 2,
                    has_shield_plate: false,
                    shield_width: 48.0,
                    shield_height: 32.0,
                },
                tracking: TurretTrackingConfig {
                    yaw_min_deg: -180.0,
                    yaw_max_deg: 180.0,
                    pitch_min_deg: -80.0,
                    pitch_max_deg: 0.0,
                    tracking_range: 1500.0,
                    tracking_speed_deg_sec: 120.0,
                },
                combat: TurretCombatConfig {
                    weapon_type: TurretWeaponType::MinigunBullet,
                    fire_rate_hz: 10.0,
                    damage_per_shot: 15.0,
                    health: 150.0,
                    projectile_speed: 3000.0,
                },
                team: TurretTeamOwnership::BlueTeam,
                tag: "CeilingSentry01".to_string(),
                mount_texture: "Indus.Metal.SteelChrome01".to_string(),
                armor_texture: "Indus.Metal.BlueAlloy01".to_string(),
            },
            TurretArchetype::PerimeterRocketPod => TurretConfig {
                archetype: *self,
                position: Vec3::ZERO,
                base_rotation_yaw_deg: 0.0,
                mount: TurretMountConfig {
                    mount_type: TurretMountType::FloorPedestal,
                    pedestal_radius: 40.0,
                    pedestal_height: 24.0,
                    barrel_length: 48.0,
                    barrel_radius: 5.0,
                    barrel_count: 2,
                    has_shield_plate: true,
                    shield_width: 64.0,
                    shield_height: 40.0,
                },
                tracking: TurretTrackingConfig {
                    yaw_min_deg: -60.0,
                    yaw_max_deg: 60.0,
                    pitch_min_deg: -5.0,
                    pitch_max_deg: 35.0,
                    tracking_range: 2200.0,
                    tracking_speed_deg_sec: 40.0,
                },
                combat: TurretCombatConfig {
                    weapon_type: TurretWeaponType::RocketSalvo,
                    fire_rate_hz: 0.8,
                    damage_per_shot: 110.0,
                    health: 300.0,
                    projectile_speed: 900.0,
                },
                team: TurretTeamOwnership::RedTeam,
                tag: "PerimeterPod01".to_string(),
                mount_texture: "Indus.Metal.CatwalkTruss01".to_string(),
                armor_texture: "Indus.Metal.RedAlloy01".to_string(),
            },
            TurretArchetype::ShockBeamTurret => TurretConfig {
                archetype: *self,
                position: Vec3::ZERO,
                base_rotation_yaw_deg: 0.0,
                mount: TurretMountConfig {
                    mount_type: TurretMountType::WallBracket,
                    pedestal_radius: 28.0,
                    pedestal_height: 20.0,
                    barrel_length: 56.0,
                    barrel_radius: 4.0,
                    barrel_count: 1,
                    has_shield_plate: true,
                    shield_width: 56.0,
                    shield_height: 36.0,
                },
                tracking: TurretTrackingConfig {
                    yaw_min_deg: -75.0,
                    yaw_max_deg: 75.0,
                    pitch_min_deg: -30.0,
                    pitch_max_deg: 30.0,
                    tracking_range: 2500.0,
                    tracking_speed_deg_sec: 90.0,
                },
                combat: TurretCombatConfig {
                    weapon_type: TurretWeaponType::ShockBeam,
                    fire_rate_hz: 2.0,
                    damage_per_shot: 55.0,
                    health: 220.0,
                    projectile_speed: 0.0, // Instant hitscan
                },
                team: TurretTeamOwnership::Neutral,
                tag: "ShockSentry01".to_string(),
                mount_texture: "Indus.Metal.SkaarjTech01".to_string(),
                armor_texture: "Indus.Metal.PurpleArmor01".to_string(),
            },
            TurretArchetype::SniperFlakBunker => TurretConfig {
                archetype: *self,
                position: Vec3::ZERO,
                base_rotation_yaw_deg: 0.0,
                mount: TurretMountConfig {
                    mount_type: TurretMountType::FloorPedestal,
                    pedestal_radius: 56.0,
                    pedestal_height: 36.0,
                    barrel_length: 52.0,
                    barrel_radius: 7.0,
                    barrel_count: 1,
                    has_shield_plate: true,
                    shield_width: 96.0,
                    shield_height: 52.0,
                },
                tracking: TurretTrackingConfig {
                    yaw_min_deg: -45.0,
                    yaw_max_deg: 45.0,
                    pitch_min_deg: -20.0,
                    pitch_max_deg: 20.0,
                    tracking_range: 1800.0,
                    tracking_speed_deg_sec: 50.0,
                },
                combat: TurretCombatConfig {
                    weapon_type: TurretWeaponType::FlakChunk,
                    fire_rate_hz: 1.2,
                    damage_per_shot: 95.0,
                    health: 400.0,
                    projectile_speed: 1400.0,
                },
                team: TurretTeamOwnership::BlueTeam,
                tag: "FlakBunker01".to_string(),
                mount_texture: "Indus.Metal.ConcreteReinforced01".to_string(),
                armor_texture: "Indus.Metal.FlakSteel01".to_string(),
            },
        }
    }
}

/// Structural mounting configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurretMountType {
    FloorPedestal,
    CeilingSuspended,
    WallBracket,
}

impl TurretMountType {
    pub fn all() -> &'static [TurretMountType] {
        &[
            TurretMountType::FloorPedestal,
            TurretMountType::CeilingSuspended,
            TurretMountType::WallBracket,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TurretMountType::FloorPedestal => "Floor Pedestal Swivel",
            TurretMountType::CeilingSuspended => "Ceiling Suspended Bracket",
            TurretMountType::WallBracket => "Wall Cantilever Bracket",
        }
    }
}

/// Weapon projectile classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurretWeaponType {
    PlasmaBolt,
    MinigunBullet,
    RocketSalvo,
    ShockBeam,
    FlakChunk,
}

impl TurretWeaponType {
    pub fn all() -> &'static [TurretWeaponType] {
        &[
            TurretWeaponType::PlasmaBolt,
            TurretWeaponType::MinigunBullet,
            TurretWeaponType::RocketSalvo,
            TurretWeaponType::ShockBeam,
            TurretWeaponType::FlakChunk,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TurretWeaponType::PlasmaBolt => "Energy Plasma Bolt",
            TurretWeaponType::MinigunBullet => "Kinetic Minigun Bullet (Hitscan)",
            TurretWeaponType::RocketSalvo => "Explosive Rocket Salvo",
            TurretWeaponType::ShockBeam => "Shock Ionization Beam",
            TurretWeaponType::FlakChunk => "Flak Shrapnel Chunks",
        }
    }
}

/// Team ownership for friendly fire and bot threat evaluation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurretTeamOwnership {
    Neutral,
    RedTeam,
    BlueTeam,
    GreenTeam,
    GoldTeam,
}

impl TurretTeamOwnership {
    pub fn all() -> &'static [TurretTeamOwnership] {
        &[
            TurretTeamOwnership::Neutral,
            TurretTeamOwnership::RedTeam,
            TurretTeamOwnership::BlueTeam,
            TurretTeamOwnership::GreenTeam,
            TurretTeamOwnership::GoldTeam,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            TurretTeamOwnership::Neutral => "Neutral / Hostile to All",
            TurretTeamOwnership::RedTeam => "Red Team (Defenders)",
            TurretTeamOwnership::BlueTeam => "Blue Team (Attackers)",
            TurretTeamOwnership::GreenTeam => "Green Team",
            TurretTeamOwnership::GoldTeam => "Gold Team",
        }
    }

    pub fn team_index(&self) -> u8 {
        match self {
            TurretTeamOwnership::RedTeam => 0,
            TurretTeamOwnership::BlueTeam => 1,
            TurretTeamOwnership::GreenTeam => 2,
            TurretTeamOwnership::GoldTeam => 3,
            TurretTeamOwnership::Neutral => 255,
        }
    }
}

/// Mount geometry dimensions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurretMountConfig {
    pub mount_type: TurretMountType,
    pub pedestal_radius: f32,
    pub pedestal_height: f32,
    pub barrel_length: f32,
    pub barrel_radius: f32,
    pub barrel_count: usize,
    pub has_shield_plate: bool,
    pub shield_width: f32,
    pub shield_height: f32,
}

/// Swivel traversal and elevation tracking limits
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurretTrackingConfig {
    pub yaw_min_deg: f32,
    pub yaw_max_deg: f32,
    pub pitch_min_deg: f32,
    pub pitch_max_deg: f32,
    pub tracking_range: f32,
    pub tracking_speed_deg_sec: f32,
}

/// Combat characteristics
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurretCombatConfig {
    pub weapon_type: TurretWeaponType,
    pub fire_rate_hz: f32,
    pub damage_per_shot: f32,
    pub health: f32,
    pub projectile_speed: f32,
}

/// Complete configuration for Auto Defense Turret Studio
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurretConfig {
    pub archetype: TurretArchetype,
    pub position: Vec3,
    pub base_rotation_yaw_deg: f32,
    pub mount: TurretMountConfig,
    pub tracking: TurretTrackingConfig,
    pub combat: TurretCombatConfig,
    pub team: TurretTeamOwnership,
    pub tag: String,
    pub mount_texture: String,
    pub armor_texture: String,
}

impl Default for TurretConfig {
    fn default() -> Self {
        TurretArchetype::CoastalDefenseBattery.default_config()
    }
}

/// Geometric tracking metrics and line of fire calculations
#[derive(Debug, Clone, PartialEq)]
pub struct TurretFrustumMetrics {
    pub current_yaw_deg: f32,
    pub current_pitch_deg: f32,
    pub muzzle_origin: Vec3,
    pub aiming_direction: Vec3,
    pub swept_yaw_arc_deg: f32,
    pub swept_pitch_arc_deg: f32,
    pub coverage_volume_radius: f32,
}

/// Bot AI cover and evasion waypoint
#[derive(Debug, Clone, PartialEq)]
pub struct TurretEvasionWaypoint {
    pub name: String,
    pub location: Vec3,
    pub is_flank_route: bool,
    pub has_direct_line_of_sight: bool,
}

/// Diagnostic safety hazard for turret emplacements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurretHazard {
    pub is_critical: bool,
    pub title: String,
    pub description: String,
    pub recommendation: String,
}

/// Computes animated traversal state, muzzle origin, and aiming direction at simulated time t
pub fn calculate_turret_tracking_metrics(
    config: &TurretConfig,
    simulated_time_seconds: f32,
) -> TurretFrustumMetrics {
    let yaw_range = config.tracking.yaw_max_deg - config.tracking.yaw_min_deg;
    let swept_yaw = yaw_range.abs();
    let swept_pitch = (config.tracking.pitch_max_deg - config.tracking.pitch_min_deg).abs();

    // Harmonic patrol oscillation between yaw_min and yaw_max
    let period = if config.tracking.tracking_speed_deg_sec > 0.0 {
        (swept_yaw * 2.0) / config.tracking.tracking_speed_deg_sec
    } else {
        10.0
    };

    let phase = (simulated_time_seconds / period.max(0.1)) * std::f32::consts::TAU;
    let t_norm = (phase.sin() + 1.0) * 0.5; // [0..1]

    let curr_yaw = config.tracking.yaw_min_deg + t_norm * yaw_range + config.base_rotation_yaw_deg;
    let curr_pitch = config.tracking.pitch_min_deg + (config.tracking.pitch_max_deg - config.tracking.pitch_min_deg) * 0.5;

    let yaw_rad = curr_yaw.to_radians();
    let pitch_rad = curr_pitch.to_radians();

    let aim_dir = Vec3::new(
        yaw_rad.cos() * pitch_rad.cos(),
        yaw_rad.sin() * pitch_rad.cos(),
        pitch_rad.sin(),
    ).normalize_or_zero();

    let mount_up = match config.mount.mount_type {
        TurretMountType::FloorPedestal => Vec3::Z,
        TurretMountType::CeilingSuspended => -Vec3::Z,
        TurretMountType::WallBracket => Vec3::Z,
    };

    let pivot_pos = config.position + mount_up * config.mount.pedestal_height;
    let muzzle = pivot_pos + aim_dir * config.mount.barrel_length;

    TurretFrustumMetrics {
        current_yaw_deg: curr_yaw,
        current_pitch_deg: curr_pitch,
        muzzle_origin: muzzle,
        aiming_direction: aim_dir,
        swept_yaw_arc_deg: swept_yaw,
        swept_pitch_arc_deg: swept_pitch,
        coverage_volume_radius: config.tracking.tracking_range,
    }
}

/// Generates procedural 3D brush geometry for:
/// 1. Pedestal / Bracket Structural Mount Brush (`PF_SEMISOLID`)
/// 2. Armored Shield Plate & Swivel Base Brush (`PF_SEMISOLID`)
pub fn generate_turret_mount_brushes(
    config: &TurretConfig,
) -> (Brush, Brush) {
    let mut mount_brush = Brush::new("Turret_Mount", CsgOperation::Add);
    let mut shield_brush = Brush::new("Turret_Shield", CsgOperation::Add);

    let mount_up = match config.mount.mount_type {
        TurretMountType::FloorPedestal => Vec3::Z,
        TurretMountType::CeilingSuspended => -Vec3::Z,
        TurretMountType::WallBracket => Vec3::Z,
    };

    let base_yaw_rad = config.base_rotation_yaw_deg.to_radians();
    let fwd = Vec3::new(base_yaw_rad.cos(), base_yaw_rad.sin(), 0.0).normalize_or_zero();
    let right = fwd.cross(mount_up).normalize_or_zero();

    let pr = config.mount.pedestal_radius.max(8.0);
    let ph = config.mount.pedestal_height.max(8.0);
    let subs = 8;

    // 1. Generate Structural Pedestal (8-sided truncated cylinder)
    let mut bot_rim = Vec::new();
    let mut top_rim = Vec::new();

    for s in 0..subs {
        let a = (s as f32 / subs as f32) * std::f32::consts::TAU;
        let radial = fwd * a.cos() + right * a.sin();
        bot_rim.push(config.position + radial * (pr * 1.15));
        top_rim.push(config.position + mount_up * ph + radial * pr);
    }

    // Side faces
    for s in 0..subs {
        let next = (s + 1) % subs;
        let face = vec![bot_rim[s], bot_rim[next], top_rim[next], top_rim[s]];
        if let Some(mut poly) = Polygon::new(face) {
            poly.texture = config.mount_texture.clone();
            poly.flags |= 0x00200000; // PF_SEMISOLID
            mount_brush.polys.push(poly);
        }
    }

    // Bottom and Top caps
    let mut rev_bot = bot_rim.clone();
    rev_bot.reverse();
    if let Some(mut poly) = Polygon::new(rev_bot) {
        poly.texture = config.mount_texture.clone();
        poly.flags |= 0x00200000;
        mount_brush.polys.push(poly);
    }
    if let Some(mut poly) = Polygon::new(top_rim) {
        poly.texture = config.mount_texture.clone();
        poly.flags |= 0x00200000;
        mount_brush.polys.push(poly);
    }

    // 2. Generate Armored Shield Plate (Front protective visor)
    if config.mount.has_shield_plate {
        let sw = config.mount.shield_width * 0.5;
        let sh = config.mount.shield_height * 0.5;
        let st = 4.0; // Plate thickness

        let pivot = config.position + mount_up * ph;
        let front_center = pivot + fwd * (pr * 0.8);

        let p_corners = [
            front_center - right * sw - mount_up * sh,
            front_center + right * sw - mount_up * sh,
            front_center + right * sw + mount_up * sh,
            front_center - right * sw + mount_up * sh,
            front_center - right * sw - mount_up * sh + fwd * st,
            front_center + right * sw - mount_up * sh + fwd * st,
            front_center + right * sw + mount_up * sh + fwd * st,
            front_center - right * sw + mount_up * sh + fwd * st,
        ];

        let faces = [
            [0, 1, 2, 3], // Back face
            [5, 4, 7, 6], // Front face
            [4, 0, 3, 7], // Left edge
            [1, 5, 6, 2], // Right edge
            [3, 2, 6, 7], // Top edge
            [4, 5, 1, 0], // Bottom edge
        ];

        for f in faces {
            let pts = vec![p_corners[f[0]], p_corners[f[1]], p_corners[f[2]], p_corners[f[3]]];
            if let Some(mut poly) = Polygon::new(pts) {
                poly.texture = config.armor_texture.clone();
                poly.flags |= 0x00200000; // PF_SEMISOLID
                shield_brush.polys.push(poly);
            }
        }
    }

    (mount_brush, shield_brush)
}

/// Generates bot evasion, sniper vantage, and flank route waypoints
pub fn generate_turret_bot_waypoints(
    config: &TurretConfig,
    _world_brushes: &[Brush],
) -> Vec<TurretEvasionWaypoint> {
    let mut waypoints = Vec::new();

    let base_yaw_rad = config.base_rotation_yaw_deg.to_radians();
    let fwd = Vec3::new(base_yaw_rad.cos(), base_yaw_rad.sin(), 0.0).normalize_or_zero();
    let right = fwd.cross(Vec3::Z).normalize_or_zero();

    let range = config.tracking.tracking_range;
    let flank_dist = (range * 0.4).clamp(384.0, 768.0);

    // 1. Frontal Killzone Warning Node
    waypoints.push(TurretEvasionWaypoint {
        name: format!("{}_KillzoneNode", config.tag),
        location: config.position + fwd * (range * 0.5) + Vec3::new(0.0, 0.0, 32.0),
        is_flank_route: false,
        has_direct_line_of_sight: true,
    });

    // 2. Left Flank Cover Waypoint
    waypoints.push(TurretEvasionWaypoint {
        name: format!("{}_FlankLeftNode", config.tag),
        location: config.position + fwd * (range * 0.25) - right * flank_dist + Vec3::new(0.0, 0.0, 32.0),
        is_flank_route: true,
        has_direct_line_of_sight: false,
    });

    // 3. Right Flank Cover Waypoint
    waypoints.push(TurretEvasionWaypoint {
        name: format!("{}_FlankRightNode", config.tag),
        location: config.position + fwd * (range * 0.25) + right * flank_dist + Vec3::new(0.0, 0.0, 32.0),
        is_flank_route: true,
        has_direct_line_of_sight: false,
    });

    // 4. Rear Infiltration Blind-Spot Node
    waypoints.push(TurretEvasionWaypoint {
        name: format!("{}_BlindSpotNode", config.tag),
        location: config.position - fwd * (config.mount.pedestal_radius + 64.0) + Vec3::new(0.0, 0.0, 32.0),
        is_flank_route: true,
        has_direct_line_of_sight: false,
    });

    waypoints
}

/// Audits turret configuration against safety, balance, and kinematic clearance hazards
pub fn audit_turret_safety(
    config: &TurretConfig,
    world_brushes: &[Brush],
) -> Vec<TurretHazard> {
    let mut hazards = Vec::new();

    // 1. Blind Elevation (pitch_max <= pitch_min)
    if config.tracking.pitch_max_deg <= config.tracking.pitch_min_deg {
        hazards.push(TurretHazard {
            is_critical: true,
            title: "Invalid Elevation Pitch Limits".to_string(),
            description: format!("Pitch Max ({:.1}°) is less than or equal to Pitch Min ({:.1}°). The turret cannot aim vertically.",
                config.tracking.pitch_max_deg, config.tracking.pitch_min_deg),
            recommendation: "Ensure Pitch Max is strictly greater than Pitch Min.".to_string(),
        });
    }

    // 2. Excessive DPS / Instagib Turret Hazard (DPS > 300)
    let dps = config.combat.damage_per_shot * config.combat.fire_rate_hz;
    if dps > 300.0 {
        hazards.push(TurretHazard {
            is_critical: false,
            title: format!("Punishing Turret DPS ({:.0} HP/s)", dps),
            description: "Turret damage per second exceeds 300 HP/s. Attackers will be eliminated almost instantaneously without evasion time.".to_string(),
            recommendation: "Reduce Damage Per Shot or Fire Rate to keep DPS under 150..250 HP/s.".to_string(),
        });
    }

    // 3. Excessive Tracking Range (>4000 uu)
    if config.tracking.tracking_range > 4000.0 {
        hazards.push(TurretHazard {
            is_critical: true,
            title: format!("Excessive Tracking Range ({:.0} uu)", config.tracking.tracking_range),
            description: "Tracking range exceeds 4000 uu. The turret will engage enemies across entire outdoor skyboxes or unrendered zones.".to_string(),
            recommendation: "Cap tracking range between 1500 and 3000 uu.".to_string(),
        });
    }

    // 4. Close-Range Solid Geometry Occlusion (<64 uu from pivot)
    let pivot = config.position + Vec3::new(0.0, 0.0, config.mount.pedestal_height);
    for brush in world_brushes {
        for poly in &brush.polys {
            for v in &poly.vertices {
                let dist = (*v - pivot).length();
                if dist < 48.0 && dist > 1.0 {
                    hazards.push(TurretHazard {
                        is_critical: false,
                        title: "Potential Geometry Compenetration".to_string(),
                        description: format!("Brush polygon vertex is located only {:.1} uu from the turret pivot.", dist),
                        recommendation: "Reposition turret or carve clear clearance around the mount.".to_string(),
                    });
                    break;
                }
            }
        }
    }

    hazards
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turret_archetypes_and_defaults() {
        for &arch in TurretArchetype::all() {
            let cfg = arch.default_config();
            assert_eq!(cfg.archetype, arch);
            assert!(cfg.mount.pedestal_radius >= 24.0);
            assert!(cfg.mount.barrel_length >= 32.0);
            assert!(cfg.tracking.tracking_range >= 1000.0);
            assert!(cfg.combat.health >= 100.0);
        }
    }

    #[test]
    fn test_turret_tracking_metrics_and_aiming_direction() {
        let cfg = TurretArchetype::CoastalDefenseBattery.default_config();
        let m0 = calculate_turret_tracking_metrics(&cfg, 0.0);
        assert!(m0.aiming_direction.length() > 0.99);
        assert!(m0.muzzle_origin.length() > 0.0);
        assert_eq!(m0.swept_yaw_arc_deg, 180.0);

        let m1 = calculate_turret_tracking_metrics(&cfg, 2.5);
        assert!(m1.aiming_direction.length() > 0.99);
    }

    #[test]
    fn test_generate_turret_mount_brushes() {
        let cfg = TurretArchetype::CoastalDefenseBattery.default_config();
        let (mount_b, shield_b) = generate_turret_mount_brushes(&cfg);

        // Mount has cylinder side faces + 2 caps = 8 + 2 = 10 faces
        assert_eq!(mount_b.polys.len(), 10);
        assert_eq!(mount_b.polys[0].flags & 0x00200000, 0x00200000); // PF_SEMISOLID

        // Shield plate has 6 faces
        assert_eq!(shield_b.polys.len(), 6);
        assert_eq!(shield_b.polys[0].flags & 0x00200000, 0x00200000); // PF_SEMISOLID
    }

    #[test]
    fn test_turret_bot_waypoints_and_safety_audit() {
        let mut cfg = TurretConfig::default();
        cfg.tracking.pitch_min_deg = 30.0;
        cfg.tracking.pitch_max_deg = 10.0; // Invalid
        cfg.tracking.tracking_range = 5000.0; // Excessive

        let hazards = audit_turret_safety(&cfg, &[]);
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Invalid Elevation")));
        assert!(hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive Tracking")));

        let waypoints = generate_turret_bot_waypoints(&cfg, &[]);
        assert_eq!(waypoints.len(), 4);
    }
}
