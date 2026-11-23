use glam::Vec3;
use ut99_editor::editor_state::{EditorState, FogPresetId, FogShellTargetMode};
use ut99_format::{ActorRecord, TaggedProperty, TaggedValue, UnrLevel};

#[test]
fn test_zone_fog_profiles_collection_and_defaults() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    // 1. ZoneInfo with custom fog settings
    let mut zone1 = ActorRecord::default();
    zone1.class_name = "ZoneInfo".to_string();
    zone1.name = "SwampZoneInfo".to_string();
    zone1.tag = "SwampTag".to_string();
    zone1.location = Vec3::new(100.0, 200.0, 50.0);
    zone1.properties.insert(
        "bFogZone".to_string(),
        TaggedProperty {
            name: "bFogZone".to_string(),
            array_index: 0,
            value: TaggedValue::Bool(true),
        },
    );
    zone1.properties.insert(
        "FogDistance".to_string(),
        TaggedProperty {
            name: "FogDistance".to_string(),
            array_index: 0,
            value: TaggedValue::Float(3200.0),
        },
    );
    zone1.properties.insert(
        "FogColor".to_string(),
        TaggedProperty {
            name: "FogColor".to_string(),
            array_index: 0,
            value: TaggedValue::Color {
                r: 51,
                g: 102,
                b: 153,
                a: 255,
            },
        },
    );
    level.actors.push(zone1);

    // 2. WaterZone with implicit fluid fog
    let mut water = ActorRecord::default();
    water.class_name = "WaterZone".to_string();
    water.name = "OceanPool".to_string();
    water.location = Vec3::new(500.0, -300.0, -100.0);
    level.actors.push(water);

    // 3. Clear ZoneInfo without fog
    let mut zone_clear = ActorRecord::default();
    zone_clear.class_name = "ZoneInfo".to_string();
    zone_clear.name = "ArenaHall".to_string();
    zone_clear.location = Vec3::new(1000.0, 0.0, 0.0);
    level.actors.push(zone_clear);

    state.loaded_unr = Some(level);

    let profiles = state.collect_level_zone_fog_profiles();
    assert_eq!(profiles.len(), 3, "Must detect all 3 zone actors");

    // Profile 1: Custom Fog
    let p0 = &profiles[0];
    assert_eq!(p0.actor_name, "SwampZoneInfo");
    assert!(p0.b_fog_zone);
    assert_eq!(p0.fog_distance, 3200.0);
    assert!((p0.fog_color[0] - 51.0 / 255.0).abs() < 1e-3);

    // Profile 2: WaterZone default fluid fog
    let p1 = &profiles[1];
    assert_eq!(p1.class_name, "WaterZone");
    assert!(p1.b_fog_zone, "WaterZone must enable fog by default");
    assert_eq!(p1.fog_distance, 900.0, "WaterZone default cutoff is 900 uu");
    assert!(p1.fog_density >= 1.2, "Fluid zones have high density factor");

    // Profile 3: Clear ZoneInfo
    let p2 = &profiles[2];
    assert_eq!(p2.actor_name, "ArenaHall");
    assert!(!p2.b_fog_zone, "Clear ZoneInfo must have fog disabled");
    assert_eq!(p2.visibility_at(5000.0), 1.0, "Clear zone maintains 100% visibility");
}

#[test]
fn test_apply_fog_preset_and_undo_redo() {
    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut zone = ActorRecord::default();
    zone.class_name = "ZoneInfo".to_string();
    zone.name = "CavernZone".to_string();
    zone.location = Vec3::new(0.0, 0.0, 0.0);
    zone.properties.insert(
        "FogDistance".to_string(),
        TaggedProperty {
            name: "FogDistance".to_string(),
            array_index: 0,
            value: TaggedValue::Float(4000.0),
        },
    );
    level.actors.push(zone);
    state.loaded_unr = Some(level);

    // Initial state
    state.distance_fog_enabled = false;
    state.fog_end = 2500.0;

    // Apply Volcanic Sulfur Ash Preset
    let applied = state.apply_fog_preset_to_zone(0, FogPresetId::VolcanicSulfurAsh);
    assert!(applied);

    // Verify actor properties updated
    let actor = &state.loaded_unr.as_ref().unwrap().actors[0];
    if let Some(prop) = actor.properties.get("FogDistance") {
        match prop.value {
            TaggedValue::Float(f) => assert_eq!(f, 1600.0),
            _ => panic!("Expected Float FogDistance"),
        }
    } else {
        panic!("Missing FogDistance property");
    }

    // Verify viewport synchronization
    assert!(state.distance_fog_enabled);
    assert_eq!(state.fog_end, 1600.0);
    assert_eq!(state.fog_start, 120.0);

    // Test Undo
    state.undo();
    assert!(!state.distance_fog_enabled, "Undo must restore previous viewport fog state");
    assert_eq!(state.fog_end, 2500.0);
    let actor_reverted = &state.loaded_unr.as_ref().unwrap().actors[0];
    if let Some(prop) = actor_reverted.properties.get("FogDistance") {
        match prop.value {
            TaggedValue::Float(f) => assert_eq!(f, 4000.0),
            _ => panic!("Expected Float FogDistance"),
        }
    }

    // Test Redo
    state.redo();
    assert!(state.distance_fog_enabled, "Redo must re-apply volcanic fog");
    assert_eq!(state.fog_end, 1600.0);
    let actor_redone = &state.loaded_unr.as_ref().unwrap().actors[0];
    if let Some(prop) = actor_redone.properties.get("FogDistance") {
        match prop.value {
            TaggedValue::Float(f) => assert_eq!(f, 1600.0),
            _ => panic!("Expected Float FogDistance"),
        }
    }
}

#[test]
fn test_weapon_visibility_ratings_and_sniper_sightline() {
    // 1. Dense murky fog (start 100, end 1000, density 1.0)
    let ratings_dense = EditorState::compute_weapon_visibility_ratings(100.0, 1000.0, 1.0);
    assert_eq!(ratings_dense.len(), 5);

    // Melee range (256 uu)
    let (_, dist0, vis0) = ratings_dense[0];
    assert_eq!(dist0, 256.0);
    assert!(vis0 > 0.75, "Melee combat must remain visible: {}", vis0);

    // Medium combat range (768 uu)
    let (_, dist1, vis1) = ratings_dense[1];
    assert_eq!(dist1, 768.0);
    assert!(vis1 < 0.40, "Flak range in dense fog must be obscured: {}", vis1);

    // Sniper sightline (3072 uu) -> beyond cutoff
    let (_, dist3, vis3) = ratings_dense[3];
    assert_eq!(dist3, 3072.0);
    assert_eq!(vis3, 0.0, "Sniper sightline beyond cutoff must be 0% visible");

    // 2. Clear tournament atmosphere (start 3000, end 12000, density 0.8)
    let ratings_tournament = EditorState::compute_weapon_visibility_ratings(3000.0, 12000.0, 0.8);
    for &(name, dist, vis) in &ratings_tournament {
        if dist <= 3000.0 {
            assert_eq!(vis, 1.0, "Sub-start distance '{}' must be 100% visible", name);
        } else if dist == 3072.0 {
            assert!(vis > 0.95, "Tournament sniper range must have >95% clarity: {}", vis);
        }
    }
}

#[test]
fn test_place_fog_zone_at_pivot_and_settings() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(512.0, -256.0, 128.0);

    let idx = state.place_fog_zone_at_pivot(FogPresetId::DeepOceanicWater);
    assert_eq!(idx, 0);

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    let actor = &level.actors[0];
    assert_eq!(actor.class_name, "ZoneInfo");
    assert_eq!(actor.location, Vec3::new(512.0, -256.0, 128.0));

    // Viewport synchronized
    assert!(state.distance_fog_enabled);
    assert_eq!(state.fog_end, 900.0);
    assert_eq!(state.fog_start, 50.0);
    assert_eq!(state.selected_actor_index, Some(0));

    // Verify settings defaults
    assert!(state.zone_fog_settings.show_distance_shells);
    assert_eq!(state.zone_fog_settings.shell_target_mode, FogShellTargetMode::ActiveCamera);

    // Test Undo placement
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0, "Undo must remove placed fog zone");
}
