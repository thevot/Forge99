use std::collections::HashMap;
use std::path::PathBuf;
use ut99_format::unr::tagged_property::TaggedValue;
use ut99_format::unr::UnrLevel;

fn audit_map(map_name: &str) {
    let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament/Maps");
    let map_path = maps_dir.join(map_name);
    if !map_path.exists() {
        println!("Map {} not found, skipping.", map_name);
        return;
    }

    let Ok(lvl) = UnrLevel::open(&map_path) else {
        println!("Cannot open {}, skipping.", map_name);
        return;
    };

    println!("============================================================");
    println!("FORENSIC AUDIT: {}", map_name);
    println!("============================================================");

        // Bounding box of all actors
        let mut min_pos = glam::Vec3::splat(f32::MAX);
        let mut max_pos = glam::Vec3::splat(f32::MIN);
        let mut class_counts: HashMap<String, usize> = HashMap::new();
        let mut teleporters: Vec<(String, String, glam::Vec3, Option<String>)> = Vec::new();
        let mut weapons: Vec<(String, glam::Vec3)> = Vec::new();
        let mut defense_points: Vec<(String, glam::Vec3, bool, u8)> = Vec::new();
        let mut alternate_paths: Vec<(glam::Vec3, u8, f32, bool)> = Vec::new();
        let mut flag_bases: Vec<(String, u8, glam::Vec3)> = Vec::new();
        let mut zone_infos: Vec<(String, glam::Vec3, glam::Vec3)> = Vec::new();
        let mut sky_zones: Vec<glam::Vec3> = Vec::new();
        let mut lights: Vec<(glam::Vec3, bool, u8, u8)> = Vec::new();

        for a in &lvl.actors {
            let cls = a.class_name.rsplit('.').next().unwrap_or(&a.class_name).to_string();
            *class_counts.entry(cls.clone()).or_insert(0) += 1;

            let loc = a.location;
            if loc.x.is_finite() && loc.y.is_finite() && loc.z.is_finite() {
                min_pos = min_pos.min(loc);
                max_pos = max_pos.max(loc);
            }

            let cls_low = cls.to_ascii_lowercase();

            if cls_low.contains("flagbase") {
                let team = a.properties.get("Team").and_then(|p| match &p.value {
                    TaggedValue::Byte(b) => Some(*b),
                    TaggedValue::Int(i) => Some(*i as u8),
                    _ => None,
                }).unwrap_or(255);
                flag_bases.push((a.name.clone(), team, loc));
            } else if cls_low.contains("teleporter") {
                let url = a.properties.get("URL").and_then(|p| match &p.value {
                    TaggedValue::Str(s) => Some(s.clone()),
                    _ => None,
                });
                teleporters.push((a.name.clone(), cls.clone(), loc, url));
            } else if cls_low.contains("defensepoint") {
                let is_sniping = a.properties.get("bSniping").map_or(false, |p| matches!(p.value, TaggedValue::Bool(true)));
                let prio = a.properties.get("priority").and_then(|p| match &p.value {
                    TaggedValue::Byte(b) => Some(*b),
                    _ => None,
                }).unwrap_or(0);
                defense_points.push((a.name.clone(), loc, is_sniping, prio));
            } else if cls_low.contains("alternatepath") {
                let team = a.properties.get("team").and_then(|p| match &p.value {
                    TaggedValue::Byte(b) => Some(*b),
                    TaggedValue::Int(i) => Some(*i as u8),
                    _ => None,
                }).unwrap_or(255);
                let weight = a.properties.get("SelectionWeight").and_then(|p| match &p.value {
                    TaggedValue::Float(f) => Some(*f),
                    _ => None,
                }).unwrap_or(1.0);
                let ret_only = a.properties.get("bReturnOnly").map_or(false, |p| matches!(p.value, TaggedValue::Bool(true)));
                alternate_paths.push((loc, team, weight, ret_only));
            } else if cls_low.contains("skyzoneinfo") {
                sky_zones.push(loc);
            } else if cls_low.contains("zoneinfo") {
                let grav = a.properties.get("ZoneGravity").and_then(|p| match &p.value {
                    TaggedValue::Vector(v) => Some(glam::Vec3::new(v.x, v.y, v.z)),
                    _ => None,
                }).unwrap_or(glam::Vec3::new(0.0, 0.0, -950.0));
                zone_infos.push((a.name.clone(), loc, grav));
            } else if cls_low.contains("light") {
                let corona = a.properties.get("bCorona").map_or(false, |p| matches!(p.value, TaggedValue::Bool(true)));
                let br = a.properties.get("LightBrightness").and_then(|p| match &p.value {
                    TaggedValue::Byte(b) => Some(*b),
                    _ => None,
                }).unwrap_or(64);
                let rad = a.properties.get("LightRadius").and_then(|p| match &p.value {
                    TaggedValue::Byte(b) => Some(*b),
                    _ => None,
                }).unwrap_or(32);
                lights.push((loc, corona, br, rad));
            } else if matches!(
                cls_low.as_str(),
                "sniperrifle" | "ut_flakcannon" | "ut_eightball" | "shockrifle" | "minigun2" | "pulsegun" | "warheadlauncher" | "ut_biorifle" | "ripper"
            ) {
                weapons.push((cls.clone(), loc));
            }
        }

        println!("Level Bounding Box: min=({:.0}, {:.0}, {:.0}) max=({:.0}, {:.0}, {:.0})", min_pos.x, min_pos.y, min_pos.z, max_pos.x, max_pos.y, max_pos.z);
        println!("Dimensions: SpanX={:.0}, SpanY={:.0}, SpanZ={:.0}", max_pos.x - min_pos.x, max_pos.y - min_pos.y, max_pos.z - min_pos.z);

        if flag_bases.len() >= 2 {
            let p0 = flag_bases[0].2;
            let p1 = flag_bases[1].2;
            let flag_dist = p0.distance(p1);
            println!("CTF Flag Standoff Distance: {:.1} UU (Teams: {} & {})", flag_dist, flag_bases[0].1, flag_bases[1].1);
            println!("  Flag 0: {}@({:.0}, {:.0}, {:.0})", flag_bases[0].0, p0.x, p0.y, p0.z);
            println!("  Flag 1: {}@({:.0}, {:.0}, {:.0})", flag_bases[1].0, p1.x, p1.y, p1.z);
        }

        println!("Teleporters: count={}", teleporters.len());
        for (name, cls, loc, url) in &teleporters {
            println!("  Teleporter {}: {} @ ({:.0}, {:.0}, {:.0}) -> URL='{}'", name, cls, loc.x, loc.y, loc.z, url.as_deref().unwrap_or("<none>"));
        }

        println!("Sniper & DefensePoints: count={}", defense_points.len());
        let sniping_count = defense_points.iter().filter(|(_, _, s, _)| *s).count();
        println!("  Sniping DefensePoints (bSniping=true): {}", sniping_count);
        for (name, loc, is_s, prio) in &defense_points {
            if *is_s {
                println!("  Sniper Nest {}: loc=({:.0}, {:.0}, {:.0}), prio={}", name, loc.x, loc.y, loc.z, prio);
            }
        }

        println!("AlternatePaths: count={}", alternate_paths.len());
        for (loc, team, weight, ret_only) in &alternate_paths {
            println!("  AlternatePath Team {}: weight={:.1}, return_only={}, loc=({:.0}, {:.0}, {:.0})", team, weight, ret_only, loc.x, loc.y, loc.z);
        }

        println!("Weapons: count={}", weapons.len());
        for (cls, loc) in &weapons {
            println!("  Weapon {}: loc=({:.0}, {:.0}, {:.0})", cls, loc.x, loc.y, loc.z);
        }

        println!("ZoneInfos & Gravity: count={}", zone_infos.len());
        for (name, loc, grav) in &zone_infos {
            println!("  Zone {}: loc=({:.0}, {:.0}, {:.0}), gravity=({:.0}, {:.0}, {:.0})", name, loc.x, loc.y, loc.z, grav.x, grav.y, grav.z);
        }

        println!("SkyZones: count={}", sky_zones.len());
        for loc in &sky_zones {
            println!("  SkyZone @ ({:.0}, {:.0}, {:.0})", loc.x, loc.y, loc.z);
        }
}

#[test]
fn test_forensics_original_face() {
    audit_map("CTF-Face.unr");
}

#[test]
fn test_forensics_ctf_face_2() {
    audit_map("CTF-Face][.unr");
}

#[test]
fn test_forensics_ctf_face_se() {
    audit_map("CTF-Face-SE.unr");
}

#[test]
fn test_forensics_ctf_lavagiant() {
    audit_map("CTF-LavaGiant.unr");
}

#[test]
fn test_forensics_dm_morpheus() {
    audit_map("DM-Morpheus.unr");
}

#[test]
fn test_lavagiant_deep_forensics() {
    let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament/Maps");
    let map_path = maps_dir.join("CTF-LavaGiant.unr");
    let Ok(lvl) = UnrLevel::open(&map_path) else {
        println!("Cannot open CTF-LavaGiant.unr");
        return;
    };

    println!("============================================================");
    println!("LAVAGIANT ARCHITECTURAL & ASYMMETRY FORENSIC AUDIT");
    println!("============================================================");

    let mut red_actors = Vec::new();
    let mut blue_actors = Vec::new();
    let mut midfield_actors = Vec::new();
    let mut movers = Vec::new();
    let mut lava_zones = Vec::new();
    let mut player_starts = Vec::new();

    for a in &lvl.actors {
        let cls = a.class_name.rsplit('.').next().unwrap_or(&a.class_name).to_string();
        let loc = a.location;
        let cls_low = cls.to_ascii_lowercase();

        if cls_low.contains("playerstart") {
            let team = a.properties.get("TeamNumber").and_then(|p| match &p.value {
                TaggedValue::Byte(b) => Some(*b),
                TaggedValue::Int(i) => Some(*i as u8),
                _ => None,
            }).unwrap_or(255);
            player_starts.push((a.name.clone(), team, loc));
        } else if cls_low.contains("mover") {
            movers.push((a.name.clone(), loc));
        } else if cls_low.contains("zone") {
            let is_lava = a.properties.get("DamageType").map_or(false, |p| match &p.value {
                TaggedValue::Name(n) => n.to_ascii_lowercase().contains("burn") || n.to_ascii_lowercase().contains("lava"),
                _ => false,
            });
            let dmg = a.properties.get("DamagePerSec").and_then(|p| match &p.value {
                TaggedValue::Int(i) => Some(*i),
                TaggedValue::Byte(b) => Some(*b as i32),
                _ => None,
            }).unwrap_or(0);
            lava_zones.push((a.name.clone(), cls.clone(), loc, is_lava, dmg));
        }

        // Divide map by X: Red base is West (X < -1500), Blue base is East (X > 1500), Midfield (-1500 <= X <= 1500)
        if loc.x < -1500.0 {
            red_actors.push((a.name.clone(), cls.clone(), loc));
        } else if loc.x > 1500.0 {
            blue_actors.push((a.name.clone(), cls.clone(), loc));
        } else {
            midfield_actors.push((a.name.clone(), cls.clone(), loc));
        }
    }

    println!("Total Actors: {}", lvl.actors.len());
    println!("Red Territory (X < -1500): {} actors", red_actors.len());
    println!("Blue Territory (X > 1500): {} actors", blue_actors.len());
    println!("Midfield Canyon (-1500 <= X <= 1500): {} actors", midfield_actors.len());

    println!("\nPlayerStarts Breakdown:");
    for (name, team, loc) in &player_starts {
        println!("  {} Team {}: loc=({:.0}, {:.0}, {:.0})", name, team, loc.x, loc.y, loc.z);
    }

    println!("\nMovers Breakdown (count={}):", movers.len());
    for (name, loc) in &movers {
        println!("  Mover {}: loc=({:.0}, {:.0}, {:.0})", name, loc.x, loc.y, loc.z);
    }

    println!("\nZoneInfos & Hazard Lava Zones:");
    for (name, cls, loc, is_l, dmg) in &lava_zones {
        println!("  {} ({}): loc=({:.0}, {:.0}, {:.0}), is_lava={}, dmg/s={}", name, cls, loc.x, loc.y, loc.z, is_l, dmg);
    }

    // Weapons in Red vs Blue vs Midfield
    println!("\nWeapons in Red Territory:");
    for (name, cls, loc) in &red_actors {
        let l = cls.to_ascii_lowercase();
        if matches!(l.as_str(), "sniperrifle" | "ut_flakcannon" | "ut_eightball" | "shockrifle" | "minigun2" | "pulsegun" | "warheadlauncher" | "ut_biorifle" | "ripper" | "chainsaw" | "enforcer") {
            println!("  Red Weapon {}: {} @ ({:.0}, {:.0}, {:.0})", name, cls, loc.x, loc.y, loc.z);
        }
    }

    println!("\nWeapons in Blue Territory:");
    for (name, cls, loc) in &blue_actors {
        let l = cls.to_ascii_lowercase();
        if matches!(l.as_str(), "sniperrifle" | "ut_flakcannon" | "ut_eightball" | "shockrifle" | "minigun2" | "pulsegun" | "warheadlauncher" | "ut_biorifle" | "ripper" | "chainsaw" | "enforcer") {
            println!("  Blue Weapon {}: {} @ ({:.0}, {:.0}, {:.0})", name, cls, loc.x, loc.y, loc.z);
        }
    }

    println!("\nWeapons in Midfield Canyon:");
    for (name, cls, loc) in &midfield_actors {
        let l = cls.to_ascii_lowercase();
        if matches!(l.as_str(), "sniperrifle" | "ut_flakcannon" | "ut_eightball" | "shockrifle" | "minigun2" | "pulsegun" | "warheadlauncher" | "ut_biorifle" | "ripper" | "chainsaw" | "enforcer") {
            println!("  Midfield Weapon {}: {} @ ({:.0}, {:.0}, {:.0})", name, cls, loc.x, loc.y, loc.z);
        }
    }
}

#[test]
fn test_lavagiant_architecture_deep_dive() {
    let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament/Maps");
    let map_path = maps_dir.join("CTF-LavaGiant.unr");
    let Ok(lvl) = UnrLevel::open(&map_path) else {
        println!("Cannot open CTF-LavaGiant.unr");
        return;
    };

    println!("============================================================");
    println!("LAVAGIANT FULL ARCHITECTURAL & TOPOLOGICAL DEEP DIVE");
    println!("============================================================");

    // 1. Level Bounds from PathNodes and Actors
    let mut min_pos = glam::Vec3::splat(f32::MAX);
    let mut max_pos = glam::Vec3::splat(f32::MIN);
    let mut path_count = 0;
    for a in &lvl.actors {
        if a.class_name.to_ascii_lowercase().contains("pathnode") || a.class_name.to_ascii_lowercase().contains("playerstart") {
            min_pos = min_pos.min(a.location);
            max_pos = max_pos.max(a.location);
            path_count += 1;
        }
    }
    println!("Navigation Mesh Bounds ({} nodes): Min=({:.0}, {:.0}, {:.0}), Max=({:.0}, {:.0}, {:.0})", path_count, min_pos.x, min_pos.y, min_pos.z, max_pos.x, max_pos.y, max_pos.z);
    println!("Total Level Span: dx={:.0}, dy={:.0}, dz={:.0}", max_pos.x - min_pos.x, max_pos.y - min_pos.y, max_pos.z - min_pos.z);

    // 2. Zone Breakdown
    println!("\nZone Names & ZoneInfos:");
    for a in &lvl.actors {
        if a.class_name.to_ascii_lowercase().contains("zone") {
            let zone_name = a.properties.get("ZoneName").and_then(|p| match &p.value {
                TaggedValue::Str(s) => Some(s.clone()),
                _ => None,
            }).unwrap_or_else(|| "<unnamed>".to_string());
            let loc = a.location;
            println!("  Zone Actor '{}' ({}): ZoneName='{}', Loc=({:.0}, {:.0}, {:.0})", a.name, a.class_name, zone_name, loc.x, loc.y, loc.z);
        }
    }

    // 3. Flags and nearby actors
    for a in &lvl.actors {
        if a.class_name.to_ascii_lowercase().contains("flagbase") {
            println!("\nFlagBase '{}' @ ({:.0}, {:.0}, {:.0}):", a.name, a.location.x, a.location.y, a.location.z);
            for other in &lvl.actors {
                let d = other.location.distance(a.location);
                if d > 0.0 && d < 1200.0 && (other.class_name.contains("PlayerStart") || other.class_name.contains("Sniper") || other.class_name.contains("DefensePoint") || other.class_name.contains("Teleporter")) {
                    println!("    Neighbor '{}' ({}): dist={:.0}, loc=({:.0}, {:.0}, {:.0})", other.name, other.class_name, d, other.location.x, other.location.y, other.location.z);
                }
            }
        }
    }

    // 4. Textures used in the map
    let mut textures = std::collections::HashSet::new();
    for a in &lvl.actors {
        for (prop_name, prop) in &a.properties {
            if prop_name.to_ascii_lowercase().contains("texture") || prop_name.to_ascii_lowercase().contains("skin") {
                if let TaggedValue::Name(n) = &prop.value {
                    textures.insert(n.clone());
                }
            }
        }
    }
    println!("\nSample Key Textures (count={}):", textures.len());
    let mut sample_tex: Vec<_> = textures.into_iter().collect();
    sample_tex.sort();
}

#[test]
fn test_audit_official_lavagiant_brushes() {
    let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../UnrealTournament/Maps");
    let map_path = maps_dir.join("CTF-LavaGiant.unr");
    let Ok(lvl) = UnrLevel::open(&map_path) else {
        println!("Cannot open CTF-LavaGiant.unr");
        return;
    };

    println!("============================================================");
    println!("OFFICIAL CTF-LAVAGIANT BRUSH & ARCHITECTURAL DNA");
    println!("============================================================");

    println!("Total Brushes in official LavaGiant: {}", lvl.brushes.len());
    let mut subtract_count = 0;
    let mut add_count = 0;
    for (i, b) in lvl.brushes.iter().enumerate() {
        let oper = b.actor.csg_oper;
        if oper == 2 {
            subtract_count += 1;
            let mut b_min = glam::Vec3::splat(f32::MAX);
            let mut b_max = glam::Vec3::splat(f32::MIN);
            for p in &b.local_polys.polys {
                for v in &p.vertices {
                    let wv = b.actor.location + *v;
                    b_min = b_min.min(wv);
                    b_max = b_max.max(wv);
                }
            }
            let size = b_max - b_min;
            println!("  Subtract Brush #{}: name={}, loc=({:.0}, {:.0}, {:.0}), size=({:.0}, {:.0}, {:.0})", i, b.actor.name, b.actor.location.x, b.actor.location.y, b.actor.location.z, size.x, size.y, size.z);
        } else if oper == 1 { add_count += 1; }
    }
    println!("Subtract brushes: {}, Add brushes: {}", subtract_count, add_count);
}




