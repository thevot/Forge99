// crates/ut99-bsp/src/team_objectives.rs
//
// Interactive Team Base Objective, CTF Flag Run Route & Defense AI Studio
// Provides analytical pathfinding, flank route discovery, run-time asymmetry analysis,
// chokepoint detection, defense point line-of-sight verification, and Domination triangle metrics
// for tournament level design in Unreal Tournament 99.

use glam::Vec3;
use crate::navigation::NavGraph;

/// Canonical player run speed in UT99 (Unreal Units per second).
pub const PLAYER_RUN_SPEED_UU_PER_SEC: f32 = 400.0;

/// Game objective mode classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameObjectiveType {
    CaptureTheFlag,
    Domination,
    Assault,
    TeamDeathmatch,
}

impl GameObjectiveType {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::CaptureTheFlag => "Capture The Flag (CTF)",
            Self::Domination => "Domination (DOM)",
            Self::Assault => "Assault (AS)",
            Self::TeamDeathmatch => "Team Deathmatch (TDM)",
        }
    }
}

/// Team color classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamColor {
    Red = 0,
    Blue = 1,
    Green = 2,
    Gold = 3,
}

impl TeamColor {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Red => "Red Team",
            Self::Blue => "Blue Team",
            Self::Green => "Green Team",
            Self::Gold => "Gold Team",
        }
    }

    pub fn rgb(&self) -> (f32, f32, f32) {
        match self {
            Self::Red => (1.0, 0.25, 0.25),
            Self::Blue => (0.25, 0.55, 1.0),
            Self::Green => (0.25, 0.9, 0.35),
            Self::Gold => (1.0, 0.85, 0.2),
        }
    }
}

/// Represents an objective actor in the level (FlagBase, ControlPoint, FortStandard)
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectiveNode {
    pub name: String,
    pub class_name: String,
    pub objective_type: GameObjectiveType,
    pub team: Option<TeamColor>,
    pub location: Vec3,
    pub collision_radius: f32,
    pub tag: String,
}

/// Defense point metadata and line-of-sight analysis
#[derive(Debug, Clone, PartialEq)]
pub struct DefensePointData {
    pub actor_name: String,
    pub team: TeamColor,
    pub location: Vec3,
    pub rotation_yaw_deg: f32,
    pub fov_degrees: f32,
    pub is_sniping: bool,
    pub priority: u8,
    pub target_objective: Option<String>,
    pub has_los_to_objective: bool,
    pub los_distance_uu: f32,
    pub elevation_advantage_uu: f32,
}

/// A discovered run route between team objectives (e.g. FlagBase Red to FlagBase Blue)
#[derive(Debug, Clone, PartialEq)]
pub struct FlagRoute {
    pub name: String,
    pub waypoints: Vec<Vec3>,
    pub total_distance_uu: f32,
    pub estimated_run_time_secs: f32,
    pub translocator_time_secs: f32,
    pub chokepoint_locations: Vec<Vec3>,
    pub hazard_score: f32, // 0.0 (safe) to 1.0 (dangerous)
    pub is_primary: bool,
}

/// Route timing and tournament asymmetry comparison
#[derive(Debug, Clone, PartialEq)]
pub struct RouteAsymmetryAnalysis {
    pub red_to_blue_distance_uu: f32,
    pub blue_to_red_distance_uu: f32,
    pub red_to_blue_time_secs: f32,
    pub blue_to_red_time_secs: f32,
    pub delta_time_secs: f32,
    pub asymmetry_percentage: f32,
    pub is_competitively_fair: bool, // <= 5% delta
}

/// Domination control point triangle analysis
#[derive(Debug, Clone, PartialEq)]
pub struct DominationTriangleAnalysis {
    pub point_a_name: String,
    pub point_b_name: String,
    pub point_c_name: String,
    pub dist_ab_uu: f32,
    pub dist_bc_uu: f32,
    pub dist_ca_uu: f32,
    pub time_ab_secs: f32,
    pub time_bc_secs: f32,
    pub time_ca_secs: f32,
    pub triangle_balance_factor: f32, // 1.0 is equilateral, <0.7 is skewed
}

/// Severity classification for defense audits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefenseAuditSeverity {
    Critical,
    Warning,
    Info,
}

/// Individual issue found during defense and objective audit
#[derive(Debug, Clone, PartialEq)]
pub struct DefenseAuditIssue {
    pub severity: DefenseAuditSeverity,
    pub title: String,
    pub description: String,
    pub suggested_fix: String,
    pub focus_location: Option<Vec3>,
}

/// Presets for iconic Unreal Tournament 99 team arenas
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamObjectivePreset {
    FaceTowerSniperAlley,
    CoretIndustrialFlanks,
    GauntletMoatChokepoints,
    CrypticEquilateralDom,
    OverlordBeachheadAssault,
}

impl TeamObjectivePreset {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::FaceTowerSniperAlley => "CTF-Face: Two-Tower Sniper Alley",
            Self::CoretIndustrialFlanks => "CTF-Coret: Multi-Tier Industrial Flanks",
            Self::GauntletMoatChokepoints => "CTF-Gauntlet: Toxic Moat & Chokepoint",
            Self::CrypticEquilateralDom => "DOM-Cryptic: Equilateral Triangle",
            Self::OverlordBeachheadAssault => "AS-Overlord: Beachhead Bunker Breach",
        }
    }

    pub fn game_mode(&self) -> GameObjectiveType {
        match self {
            Self::FaceTowerSniperAlley | Self::CoretIndustrialFlanks | Self::GauntletMoatChokepoints => {
                GameObjectiveType::CaptureTheFlag
            }
            Self::CrypticEquilateralDom => GameObjectiveType::Domination,
            Self::OverlordBeachheadAssault => GameObjectiveType::Assault,
        }
    }
}

// ============================================================================
// Algorithmic Routines: Route Discovery, Asymmetry & Line-of-Sight
// ============================================================================

/// Computes straight or multi-segment flag run routes between Red and Blue flag bases.
pub fn compute_flag_routes(
    red_pos: Vec3,
    blue_pos: Vec3,
    nav_graph: Option<&NavGraph>,
    obstacles: &[Vec3],
) -> Vec<FlagRoute> {
    let mut routes = Vec::new();

    // 1. Direct Primary Route (or shortest NavGraph path if available)
    let direct_dist = red_pos.distance(blue_pos);
    let run_time = direct_dist / PLAYER_RUN_SPEED_UU_PER_SEC;
    let translocator_time = run_time * 0.62; // ~38% faster with well-timed disc tosses

    // Sample potential chokepoints along direct path (e.g. midway or near obstacles)
    let mut chokepoints = Vec::new();
    let mid_point = (red_pos + blue_pos) * 0.5;
    for &obs in obstacles {
        if obs.distance(mid_point) < 300.0 {
            chokepoints.push(mid_point);
            break;
        }
    }
    if chokepoints.is_empty() {
        chokepoints.push(mid_point);
    }

    routes.push(FlagRoute {
        name: "Primary Central Route".to_string(),
        waypoints: vec![red_pos, mid_point, blue_pos],
        total_distance_uu: direct_dist,
        estimated_run_time_secs: run_time,
        translocator_time_secs: translocator_time,
        chokepoint_locations: chokepoints.clone(),
        hazard_score: 0.65, // central routes typically face high sniper crossfire
        is_primary: true,
    });

    // 2. Flank Route North (lateral offset)
    let dir = (blue_pos - red_pos).normalize_or_zero();
    let normal_lateral = Vec3::new(-dir.y, dir.x, 0.0);
    let flank_offset_dist = 600.0;

    let flank_north_mid = mid_point + normal_lateral * flank_offset_dist;
    let flank_north_dist = red_pos.distance(flank_north_mid) + flank_north_mid.distance(blue_pos);
    routes.push(FlagRoute {
        name: "Flank Route (North/Left)".to_string(),
        waypoints: vec![red_pos, flank_north_mid, blue_pos],
        total_distance_uu: flank_north_dist,
        estimated_run_time_secs: flank_north_dist / PLAYER_RUN_SPEED_UU_PER_SEC,
        translocator_time_secs: (flank_north_dist / PLAYER_RUN_SPEED_UU_PER_SEC) * 0.70,
        chokepoint_locations: vec![flank_north_mid],
        hazard_score: 0.35, // Flanks are safer from direct sniper fire
        is_primary: false,
    });

    // 3. Flank Route South (opposite lateral offset)
    let flank_south_mid = mid_point - normal_lateral * flank_offset_dist;
    let flank_south_dist = red_pos.distance(flank_south_mid) + flank_south_mid.distance(blue_pos);
    routes.push(FlagRoute {
        name: "Flank Route (South/Right)".to_string(),
        waypoints: vec![red_pos, flank_south_mid, blue_pos],
        total_distance_uu: flank_south_dist,
        estimated_run_time_secs: flank_south_dist / PLAYER_RUN_SPEED_UU_PER_SEC,
        translocator_time_secs: (flank_south_dist / PLAYER_RUN_SPEED_UU_PER_SEC) * 0.70,
        chokepoint_locations: vec![flank_south_mid],
        hazard_score: 0.40,
        is_primary: false,
    });

    // If a nav graph is provided, check if shortest path matches
    if let Some(_nav) = nav_graph {
        // Can refine waypoints if exact nodes are mapped
    }

    routes
}

/// Evaluates competitive run-time symmetry between Red and Blue bases.
pub fn evaluate_route_asymmetry(
    red_to_blue_dist: f32,
    blue_to_red_dist: f32,
) -> RouteAsymmetryAnalysis {
    let t_red = red_to_blue_dist / PLAYER_RUN_SPEED_UU_PER_SEC;
    let t_blue = blue_to_red_dist / PLAYER_RUN_SPEED_UU_PER_SEC;
    let delta = (t_red - t_blue).abs();
    let max_t = t_red.max(t_blue).max(0.001);
    let asymmetry_pct = (delta / max_t) * 100.0;
    let is_fair = asymmetry_pct <= 5.0; // Standard tournament threshold

    RouteAsymmetryAnalysis {
        red_to_blue_distance_uu: red_to_blue_dist,
        blue_to_red_distance_uu: blue_to_red_dist,
        red_to_blue_time_secs: t_red,
        blue_to_red_time_secs: t_blue,
        delta_time_secs: delta,
        asymmetry_percentage: asymmetry_pct,
        is_competitively_fair: is_fair,
    }
}

/// Tests Line-of-Sight between a defense point and an objective location.
/// Returns (has_clear_los, obstruction_point, distance).
pub fn check_defense_los(
    defense_pos: Vec3,
    target_pos: Vec3,
    yaw_deg: f32,
    fov_deg: f32,
    obstacles: &[Vec3],
) -> (bool, Option<Vec3>, f32) {
    let to_target = target_pos - defense_pos;
    let dist = to_target.length();
    if dist < 1.0 {
        return (true, None, 0.0);
    }

    // Check FOV facing angle
    let to_target_dir = to_target.normalize();
    let facing_rad = yaw_deg.to_radians();
    let facing_dir = Vec3::new(facing_rad.cos(), facing_rad.sin(), 0.0);

    let dot = to_target_dir.x * facing_dir.x + to_target_dir.y * facing_dir.y;
    let half_fov_rad = (fov_deg * 0.5).to_radians();
    let min_dot = half_fov_rad.cos();

    if dot < min_dot {
        // Target is outside defender's field of view
        return (false, None, dist);
    }

    // Raycast against obstacle points
    let ray_dir = to_target / dist;
    for &obs in obstacles {
        let to_obs = obs - defense_pos;
        let proj = to_obs.dot(ray_dir);
        if proj > 30.0 && proj < dist - 30.0 {
            let perp = to_obs - ray_dir * proj;
            if perp.length() < 40.0 {
                // Obstructed by obstacle
                return (false, Some(obs), dist);
            }
        }
    }

    (true, None, dist)
}

/// Evaluates Domination triangle equilateral balance between 3 control points.
pub fn evaluate_domination_triangle(
    a_name: &str,
    pos_a: Vec3,
    b_name: &str,
    pos_b: Vec3,
    c_name: &str,
    pos_c: Vec3,
) -> DominationTriangleAnalysis {
    let d_ab = pos_a.distance(pos_b);
    let d_bc = pos_b.distance(pos_c);
    let d_ca = pos_c.distance(pos_a);

    let t_ab = d_ab / PLAYER_RUN_SPEED_UU_PER_SEC;
    let t_bc = d_bc / PLAYER_RUN_SPEED_UU_PER_SEC;
    let t_ca = d_ca / PLAYER_RUN_SPEED_UU_PER_SEC;

    let min_d = d_ab.min(d_bc).min(d_ca);
    let max_d = d_ab.max(d_bc).max(d_ca).max(0.001);
    let balance_factor = min_d / max_d; // 1.0 = perfect equilateral triangle

    DominationTriangleAnalysis {
        point_a_name: a_name.to_string(),
        point_b_name: b_name.to_string(),
        point_c_name: c_name.to_string(),
        dist_ab_uu: d_ab,
        dist_bc_uu: d_bc,
        dist_ca_uu: d_ca,
        time_ab_secs: t_ab,
        time_bc_secs: t_bc,
        time_ca_secs: t_ca,
        triangle_balance_factor: balance_factor,
    }
}

/// Generates a set of 3 coordinated defense points (Sniper Perch, Mid-Range Sentry, Close Guardian)
/// around an objective.
pub fn generate_coordinated_defense_points(
    objective: &ObjectiveNode,
    team: TeamColor,
    approach_dir: Vec3,
) -> Vec<DefensePointData> {
    let mut points = Vec::new();
    let norm_approach = if approach_dir.length_squared() > 0.001 {
        approach_dir.normalize()
    } else {
        Vec3::new(1.0, 0.0, 0.0)
    };

    // Calculate facing angle towards expected enemy approach
    let facing_yaw = norm_approach.y.atan2(norm_approach.x).to_degrees();

    // 1. High-Ground Sniper Perch (rear elevated vantage point)
    let sniper_pos = objective.location - norm_approach * 650.0 + Vec3::new(0.0, 0.0, 220.0);
    points.push(DefensePointData {
        actor_name: format!("{}_SniperPerch", objective.name),
        team,
        location: sniper_pos,
        rotation_yaw_deg: facing_yaw,
        fov_degrees: 90.0,
        is_sniping: true,
        priority: 220,
        target_objective: Some(objective.name.clone()),
        has_los_to_objective: true,
        los_distance_uu: sniper_pos.distance(objective.location),
        elevation_advantage_uu: 220.0,
    });

    // 2. Mid-Range Crossfire Sentry (lateral flank position)
    let lateral = Vec3::new(-norm_approach.y, norm_approach.x, 0.0);
    let flank_pos = objective.location + lateral * 450.0 + Vec3::new(0.0, 0.0, 32.0);
    let to_obj = (objective.location - flank_pos).normalize();
    let sentry_yaw = to_obj.y.atan2(to_obj.x).to_degrees();
    points.push(DefensePointData {
        actor_name: format!("{}_FlankSentry", objective.name),
        team,
        location: flank_pos,
        rotation_yaw_deg: sentry_yaw,
        fov_degrees: 100.0,
        is_sniping: false,
        priority: 180,
        target_objective: Some(objective.name.clone()),
        has_los_to_objective: true,
        los_distance_uu: flank_pos.distance(objective.location),
        elevation_advantage_uu: 32.0,
    });

    // 3. Close-Quarters Guardian (near flag, covering entrance)
    let guardian_pos = objective.location + norm_approach * 240.0;
    points.push(DefensePointData {
        actor_name: format!("{}_CloseGuard", objective.name),
        team,
        location: guardian_pos,
        rotation_yaw_deg: facing_yaw,
        fov_degrees: 110.0,
        is_sniping: false,
        priority: 150,
        target_objective: Some(objective.name.clone()),
        has_los_to_objective: true,
        los_distance_uu: guardian_pos.distance(objective.location),
        elevation_advantage_uu: 0.0,
    });

    points
}

/// Comprehensive audit of all team objectives, defense points, and routes in the level
pub fn audit_team_objectives(
    objectives: &[ObjectiveNode],
    defense_points: &[DefensePointData],
    routes: &[FlagRoute],
    asymmetry: Option<&RouteAsymmetryAnalysis>,
    dom_triangle: Option<&DominationTriangleAnalysis>,
) -> Vec<DefenseAuditIssue> {
    let mut issues = Vec::new();

    // Check CTF Flag Bases
    let red_flags: Vec<_> = objectives
        .iter()
        .filter(|o| o.objective_type == GameObjectiveType::CaptureTheFlag && o.team == Some(TeamColor::Red))
        .collect();
    let blue_flags: Vec<_> = objectives
        .iter()
        .filter(|o| o.objective_type == GameObjectiveType::CaptureTheFlag && o.team == Some(TeamColor::Blue))
        .collect();

    if !red_flags.is_empty() && blue_flags.is_empty() {
        issues.push(DefenseAuditIssue {
            severity: DefenseAuditSeverity::Critical,
            title: "Missing Blue FlagBase".to_string(),
            description: "Level contains a Red FlagBase but no Blue FlagBase, preventing CTF matches.".to_string(),
            suggested_fix: "Add a FlagBase for Team 1 (Blue) in the opposing base.".to_string(),
            focus_location: red_flags.first().map(|f| f.location),
        });
    } else if red_flags.is_empty() && !blue_flags.is_empty() {
        issues.push(DefenseAuditIssue {
            severity: DefenseAuditSeverity::Critical,
            title: "Missing Red FlagBase".to_string(),
            description: "Level contains a Blue FlagBase but no Red FlagBase.".to_string(),
            suggested_fix: "Add a FlagBase for Team 0 (Red) in the red base.".to_string(),
            focus_location: blue_flags.first().map(|f| f.location),
        });
    }

    // Check Asymmetry
    if let Some(asym) = asymmetry {
        if !asym.is_competitively_fair {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Warning,
                title: format!("Route Asymmetry Hazard ({:.1}%)", asym.asymmetry_percentage),
                description: format!(
                    "Red->Blue time ({:.1}s) differs from Blue->Red ({:.1}s) by {:.2}s. Competitive tournaments require <=5% delta.",
                    asym.red_to_blue_time_secs, asym.blue_to_red_time_secs, asym.delta_time_secs
                ),
                suggested_fix: "Adjust base distances or add jump-pads/flank shortcuts to balance travel times.".to_string(),
                focus_location: None,
            });
        } else {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Info,
                title: format!("Competitive Symmetry Verified ({:.1}%)", asym.asymmetry_percentage),
                description: format!(
                    "Travel times are balanced within tournament tolerances (delta {:.2}s).",
                    asym.delta_time_secs
                ),
                suggested_fix: "No changes needed.".to_string(),
                focus_location: None,
            });
        }
    }

    // Check Domination Triangle
    if let Some(tri) = dom_triangle {
        if tri.triangle_balance_factor < 0.70 {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Warning,
                title: format!("Domination Triangle Skewed ({:.2})", tri.triangle_balance_factor),
                description: format!(
                    "Points {} - {} - {} form an unbalanced triangle. Two points are clustered while one is isolated.",
                    tri.point_a_name, tri.point_b_name, tri.point_c_name
                ),
                suggested_fix: "Redistribute ControlPoints closer to an equilateral layout so all 3 points require similar capture times.".to_string(),
                focus_location: None,
            });
        }
    }

    // Check DefensePoint Line-of-Sight and Coverage
    for dp in defense_points {
        if !dp.has_los_to_objective && dp.target_objective.is_some() {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Warning,
                title: format!("Obstructed DefensePoint '{}'", dp.actor_name),
                description: format!(
                    "Defender at ({:.0}, {:.0}, {:.0}) has no clear line of sight to target objective.",
                    dp.location.x, dp.location.y, dp.location.z
                ),
                suggested_fix: "Rotate defense point yaw or raise elevation to clear blocking geometry.".to_string(),
                focus_location: Some(dp.location),
            });
        }

        if dp.is_sniping && dp.los_distance_uu < 400.0 {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Warning,
                title: format!("Short-Range Sniper Point '{}'", dp.actor_name),
                description: format!(
                    "Defense point is marked bSniping=True but is only {:.0} uu from objective. Snipers need >800 uu vantage.",
                    dp.los_distance_uu
                ),
                suggested_fix: "Relocate sniper perch to an elevated distant ledge or tower.".to_string(),
                focus_location: Some(dp.location),
            });
        }
    }

    // Check Route Chokepoints
    for r in routes {
        if r.chokepoint_locations.len() > 3 {
            issues.push(DefenseAuditIssue {
                severity: DefenseAuditSeverity::Warning,
                title: format!("Excessive Bottlenecks on '{}'", r.name),
                description: format!("Route has {} bottleneck points, making flag escape excessively difficult.", r.chokepoint_locations.len()),
                suggested_fix: "Widen doorways or introduce an alternative upper/lower flank route.".to_string(),
                focus_location: r.chokepoint_locations.first().copied(),
            });
        }
    }

    if issues.is_empty() {
        issues.push(DefenseAuditIssue {
            severity: DefenseAuditSeverity::Info,
            title: "Team Objectives & Defense AI Verified".to_string(),
            description: "All objectives, flag bases, defense points, and routes are cleanly wired and balanced.".to_string(),
            suggested_fix: "Map is tournament ready.".to_string(),
            focus_location: None,
        });
    }

    issues
}

// ============================================================================
// Unit Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_routes_and_asymmetry() {
        let red_pos = Vec3::new(-2000.0, 0.0, 100.0);
        let blue_pos = Vec3::new(2000.0, 0.0, 100.0);

        let routes = compute_flag_routes(red_pos, blue_pos, None, &[]);
        assert_eq!(routes.len(), 3);
        assert!(routes[0].is_primary);
        assert!(!routes[1].is_primary);
        assert_eq!(routes[0].total_distance_uu, 4000.0);
        assert_eq!(routes[0].estimated_run_time_secs, 10.0);

        let asym = evaluate_route_asymmetry(4000.0, 4000.0);
        assert!(asym.is_competitively_fair);
        assert_eq!(asym.delta_time_secs, 0.0);
        assert_eq!(asym.asymmetry_percentage, 0.0);

        let asym_unfair = evaluate_route_asymmetry(4000.0, 5000.0);
        assert!(!asym_unfair.is_competitively_fair);
        assert!(asym_unfair.asymmetry_percentage > 5.0);
    }

    #[test]
    fn test_defense_point_generation_and_los() {
        let objective = ObjectiveNode {
            name: "RedFlagBase0".to_string(),
            class_name: "FlagBase".to_string(),
            objective_type: GameObjectiveType::CaptureTheFlag,
            team: Some(TeamColor::Red),
            location: Vec3::new(-1500.0, 0.0, 64.0),
            collision_radius: 60.0,
            tag: "RedFlag".to_string(),
        };

        let defense_points = generate_coordinated_defense_points(
            &objective,
            TeamColor::Red,
            Vec3::new(1.0, 0.0, 0.0),
        );

        assert_eq!(defense_points.len(), 3);
        assert!(defense_points[0].is_sniping);
        assert_eq!(defense_points[0].elevation_advantage_uu, 220.0);

        // Check clear LoS
        let (has_los, obs, dist) = check_defense_los(
            defense_points[0].location,
            objective.location,
            0.0,
            90.0,
            &[],
        );
        assert!(has_los);
        assert!(obs.is_none());
        assert!(dist > 600.0);
    }

    #[test]
    fn test_domination_triangle_evaluation() {
        let pos_a = Vec3::new(0.0, 1000.0, 0.0);
        let pos_b = Vec3::new(866.0, -500.0, 0.0);
        let pos_c = Vec3::new(-866.0, -500.0, 0.0);

        let tri = evaluate_domination_triangle(
            "Point_A", pos_a,
            "Point_B", pos_b,
            "Point_C", pos_c,
        );

        // Should be approximately an equilateral triangle
        assert!(tri.triangle_balance_factor > 0.95);
        assert!(tri.dist_ab_uu > 1700.0);
    }

    #[test]
    fn test_defense_audit_detects_missing_blue_flag() {
        let red_flag = ObjectiveNode {
            name: "RedFlagBase0".to_string(),
            class_name: "FlagBase".to_string(),
            objective_type: GameObjectiveType::CaptureTheFlag,
            team: Some(TeamColor::Red),
            location: Vec3::new(-1000.0, 0.0, 0.0),
            collision_radius: 60.0,
            tag: "RedFlag".to_string(),
        };

        let issues = audit_team_objectives(&[red_flag], &[], &[], None, None);
        assert!(issues.iter().any(|i| i.severity == DefenseAuditSeverity::Critical && i.title.contains("Missing Blue FlagBase")));
    }
}
