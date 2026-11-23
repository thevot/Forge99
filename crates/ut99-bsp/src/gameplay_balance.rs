use glam::Vec3;
use crate::navigation::NavGraph;

/// Canonical running speed of a tournament player in Unreal Tournament 99 (Unreal Units per second).
pub const PLAYER_RUN_SPEED_UU_PER_SEC: f32 = 400.0;

/// Classification category of an in-game pickup item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemTier {
    SuperArmor,   // ShieldBelt (+150), Armor2 (+100)
    SuperHealth,  // HealthPack / Keg of Health (+100)
    PowerUp,      // UDamage (Amp), Invisibility
    PowerWeapon,  // ShockRifle, FlakCannon, Rocket Launcher, Sniper
    StandardWeapon,// Minigun, PulseGun, BioRifle, Chainsaw
    SuperWeapon,  // WarheadLauncher (Redeemer)
    SmallHealth,  // MedBox (+20), HealthVial (+5)
    ThighPads,    // ThighPads (+50)
    Ammo,         // Generic ammo box
}

/// Represents a detected weapon, armor, or powerup pickup in the map
#[derive(Debug, Clone, PartialEq)]
pub struct PickupItem {
    pub name: String,
    pub class_name: String,
    pub tier: ItemTier,
    pub location: Vec3,
    pub respawn_time_secs: f32, // Canonical respawn delay: Weapon=1s, Armor=27s, UDamage=83s, MegaHealth=55s
}

/// Shortest navigation path timing between two major tournament pickups
#[derive(Debug, Clone, PartialEq)]
pub struct PickupPairTiming {
    pub item_a: String,
    pub item_b: String,
    pub distance_uu: f32,
    pub run_time_secs: f32,
    pub is_stacking_hazard: bool, // True if both are super items and running time < 6.0s
}

/// Potential competitive balance issue detected in the map layout
#[derive(Debug, Clone, PartialEq)]
pub struct BalanceIssue {
    pub title: String,
    pub severity: BalanceSeverity,
    pub description: String,
    pub suggested_fix: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceSeverity {
    Critical, // Stacking super-items or weapons inaccessible from spawns
    Warning,  // Long run times to defense
    Info,     // Balanced configuration
}

/// Comprehensive tournament gameplay and item timing audit report
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ItemBalanceReport {
    pub pickups: Vec<PickupItem>,
    pub pair_timings: Vec<PickupPairTiming>,
    pub spawn_times_to_weapon: Vec<(String, f32)>, // (PlayerStart name, time in seconds to nearest weapon)
    pub issues: Vec<BalanceIssue>,
    pub average_spawn_to_weapon_secs: f32,
    pub is_tournament_certified: bool,
}

/// Analyzes item placement balance, respawn routes, and travel times across the level.
pub fn analyze_item_balance(
    actors: &[(String, String, Vec3)], // (name, class_name, location)
    nav_graph: &NavGraph,
) -> ItemBalanceReport {
    let mut report = ItemBalanceReport::default();
    let mut spawns: Vec<(String, Vec3)> = Vec::new();

    // 1. Identify pickups and player starts
    for (name, class_name, loc) in actors {
        let clean_class = if let Some(dot) = class_name.rfind('.') {
            &class_name[dot + 1..]
        } else {
            class_name.as_str()
        };

        if clean_class.eq_ignore_ascii_case("PlayerStart") {
            spawns.push((name.clone(), *loc));
            continue;
        }

        let tier_opt = match clean_class.to_lowercase().as_str() {
            "ut_shieldbelt" | "shieldbelt" => Some((ItemTier::SuperArmor, 27.0)),
            "armor2" | "bodyarmor" => Some((ItemTier::SuperArmor, 27.0)),
            "thighpads" => Some((ItemTier::ThighPads, 27.0)),
            "udamage" | "damageamplifier" => Some((ItemTier::PowerUp, 83.0)),
            "utinvisibility" | "invisibility" => Some((ItemTier::PowerUp, 83.0)),
            "healthpack" | "superhealth" | "kegofhealth" => Some((ItemTier::SuperHealth, 55.0)),
            "medbox" | "healthvial" => Some((ItemTier::SmallHealth, 20.0)),
            "shockrifle" | "ut_flakcannon" | "flakcannon" | "ut_eightball" | "rocketlauncher" | "sniperrifle" => {
                Some((ItemTier::PowerWeapon, 1.0)) // Weapon stay enabled by default in UT99 competitive play
            }
            "minigun2" | "minigun" | "pulsegun" | "ut_biorifle" | "biorifle" | "chainsaw" => {
                Some((ItemTier::StandardWeapon, 1.0))
            }
            "warheadlauncher" | "redeemer" => Some((ItemTier::SuperWeapon, 110.0)),
            _ => None,
        };

        if let Some((tier, respawn_time)) = tier_opt {
            report.pickups.push(PickupItem {
                name: if name.is_empty() { clean_class.to_string() } else { name.clone() },
                class_name: clean_class.to_string(),
                tier,
                location: *loc,
                respawn_time_secs: respawn_time,
            });
        }
    }

    // 2. Measure pairwise timings between major items (SuperArmor, SuperHealth, PowerUp, SuperWeapon)
    let major_pickups: Vec<&PickupItem> = report.pickups.iter().filter(|p| {
        matches!(p.tier, ItemTier::SuperArmor | ItemTier::SuperHealth | ItemTier::PowerUp | ItemTier::SuperWeapon)
    }).collect();

    let n_major = major_pickups.len();
    for i in 0..n_major {
        for j in (i + 1)..n_major {
            let p_a = major_pickups[i];
            let p_b = major_pickups[j];

            let dist = calculate_travel_distance(p_a.location, p_b.location, nav_graph);
            let time_secs = dist / PLAYER_RUN_SPEED_UU_PER_SEC;

            let is_stacking = time_secs < 6.0 && (
                (p_a.tier == ItemTier::SuperArmor && p_b.tier == ItemTier::PowerUp) ||
                (p_a.tier == ItemTier::PowerUp && p_b.tier == ItemTier::SuperArmor) ||
                (p_a.tier == ItemTier::SuperArmor && p_b.tier == ItemTier::SuperHealth) ||
                (p_a.tier == ItemTier::SuperHealth && p_b.tier == ItemTier::SuperArmor)
            );

            if is_stacking {
                report.issues.push(BalanceIssue {
                    title: format!("⚠ Item Stacking Hazard: {} ↔ {}", p_a.name, p_b.name),
                    severity: BalanceSeverity::Critical,
                    description: format!(
                        "Super items are only {:.1}s run time apart ({:.0} uu). Allows single dominant player to lock down both items.",
                        time_secs, dist
                    ),
                    suggested_fix: "Separate major powerups to opposing sides of the arena (min 8-10 seconds running time).".to_string(),
                });
            }

            report.pair_timings.push(PickupPairTiming {
                item_a: p_a.name.clone(),
                item_b: p_b.name.clone(),
                distance_uu: dist,
                run_time_secs: time_secs,
                is_stacking_hazard: is_stacking,
            });
        }
    }

    // 3. Measure PlayerStart to nearest weapon travel times
    let weapons: Vec<&PickupItem> = report.pickups.iter().filter(|p| {
        matches!(p.tier, ItemTier::PowerWeapon | ItemTier::StandardWeapon)
    }).collect();

    let mut total_spawn_time = 0.0;
    for (spawn_name, spawn_loc) in &spawns {
        let mut min_time = f32::MAX;
        for w in &weapons {
            let dist = calculate_travel_distance(*spawn_loc, w.location, nav_graph);
            let t = dist / PLAYER_RUN_SPEED_UU_PER_SEC;
            if t < min_time {
                min_time = t;
            }
        }

        if min_time == f32::MAX {
            min_time = 999.0;
        }

        if min_time > 10.0 {
            report.issues.push(BalanceIssue {
                title: format!("⚠ Spawn Weapon Starvation: {}", spawn_name),
                severity: BalanceSeverity::Warning,
                description: format!("Player spawning at '{}' must run for {:.1}s before reaching any weapon.", spawn_name, min_time),
                suggested_fix: "Place a starter weapon (Shock, Pulse, Bio) within 3-5 seconds of this spawn point.".to_string(),
            });
        }

        total_spawn_time += min_time;
        report.spawn_times_to_weapon.push((spawn_name.clone(), min_time));
    }

    if !spawns.is_empty() {
        report.average_spawn_to_weapon_secs = total_spawn_time / spawns.len() as f32;
    }

    // 4. Certification Check
    let has_critical_issues = report.issues.iter().any(|i| i.severity == BalanceSeverity::Critical);
    let has_enough_weapons = weapons.len() >= 3 || spawns.len() <= 2;
    report.is_tournament_certified = !has_critical_issues && has_enough_weapons && report.average_spawn_to_weapon_secs < 8.0;

    report
}

/// Helper calculating travel distance either via navigation path nodes or straight 3D line
fn calculate_travel_distance(start: Vec3, end: Vec3, nav_graph: &NavGraph) -> f32 {
    if nav_graph.nodes.is_empty() {
        return start.distance(end);
    }

    // Find nearest nav node to start and end
    let start_node = nav_graph.find_nearest_node(start);
    let end_node = nav_graph.find_nearest_node(end);

    if let (Some(n_s), Some(n_e)) = (start_node, end_node) {
        if n_s == n_e {
            return start.distance(end);
        }

        // Try Dijkstra path
        if let Some(path) = nav_graph.find_shortest_path(n_s, n_e) {
            let mut path_dist = start.distance(nav_graph.nodes[n_s].location);
            for w in 0..path.len().saturating_sub(1) {
                let p0 = nav_graph.nodes[path[w]].location;
                let p1 = nav_graph.nodes[path[w + 1]].location;
                path_dist += p0.distance(p1);
            }
            path_dist += nav_graph.nodes[n_e].location.distance(end);
            return path_dist;
        }
    }

    start.distance(end)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_item_balance_stacking() {
        let actors = vec![
            ("Spawn1".to_string(), "PlayerStart".to_string(), Vec3::new(0.0, 0.0, 0.0)),
            ("Shield1".to_string(), "UT_ShieldBelt".to_string(), Vec3::new(100.0, 0.0, 0.0)),
            ("Amp1".to_string(), "UDamage".to_string(), Vec3::new(300.0, 0.0, 0.0)), // Only 200 uu away = 0.5s!
            ("Shock1".to_string(), "ShockRifle".to_string(), Vec3::new(50.0, 50.0, 0.0)),
        ];

        let nav = NavGraph::default();
        let report = analyze_item_balance(&actors, &nav);

        assert_eq!(report.pickups.len(), 3);
        assert!(!report.is_tournament_certified, "Map with 0.5s Shield+Amp stacking must fail certification");
        assert!(report.issues.iter().any(|i| i.severity == BalanceSeverity::Critical));
    }

    #[test]
    fn test_analyze_item_balance_fair() {
        let actors = vec![
            ("Spawn1".to_string(), "PlayerStart".to_string(), Vec3::new(0.0, 0.0, 0.0)),
            ("Spawn2".to_string(), "PlayerStart".to_string(), Vec3::new(4000.0, 0.0, 0.0)),
            ("Shield1".to_string(), "UT_ShieldBelt".to_string(), Vec3::new(1000.0, 0.0, 0.0)),
            ("Amp1".to_string(), "UDamage".to_string(), Vec3::new(5000.0, 0.0, 0.0)), // 4000 uu away = 10s!
            ("Flak1".to_string(), "UT_FlakCannon".to_string(), Vec3::new(200.0, 0.0, 0.0)),
            ("Shock1".to_string(), "ShockRifle".to_string(), Vec3::new(3800.0, 0.0, 0.0)),
            ("Sniper1".to_string(), "SniperRifle".to_string(), Vec3::new(2500.0, 0.0, 0.0)),
        ];

        let nav = NavGraph::default();
        let report = analyze_item_balance(&actors, &nav);

        assert_eq!(report.pickups.len(), 5);
        assert!(report.is_tournament_certified, "Well-spaced competitive map must be tournament certified");
        assert!(report.average_spawn_to_weapon_secs < 2.0);
    }
}
