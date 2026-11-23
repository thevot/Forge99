use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;
use ut99_format::unr::UnrLevel;
use glam::Vec3;

fn main() {
    let maps_dir = Path::new("UnrealTournament").join("Maps");
    let mut dom_maps = Vec::new();

    if let Ok(entries) = fs::read_dir(&maps_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case("unr") {
                    let file_name = path.file_name().unwrap().to_string_lossy().to_string();
                    if file_name.starts_with("DOM-") {
                        dom_maps.push(path);
                    }
                }
            }
        }
    }

    dom_maps.sort();
    println!("Found {} official DOM maps in {:?}", dom_maps.len(), maps_dir);

    for map_path in &dom_maps {
        let map_name = map_path.file_name().unwrap().to_string_lossy();
        println!("\n================================================================================");
        println!("MAP: {}", map_name);
        println!("================================================================================");

        let level = match UnrLevel::open(map_path) {
            Ok(lvl) => lvl,
            Err(e) => {
                eprintln!("Failed to parse UNR {:?}: {}", map_path, e);
                continue;
            }
        };

        // Class distribution
        let mut class_counts: BTreeMap<String, usize> = BTreeMap::new();
        for actor in &level.actors {
            *class_counts.entry(actor.class_name.clone()).or_default() += 1;
        }

        println!("Total actors: {}", level.actors.len());

        // ControlPoints
        let mut control_points = Vec::new();
        let mut player_starts = Vec::new();
        let mut defense_points = Vec::new();
        let mut powerups = Vec::new();

        for actor in &level.actors {
            let cls = actor.class_name.to_lowercase();
            if cls == "controlpoint" {
                control_points.push(actor);
            } else if cls == "playerstart" {
                player_starts.push(actor);
            } else if cls == "defensepoint" {
                defense_points.push(actor);
            } else if cls == "ut_shieldbelt" || cls == "udamage" || cls == "armor2" || cls == "thighpads" {
                powerups.push(actor);
            }
        }

        println!("--- ACTOR SUMMARY ---");
        println!("  ControlPoint: {}", control_points.len());
        println!("  DefensePoint: {}", defense_points.len());
        println!("  PlayerStart: {}", player_starts.len());
        println!("  Top Key Classes: ");
        let mut sorted_classes: Vec<_> = class_counts.iter().collect();
        sorted_classes.sort_by(|a, b| b.1.cmp(a.1));
        for (cls, count) in sorted_classes.iter().take(12) {
            println!("    {}: {}", cls, count);
        }

        // Detailed ControlPoint inspection
        println!("\n--- CONTROL POINTS ---");
        let mut cp_locs = Vec::new();
        for (i, cp) in control_points.iter().enumerate() {
            let name = &cp.name;
            let point_name = cp.properties.get("PointName").map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "<none>".to_string());
            let loc = cp.location;
            let tag = &cp.tag;
            let event = &cp.event;
            let col_r = cp.properties.get("CollisionRadius").map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "<default>".to_string());
            let col_h = cp.properties.get("CollisionHeight").map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "<default>".to_string());
            let light_type = cp.light_type;
            let light_effect = cp.light_effect;
            let light_hue = cp.light_hue;
            let light_rad = cp.light_radius;
            let light_bright = cp.light_brightness;
            let sound = cp.properties.get("AmbientSound").or_else(|| cp.properties.get("ControlSound")).map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "<none>".to_string());

            cp_locs.push((name.clone(), point_name.clone(), Vec3::new(loc.x, loc.y, loc.z)));

            println!("  [CP #{}] Name: {} | PointName: {} | Loc: ({:.1}, {:.1}, {:.1})", i + 1, name, point_name, loc.x, loc.y, loc.z);
            println!("       Tag: '{}' | Event: '{}' | Collision: (R: {}, H: {})", tag, event, col_r, col_h);
            println!("       Light: Type={}, Effect={}, Hue={}, Rad={}, Bright={}", light_type, light_effect, light_hue, light_rad, light_bright);
            if sound != "<none>" {
                println!("       Sound: {}", sound);
            }

            // Print all properties of first CP in map for deep discovery
            if i == 0 {
                println!("       [All Properties of CP 0]:");
                for (prop_name, prop_val) in &cp.properties {
                    println!("         {}: {:?}", prop_name, prop_val.value);
                }
            }
        }

        // Pairwise distances between control points
        if cp_locs.len() >= 2 {
            println!("\n--- CONTROL POINT PAIRWISE DISTANCES ---");
            for i in 0..cp_locs.len() {
                for j in (i + 1)..cp_locs.len() {
                    let d = cp_locs[i].2.distance(cp_locs[j].2);
                    let d_xy = Vec3::new(cp_locs[i].2.x, cp_locs[i].2.y, 0.0).distance(Vec3::new(cp_locs[j].2.x, cp_locs[j].2.y, 0.0));
                    let d_z = (cp_locs[i].2.z - cp_locs[j].2.z).abs();
                    println!("    {} <-> {}: Euclidean = {:.1} UU | XY = {:.1} UU | Z = {:.1} UU", 
                        cp_locs[i].1, cp_locs[j].1, d, d_xy, d_z);
                }
            }
        }

        // PlayerStart analysis
        let mut teams: HashMap<String, usize> = HashMap::new();
        for ps in &player_starts {
            let t = ps.properties.get("TeamNumber").map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "0/Default".to_string());
            *teams.entry(t).or_default() += 1;
        }
        println!("\n--- PLAYERSTARTS (Total: {}) ---", player_starts.len());
        println!("  TeamNumber breakdown: {:?}", teams);

        // Distance from PlayerStarts to closest ControlPoint
        if !cp_locs.is_empty() && !player_starts.is_empty() {
            let mut min_dists_to_cps = vec![0.0f32; cp_locs.len()];
            for (c_idx, (_, _, cp_pos)) in cp_locs.iter().enumerate() {
                let mut min_d = f32::MAX;
                for ps in &player_starts {
                    let ps_pos = Vec3::new(ps.location.x, ps.location.y, ps.location.z);
                    let d = ps_pos.distance(*cp_pos);
                    if d < min_d { min_d = d; }
                }
                min_dists_to_cps[c_idx] = min_d;
            }
            print!("  Min distance from any PlayerStart to CP: ");
            for (c_idx, (_, pt_name, _)) in cp_locs.iter().enumerate() {
                print!("{}: {:.1} UU  ", pt_name, min_dists_to_cps[c_idx]);
            }
            println!();
        }

        // DefensePoints analysis
        if !defense_points.is_empty() {
            println!("\n--- DEFENSE POINTS (Total: {}) ---", defense_points.len());
            let mut dp_fort_tags: HashMap<String, usize> = HashMap::new();
            for dp in &defense_points {
                let ft = dp.properties.get("FortTag").map(|p| format!("{:?}", p.value)).unwrap_or_else(|| "<none>".to_string());
                *dp_fort_tags.entry(ft).or_default() += 1;
            }
            println!("  FortTag breakdown: {:?}", dp_fort_tags);
        }

        // Nearby actors around each CP (within 400 UU)
        println!("\n--- ACTORS IN PROXIMITY (< 450 UU) OF CONTROL POINTS ---");
        for (_, pt_name, cp_pos) in &cp_locs {
            let mut nearby = Vec::new();
            for actor in &level.actors {
                if actor.class_name.eq_ignore_ascii_case("controlpoint") { continue; }
                let a_pos = Vec3::new(actor.location.x, actor.location.y, actor.location.z);
                let dist = a_pos.distance(*cp_pos);
                if dist < 450.0 {
                    nearby.push((actor.class_name.clone(), actor.name.clone(), dist));
                }
            }
            nearby.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap());
            print!("  Near {}: ", pt_name);
            let summary: Vec<String> = nearby.iter().take(8).map(|(cls, _nm, d)| format!("{}({:.0}uu)", cls, d)).collect();
            println!("{}", summary.join(", "));
        }
    }
}
