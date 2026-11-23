use std::fs;
use std::path::Path;
use ut99_format::unr::UnrLevel;
use ut99_format::TaggedValue;

fn main() {
    let maps_dir = Path::new("UnrealTournament").join("Maps");
    if !maps_dir.exists() {
        eprintln!("Maps dir not found at {:?}", maps_dir);
        return;
    }

    let mut map_files: Vec<_> = fs::read_dir(&maps_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |s| s.starts_with("CTF-") && s.ends_with(".unr"))
        })
        .collect();
    map_files.sort();

    println!("================================================================================");
    println!("FORENSIC SURVEY: {} OFFICIAL CTF MAPS", map_files.len());
    println!("================================================================================");

    for map_path in &map_files {
        let map_name = map_path.file_name().unwrap().to_str().unwrap();

        let level = match UnrLevel::open(map_path) {
            Ok(lvl) => lvl,
            Err(e) => {
                eprintln!("Error parsing {}: {}", map_name, e);
                continue;
            }
        };

        // Extract FlagBases
        let mut flag_bases = Vec::new();
        let mut player_starts = Vec::new();
        let mut defense_points = Vec::new();
        let mut alternate_paths = Vec::new();
        let mut sniper_rifles = Vec::new();
        let mut mega_powerups = Vec::new();
        let mut teleporters = Vec::new();
        let mut movers = Vec::new();

        for a in &level.actors {
            let class_lower = a.class_name.to_lowercase();
            if class_lower.contains("flagbase") {
                let team: Option<u8> = a.properties.get("Team").and_then(|p| match p.value {
                    TaggedValue::Byte(b) => Some(b),
                    TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                });
                flag_bases.push((a.name.clone(), team, a.location));
            } else if class_lower == "playerstart" {
                let team: u8 = a.properties.get("TeamNumber").and_then(|p| match p.value {
                    TaggedValue::Byte(b) => Some(b),
                    TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }).unwrap_or(0);
                player_starts.push((a.name.clone(), team, a.location));
            } else if class_lower.contains("defensepoint") {
                let team: Option<u8> = a.properties.get("team").or_else(|| a.properties.get("Team")).and_then(|p| match p.value {
                    TaggedValue::Byte(b) => Some(b),
                    TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                });
                let fort_tag = a.properties.get("FortTag").and_then(|p| match &p.value {
                    TaggedValue::Str(s) => Some(s.clone()),
                    _ => None,
                });
                let priority: u8 = a.properties.get("priority").or_else(|| a.properties.get("Priority")).and_then(|p| match p.value {
                    TaggedValue::Byte(b) => Some(b),
                    TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }).unwrap_or(0);
                let b_sniping: bool = a.properties.get("bSniping").and_then(|p| match p.value {
                    TaggedValue::Bool(b) => Some(b),
                    _ => None,
                }).unwrap_or(false);
                defense_points.push((a.name.clone(), team, fort_tag, priority, b_sniping, a.location));
            } else if class_lower.contains("alternatepath") {
                let team: u8 = a.properties.get("team").or_else(|| a.properties.get("Team")).and_then(|p| match p.value {
                    TaggedValue::Byte(b) => Some(b),
                    TaggedValue::Int(i) => Some(i as u8),
                    _ => None,
                }).unwrap_or(0);
                let weight: f32 = a.properties.get("SelectionWeight").and_then(|p| match p.value {
                    TaggedValue::Float(f) => Some(f),
                    _ => None,
                }).unwrap_or(1.0);
                let b_return_only: bool = a.properties.get("bReturnOnly").and_then(|p| match p.value {
                    TaggedValue::Bool(b) => Some(b),
                    _ => None,
                }).unwrap_or(false);
                alternate_paths.push((team, weight, b_return_only, a.location));
            } else if class_lower.contains("sniperrifle") {
                sniper_rifles.push(a.location);
            } else if class_lower.contains("ut_shieldbelt") || class_lower.contains("ut_damageamplifier") || class_lower.contains("udamage") || class_lower.contains("superhealth") {
                mega_powerups.push((a.class_name.clone(), a.location));
            } else if class_lower == "teleporter" || class_lower.contains("visibleteleporter") {
                teleporters.push(a.location);
            } else if class_lower == "mover" || class_lower.contains("mover") {
                movers.push(a.location);
            }
        }

        println!("\n--------------------------------------------------------------------------------");
        println!("MAP: {}", map_name);
        println!("  Total Actors: {}", level.actors.len());
        println!("  FlagBases: {}", flag_bases.len());
        for (name, team, loc) in &flag_bases {
            let t_str = match team {
                Some(0) => "Red (Team 0)",
                Some(1) => "Blue (Team 1)",
                Some(t) => format!("Team {}", t).leak(),
                None => "Unassigned",
            };
            println!("    FlagBase: {} [{}] at loc=({:.1}, {:.1}, {:.1})", name, t_str, loc.x, loc.y, loc.z);
        }

        if flag_bases.len() >= 2 {
            let red_fb = flag_bases.iter().find(|(_, t, _)| *t == Some(0)).or_else(|| flag_bases.first());
            let blue_fb = flag_bases.iter().find(|(_, t, _)| *t == Some(1)).or_else(|| flag_bases.get(1));
            if let (Some((_, _, r_loc)), Some((_, _, b_loc))) = (red_fb, blue_fb) {
                let dist = (*r_loc - *b_loc).length();
                let delta_z = (r_loc.z - b_loc.z).abs();
                println!("  Flag-to-Flag Distance: {:.1} UU (Delta Z = {:.1} UU)", dist, delta_z);
            }
        }

        // Spawns
        let red_spawns: Vec<_> = player_starts.iter().filter(|(_, t, _)| *t == 0).collect();
        let blue_spawns: Vec<_> = player_starts.iter().filter(|(_, t, _)| *t == 1).collect();
        let other_spawns: Vec<_> = player_starts.iter().filter(|(_, t, _)| *t != 0 && *t != 1).collect();
        println!("  PlayerStarts: Total {} (Red: {}, Blue: {}, Other: {})", player_starts.len(), red_spawns.len(), blue_spawns.len(), other_spawns.len());

        // Spawn to Flag Standoff
        if let Some((_, _, r_loc)) = flag_bases.iter().find(|(_, t, _)| *t == Some(0)) {
            let mut min_d = f32::MAX;
            let mut max_d = 0.0f32;
            let mut sum_d = 0.0f32;
            for (_, _, s_loc) in &red_spawns {
                let d = (*s_loc - *r_loc).length();
                if d < min_d { min_d = d; }
                if d > max_d { max_d = d; }
                sum_d += d;
            }
            if !red_spawns.is_empty() {
                println!("    Red Spawn-to-Flag: Min {:.1} UU, Max {:.1} UU, Avg {:.1} UU", min_d, max_d, sum_d / red_spawns.len() as f32);
            }
        }

        println!("  DefensePoints: {} (Sniping: {})", defense_points.len(), defense_points.iter().filter(|(_, _, _, _, s, _)| *s).count());
        for (name, team, tag, pri, is_snipe, loc) in defense_points.iter().take(6) {
            println!("    DP: {} Team={:?} Tag={:?} Pri={} Snipe={} Loc=({:.1}, {:.1}, {:.1})", name, team, tag, pri, is_snipe, loc.x, loc.y, loc.z);
        }
        if defense_points.len() > 6 {
            println!("    ... and {} more DefensePoints", defense_points.len() - 6);
        }

        println!("  AlternatePaths: {}", alternate_paths.len());
        for (team, weight, ret_only, loc) in &alternate_paths {
            println!("    AltPath: Team={} Weight={:.1} ReturnOnly={} Loc=({:.1}, {:.1}, {:.1})", team, weight, ret_only, loc.x, loc.y, loc.z);
        }

        println!("  Sniper Rifles: {} | Teleporters: {} | Movers/Lifts: {} | Mega Powerups: {}", 
            sniper_rifles.len(), teleporters.len(), movers.len(), mega_powerups.len());
        for (p_class, loc) in &mega_powerups {
            println!("    Powerup: {} at ({:.1}, {:.1}, {:.1})", p_class, loc.x, loc.y, loc.z);
        }
    }
}
