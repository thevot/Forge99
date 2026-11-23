use glam::Vec3;
use crate::Brush;

/// Ballistic projectile parameters matching Unreal Tournament 1999 weapon classes
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BallisticProjectile {
    pub name: String,
    pub initial_speed: f32,       // Muzzle velocity in uu/s
    pub gravity_scale: f32,       // 0.0 for rockets/hitscan, 1.0 for grenades
    pub bounces_max: usize,       // Maximum surface ricochets before detonation/stopping
    pub elasticity: f32,          // Restitution velocity retention factor on bounce (0.0..1.0)
    pub damage_radius: f32,       // Splash explosion / blast area radius in uu
    pub damage_amount: f32,       // Direct impact or center blast damage
    pub trail_color_rgba: [f32; 4],
}

impl Default for BallisticProjectile {
    fn default() -> Self {
        Self {
            name: "DefaultRocket".to_string(),
            initial_speed: 900.0,
            gravity_scale: 0.0,
            bounces_max: 0,
            elasticity: 0.0,
            damage_radius: 220.0,
            damage_amount: 100.0,
            trail_color_rgba: [1.0, 0.5, 0.1, 0.9],
        }
    }
}

/// Canonical weapon combat firing modes in Unreal Tournament 1999
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum WeaponCombatPreset {
    EnforcerHitscan,
    ShockComboDetonation,
    FlakShrapnelSpray,
    EightballGrenadeArc,
    RocketVolleyBarrage,
    SniperSightlineLongshot,
}

impl WeaponCombatPreset {
    pub fn all() -> &'static [WeaponCombatPreset] {
        &[
            WeaponCombatPreset::EnforcerHitscan,
            WeaponCombatPreset::ShockComboDetonation,
            WeaponCombatPreset::FlakShrapnelSpray,
            WeaponCombatPreset::EightballGrenadeArc,
            WeaponCombatPreset::RocketVolleyBarrage,
            WeaponCombatPreset::SniperSightlineLongshot,
        ]
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            WeaponCombatPreset::EnforcerHitscan => "🔫 Enforcer (Hitscan Kinetic Pistol / 17 DMG)",
            WeaponCombatPreset::ShockComboDetonation => "⚡ Shock Combo (Core Plasma Sphere / 250 uu AOE Blast)",
            WeaponCombatPreset::FlakShrapnelSpray => "💥 Flak Cannon (Kinetic Shrapnel Ricochet / 2 Bounces)",
            WeaponCombatPreset::EightballGrenadeArc => "💣 UT Eightball (Bouncing Ballistic Grenade / 220 uu Splash)",
            WeaponCombatPreset::RocketVolleyBarrage => "🚀 Rocket Launcher (Linear Propulsion / Direct Hit)",
            WeaponCombatPreset::SniperSightlineLongshot => "🎯 Sniper Rifle (Infinite Velocity Sightline / 150 Headshot)",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            WeaponCombatPreset::EnforcerHitscan => {
                "Rapid kinetic hitscan fire with near-infinite velocity and direct line-of-sight tracer."
            }
            WeaponCombatPreset::ShockComboDetonation => {
                "Floating plasma core orb (800 uu/s) detonated by primary hitscan beam producing massive 250 uu area shockwave."
            }
            WeaponCombatPreset::FlakShrapnelSpray => {
                "High-speed red-hot flak chunks (2500 uu/s) with slight gravity drop and 2 ricochets off solid BSP walls."
            }
            WeaponCombatPreset::EightballGrenadeArc => {
                "Lobbed secondary explosive grenade under full UT99 gravity (-950 uu/s²) with up to 3 surface bounces."
            }
            WeaponCombatPreset::RocketVolleyBarrage => {
                "Unstoppable linear rocket projectile (900 uu/s) flying straight until geometry impact and detonation."
            }
            WeaponCombatPreset::SniperSightlineLongshot => {
                "High-caliber sniper sightline audit for tournament sightline clearance and tower sniping angles."
            }
        }
    }

    pub fn to_projectile(&self) -> BallisticProjectile {
        match self {
            WeaponCombatPreset::EnforcerHitscan => BallisticProjectile {
                name: "EnforcerRound".to_string(),
                initial_speed: 15000.0,
                gravity_scale: 0.0,
                bounces_max: 0,
                elasticity: 0.0,
                damage_radius: 12.0,
                damage_amount: 17.0,
                trail_color_rgba: [0.95, 0.85, 0.30, 0.95],
            },
            WeaponCombatPreset::ShockComboDetonation => BallisticProjectile {
                name: "ShockBall".to_string(),
                initial_speed: 800.0,
                gravity_scale: 0.0,
                bounces_max: 0,
                elasticity: 0.0,
                damage_radius: 250.0,
                damage_amount: 165.0,
                trail_color_rgba: [0.75, 0.25, 0.95, 0.90],
            },
            WeaponCombatPreset::FlakShrapnelSpray => BallisticProjectile {
                name: "FlakChunk".to_string(),
                initial_speed: 2500.0,
                gravity_scale: 0.4,
                bounces_max: 2,
                elasticity: 0.65,
                damage_radius: 32.0,
                damage_amount: 35.0,
                trail_color_rgba: [1.0, 0.70, 0.15, 0.85],
            },
            WeaponCombatPreset::EightballGrenadeArc => BallisticProjectile {
                name: "Grenade".to_string(),
                initial_speed: 1000.0,
                gravity_scale: 1.0,
                bounces_max: 3,
                elasticity: 0.70,
                damage_radius: 220.0,
                damage_amount: 100.0,
                trail_color_rgba: [0.95, 0.20, 0.15, 0.90],
            },
            WeaponCombatPreset::RocketVolleyBarrage => BallisticProjectile {
                name: "Rocket".to_string(),
                initial_speed: 900.0,
                gravity_scale: 0.0,
                bounces_max: 0,
                elasticity: 0.0,
                damage_radius: 220.0,
                damage_amount: 100.0,
                trail_color_rgba: [1.0, 0.45, 0.05, 0.95],
            },
            WeaponCombatPreset::SniperSightlineLongshot => BallisticProjectile {
                name: "SniperBullet".to_string(),
                initial_speed: 25000.0,
                gravity_scale: 0.0,
                bounces_max: 0,
                elasticity: 0.0,
                damage_radius: 8.0,
                damage_amount: 150.0,
                trail_color_rgba: [0.20, 0.90, 0.40, 0.95],
            },
        }
    }
}

/// Point on a simulated projectile trajectory
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TrajectoryPoint {
    pub position: Vec3,
    pub velocity: Vec3,
    pub time_sec: f32,
    pub is_bounce: bool,
    pub is_impact: bool,
    pub normal: Option<Vec3>,
}

/// Audit report evaluating line-of-sight and weapon sightline obstruction
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SightlineAuditResult {
    pub is_unobstructed: bool,
    pub hit_distance: f32,
    pub total_distance: f32,
    pub occlusion_ratio: f32,
    pub hit_point: Option<Vec3>,
    pub first_blocking_brush: Option<String>,
}

/// Simulates weapon projectile trajectory step by step against world brushes,
/// reflecting velocities off surface normals on ricochet and stopping on final impact.
pub fn simulate_weapon_trajectory(
    origin: Vec3,
    direction: Vec3,
    proj: &BallisticProjectile,
    brushes: &[Brush],
    max_time: f32,
    dt: f32,
) -> Vec<TrajectoryPoint> {
    let mut points = Vec::new();
    let norm_dir = direction.normalize_or_zero();
    let mut pos = origin;
    let mut vel = norm_dir * proj.initial_speed;
    let mut time = 0.0;
    let mut bounces_remaining = proj.bounces_max;

    points.push(TrajectoryPoint {
        position: pos,
        velocity: vel,
        time_sec: 0.0,
        is_bounce: false,
        is_impact: false,
        normal: None,
    });

    let gravity_accel = Vec3::new(0.0, 0.0, -950.0 * proj.gravity_scale);

    while time < max_time {
        // Accelerate velocity by gravity
        vel += gravity_accel * dt;
        let next_pos = pos + vel * dt;
        let step_vector = next_pos - pos;
        let step_dist = step_vector.length();

        if step_dist < 1e-4 {
            time += dt;
            continue;
        }

        let ray_dir = step_vector / step_dist;
        let mut closest_hit: Option<(f32, Vec3, &Brush)> = None;

        // Raycast against all world brushes
        for brush in brushes {
            for poly in &brush.polys {
                let n = poly.plane.normal;
                let denom = ray_dir.dot(n);
                // Facing plane
                if denom < -1e-4 {
                    let d = (poly.plane.dist - pos.dot(n)) / denom;
                    if d >= 0.0 && d <= step_dist {
                        // Check if within bounds of the polygon
                        let hit_p = pos + ray_dir * d;
                        if is_point_inside_poly(hit_p, poly) {
                            if closest_hit.as_ref().map(|(min_d, _, _)| d < *min_d).unwrap_or(true) {
                                closest_hit = Some((d, n, brush));
                            }
                        }
                    }
                }
            }
        }

        if let Some((hit_dist, hit_normal, _)) = closest_hit {
            let hit_pos = pos + ray_dir * hit_dist;
            time += (hit_dist / step_dist) * dt;

            if bounces_remaining > 0 {
                // Ricochet bounce: reflect velocity vector R = V - 2(V . N)N
                let reflected_v = (vel - 2.0 * vel.dot(hit_normal) * hit_normal) * proj.elasticity;
                bounces_remaining -= 1;
                vel = reflected_v;
                pos = hit_pos + hit_normal * 0.5; // slight nudge away from surface to prevent tunneling

                points.push(TrajectoryPoint {
                    position: hit_pos,
                    velocity: vel,
                    time_sec: time,
                    is_bounce: true,
                    is_impact: false,
                    normal: Some(hit_normal),
                });
            } else {
                // Final detonation / impact
                points.push(TrajectoryPoint {
                    position: hit_pos,
                    velocity: Vec3::ZERO,
                    time_sec: time,
                    is_bounce: false,
                    is_impact: true,
                    normal: Some(hit_normal),
                });
                break;
            }
        } else {
            pos = next_pos;
            time += dt;
            points.push(TrajectoryPoint {
                position: pos,
                velocity: vel,
                time_sec: time,
                is_bounce: false,
                is_impact: false,
                normal: None,
            });
        }
    }

    points
}

/// Helper function testing if a coplanar point lies within convex polygon boundaries
fn is_point_inside_poly(p: Vec3, poly: &crate::Polygon) -> bool {
    if poly.vertices.len() < 3 {
        return false;
    }
    let n = poly.plane.normal;
    for i in 0..poly.vertices.len() {
        let v0 = poly.vertices[i];
        let v1 = poly.vertices[(i + 1) % poly.vertices.len()];
        let edge = v1 - v0;
        let to_p = p - v0;
        let cross = edge.cross(to_p);
        if cross.dot(n) < -1e-2 {
            return false;
        }
    }
    true
}

/// Audits line-of-sight clearance between muzzle position and target across all BSP brushes
pub fn audit_combat_sightline(
    muzzle_pos: Vec3,
    target_pos: Vec3,
    brushes: &[Brush],
) -> SightlineAuditResult {
    let diff = target_pos - muzzle_pos;
    let total_dist = diff.length();
    if total_dist < 1e-4 {
        return SightlineAuditResult {
            is_unobstructed: true,
            hit_distance: 0.0,
            total_distance: 0.0,
            occlusion_ratio: 0.0,
            hit_point: None,
            first_blocking_brush: None,
        };
    }

    let ray_dir = diff / total_dist;
    let mut closest_hit: Option<(f32, Vec3, String)> = None;

    for brush in brushes {
        for poly in &brush.polys {
            let n = poly.plane.normal;
            let denom = ray_dir.dot(n);
            if denom < -1e-4 {
                let d = (poly.plane.dist - muzzle_pos.dot(n)) / denom;
                if d >= 0.0 && d < total_dist {
                    let hit_p = muzzle_pos + ray_dir * d;
                    if is_point_inside_poly(hit_p, poly) {
                        if closest_hit.as_ref().map(|(min_d, _, _)| d < *min_d).unwrap_or(true) {
                            closest_hit = Some((d, hit_p, brush.name.clone()));
                        }
                    }
                }
            }
        }
    }

    if let Some((d, hit_p, brush_name)) = closest_hit {
        SightlineAuditResult {
            is_unobstructed: false,
            hit_distance: d,
            total_distance: total_dist,
            occlusion_ratio: 1.0 - (d / total_dist),
            hit_point: Some(hit_p),
            first_blocking_brush: Some(brush_name),
        }
    } else {
        SightlineAuditResult {
            is_unobstructed: true,
            hit_distance: total_dist,
            total_distance: total_dist,
            occlusion_ratio: 0.0,
            hit_point: None,
            first_blocking_brush: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CsgOperation, Polygon};

    #[test]
    fn test_weapon_presets_and_projectiles() {
        let presets = WeaponCombatPreset::all();
        assert_eq!(presets.len(), 6);

        for &p in presets {
            assert!(!p.display_name().is_empty());
            assert!(!p.description().is_empty());
            let proj = p.to_projectile();
            assert!(proj.initial_speed >= 800.0);
            assert!(proj.damage_amount > 0.0);
        }
    }

    #[test]
    fn test_sightline_unobstructed_and_obstructed() {
        let start = Vec3::new(0.0, 0.0, 0.0);
        let end = Vec3::new(0.0, 1000.0, 0.0);

        // 1. Empty brushes -> 100% unobstructed
        let audit_clear = audit_combat_sightline(start, end, &[]);
        assert!(audit_clear.is_unobstructed);
        assert_eq!(audit_clear.hit_distance, 1000.0);
        assert_eq!(audit_clear.occlusion_ratio, 0.0);

        // 2. Solid wall at Y=400
        let poly = Polygon::new(vec![
            Vec3::new(-200.0, 400.0, -200.0),
            Vec3::new(200.0, 400.0, -200.0),
            Vec3::new(200.0, 400.0, 200.0),
            Vec3::new(-200.0, 400.0, 200.0),
        ]).unwrap();

        let mut wall = Brush::new("MiddleWall", CsgOperation::Add);
        wall.polys.push(poly);

        let audit_blocked = audit_combat_sightline(start, end, &[wall]);
        assert!(!audit_blocked.is_unobstructed);
        assert!((audit_blocked.hit_distance - 400.0).abs() < 1e-2);
        assert_eq!(audit_blocked.first_blocking_brush, Some("MiddleWall".to_string()));
    }

    #[test]
    fn test_grenade_ballistics_arc() {
        let grenade = WeaponCombatPreset::EightballGrenadeArc.to_projectile();
        let traj = simulate_weapon_trajectory(
            Vec3::new(0.0, 0.0, 50.0),
            Vec3::new(0.0, 1.0, 0.5),
            &grenade,
            &[],
            1.5,
            0.05,
        );

        assert!(traj.len() > 10);
        // Due to gravity, Z velocity should decrease over time
        let first_vz = traj[1].velocity.z;
        let last_vz = traj.last().unwrap().velocity.z;
        assert!(last_vz < first_vz, "Gravity must pull projectile velocity downward");
    }
}
