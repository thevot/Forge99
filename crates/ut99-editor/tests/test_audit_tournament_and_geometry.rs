use glam::Vec3;
use ut99_bsp::{
    analyze_item_balance, build_arch, build_cube, ArchConfig, ArchType, BalanceSeverity,
    CsgOperation, ItemTier, NavGraph, PF_ANTI_PORTAL, PF_NOT_SOLID,
};
use ut99_editor::editor_state::{BuilderShape, EditorState};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_parametric_arch_builder_types_and_watertightness() {
    // 1. Verify all 4 architectural arch styles
    assert_eq!(ArchType::ALL.len(), 4);

    for &arch_type in ArchType::ALL {
        let config = ArchConfig {
            arch_type,
            span_width: 256.0,
            jamb_height: 128.0,
            total_height: 300.0,
            depth: 64.0,
            wall_thickness: 32.0,
            segments: 8,
            is_doorway: true,
        };

        let brush = build_arch("TestArch", CsgOperation::Add, &config);
        assert!(
            !brush.polys.is_empty(),
            "Arch {:?} must generate polygons",
            arch_type
        );

        // Verify polygon structural integrity
        for (i, poly) in brush.polys.iter().enumerate() {
            assert!(
                poly.vertices.len() >= 3,
                "Arch {:?} poly {} must have >= 3 vertices",
                arch_type,
                i
            );
            let normal_len = poly.plane.normal.length();
            assert!(
                (normal_len - 1.0).abs() < 1e-3,
                "Arch {:?} poly {} normal must be unit-length (len = {})",
                arch_type,
                i,
                normal_len
            );
        }
    }

    // 2. Test hollow doorway vs solid decorative arch
    let doorway_cfg = ArchConfig {
        arch_type: ArchType::SemiCircular,
        span_width: 200.0,
        jamb_height: 100.0,
        total_height: 250.0,
        depth: 48.0,
        wall_thickness: 24.0,
        segments: 6,
        is_doorway: true,
    };
    let doorway_brush = build_arch("DoorwayArch", CsgOperation::Add, &doorway_cfg);

    let solid_cfg = ArchConfig {
        is_doorway: false,
        ..doorway_cfg.clone()
    };
    let solid_brush = build_arch("SolidArch", CsgOperation::Add, &solid_cfg);

    assert!(
        doorway_brush.polys.len() > 0 && solid_brush.polys.len() > 0,
        "Both doorway and solid arch must produce valid geometry"
    );

    // 3. Test EditorState integration and BuilderShape::Arch
    let mut state = EditorState::default();
    state.set_builder_arch(doorway_cfg.clone());
    assert_eq!(state.builder_shape, BuilderShape::Arch);
    assert_eq!(state.builder_arch_config.arch_type, ArchType::SemiCircular);
    assert!(!state.builder_brush.polys.is_empty());
}

#[test]
fn test_warpzone_pairing_and_unpaired_diagnostic() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. Paired WarpZone A -> B
    let mut wz_a = ActorRecord::default();
    wz_a.class_name = "Engine.WarpZoneInfo".to_string();
    wz_a.name = "WarpZone_East".to_string();
    wz_a.location = Vec3::new(0.0, 0.0, 0.0);
    wz_a.properties.insert(
        "OtherSide".to_string(),
        TaggedProperty {
            name: "OtherSide".to_string(),
            array_index: 0,
            value: TaggedValue::Str("WarpZone_West".to_string()),
        },
    );
    wz_a.properties.insert(
        "ZoneTag".to_string(),
        TaggedProperty {
            name: "ZoneTag".to_string(),
            array_index: 0,
            value: TaggedValue::Str("WZ_E".to_string()),
        },
    );
    level.actors.push(wz_a);

    // 2. Paired WarpZone B -> A
    let mut wz_b = ActorRecord::default();
    wz_b.class_name = "Engine.WarpZoneInfo".to_string();
    wz_b.name = "WarpZone_West".to_string();
    wz_b.location = Vec3::new(2000.0, 1500.0, 300.0);
    wz_b.properties.insert(
        "OtherSide".to_string(),
        TaggedProperty {
            name: "OtherSide".to_string(),
            array_index: 0,
            value: TaggedValue::Str("WZ_E".to_string()),
        },
    );
    wz_b.properties.insert(
        "ZoneTag".to_string(),
        TaggedProperty {
            name: "ZoneTag".to_string(),
            array_index: 0,
            value: TaggedValue::Str("WZ_W".to_string()),
        },
    );
    level.actors.push(wz_b);

    // 3. Unpaired / broken destination WarpZone
    let mut wz_orphan = ActorRecord::default();
    wz_orphan.class_name = "Engine.WarpZoneInfo".to_string();
    wz_orphan.name = "WarpZone_Void".to_string();
    wz_orphan.location = Vec3::new(5000.0, -5000.0, 0.0);
    wz_orphan.properties.insert(
        "OtherSide".to_string(),
        TaggedProperty {
            name: "OtherSide".to_string(),
            array_index: 0,
            value: TaggedValue::Str("NonExistentZone".to_string()),
        },
    );
    level.actors.push(wz_orphan);

    state.loaded_unr = Some(level);

    // Analyze portal visualizer pairs
    let pairs = state.get_warpzone_pairs();
    assert_eq!(pairs.len(), 3);

    // Pair A
    let pair_a = pairs.iter().find(|p| p.source_name == "WarpZone_East").unwrap();
    assert!(pair_a.is_paired);
    assert_eq!(pair_a.target_name, "WarpZone_West");
    assert!((pair_a.distance - Vec3::new(0.0, 0.0, 0.0).distance(Vec3::new(2000.0, 1500.0, 300.0))).abs() < 1.0);

    // Pair B
    let pair_b = pairs.iter().find(|p| p.source_name == "WarpZone_West").unwrap();
    assert!(pair_b.is_paired);
    assert_eq!(pair_b.target_name, "WarpZone_East");

    // Orphan
    let orphan = pairs.iter().find(|p| p.source_name == "WarpZone_Void").unwrap();
    assert!(!orphan.is_paired);
    assert_eq!(orphan.target_name, "NonExistentZone");
    assert_eq!(orphan.distance, 0.0);
}

#[test]
fn test_anti_portal_flagging_and_culling_efficiency() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Add two world brushes: a room cube and an occlusion wall
    let room_cube = build_cube("Room", CsgOperation::Add, Vec3::new(512.0, 512.0, 256.0));
    let occluder_cube = build_cube("Occluder", CsgOperation::Add, Vec3::new(32.0, 256.0, 256.0));
    state.world_brushes.push(room_cube);
    state.world_brushes.push(occluder_cube);

    // Initial state: 0 anti-portals
    let initial_report = state.analyze_anti_portals();
    assert_eq!(initial_report.total_anti_portals, 0);
    assert_eq!(initial_report.total_world_polys, 12);
    assert_eq!(initial_report.estimated_culling_efficiency, 0.0);

    // Flag brush 1 as anti-portal
    state.selected_brush_index = Some(1);
    let success = state.make_selected_brush_anti_portal();
    assert!(success);

    // Verify polygon flags
    let occluder = &state.world_brushes[1];
    for poly in &occluder.polys {
        assert_ne!(poly.flags & PF_ANTI_PORTAL, 0);
        assert_ne!(poly.flags & PF_NOT_SOLID, 0);
    }

    // Verify analyzer report
    let updated_report = state.analyze_anti_portals();
    assert_eq!(updated_report.total_anti_portals, 1);
    assert_eq!(updated_report.anti_portal_brushes, vec![1]);
    assert_eq!(updated_report.total_anti_portal_polys, 6);
    assert!(updated_report.estimated_culling_efficiency > 10.0);

    // Test Undo
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.analyze_anti_portals().total_anti_portals, 0);
    assert_eq!(state.world_brushes[1].polys[0].flags & PF_ANTI_PORTAL, 0);

    // Test Redo
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.analyze_anti_portals().total_anti_portals, 1);
    assert_ne!(state.world_brushes[1].polys[0].flags & PF_ANTI_PORTAL, 0);
}

#[test]
fn test_tournament_item_balance_simulator_and_stacking() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. Super Armor and UDamage placed close together (300 UU ~ 0.75s) -> Critical Stacking Risk!
    let mut shield = ActorRecord::default();
    shield.class_name = "Botpack.UT_ShieldBelt".to_string();
    shield.name = "ShieldBelt_Arena".to_string();
    shield.location = Vec3::new(0.0, 0.0, 0.0);
    level.actors.push(shield);

    let mut udam = ActorRecord::default();
    udam.class_name = "Botpack.UDamage".to_string();
    udam.name = "UDamage_Alcove".to_string();
    udam.location = Vec3::new(300.0, 0.0, 0.0);
    level.actors.push(udam);

    // 2. Power Weapon: Shock Rifle
    let mut shock = ActorRecord::default();
    shock.class_name = "Botpack.ShockRifle".to_string();
    shock.name = "Shock_Platform".to_string();
    shock.location = Vec3::new(1200.0, 0.0, 0.0);
    level.actors.push(shock);

    // 3. Balanced PlayerStart near Shock Rifle (200 UU ~ 0.5s)
    let mut spawn_near = ActorRecord::default();
    spawn_near.class_name = "Engine.PlayerStart".to_string();
    spawn_near.name = "Spawn_West".to_string();
    spawn_near.location = Vec3::new(1400.0, 0.0, 0.0);
    level.actors.push(spawn_near);

    // 4. Starved PlayerStart far away (6000 UU ~ 15.0s > 10.0s threshold)
    let mut spawn_far = ActorRecord::default();
    spawn_far.class_name = "Engine.PlayerStart".to_string();
    spawn_far.name = "Spawn_FarNorth".to_string();
    spawn_far.location = Vec3::new(7200.0, 0.0, 0.0);
    level.actors.push(spawn_far);

    // Also verify standalone analyze_item_balance direct API
    let raw_actors: Vec<(String, String, Vec3)> = vec![
        ("ShieldBelt_Arena".into(), "Botpack.UT_ShieldBelt".into(), Vec3::new(0.0, 0.0, 0.0)),
        ("UDamage_Alcove".into(), "Botpack.UDamage".into(), Vec3::new(300.0, 0.0, 0.0)),
        ("Shock_Platform".into(), "Botpack.ShockRifle".into(), Vec3::new(1200.0, 0.0, 0.0)),
        ("Spawn_West".into(), "Engine.PlayerStart".into(), Vec3::new(1400.0, 0.0, 0.0)),
        ("Spawn_FarNorth".into(), "Engine.PlayerStart".into(), Vec3::new(7200.0, 0.0, 0.0)),
    ];
    let nav_graph = NavGraph::new();
    let direct_report = analyze_item_balance(&raw_actors, &nav_graph);
    assert_eq!(direct_report.pickups.len(), 3);
    assert_eq!(direct_report.pair_timings.len(), 1);
    assert!(direct_report.pair_timings[0].is_stacking_hazard);

    state.loaded_unr = Some(level);

    // Run item balance simulation through EditorState
    let report = state.analyze_map_gameplay_balance();

    // Pickups identification
    assert_eq!(report.pickups.len(), 3);
    let p_shield = report.pickups.iter().find(|p| p.name == "ShieldBelt_Arena").unwrap();
    assert_eq!(p_shield.tier, ItemTier::SuperArmor);
    assert_eq!(p_shield.respawn_time_secs, 27.0);

    let p_udam = report.pickups.iter().find(|p| p.name == "UDamage_Alcove").unwrap();
    assert_eq!(p_udam.tier, ItemTier::PowerUp);
    assert_eq!(p_udam.respawn_time_secs, 83.0);

    let p_shock = report.pickups.iter().find(|p| p.name == "Shock_Platform").unwrap();
    assert_eq!(p_shock.tier, ItemTier::PowerWeapon);
    assert_eq!(p_shock.respawn_time_secs, 1.0);

    // Pair Timing & Stacking Hazard
    assert_eq!(report.pair_timings.len(), 1);
    let pair = &report.pair_timings[0];
    assert_eq!(pair.distance_uu, 300.0);
    assert!((pair.run_time_secs - 0.75).abs() < 1e-3);
    assert!(pair.is_stacking_hazard);

    // Issue Analysis
    let critical_issue = report.issues.iter().find(|i| i.severity == BalanceSeverity::Critical);
    assert!(critical_issue.is_some(), "Must report critical item stacking hazard");
    assert!(critical_issue.unwrap().title.contains("Item Stacking Hazard"));

    let warning_issue = report.issues.iter().find(|i| i.severity == BalanceSeverity::Warning);
    assert!(warning_issue.is_some(), "Must report weapon starvation warning for isolated spawn");
    assert!(warning_issue.unwrap().title.contains("Spawn Weapon Starvation"));

    // Spawn travel times
    assert_eq!(report.spawn_times_to_weapon.len(), 2);
    let time_near = report.spawn_times_to_weapon.iter().find(|(s, _)| s == "Spawn_West").unwrap().1;
    assert!((time_near - 0.5).abs() < 1e-3);

    let time_far = report.spawn_times_to_weapon.iter().find(|(s, _)| s == "Spawn_FarNorth").unwrap().1;
    assert!((time_far - 15.0).abs() < 1e-3);

    // Not tournament certified due to stacking hazard
    assert!(!report.is_tournament_certified);
}
