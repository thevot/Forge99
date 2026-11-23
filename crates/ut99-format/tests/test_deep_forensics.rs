use std::collections::HashMap;
use std::path::PathBuf;
use ut99_format::unr::tagged_property::TaggedValue;
use ut99_format::unr::UnrLevel;

#[test]
fn test_run_deep_corpus_forensics() {
    let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps");
    if !maps_dir.exists() {
        eprintln!("Maps dir not found at {:?}", maps_dir);
        return;
    }

    let entries = std::fs::read_dir(&maps_dir).expect("Read maps dir");
    let mut map_paths = Vec::new();
    for entry in entries.flatten() {
        let p = entry.path();
        if p.extension().map_or(false, |e| e.eq_ignore_ascii_case("unr")) {
            let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(stem.to_ascii_lowercase().as_str(), "entry" | "ut-logo-map" | "utcredits" | "cityintro")
                && !stem.to_ascii_lowercase().starts_with("eol_")
            {
                map_paths.push(p);
            }
        }
    }
    map_paths.sort();

    println!("Total maps to analyze: {}", map_paths.len());

    let mut total_actors = 0usize;
    let mut total_movers = 0usize;
    let mut total_kickers = 0usize;
    let mut total_teleporters = 0usize;
    let mut total_lights = 0usize;
    let mut total_corona_lights = 0usize;
    let mut total_defense_points = 0usize;
    let mut total_alternate_paths = 0usize;
    let mut total_sniping_defense_points = 0usize;
    let mut total_triggers = 0usize;
    let mut total_special_events = 0usize;
    let mut total_dispatchers = 0usize;

    let mut mover_initial_states: HashMap<String, usize> = HashMap::new();
    let mut mover_encroach_types: HashMap<String, usize> = HashMap::new();
    let mut mover_opening_sounds: HashMap<String, usize> = HashMap::new();
    let mut mover_move_times: Vec<f32> = Vec::new();
    let mut mover_stay_open_times: Vec<f32> = Vec::new();

    let mut light_types: HashMap<String, usize> = HashMap::new();
    let mut light_effects: HashMap<String, usize> = HashMap::new();
    let mut light_radii: Vec<u8> = Vec::new();
    let mut light_brightnesses: Vec<u8> = Vec::new();

    let mut kicker_velocities: Vec<glam::Vec3> = Vec::new();
    let mut teleporter_changes_yaw = 0usize;
    let mut teleporter_changes_vel = 0usize;

    let mut defense_point_priorities: HashMap<u8, usize> = HashMap::new();
    let mut alternate_path_weights: Vec<f32> = Vec::new();
    let mut alternate_path_return_only = 0usize;

    let mut corridor_widths: Vec<f32> = Vec::new();
    let mut room_heights: Vec<f32> = Vec::new();

    let mut actor_class_counts: HashMap<String, usize> = HashMap::new();
    let mut sound_counts: HashMap<String, usize> = HashMap::new();

    for path in &map_paths {
        let _fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let Ok(lvl) = UnrLevel::open(path) else {
            continue;
        };

        for actor in &lvl.actors {
            let cls_short = actor.class_name.rsplit('.').next().unwrap_or(&actor.class_name).to_string();
            let cls_low = cls_short.to_ascii_lowercase();
            *actor_class_counts.entry(cls_short.clone()).or_insert(0) += 1;
            total_actors += 1;

            if let Some(snd) = actor.object_paths.get("AmbientSound") {
                *sound_counts.entry(snd.clone()).or_insert(0) += 1;
            }

            if cls_low.contains("mover") {
                total_movers += 1;
                if let Some(p) = actor.properties.get("InitialState") {
                    if let TaggedValue::Name(ref n) = p.value {
                        *mover_initial_states.entry(n.clone()).or_insert(0) += 1;
                    }
                }
                if let Some(p) = actor.properties.get("MoverEncroachType") {
                    if let TaggedValue::Byte(b) = p.value {
                        let name = match b {
                            0 => "ME_StopWhenEncroach",
                            1 => "ME_ReturnWhenEncroach",
                            2 => "ME_CrushWhenEncroach",
                            3 => "ME_IgnoreWhenEncroach",
                            _ => "Unknown",
                        };
                        *mover_encroach_types.entry(name.to_string()).or_insert(0) += 1;
                    }
                }
                if let Some(s) = actor.object_paths.get("OpeningSound") {
                    *mover_opening_sounds.entry(s.clone()).or_insert(0) += 1;
                }
                if let Some(p) = actor.properties.get("MoveTime") {
                    if let TaggedValue::Float(f) = p.value {
                        mover_move_times.push(f);
                    }
                }
                if let Some(p) = actor.properties.get("StayOpenTime") {
                    if let TaggedValue::Float(f) = p.value {
                        mover_stay_open_times.push(f);
                    }
                }
            } else if cls_low.contains("light") && !cls_low.contains("lightcone") && !cls_low.contains("lightbox") && !cls_low.contains("tubelight") {
                total_lights += 1;
                if let Some(p) = actor.properties.get("bCorona") {
                    if let TaggedValue::Bool(b) = p.value {
                        if b { total_corona_lights += 1; }
                    }
                }
                if let Some(p) = actor.properties.get("LightType") {
                    if let TaggedValue::Byte(b) = p.value {
                        let lt_name = match b {
                            0 => "LT_None",
                            1 => "LT_Steady",
                            2 => "LT_Pulse",
                            3 => "LT_Blink",
                            4 => "LT_Flicker",
                            5 => "LT_Strobe",
                            6 => "LT_BackdropLight",
                            7 => "LT_SubtlePulse",
                            8 => "LT_TexturePaletteOnce",
                            9 => "LT_TexturePaletteLoop",
                            _ => "Other",
                        };
                        *light_types.entry(lt_name.to_string()).or_insert(0) += 1;
                    }
                } else {
                    *light_types.entry("LT_Steady (Default)".to_string()).or_insert(0) += 1;
                }
                if let Some(p) = actor.properties.get("LightEffect") {
                    if let TaggedValue::Byte(b) = p.value {
                        let le_name = match b {
                            0 => "LE_None",
                            1 => "LE_TorchWaver",
                            2 => "LE_FireWaver",
                            3 => "LE_WateryShimmer",
                            4 => "LE_Searchlight",
                            5 => "LE_SlowWave",
                            6 => "LE_FastWave",
                            7 => "LE_CloudCast",
                            8 => "LE_StaticSpot",
                            9 => "LE_Shock",
                            10 => "LE_Disco",
                            11 => "LE_Warp",
                            12 => "LE_Spotlight",
                            13 => "LE_NonIncidence",
                            14 => "LE_Shell",
                            15 => "LE_OmniBumpMap",
                            16 => "LE_Interference",
                            17 => "LE_Cylinder",
                            18 => "LE_Rotor",
                            19 => "LE_Unused",
                            _ => "Other",
                        };
                        *light_effects.entry(le_name.to_string()).or_insert(0) += 1;
                    }
                }
                if actor.light_brightness > 0 {
                    light_brightnesses.push(actor.light_brightness);
                }
                if actor.light_radius > 0 {
                    light_radii.push(actor.light_radius);
                }
            } else if cls_low == "kicker" || cls_low == "ut_jumppad" {
                total_kickers += 1;
                if let Some(p) = actor.properties.get("KickVelocity") {
                    if let TaggedValue::Vector(v) = p.value {
                        kicker_velocities.push(v);
                    }
                }
            } else if cls_low.contains("teleporter") {
                total_teleporters += 1;
                if let Some(p) = actor.properties.get("bChangesYaw") {
                    if let TaggedValue::Bool(b) = p.value {
                        if b { teleporter_changes_yaw += 1; }
                    }
                }
                if let Some(p) = actor.properties.get("bChangesVelocity") {
                    if let TaggedValue::Bool(b) = p.value {
                        if b { teleporter_changes_vel += 1; }
                    }
                }
            } else if cls_low == "defensepoint" {
                total_defense_points += 1;
                if let Some(p) = actor.properties.get("bSniping") {
                    if let TaggedValue::Bool(b) = p.value {
                        if b { total_sniping_defense_points += 1; }
                    }
                }
                if let Some(p) = actor.properties.get("priority") {
                    if let TaggedValue::Byte(b) = p.value {
                        *defense_point_priorities.entry(b).or_insert(0) += 1;
                    }
                }
            } else if cls_low == "alternatepath" {
                total_alternate_paths += 1;
                if let Some(p) = actor.properties.get("SelectionWeight") {
                    if let TaggedValue::Float(f) = p.value {
                        alternate_path_weights.push(f);
                    }
                }
                if let Some(p) = actor.properties.get("bReturnOnly") {
                    if let TaggedValue::Bool(b) = p.value {
                        if b { alternate_path_return_only += 1; }
                    }
                }
            } else if cls_low == "trigger" || cls_low == "teamtrigger" || cls_low == "timedtrigger" {
                total_triggers += 1;
            } else if cls_low == "specialevent" {
                total_special_events += 1;
            } else if cls_low == "dispatcher" {
                total_dispatchers += 1;
            }
        }

        // Measure brush sizes for subtractive brushes
        for b in &lvl.brushes {
            if b.actor.csg_oper == 2 { // Subtract
                let mut min_pt = glam::Vec3::splat(f32::MAX);
                let mut max_pt = glam::Vec3::splat(f32::MIN);
                for wp in &b.world_polys {
                    for v in &wp.vertices {
                        min_pt = min_pt.min(*v);
                        max_pt = max_pt.max(*v);
                    }
                }
                let ext = max_pt - min_pt;
                if ext.x > 0.0 && ext.x < 16384.0 && ext.y > 0.0 && ext.y < 16384.0 && ext.z > 0.0 && ext.z < 8192.0 {
                    let min_dim = ext.x.min(ext.y);
                    let max_dim = ext.x.max(ext.y);
                    if min_dim >= 96.0 && min_dim <= 512.0 && max_dim > min_dim * 1.5 {
                        corridor_widths.push(min_dim);
                    }
                    if ext.z >= 128.0 && ext.z <= 4096.0 {
                        room_heights.push(ext.z);
                    }
                }
            }
        }
    }

    println!("\n=== COMPREHENSIVE FORENSIC ANALYSIS SUMMARY ===");
    println!("Total Maps Analyzed: {}", map_paths.len());
    println!("Total Actors: {}", total_actors);
    println!("Total Movers: {}", total_movers);
    println!("Total Lights: {} (with Corona: {})", total_lights, total_corona_lights);
    println!("Total Kickers/JumpPads: {}", total_kickers);
    println!("Total Teleporters: {} (YawChange: {}, VelChange: {})", total_teleporters, teleporter_changes_yaw, teleporter_changes_vel);
    println!("Total DefensePoints: {} (Sniping: {})", total_defense_points, total_sniping_defense_points);
    println!("Total AlternatePaths: {} (bReturnOnly: {})", total_alternate_paths, alternate_path_return_only);
    println!("Total Interactive Triggers: {} (Triggers={}, SpecialEvents={}, Dispatchers={})", total_triggers + total_special_events + total_dispatchers, total_triggers, total_special_events, total_dispatchers);

    println!("\n--- MOVER INITIAL STATES ---");
    let mut states: Vec<_> = mover_initial_states.into_iter().collect();
    states.sort_by(|a, b| b.1.cmp(&a.1));
    for (s, c) in states.iter().take(10) {
        println!("  {}: {}", s, c);
    }

    println!("\n--- MOVER ENCROACH BEHAVIORS ---");
    let mut encroaches: Vec<_> = mover_encroach_types.into_iter().collect();
    encroaches.sort_by(|a, b| b.1.cmp(&a.1));
    for (e, c) in encroaches {
        println!("  {}: {}", e, c);
    }

    if !mover_move_times.is_empty() {
        let avg_move = mover_move_times.iter().sum::<f32>() / mover_move_times.len() as f32;
        println!("\nAverage Mover MoveTime: {:.2}s", avg_move);
    }
    if !mover_stay_open_times.is_empty() {
        let avg_stay = mover_stay_open_times.iter().sum::<f32>() / mover_stay_open_times.len() as f32;
        println!("Average Mover StayOpenTime: {:.2}s", avg_stay);
    }

    println!("\n--- LIGHT TYPES ---");
    let mut ltypes: Vec<_> = light_types.into_iter().collect();
    ltypes.sort_by(|a, b| b.1.cmp(&a.1));
    for (t, c) in ltypes.iter().take(10) {
        println!("  {}: {}", t, c);
    }

    println!("\n--- LIGHT EFFECTS ---");
    let mut leffs: Vec<_> = light_effects.into_iter().collect();
    leffs.sort_by(|a, b| b.1.cmp(&a.1));
    for (e, c) in leffs.iter().take(10) {
        println!("  {}: {}", e, c);
    }

    if !light_brightnesses.is_empty() {
        let avg_b = light_brightnesses.iter().map(|&b| b as usize).sum::<usize>() as f32 / light_brightnesses.len() as f32;
        let avg_r = light_radii.iter().map(|&r| r as usize).sum::<usize>() as f32 / light_radii.len() as f32;
        println!("\nAverage Light Brightness: {:.1}, Average Light Radius: {:.1}", avg_b, avg_r);
    }

    if !kicker_velocities.is_empty() {
        let avg_vx = kicker_velocities.iter().map(|v| v.x.abs()).sum::<f32>() / kicker_velocities.len() as f32;
        let avg_vy = kicker_velocities.iter().map(|v| v.y.abs()).sum::<f32>() / kicker_velocities.len() as f32;
        let avg_vz = kicker_velocities.iter().map(|v| v.z).sum::<f32>() / kicker_velocities.len() as f32;
        println!("\nAverage Kicker KickVelocity: XY_mag={:.1}, Z_lift={:.1}", (avg_vx*avg_vx + avg_vy*avg_vy).sqrt(), avg_vz);
    }

    if !corridor_widths.is_empty() {
        let mut sorted_w = corridor_widths.clone();
        sorted_w.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_w = sorted_w[sorted_w.len() / 2];
        let avg_w = sorted_w.iter().sum::<f32>() / sorted_w.len() as f32;
        println!("\nCorridor Widths: count={}, avg={:.1} UU, median={:.1} UU, min={:.1} UU, max={:.1} UU", sorted_w.len(), avg_w, median_w, sorted_w.first().unwrap(), sorted_w.last().unwrap());
    }

    if !room_heights.is_empty() {
        let mut sorted_h = room_heights.clone();
        sorted_h.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_h = sorted_h[sorted_h.len() / 2];
        let avg_h = sorted_h.iter().sum::<f32>() / sorted_h.len() as f32;
        println!("Room Heights: count={}, avg={:.1} UU, median={:.1} UU, min={:.1} UU, max={:.1} UU", sorted_h.len(), avg_h, median_h, sorted_h.first().unwrap(), sorted_h.last().unwrap());
    }

    println!("\n--- TOP 25 ACTOR CLASSES IN CORPUS ---");
    let mut top_actors: Vec<_> = actor_class_counts.into_iter().collect();
    top_actors.sort_by(|a, b| b.1.cmp(&a.1));
    for (cls, cnt) in top_actors.iter().take(25) {
        println!("  {}: {}", cls, cnt);
    }

    println!("\n--- TOP 15 AMBIENT SOUNDS IN CORPUS ---");
    let mut top_sounds: Vec<_> = sound_counts.into_iter().collect();
    top_sounds.sort_by(|a, b| b.1.cmp(&a.1));
    for (snd, cnt) in top_sounds.iter().take(15) {
        println!("  {}: {}", snd, cnt);
    }
}

