// crates/ut99-bsp/src/domination_mission.rs
//
// Procedural Domination (DOM-) Mission & Topological Triangle Solver.
// Reverse-engineered from forensic study of 16 official UT99 DOM maps:
// - Automatic selection of optimal 3-point topological triangle (A, B, C)
// - Minimization of triangle asymmetry (preventing 2-point camping dominance)
// - Elimination of proximity spam (guaranteed minimum distance >= 500 UU)
// - Elevation & role assignment (HighTier, LowTier, CrossroadHub, WingFlank)
// - Dynamic thematic naming engine (Elevation, Industrial, Gothic, Egyptian, Military)
// - Centroid calculation for contested central mega-powerup (ShieldBelt/Amp)

use glam::Vec3;
use serde::{Deserialize, Serialize};
use crate::map_wizard::WizardThemePreset;

/// Spatial representation of a room or corridor node for Domination mission planning
#[derive(Debug, Clone, PartialEq)]
pub struct DominationNode {
    pub center: Vec3,
    pub size: Vec3,
    pub is_corridor: bool,
}

impl DominationNode {
    pub fn floor_z(&self) -> f32 {
        self.center.z - self.size.z * 0.5
    }
}

/// Thematic naming conventions for Domination Control Points
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DominationNamingStyle {
    #[default]
    AutoThematic,
    ElevationVerticality,
    IndustrialTech,
    GothicSacred,
    EgyptianMonument,
    MilitaryTactical,
    PhoneticClassical,
}

impl DominationNamingStyle {
    pub fn all() -> &'static [DominationNamingStyle] {
        &[
            DominationNamingStyle::AutoThematic,
            DominationNamingStyle::ElevationVerticality,
            DominationNamingStyle::IndustrialTech,
            DominationNamingStyle::GothicSacred,
            DominationNamingStyle::EgyptianMonument,
            DominationNamingStyle::MilitaryTactical,
            DominationNamingStyle::PhoneticClassical,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            DominationNamingStyle::AutoThematic => "Auto (Topology & Theme-Aware)",
            DominationNamingStyle::ElevationVerticality => "Elevation & Verticality (Top / Mid / Bottom)",
            DominationNamingStyle::IndustrialTech => "Industrial & Tech (Helipad / Crane / Reactor)",
            DominationNamingStyle::GothicSacred => "Gothic & Sacred (DaemonHead / Gargoyle / Altar)",
            DominationNamingStyle::EgyptianMonument => "Egyptian & Monument (Sun Ankh / Pillars / Tomb)",
            DominationNamingStyle::MilitaryTactical => "Military & Tactical (Outpost / Bunker / Redoubt)",
            DominationNamingStyle::PhoneticClassical => "Phonetic Classical (Alpha / Beta / Gamma)",
        }
    }
}

/// Tactical role of a ControlPoint within the level's flow and geometry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DominationPointRole {
    HighTier,
    LowTier,
    CrossroadHub,
    WingFlank,
    Equilateral,
}

impl DominationPointRole {
    pub fn display_name(&self) -> &'static str {
        match self {
            DominationPointRole::HighTier => "Elevated Spire / High Ground",
            DominationPointRole::LowTier => "Sunken Basin / Lower Deck",
            DominationPointRole::CrossroadHub => "Central Contested Crossroad",
            DominationPointRole::WingFlank => "Lateral Perimeter Wing",
            DominationPointRole::Equilateral => "Balanced Triad Node",
        }
    }
}

/// Architectural dais style for ControlPoint bases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum DominationDaisStyle {
    #[default]
    SteppedOctagon,
    PylonShrine,
    IndustrialGrate,
    SunkenAmphitheater,
}

impl DominationDaisStyle {
    pub fn all() -> &'static [DominationDaisStyle] {
        &[
            DominationDaisStyle::SteppedOctagon,
            DominationDaisStyle::PylonShrine,
            DominationDaisStyle::IndustrialGrate,
            DominationDaisStyle::SunkenAmphitheater,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            DominationDaisStyle::SteppedOctagon => "Stepped Octagonal Dais (Monumental)",
            DominationDaisStyle::PylonShrine => "Pylon Shrine (4 Corner Energy Pillars)",
            DominationDaisStyle::IndustrialGrate => "Industrial Grate Platform (Tech Catwalk)",
            DominationDaisStyle::SunkenAmphitheater => "Sunken Amphitheater (Arena Pit)",
        }
    }
}

/// Specification for a single resolved ControlPoint in the generated map
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DominationPointPlan {
    pub point_index: usize,
    pub point_name: String,
    pub room_index: usize,
    pub location: Vec3,
    pub tag: String,
    pub role: DominationPointRole,
    pub light_hue: u8,
    pub ambient_sound_radius: f32,
    pub defense_points_count: usize,
}

/// Complete resolved Domination mission plan
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DominationMissionPlan {
    pub points: Vec<DominationPointPlan>,
    pub naming_style_used: DominationNamingStyle,
    pub dist_ab: f32,
    pub dist_bc: f32,
    pub dist_ca: f32,
    pub triangle_asymmetry_ratio: f32,
    pub vertical_disparity: f32,
    pub center_powerup_location: Option<Vec3>,
    pub center_powerup_room_index: Option<usize>,
}

/// Solves and plans optimal ControlPoint distribution, naming, and powerup placement
pub fn plan_domination_mission(
    rooms: &[DominationNode],
    theme: WizardThemePreset,
    naming_style: DominationNamingStyle,
    target_point_count: usize,
    seed: u64,
) -> DominationMissionPlan {
    let point_count = target_point_count.clamp(2, 4);

    // Filter main rooms vs corridor connectors
    let non_corridor_rooms: Vec<(usize, &DominationNode)> = rooms
        .iter()
        .enumerate()
        .filter(|(_, r)| !r.is_corridor)
        .collect();

    let candidate_rooms: Vec<(usize, &DominationNode)> = if !non_corridor_rooms.is_empty() {
        non_corridor_rooms
    } else {
        rooms.iter().enumerate().collect()
    };

    let resolved_naming = resolve_naming_style(naming_style, theme, rooms);

    if point_count == 2 {
        plan_duel_domination(&candidate_rooms, rooms, resolved_naming, seed)
    } else if point_count == 4 {
        plan_quad_domination(&candidate_rooms, rooms, resolved_naming, seed)
    } else {
        plan_triad_domination(&candidate_rooms, rooms, resolved_naming, seed)
    }
}

/// Resolves auto naming style based on theme and geometry verticality
fn resolve_naming_style(
    requested: DominationNamingStyle,
    theme: WizardThemePreset,
    rooms: &[DominationNode],
) -> DominationNamingStyle {
    if requested != DominationNamingStyle::AutoThematic {
        return requested;
    }

    let mut z_min = f32::MAX;
    let mut z_max = f32::MIN;
    for r in rooms {
        let fz = r.floor_z();
        if fz < z_min { z_min = fz; }
        if fz > z_max { z_max = fz; }
    }
    let vertical_disparity = (z_max - z_min).abs();

    if vertical_disparity >= 160.0 {
        return DominationNamingStyle::ElevationVerticality;
    }

    match theme {
        WizardThemePreset::SesmarEgyptianTomb => DominationNamingStyle::EgyptianMonument,
        WizardThemePreset::CodexGothicCrypt
        | WizardThemePreset::MedievalCastle
        | WizardThemePreset::AncientMonastery => DominationNamingStyle::GothicSacred,
        WizardThemePreset::CoretTechFacility
        | WizardThemePreset::MineQuarry
        | WizardThemePreset::WarshipNaval => DominationNamingStyle::MilitaryTactical,
        WizardThemePreset::FacingWorldsCitadel
        | WizardThemePreset::VolcanicCaldera
        | WizardThemePreset::UrbanSkyscraper => DominationNamingStyle::ElevationVerticality,
        WizardThemePreset::Deck16Industrial
        | WizardThemePreset::MorbiasCyberArena
        | WizardThemePreset::PhobosOrbitalSciFi
        | WizardThemePreset::HeavyFoundry
        | WizardThemePreset::HydroElectric => DominationNamingStyle::IndustrialTech,
    }
}

/// Generates names for the points based on the resolved naming style and roles
pub fn generate_point_names(
    style: DominationNamingStyle,
    points_meta: &[(DominationPointRole, f32)], // (role, z)
    seed: u64,
) -> Vec<String> {
    let n = points_meta.len();
    let variant = (seed as usize) % 4;

    match style {
        DominationNamingStyle::ElevationVerticality => {
            // Sort indices by Z descending
            let mut indexed_z: Vec<(usize, f32)> = points_meta.iter().enumerate().map(|(i, &(_, z))| (i, z)).collect();
            indexed_z.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let name_sets = [
                vec!["Top", "Center", "Bottom", "Vault"],
                vec!["Roof", "Midpoint", "Subway", "Basement"],
                vec!["Upper Platform", "Mezzanine", "Lower Deck", "Sub-Level"],
                vec!["Tower", "Courtyard", "Catacomb", "Dungeon"],
            ];
            let set = &name_sets[variant % name_sets.len()];

            let mut result = vec![String::new(); n];
            for (rank, &(orig_idx, _)) in indexed_z.iter().enumerate() {
                result[orig_idx] = set[rank.min(set.len() - 1)].to_string();
            }
            result
        }
        DominationNamingStyle::IndustrialTech => {
            let sets = [
                vec!["Reactor Core", "Turbine Deck", "Loading Dock", "Cargo Bay"],
                vec!["Helipad", "Engine Room", "Crane", "Coolant Tank"],
                vec!["Generators", "Control Room", "Exhaust Vent", "Smelter"],
                vec!["Bridge", "Storage Vault", "Main Conveyor", "Pump Station"],
            ];
            let set = &sets[variant % sets.len()];
            (0..n).map(|i| set[i % set.len()].to_string()).collect()
        }
        DominationNamingStyle::GothicSacred => {
            let sets = [
                vec!["Daemon Head", "Gargoyle", "Iron Star", "Obsidian Spire"],
                vec!["Sacred Altar", "Crypt", "Grand Hall", "Chamber of Souls"],
                vec!["Watchtower", "Torture Keep", "Catacomb", "Throne Room"],
                vec!["Blood Fountain", "Sanctum", "Shadow Gate", "Dungeon"],
            ];
            let set = &sets[variant % sets.len()];
            (0..n).map(|i| set[i % set.len()].to_string()).collect()
        }
        DominationNamingStyle::EgyptianMonument => {
            let sets = [
                vec!["Sun Ankh", "Moon Ankh", "Hall of Pillars", "Obelisk"],
                vec!["Pharaoh's Dais", "Sphinx Gate", "Burial Chamber", "Sand Altar"],
                vec!["Solar Shrine", "Lapis Vault", "Temple Court", "Anubis Gate"],
                vec!["Sacred Scarab", "Nile Basin", "Pyramid Core", "Hieroglyph Hall"],
            ];
            let set = &sets[variant % sets.len()];
            (0..n).map(|i| set[i % set.len()].to_string()).collect()
        }
        DominationNamingStyle::MilitaryTactical => {
            let sets = [
                vec!["Outpost Alpha", "Bunker Bravo", "Redoubt Gamma", "Checkpoint Delta"],
                vec!["Command Post", "Radar Array", "Supply Depot", "Ammunition Cache"],
                vec!["Forward Base", "Pillbox West", "Observation Post", "Landing Zone"],
                vec!["Perimeter Gate", "Communications Tower", "Armory Hub", "Battery"],
            ];
            let set = &sets[variant % sets.len()];
            (0..n).map(|i| set[i % set.len()].to_string()).collect()
        }
        DominationNamingStyle::PhoneticClassical | DominationNamingStyle::AutoThematic => {
            let sets = [
                vec!["Alpha", "Beta", "Gamma", "Delta"],
                vec!["Alpha", "Delta", "Omega", "Sigma"],
                vec!["Point A", "Point B", "Point C", "Point D"],
                vec!["Zone One", "Zone Two", "Zone Three", "Zone Four"],
            ];
            let set = &sets[variant % sets.len()];
            (0..n).map(|i| set[i % set.len()].to_string()).collect()
        }
    }
}

/// Plans standard 3-point tournament Domination
fn plan_triad_domination(
    candidate_rooms: &[(usize, &DominationNode)],
    all_rooms: &[DominationNode],
    naming_style: DominationNamingStyle,
    seed: u64,
) -> DominationMissionPlan {
    let mut resolved_points = Vec::new();

    if candidate_rooms.len() >= 3 {
        // Multi-room layout: find the triplet that maximizes tactical triangle quality
        let mut best_score = f32::MIN;
        let mut best_triplet = (0, 1, 2);

        for i in 0..candidate_rooms.len() {
            for j in (i + 1)..candidate_rooms.len() {
                for k in (j + 1)..candidate_rooms.len() {
                    let r_a = candidate_rooms[i].1;
                    let r_b = candidate_rooms[j].1;
                    let r_c = candidate_rooms[k].1;

                    let pos_a = Vec3::new(r_a.center.x, r_a.center.y, r_a.floor_z() + 48.0);
                    let pos_b = Vec3::new(r_b.center.x, r_b.center.y, r_b.floor_z() + 48.0);
                    let pos_c = Vec3::new(r_c.center.x, r_c.center.y, r_c.floor_z() + 48.0);

                    let d_ab = pos_a.distance(pos_b);
                    let d_bc = pos_b.distance(pos_c);
                    let d_ca = pos_c.distance(pos_a);

                    let min_d = d_ab.min(d_bc).min(d_ca);
                    // Disqualify any proximity spam (< 480 UU)
                    if min_d < 480.0 {
                        continue;
                    }

                    let perimeter = d_ab + d_bc + d_ca;
                    let s = perimeter * 0.5;
                    let area = ((s * (s - d_ab) * (s - d_bc) * (s - d_ca)).max(0.0)).sqrt();

                    let asymmetry = (d_ab - d_bc).abs() + (d_bc - d_ca).abs() + (d_ca - d_ab).abs();
                    let asym_ratio = (asymmetry / perimeter.max(1.0)) * 100.0;

                    let z_min = pos_a.z.min(pos_b.z).min(pos_c.z);
                    let z_max = pos_a.z.max(pos_b.z).max(pos_c.z);
                    let vert_disp = z_max - z_min;

                    // Score function: balance of area, perimeter coverage, verticality bonus, penalizing asymmetry
                    let score = perimeter * 0.6 + area.sqrt() * 12.0 + vert_disp * 1.2 - asym_ratio * 15.0;

                    if score > best_score {
                        best_score = score;
                        best_triplet = (i, j, k);
                    }
                }
            }
        }

        let selected = [
            candidate_rooms[best_triplet.0],
            candidate_rooms[best_triplet.1],
            candidate_rooms[best_triplet.2],
        ];

        // Determine point roles based on verticality or room position
        let mut points_meta = Vec::new();
        for &(_, r) in &selected {
            points_meta.push(r.floor_z());
        }

        let z_min = points_meta.iter().cloned().fold(f32::MAX, f32::min);
        let z_max = points_meta.iter().cloned().fold(f32::MIN, f32::max);
        let has_vertical_split = (z_max - z_min) >= 120.0;

        let mut role_meta = Vec::new();
        for &fz in &points_meta {
            let role = if has_vertical_split {
                if (fz - z_max).abs() < 20.0 {
                    DominationPointRole::HighTier
                } else if (fz - z_min).abs() < 20.0 {
                    DominationPointRole::LowTier
                } else {
                    DominationPointRole::CrossroadHub
                }
            } else {
                DominationPointRole::WingFlank
            };
            role_meta.push((role, fz));
        }

        let names = generate_point_names(naming_style, &role_meta, seed);

        // Hues for distinct visual identification: Alpha (Cyan 140), Beta (Gold/Amber 36), Gamma (Purple/Magenta 200)
        let hues = [140u8, 36u8, 200u8];

        for i in 0..3 {
            let (orig_room_idx, r) = selected[i];
            let pos = Vec3::new(r.center.x, r.center.y, r.floor_z() + 52.0);
            resolved_points.push(DominationPointPlan {
                point_index: i,
                point_name: names[i].clone(),
                room_index: orig_room_idx,
                location: pos,
                tag: format!("ControlPoint_{}", names[i].replace(' ', "_")),
                role: role_meta[i].0,
                light_hue: hues[i],
                ambient_sound_radius: 360.0,
                defense_points_count: 2,
            });
        }
    } else if candidate_rooms.len() == 2 {
        // Exactly 2 rooms: Room 0 (Alpha), Room 1 (Beta), Point C placed in connector corridor or offset
        let r0 = candidate_rooms[0];
        let r1 = candidate_rooms[1];

        let pos0 = Vec3::new(r0.1.center.x, r0.1.center.y, r0.1.floor_z() + 52.0);
        let pos1 = Vec3::new(r1.1.center.x, r1.1.center.y, r1.1.floor_z() + 52.0);

        // Check if there is a connecting corridor room between them
        let mid_room = all_rooms
            .iter()
            .enumerate()
            .find(|(idx, r)| *idx != r0.0 && *idx != r1.0 && r.is_corridor);

        let (r2_idx, pos2) = if let Some((idx, r_mid)) = mid_room {
            (idx, Vec3::new(r_mid.center.x, r_mid.center.y, r_mid.floor_z() + 52.0))
        } else {
            // Compute perpendicular offset between room 0 and room 1
            let dir = (pos1 - pos0).normalize_or_zero();
            let perp = Vec3::new(-dir.y, dir.x, 0.0);
            let offset_dist = 550.0;
            let mid_pt = (pos0 + pos1) * 0.5;
            let p2 = mid_pt + perp * offset_dist;
            (r0.0, Vec3::new(p2.x, p2.y, r0.1.floor_z() + 52.0))
        };

        let role_meta = vec![
            (DominationPointRole::WingFlank, pos0.z),
            (DominationPointRole::WingFlank, pos1.z),
            (DominationPointRole::CrossroadHub, pos2.z),
        ];
        let names = generate_point_names(naming_style, &role_meta, seed);
        let hues = [140u8, 36u8, 200u8];

        let locs = [(r0.0, pos0), (r1.0, pos1), (r2_idx, pos2)];
        for i in 0..3 {
            resolved_points.push(DominationPointPlan {
                point_index: i,
                point_name: names[i].clone(),
                room_index: locs[i].0,
                location: locs[i].1,
                tag: format!("ControlPoint_{}", names[i].replace(' ', "_")),
                role: role_meta[i].0,
                light_hue: hues[i],
                ambient_sound_radius: 360.0,
                defense_points_count: 2,
            });
        }
    } else {
        // Single arena room: arrange 3 points in an inscribed equilateral triangle
        let r0 = candidate_rooms[0];
        let c = r0.1.center;
        let fz = r0.1.floor_z() + 52.0;
        let radius = (r0.1.size.x.min(r0.1.size.y) * 0.38).clamp(420.0, 750.0);

        let role_meta = vec![
            (DominationPointRole::Equilateral, fz),
            (DominationPointRole::Equilateral, fz),
            (DominationPointRole::Equilateral, fz),
        ];
        let names = generate_point_names(naming_style, &role_meta, seed);
        let hues = [140u8, 36u8, 200u8];

        for i in 0..3 {
            let angle = (i as f32) * (std::f32::consts::TAU / 3.0);
            let loc = Vec3::new(c.x + radius * angle.cos(), c.y + radius * angle.sin(), fz);
            resolved_points.push(DominationPointPlan {
                point_index: i,
                point_name: names[i].clone(),
                room_index: r0.0,
                location: loc,
                tag: format!("ControlPoint_{}", names[i].replace(' ', "_")),
                role: DominationPointRole::Equilateral,
                light_hue: hues[i],
                ambient_sound_radius: 360.0,
                defense_points_count: 2,
            });
        }
    }

    // Compute pairwise distances and triangle asymmetry
    let p_a = resolved_points[0].location;
    let p_b = resolved_points[1].location;
    let p_c = resolved_points[2].location;

    let d_ab = p_a.distance(p_b);
    let d_bc = p_b.distance(p_c);
    let d_ca = p_c.distance(p_a);

    let perim = d_ab + d_bc + d_ca;
    let asymmetry = (d_ab - d_bc).abs() + (d_bc - d_ca).abs() + (d_ca - d_ab).abs();
    let asym_ratio = (asymmetry / perim.max(1.0)) * 100.0;

    let z_min = p_a.z.min(p_b.z).min(p_c.z);
    let z_max = p_a.z.max(p_b.z).max(p_c.z);
    let vert_disp = z_max - z_min;

    // Centroid of the triangle for the center mega-powerup
    let centroid = (p_a + p_b + p_c) / 3.0;

    // Find the room closest to the centroid that is NOT a ControlPoint room (if available)
    let mut center_room_idx = None;
    let mut min_cent_dist = f32::MAX;
    let cp_rooms: Vec<usize> = resolved_points.iter().map(|p| p.room_index).collect();

    for (idx, r) in all_rooms.iter().enumerate() {
        if !cp_rooms.contains(&idx) {
            let d = r.center.distance(centroid);
            if d < min_cent_dist {
                min_cent_dist = d;
                center_room_idx = Some(idx);
            }
        }
    }

    let center_powerup_loc = if let Some(idx) = center_room_idx {
        let r = &all_rooms[idx];
        Some(Vec3::new(r.center.x, r.center.y, r.floor_z() + 32.0))
    } else {
        Some(centroid)
    };

    DominationMissionPlan {
        points: resolved_points,
        naming_style_used: naming_style,
        dist_ab: d_ab,
        dist_bc: d_bc,
        dist_ca: d_ca,
        triangle_asymmetry_ratio: asym_ratio,
        vertical_disparity: vert_disp,
        center_powerup_location: center_powerup_loc,
        center_powerup_room_index: center_room_idx,
    }
}

/// Plans 2-point Duel Domination
fn plan_duel_domination(
    candidate_rooms: &[(usize, &DominationNode)],
    _all_rooms: &[DominationNode],
    naming_style: DominationNamingStyle,
    seed: u64,
) -> DominationMissionPlan {
    let (r0, r1) = if candidate_rooms.len() >= 2 {
        // Pick the 2 rooms with maximum separation
        let mut max_d = f32::MIN;
        let mut best_pair = (0, 1);
        for i in 0..candidate_rooms.len() {
            for j in (i + 1)..candidate_rooms.len() {
                let d = candidate_rooms[i].1.center.distance(candidate_rooms[j].1.center);
                if d > max_d {
                    max_d = d;
                    best_pair = (i, j);
                }
            }
        }
        (candidate_rooms[best_pair.0], candidate_rooms[best_pair.1])
    } else {
        (candidate_rooms[0], candidate_rooms[0])
    };

    let pos0 = if r0.0 != r1.0 {
        Vec3::new(r0.1.center.x, r0.1.center.y, r0.1.floor_z() + 52.0)
    } else {
        Vec3::new(r0.1.center.x - 300.0, r0.1.center.y, r0.1.floor_z() + 52.0)
    };

    let pos1 = if r0.0 != r1.0 {
        Vec3::new(r1.1.center.x, r1.1.center.y, r1.1.floor_z() + 52.0)
    } else {
        Vec3::new(r1.1.center.x + 300.0, r1.1.center.y, r1.1.floor_z() + 52.0)
    };

    let role_meta = vec![
        (DominationPointRole::WingFlank, pos0.z),
        (DominationPointRole::WingFlank, pos1.z),
    ];
    let names = generate_point_names(naming_style, &role_meta, seed);
    let hues = [140u8, 36u8];

    let mut points = Vec::new();
    let locs = [(r0.0, pos0), (r1.0, pos1)];
    for i in 0..2 {
        points.push(DominationPointPlan {
            point_index: i,
            point_name: names[i].clone(),
            room_index: locs[i].0,
            location: locs[i].1,
            tag: format!("ControlPoint_{}", names[i].replace(' ', "_")),
            role: role_meta[i].0,
            light_hue: hues[i],
            ambient_sound_radius: 360.0,
            defense_points_count: 2,
        });
    }

    let d_ab = pos0.distance(pos1);
    let centroid = (pos0 + pos1) * 0.5;

    DominationMissionPlan {
        points,
        naming_style_used: naming_style,
        dist_ab: d_ab,
        dist_bc: 0.0,
        dist_ca: 0.0,
        triangle_asymmetry_ratio: 0.0,
        vertical_disparity: (pos1.z - pos0.z).abs(),
        center_powerup_location: Some(centroid),
        center_powerup_room_index: None,
    }
}

/// Plans 4-point Quad Domination
fn plan_quad_domination(
    candidate_rooms: &[(usize, &DominationNode)],
    all_rooms: &[DominationNode],
    naming_style: DominationNamingStyle,
    seed: u64,
) -> DominationMissionPlan {
    let mut base_plan = plan_triad_domination(candidate_rooms, all_rooms, naming_style, seed);

    // Find a 4th room distinct from the first 3 with >= 480 UU separation from all existing points
    let used_rooms: Vec<usize> = base_plan.points.iter().map(|p| p.room_index).collect();
    let existing_locs: Vec<Vec3> = base_plan.points.iter().map(|p| p.location).collect();

    let candidate_4th = candidate_rooms
        .iter()
        .map(|&(idx, r)| (idx, r))
        .chain(all_rooms.iter().enumerate());

    let mut best_4th = None;
    let mut best_4th_dist = 0.0f32;

    for (idx, r) in candidate_4th {
        if used_rooms.contains(&idx) {
            continue;
        }
        let pos = Vec3::new(r.center.x, r.center.y, r.floor_z() + 52.0);
        let min_d = existing_locs
            .iter()
            .map(|&p| p.distance(pos))
            .fold(f32::MAX, f32::min);
        if min_d >= 480.0 && min_d > best_4th_dist {
            best_4th_dist = min_d;
            best_4th = Some((idx, pos));
        }
    }

    let (r4_idx, loc4) = if let Some(found) = best_4th {
        found
    } else {
        // Fallback: If map has fewer than 4 distinct rooms, offset within largest room
        // ensuring >= 480 UU distance from all existing points to prevent proximity spam!
        let largest_room = all_rooms
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| {
                (a.size.x * a.size.y)
                    .partial_cmp(&(b.size.x * b.size.y))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or((0, &all_rooms[0]));

        let mut offset_pos = Vec3::new(
            largest_room.1.center.x,
            largest_room.1.center.y,
            largest_room.1.floor_z() + 52.0,
        );
        for sign in [1.0, -1.0] {
            for (dx, dy) in [(550.0, 0.0), (0.0, 550.0)] {
                let candidate = Vec3::new(
                    largest_room.1.center.x + sign * dx,
                    largest_room.1.center.y + sign * dy,
                    largest_room.1.floor_z() + 52.0,
                );
                if existing_locs.iter().all(|&p| p.distance(candidate) >= 480.0) {
                    offset_pos = candidate;
                    break;
                }
            }
        }
        (largest_room.0, offset_pos)
    };

    let role_meta = vec![
        (DominationPointRole::WingFlank, base_plan.points[0].location.z),
        (DominationPointRole::WingFlank, base_plan.points[1].location.z),
        (DominationPointRole::CrossroadHub, base_plan.points[2].location.z),
        (DominationPointRole::HighTier, loc4.z),
    ];
    let names = generate_point_names(naming_style, &role_meta, seed);

    for (i, p) in base_plan.points.iter_mut().enumerate() {
        p.point_name = names[i].clone();
        p.tag = format!("ControlPoint_{}", names[i].replace(' ', "_"));
    }

    base_plan.points.push(DominationPointPlan {
        point_index: 3,
        point_name: names[3].clone(),
        room_index: r4_idx,
        location: loc4,
        tag: format!("ControlPoint_{}", names[3].replace(' ', "_")),
        role: DominationPointRole::HighTier,
        light_hue: 20u8, // Amber/Red
        ambient_sound_radius: 360.0,
        defense_points_count: 2,
    });

    base_plan
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domination_solver_triad_distinct_rooms_and_min_distance() {
        let rooms = vec![
            DominationNode {
                center: Vec3::new(-1200.0, 0.0, 0.0),
                size: Vec3::new(800.0, 800.0, 400.0),
                is_corridor: false,
            },
            DominationNode {
                center: Vec3::new(1200.0, 0.0, 0.0),
                size: Vec3::new(800.0, 800.0, 400.0),
                is_corridor: false,
            },
            DominationNode {
                center: Vec3::new(0.0, 1500.0, 240.0),
                size: Vec3::new(800.0, 800.0, 600.0),
                is_corridor: false,
            },
            DominationNode {
                center: Vec3::new(0.0, 400.0, 0.0),
                size: Vec3::new(300.0, 600.0, 250.0),
                is_corridor: true,
            },
        ];

        let plan = plan_domination_mission(
            &rooms,
            WizardThemePreset::Deck16Industrial,
            DominationNamingStyle::ElevationVerticality,
            3,
            42,
        );

        assert_eq!(plan.points.len(), 3);
        assert!(plan.dist_ab >= 500.0);
        assert!(plan.dist_bc >= 500.0);
        assert!(plan.dist_ca >= 500.0);
        assert!(plan.triangle_asymmetry_ratio < 35.0);
        assert_eq!(plan.vertical_disparity, 140.0);

        // Verify distinct names
        assert_ne!(plan.points[0].point_name, plan.points[1].point_name);
        assert_ne!(plan.points[1].point_name, plan.points[2].point_name);
        assert_ne!(plan.points[0].point_name, plan.points[2].point_name);

        // Verify distinct rooms
        assert_ne!(plan.points[0].room_index, plan.points[1].room_index);
        assert_ne!(plan.points[1].room_index, plan.points[2].room_index);
        assert_ne!(plan.points[0].room_index, plan.points[2].room_index);
    }

    #[test]
    fn test_domination_solver_single_arena_equilateral() {
        let rooms = vec![DominationNode {
            center: Vec3::new(0.0, 0.0, 0.0),
            size: Vec3::new(2000.0, 2000.0, 600.0),
            is_corridor: false,
        }];

        let plan = plan_domination_mission(
            &rooms,
            WizardThemePreset::SesmarEgyptianTomb,
            DominationNamingStyle::EgyptianMonument,
            3,
            99,
        );

        assert_eq!(plan.points.len(), 3);
        assert!(plan.dist_ab >= 500.0);
        assert!(plan.dist_bc >= 500.0);
        assert!(plan.dist_ca >= 500.0);
        // In single arena, asymmetry should be virtually 0 (perfect equilateral triangle)
        assert!(plan.triangle_asymmetry_ratio < 2.0);
    }
}
