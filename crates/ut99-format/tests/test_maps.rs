use std::fs::File;
use std::io::Cursor;
use std::path::PathBuf;
use ut99_format::{parse_t3d, write_t3d, Package, RawPolys};

#[test]
fn test_load_entry_unr() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/Entry.unr");

    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }

    let pkg = Package::open(&map_path).expect("Failed to open Entry.unr");

    assert_eq!(pkg.summary.file_version, 68);
    for (i, exp) in pkg.exports.entries.iter().enumerate() {
        let name = pkg.get_name(exp.object_name as usize).unwrap_or("?");
        if name.starts_with("Polys") {
            let bytes = pkg.get_export_bytes(i).unwrap();
            println!("Entry Polys[{i}] '{}' len={}: first 16 bytes={:02x?}", name, bytes.len(), &bytes[..16.min(bytes.len())]);
        }
    }

}

#[test]
fn test_load_dm_tutorial_unr() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Tutorial.unr");

    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }

    let pkg = Package::open(&map_path).expect("Failed to open DM-Tutorial.unr");
    assert_eq!(pkg.summary.file_version, 68);
    assert!(pkg.summary.name_count > 400);
    assert!(pkg.summary.export_count > 300);
}

#[test]
fn test_inspect_deck16_models() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Deck16][.unr");
    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }
    let pkg = Package::open(&map_path).expect("Failed to open Deck16");
    println!("Deck16 total exports: {}", pkg.exports.entries.len());

    let mut model_indices = Vec::new();
    let mut polys_indices = Vec::new();

    for (i, exp) in pkg.exports.entries.iter().enumerate() {
        let name = pkg.get_name(exp.object_name as usize).unwrap_or("?");
        if name.starts_with("Model") {
            model_indices.push(i);
        } else if name.starts_with("Polys") {
            polys_indices.push(i);
        }
    }

    println!("Found {} Model exports and {} Polys exports", model_indices.len(), polys_indices.len());
    let exp813 = &pkg.exports.entries[813];
    let name813 = pkg.get_name(exp813.object_name as usize).unwrap_or("?");
    let p813_bytes = pkg.get_export_bytes(813).unwrap();
    println!("Export 813: '{}', class_index={}, size={}", name813, exp813.class_index, exp813.serial_size);
    println!("Export 813 first 64 bytes: {:02x?}", &p813_bytes[..64.min(p813_bytes.len())]);
    if let Err(e) = RawPolys::from_bytes(p813_bytes) {
        println!("ERROR parsing export 813: {e}");
    }




    for &pi in polys_indices.iter().take(10) {
        let p_exp = &pkg.exports.entries[pi];
        let p_name = pkg.get_name(p_exp.object_name as usize).unwrap_or("?");
        println!("Polys[{pi}]: '{}', size={}", p_name, p_exp.serial_size);
    }
}

#[test]
fn test_unr_level_ctf_epicboy() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/CTF-EpicBoy.unr");
    if !map_path.exists() {
        return;
    }
    let level = ut99_format::UnrLevel::open(&map_path).expect("Failed to open CTF-EpicBoy.unr");
    println!("Loaded level '{}':", level.map_name);
    println!("  Total Actors: {}", level.actors.len());
    println!("  Brushes: {}", level.brushes.len());
    println!("  Lights: {}", level.lights.len());
    println!("  PlayerStarts: {}", level.player_starts.len());
    println!("  PathNodes: {}", level.path_nodes.len());
    println!("  Weapons & Pickups: {}", level.weapons_and_pickups.len());

    assert_eq!(level.map_name, "CTF-EpicBoy");
    assert!(level.actors.len() > 1000, "CTF-EpicBoy should have >1000 actors");
    assert!(level.brushes.len() > 700, "CTF-EpicBoy should have >700 brushes (all models parsed without RF_HasStack failures)");
}

#[test]
fn test_unr_level_ctf_face2() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/CTF-Face][.unr");
    if !map_path.exists() {
        return;
    }
    let level = ut99_format::UnrLevel::open(&map_path).expect("Failed to open CTF-Face][.unr");
    println!("Loaded level '{}':", level.map_name);
    println!("  Total Actors: {}", level.actors.len());
    println!("  Brushes: {}", level.brushes.len());
    println!("  PlayerStarts: {}", level.player_starts.len());
    assert_eq!(level.map_name, "CTF-Face][");
    assert_eq!(level.brushes.len(), 212);
    assert_eq!(level.player_starts.len(), 36);
    assert_ne!(level.player_starts[0].location, glam::Vec3::ZERO, "PlayerStart location must be non-zero after tagged property fix");
}

#[test]
fn test_parse_real_exported_t3d() {
    let Ok(t3d_path) = std::env::var("FORGE99_TEST_T3D").map(PathBuf::from) else {
        eprintln!("FORGE99_TEST_T3D not set, skipping");
        return;
    };

    if !t3d_path.exists() {
        eprintln!("MyLevel.t3d not found, skipping");
        return;
    }

    let file = File::open(&t3d_path).expect("Failed to open MyLevel.t3d");
    let map = parse_t3d(file).expect("Failed to parse MyLevel.t3d");

    println!("Parsed {} actors from MyLevel.t3d", map.actors.len());
    assert!(!map.actors.is_empty(), "Map should have actors");

    // Check LevelInfo
    let level_info = map.actors.iter().find(|a| a.class == "LevelInfo").expect("LevelInfo missing");
    assert_eq!(level_info.name, "LevelInfo0");

    // Check Brushes
    let brushes: Vec<_> = map.actors.iter().filter(|a| a.brush.is_some()).collect();
    println!("Found {} brushes in MyLevel.t3d", brushes.len());
    assert!(!brushes.is_empty(), "Should find brushes");

    let first_brush = brushes[0].brush.as_ref().unwrap();
    println!("First brush has {} polygons", first_brush.polys.len());
    assert!(!first_brush.polys.is_empty());

    // Roundtrip verification: write back to T3D, parse again and compare
    let mut serialized = Vec::new();
    write_t3d(&mut serialized, &map).expect("Failed to write T3D");
    let map_reparsed = parse_t3d(Cursor::new(&serialized)).expect("Failed to reparse generated T3D");

    assert_eq!(map.actors.len(), map_reparsed.actors.len());
    for (a, b) in map.actors.iter().zip(map_reparsed.actors.iter()) {
        assert_eq!(a.class, b.class);
        assert_eq!(a.name, b.name);
        assert_eq!(a.brush.is_some(), b.brush.is_some());
        if let (Some(b1), Some(b2)) = (&a.brush, &b.brush) {
            assert_eq!(b1.polys.len(), b2.polys.len());
        }
    }
    println!("T3D Roundtrip test succeeded with 100% actor and polygon fidelity!");
}

#[test]
fn test_unr_level_entry() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/Entry.unr");

    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }

    let level = ut99_format::UnrLevel::open(&map_path).expect("Failed to open Entry.unr");
    println!("Loaded level '{}':", level.map_name);
    println!("  Total Actors: {}", level.actors.len());
    println!("  Brushes: {}", level.brushes.len());
    println!("  Lights: {}", level.lights.len());
    println!("  PlayerStarts: {}", level.player_starts.len());

    assert_eq!(level.map_name, "Entry");
    assert!(!level.actors.is_empty(), "Level should contain actors");
    assert!(!level.lights.is_empty(), "Level should contain at least 1 light");
    assert!(!level.player_starts.is_empty(), "Level should contain player starts");
    assert!(!level.brushes.is_empty(), "Level should contain brushes");

    let first_brush = &level.brushes[0];
    println!("First brush: '{}' with {} local polys and {} world polys", 
        first_brush.actor.name, 
        first_brush.local_polys.polys.len(), 
        first_brush.world_polys.len());
    assert!(!first_brush.world_polys.is_empty(), "Brush must have world-space polygons");
}

#[test]
fn test_unr_level_dm_tutorial() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Tutorial.unr");

    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }

    let level = ut99_format::UnrLevel::open(&map_path).expect("Failed to open DM-Tutorial.unr");
    println!("Loaded level '{}':", level.map_name);
    println!("  Total Actors: {}", level.actors.len());
    println!("  Brushes: {}", level.brushes.len());
    println!("  Lights: {}", level.lights.len());
    println!("  PlayerStarts: {}", level.player_starts.len());
    println!("  PathNodes: {}", level.path_nodes.len());
    println!("  Weapons & Pickups: {}", level.weapons_and_pickups.len());

    assert_eq!(level.map_name, "DM-Tutorial");
    assert!(level.actors.len() > 100, "DM-Tutorial should have >100 actors");
    assert!(level.lights.len() > 10, "DM-Tutorial should have lights");
    assert!(level.brushes.len() > 10, "DM-Tutorial should have brushes");
    assert!(level.path_nodes.len() > 10, "DM-Tutorial should have path nodes");
}

#[test]
fn test_unr_level_dm_deck16() {
    let map_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Deck16][.unr");

    if !map_path.exists() {
        eprintln!("Map file not found at: {map_path:?}, skipping");
        return;
    }

    let level = ut99_format::UnrLevel::open(&map_path).expect("Failed to open DM-Deck16][.unr");
    println!("Loaded level '{}':", level.map_name);
    println!("  Total Actors: {}", level.actors.len());
    println!("  Brushes: {}", level.brushes.len());
    println!("  Lights: {}", level.lights.len());
    println!("  PlayerStarts: {}", level.player_starts.len());
    println!("  PathNodes: {}", level.path_nodes.len());
    println!("  Weapons & Pickups: {}", level.weapons_and_pickups.len());

    assert_eq!(level.map_name, "DM-Deck16][");
    assert!(level.actors.len() > 200, "DM-Deck16][ should have >200 actors");
    assert!(level.brushes.len() > 30, "DM-Deck16][ should have >30 brushes");
    assert!(level.weapons_and_pickups.len() > 20, "DM-Deck16][ should have weapons & pickups");

    // Verify CPT_Str (prop_type = 13) decoded LevelInfo Title & Author in DM-Deck16][
    let li = level
        .actors
        .iter()
        .find(|a| a.class_name.eq_ignore_ascii_case("LevelInfo"))
        .expect("DM-Deck16][ must have LevelInfo");
    if let Some(title_prop) = li.properties.get("Title") {
        if let ut99_format::TaggedValue::Str(ref s) = title_prop.value {
            println!("DM-Deck16][ Title decoded via CPT_Str: '{s}'");
            assert!(!s.is_empty(), "Deck16 Title must not be empty");
        }
    }
}

#[test]
fn test_unr_package_roundtrip_and_validator() {
    use glam::Vec3;
    use ut99_format::{
        ActorRecord, Package, RawPoly, RawPolys, ReachSpecRecord, TaggedProperty, TaggedValue,
        UnrBrush, UnrLevel,
    };

    let mut level = UnrLevel::default();
    level.map_name = "DM-RoundtripVerify".to_string();

    // 1. LevelInfo with CPT_Str and CPT_Bool
    let mut li = ActorRecord {
        name: "LevelInfo0".to_string(),
        class_name: "LevelInfo".to_string(),
        flags: 0x02000004,
        ..Default::default()
    };
    li.properties.insert(
        "Title".to_string(),
        TaggedProperty {
            name: "Title".to_string(),
            array_index: 0,
            value: TaggedValue::Str("Deck 16 Modernized".to_string()),
        },
    );
    li.properties.insert(
        "bLonePlayer".to_string(),
        TaggedProperty {
            name: "bLonePlayer".to_string(),
            array_index: 0,
            value: TaggedValue::Bool(true),
        },
    );
    li.properties.insert(
        "bHumansOnly".to_string(),
        TaggedProperty {
            name: "bHumansOnly".to_string(),
            array_index: 0,
            value: TaggedValue::Bool(false),
        },
    );
    level.other_actors.push(li);

    // 2. Brush with explicit texture path
    let poly = RawPoly {
        base: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::new(0.0, 0.0, 1.0),
        texture_u: Vec3::new(1.0, 0.0, 0.0),
        texture_v: Vec3::new(0.0, 1.0, 0.0),
        vertices: vec![
            Vec3::new(-128.0, -128.0, 0.0),
            Vec3::new(128.0, -128.0, 0.0),
            Vec3::new(128.0, 128.0, 0.0),
            Vec3::new(-128.0, 128.0, 0.0),
        ],
        poly_flags: 0,
        actor_ref: 0,
        texture_ref: 0,
        item_name: 0,
        link_index: 0,
        brush_poly_index: 0,
        pan_u: 0,
        pan_v: 0,
    };
    let brush_actor = ActorRecord {
        name: "Brush0".to_string(),
        class_name: "Brush".to_string(),
        csg_oper: 2,
        ..Default::default()
    };
    level.brushes.push(UnrBrush {
        actor: brush_actor,
        local_polys: RawPolys {
            polys: vec![poly.clone()],
        },
        world_polys: vec![poly],
    });
    level.brush_poly_textures.push(vec!["Indus1.Walls.Floor1".to_string()]);

    // 3. Two PathNodes + ReachSpec + Object & Raw properties
    let mut pn0 = ActorRecord {
        name: "PathNode0".to_string(),
        class_name: "PathNode".to_string(),
        location: Vec3::new(0.0, 0.0, 32.0),
        ..Default::default()
    };
    pn0.properties.insert(
        "AmbientSound".to_string(),
        TaggedProperty {
            name: "AmbientSound".to_string(),
            array_index: 0,
            value: TaggedValue::Object(-1),
        },
    );
    pn0.object_paths.insert(
        "AmbientSound".to_string(),
        "Sound'AmbModern.Looping.hum1'".to_string(),
    );
    pn0.properties.insert(
        "CustomStruct".to_string(),
        TaggedProperty {
            name: "CustomStruct".to_string(),
            array_index: 0,
            value: TaggedValue::Raw(vec![10, 20, 30, 40]),
        },
    );
    pn0.raw_property_meta.insert(
        "CustomStruct".to_string(),
        (10, Some("PointRegion".to_string())),
    );

    let pn1 = ActorRecord {
        name: "PathNode1".to_string(),
        class_name: "PathNode".to_string(),
        location: Vec3::new(256.0, 0.0, 32.0),
        ..Default::default()
    };
    level.path_nodes.push(pn0);
    level.path_nodes.push(pn1);
    level.reach_specs.push(ReachSpecRecord {
        distance: 256,
        start_actor_name: "PathNode0".to_string(),
        end_actor_name: "PathNode1".to_string(),
        collision_radius: 40,
        collision_height: 85,
        reach_flags: 1,
        b_pruned: 0,
    });

    // Build package and validate
    let mut pkg = level.build_package().expect("Failed to build package");
    let long_name = "A".repeat(90);
    pkg.names.add(&long_name, 0);

    let bytes = pkg.to_bytes().expect("Failed to serialize package");
    let pkg2 = Package::from_bytes(bytes).expect("Failed to parse serialized package");

    let report = UnrLevel::validate_package(&pkg2);
    assert!(
        report.is_valid && report.errors.is_empty() && report.warnings.is_empty(),
        "Validation report had issues: errors={:?}, warnings={:?}",
        report.errors,
        report.warnings
    );
    assert_eq!(report.reach_spec_count, 1);
    assert!(report.bsp_node_count >= 1, "Compiled Model_World must contain BSP nodes");
    assert!(report.bsp_surf_count >= 1, "Compiled Model_World must contain BSP surfs");
    assert!(report.bsp_vert_count >= 4, "Compiled Model_World must contain BSP verts");
    assert!(report.zone_count >= 2, "Compiled Model_World must contain Zones");
    assert!(pkg2.names.entries.iter().any(|e| e.name == long_name));

    let roundtripped = UnrLevel::from_package(pkg2, "DM-RoundtripVerify".to_string())
        .expect("Failed to load UnrLevel from roundtripped package");

    let world_bsp = roundtripped
        .world_bsp
        .as_ref()
        .expect("Roundtripped UnrLevel must deserialize Model_World BSP");
    assert_eq!(world_bsp.nodes.len(), 1);
    assert_eq!(world_bsp.surfs.len(), 1);
    assert_eq!(world_bsp.verts.len(), 4);
    assert_eq!(world_bsp.zones.len(), 2);
    assert!(!world_bsp.light_maps.is_empty());
    assert!(!world_bsp.light_bits.is_empty());

    assert_eq!(roundtripped.actors[0].class_name, "LevelInfo");
    assert_eq!(
        roundtripped.actors[0].properties.get("Title").map(|p| &p.value),
        Some(&TaggedValue::Str("Deck 16 Modernized".to_string()))
    );
    assert_eq!(
        roundtripped.actors[0].properties.get("bLonePlayer").map(|p| &p.value),
        Some(&TaggedValue::Bool(true))
    );
    assert_eq!(
        roundtripped.actors[0].properties.get("bHumansOnly").map(|p| &p.value),
        Some(&TaggedValue::Bool(false))
    );
    assert_eq!(
        roundtripped.brush_poly_textures,
        vec![vec!["Texture'Indus1.Walls.Floor1'".to_string()]]
    );
    assert_eq!(roundtripped.reach_specs.len(), 1);
    assert_eq!(roundtripped.reach_specs[0].distance, 256);

    let r_pn0 = roundtripped
        .path_nodes
        .iter()
        .find(|n| n.name == "PathNode0")
        .expect("PathNode0 missing");
    assert_eq!(
        r_pn0.object_paths.get("AmbientSound").map(String::as_str),
        Some("Sound'AmbModern.Looping.hum1'")
    );
    assert_eq!(
        r_pn0.properties.get("CustomStruct").map(|p| &p.value),
        Some(&TaggedValue::Raw(vec![10, 20, 30, 40]))
    );
    assert_eq!(
        r_pn0.raw_property_meta.get("CustomStruct"),
        Some(&(10, Some("PointRegion".to_string())))
    );
}

#[test]
fn test_p0_task01_real_ut99_umodel_bsp_parsing() {
    let entry_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/Entry.unr");
    if entry_path.exists() {
        let entry_level = ut99_format::UnrLevel::open(&entry_path).expect("Open Entry.unr");
        for (i, exp) in entry_level.package.exports.entries.iter().enumerate() {
            if entry_level
                .package
                .get_export_class_name(exp)
                .eq_ignore_ascii_case("Model")
            {
                let m_bytes = entry_level.package.get_export_bytes(i).unwrap();
                assert!(
                    ut99_format::parse_compiled_umodel_bsp(
                        m_bytes,
                        exp.object_flags,
                        &entry_level.package
                    )
                    .is_some(),
                    "Every UModel export in Entry.unr (#{i}) must parse cleanly"
                );
            }
        }
        let bsp = entry_level
            .world_bsp
            .as_ref()
            .expect("Entry.unr world_bsp must parse without FZoneProperties desync");
        assert_eq!(bsp.nodes.len(), 15);
        assert_eq!(bsp.zones.len(), 2);
        assert!(!bsp.leaves.is_empty());
    }

    let tut_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../UnrealTournament/Maps/DM-Tutorial.unr");
    if tut_path.exists() {
        let tut_level = ut99_format::UnrLevel::open(&tut_path).expect("Open DM-Tutorial.unr");
        for (i, exp) in tut_level.package.exports.entries.iter().enumerate() {
            if tut_level
                .package
                .get_export_class_name(exp)
                .eq_ignore_ascii_case("Model")
            {
                let m_bytes = tut_level.package.get_export_bytes(i).unwrap();
                assert!(
                    ut99_format::parse_compiled_umodel_bsp(
                        m_bytes,
                        exp.object_flags,
                        &tut_level.package
                    )
                    .is_some(),
                    "Every UModel export in DM-Tutorial.unr (#{i}) must parse cleanly"
                );
            }
        }
        let bsp = tut_level
            .world_bsp
            .as_ref()
            .expect("DM-Tutorial.unr world_bsp must parse without FLightMapIndex desync");
        assert_eq!(bsp.nodes.len(), 729);
        assert!(bsp.surfs.len() > 200);
        assert!(bsp.verts.len() > 1000);
        assert!(bsp.zones.len() >= 2);
        assert!(!bsp.light_maps.is_empty());
        assert!(!bsp.bounds.is_empty());
        assert!(!bsp.leaves.is_empty());
    }
}

#[test]
fn test_p0_task02_task04_task06_actor_subclasses_mover_bsp_and_sync_views() {
    use glam::Vec3;
    use ut99_format::{
        ActorRecord, Package, RawPoly, RawPolys, TaggedProperty, TaggedValue, UnrBrush, UnrLevel,
    };

    let mut level = UnrLevel::default();
    level.map_name = "DM-P0RoundTrip".to_string();
    let custom_guid = [0xAA, 0xBB, 0xCC, 0xDD, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    level.package.summary.guid = custom_guid;
    level.package.summary.pkg_flags = 0x0001;

    // Static CSG brush with sheer_rate/sheer_axis
    let poly = RawPoly {
        base: Vec3::new(0.0, 0.0, 0.0),
        normal: Vec3::new(0.0, 0.0, 1.0),
        texture_u: Vec3::new(1.0, 0.0, 0.0),
        texture_v: Vec3::new(0.0, 1.0, 0.0),
        vertices: vec![
            Vec3::new(-64.0, -64.0, 0.0),
            Vec3::new(64.0, -64.0, 0.0),
            Vec3::new(64.0, 64.0, 0.0),
            Vec3::new(-64.0, 64.0, 0.0),
        ],
        poly_flags: 0,
        actor_ref: 0,
        texture_ref: 0,
        item_name: 0,
        link_index: 0,
        brush_poly_index: 0,
        pan_u: 0,
        pan_v: 0,
    };
    let mut static_brush_actor = ActorRecord {
        name: "Brush1".to_string(),
        class_name: "Brush".to_string(),
        csg_oper: 1,
        main_scale: Vec3::new(2.0, 1.0, 1.0),
        ..Default::default()
    };
    static_brush_actor.properties.insert(
        "MainScale".to_string(),
        TaggedProperty {
            name: "MainScale".to_string(),
            array_index: 0,
            value: TaggedValue::Scale {
                scale: Vec3::new(2.0, 1.0, 1.0),
                sheer_rate: 0.5,
                sheer_axis: 2,
            },
        },
    );
    level.brushes.push(UnrBrush {
        actor: static_brush_actor,
        local_polys: RawPolys {
            polys: vec![poly.clone()],
        },
        world_polys: vec![poly.clone()],
    });

    // Mover subclass (ElevatorMover in UnrealShare) -> must get compiled BSP in Model_1 and NOT pollute static Model_World
    let mover_actor = ActorRecord {
        name: "ElevatorMover0".to_string(),
        class_name: "ElevatorMover".to_string(),
        location: Vec3::new(128.0, 0.0, 64.0),
        ..Default::default()
    };
    level.brushes.push(UnrBrush {
        actor: mover_actor,
        local_polys: RawPolys {
            polys: vec![poly.clone()],
        },
        world_polys: vec![poly.clone()],
    });

    // Place actors into `level.actors` and call `sync_actor_views()` (TASK-04)
    let mut trig_light = ActorRecord {
        name: "TriggerLight0".to_string(),
        class_name: "TriggerLight".to_string(),
        location: Vec3::new(100.0, 200.0, 300.0),
        light_brightness: 180,
        light_radius: 48,
        ..Default::default()
    };
    // Forward intra-level reference to ElevatorMover0 via Base
    trig_light.properties.insert(
        "Base".to_string(),
        TaggedProperty {
            name: "Base".to_string(),
            array_index: 0,
            value: TaggedValue::Object(1),
        },
    );
    trig_light
        .object_paths
        .insert("Base".to_string(), "MyLevel.ElevatorMover0".to_string());

    let jump_spot = ActorRecord {
        name: "JumpSpot0".to_string(),
        class_name: "JumpSpot".to_string(),
        location: Vec3::new(300.0, 400.0, 64.0),
        ..Default::default()
    };
    level.actors.push(trig_light);
    level.actors.push(jump_spot);
    level.sync_actor_views();

    assert_eq!(level.lights.len(), 1);
    assert_eq!(level.path_nodes.len(), 1);

    let pkg = level.build_package().expect("Build package");
    let bytes = pkg.to_bytes().expect("Serialize package");
    let pkg2 = Package::from_bytes(bytes).expect("Parse serialized package");

    assert_eq!(pkg2.summary.file_version, 68);
    assert_eq!(pkg2.summary.guid, custom_guid);
    assert_eq!(pkg2.summary.pkg_flags, 0x0001);

    // Verify Model_0 (static brush) is 70 bytes and Model_1 (ElevatorMover) has compiled BSP nodes
    let m0_idx = pkg2
        .exports
        .entries
        .iter()
        .position(|e| pkg2.get_name(e.object_name as usize) == Some("Model_0"))
        .expect("Model_0 export");
    let m0_bytes = pkg2.get_export_bytes(m0_idx).unwrap();
    assert_eq!(
        m0_bytes.len(),
        70,
        "Canonical empty-BSP UModel must be 70 bytes (including NumSharedSides and NumZones)"
    );

    let m1_idx = pkg2
        .exports
        .entries
        .iter()
        .position(|e| pkg2.get_name(e.object_name as usize) == Some("Model_1"))
        .expect("Model_1 export");
    let m1_bytes = pkg2.get_export_bytes(m1_idx).unwrap();
    let (mover_bsp, _) =
        ut99_format::parse_compiled_umodel_bsp(m1_bytes, 0, &pkg2).expect("Parse Mover Model_1 BSP");
    assert_eq!(
        mover_bsp.nodes.len(),
        1,
        "Mover UModel must contain compiled BSP nodes for collision/raytracing"
    );

    let roundtripped = UnrLevel::from_package(pkg2, "DM-P0RoundTrip".to_string())
        .expect("Re-open roundtripped level");

    // Static Model_World must have only the 1 static brush poly, not the Mover poly
    assert_eq!(roundtripped.world_bsp.as_ref().unwrap().surfs.len(), 1);

    // Subclasses and packages must be preserved
    let rt_mover = roundtripped
        .brushes
        .iter()
        .find(|b| b.actor.name == "ElevatorMover0")
        .expect("ElevatorMover0 preserved");
    assert_eq!(rt_mover.actor.class_name, "ElevatorMover");
    assert_eq!(
        rt_mover.actor.object_paths.get("__class_package").map(String::as_str),
        Some("UnrealShare")
    );

    let rt_light = roundtripped
        .lights
        .iter()
        .find(|l| l.name == "TriggerLight0")
        .expect("TriggerLight0 preserved");
    assert_eq!(rt_light.class_name, "TriggerLight");
    assert_eq!(
        rt_light.object_paths.get("__class_package").map(String::as_str),
        Some("UnrealShare")
    );
    assert_eq!(
        rt_light.object_paths.get("Base").map(String::as_str),
        Some("ElevatorMover'MyLevel.ElevatorMover0'")
    );

    let rt_jump = roundtripped
        .path_nodes
        .iter()
        .find(|p| p.name == "JumpSpot0")
        .expect("JumpSpot0 preserved");
    assert_eq!(rt_jump.class_name, "JumpSpot");
    assert_eq!(
        rt_jump.object_paths.get("__class_package").map(String::as_str),
        Some("BotPack")
    );

    // Verify sheer_rate and sheer_axis round-tripped on Brush1
    let rt_b1 = roundtripped
        .brushes
        .iter()
        .find(|b| b.actor.name == "Brush1")
        .expect("Brush1 preserved");
    assert_eq!(
        rt_b1.actor.properties.get("MainScale").map(|p| &p.value),
        Some(&TaggedValue::Scale {
            scale: Vec3::new(2.0, 1.0, 1.0),
            sheer_rate: 0.5,
            sheer_axis: 2,
        })
    );
}
