// crates/ut99-editor/tests/test_audit_atmosphere_and_tournament_rules.rs
//
// Master Audit Suite for Atmosphere, Match Rules & Tournament Studios:
// - Interactive Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
// - Interactive Arena Match Rules, Tournament Mutator & Weapon Arena Studio
// - Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
//
// Validates cross-system integration, multi-stage cascading undo/redo,
// damage absorption hierarchies, and competitive safety auditing.

use glam::Vec3;
use ut99_bsp::{
    ArmorArchetype, ArenaRulesArchetype, WeatherArchetype,
};
use ut99_editor::editor_state::EditorState;
use ut99_format::ActorRecord;

#[test]
fn test_cross_studio_simultaneous_setup() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Setup initial LevelInfo in map
    let mut unr = ut99_format::UnrLevel::default();
    let mut info = ActorRecord::default();
    info.name = "LevelInfo0".to_string();
    info.class_name = "Engine.LevelInfo".to_string();
    info.tag = "MyMapInfo".to_string();
    unr.actors.push(info);
    state.loaded_unr = Some(unr);

    // 1. Dynamic Weather Studio: Chizra Monsoon Deluge
    state.weather_settings.active_config = WeatherArchetype::ChizraMonsoonDeluge.to_config();
    state.weather_settings.active_config.position = Vec3::new(0.0, 0.0, 500.0);
    let (vol_idx, emitter_idx, light_idx, audio_idx) = state.commit_weather_to_world();

    assert_eq!(vol_idx, initial_brushes);
    assert!(emitter_idx.is_some());
    assert!(light_idx.is_some());
    assert!(audio_idx.is_some());

    // 2. Arena Match Rules Studio: LowGravSniperExtravaganza
    state.arena_rules_settings.active_config = ArenaRulesArchetype::LowGravSniperExtravaganza.to_config();
    let (rules_info_idx, mutator_indices) = state.commit_arena_rules_to_world();

    assert_eq!(rules_info_idx, Some(0)); // Updated existing LevelInfo0
    assert_eq!(mutator_indices.len(), 2); // SniperArena + LowGrav
    assert_eq!(state.level_game_type, "Botpack.DeathMatchPlus");

    // 3. Armor Stacking Studio: ClassicDeck16Stack
    state.armor_stacking_settings.active_config = ArmorArchetype::ClassicDeck16Stack.to_config();
    state.armor_stacking_settings.active_config.position = Vec3::new(200.0, 100.0, 0.0);
    state.armor_stacking_settings.active_config.shield_belt_count = 1;
    state.armor_stacking_settings.active_config.health_vial_count = 2;
    let (ped_idx, pickup_indices) = state.commit_armor_stacking_to_world();

    assert_eq!(ped_idx, initial_brushes + 1); // 2nd brush added
    assert_eq!(pickup_indices.len(), 3); // 1 ShieldBelt + 2 Flanking Vials

    // Verification of global state
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    let level = state.loaded_unr.as_ref().unwrap();

    // Verify actors count: 1 initial + 3 weather + 2 mutators + 3 armor pickups = 9
    assert_eq!(level.actors.len(), 9);

    // Verify mutators exist
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.SniperArena"));
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.LowGrav"));

    // Verify weather actors exist
    assert!(level.actors.iter().any(|a| a.class_name == "Engine.WeatherEmitter"));
    assert!(level.actors.iter().any(|a| a.class_name == "Engine.Light"));

    // Verify armor actors exist
    assert!(level.actors.iter().any(|a| a.class_name == "Botpack.UT_ShieldBelt"));
    assert_eq!(level.actors.iter().filter(|a| a.class_name == "Botpack.HealthVial").count(), 2);
}

#[test]
fn test_cascading_multi_undo_redo_atmosphere_and_rules() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    let mut unr = ut99_format::UnrLevel::default();
    let mut info = ActorRecord::default();
    info.name = "LevelInfo0".to_string();
    info.class_name = "Engine.LevelInfo".to_string();
    info.tag = "InitialInfoTag".to_string();
    unr.actors.push(info);
    state.loaded_unr = Some(unr);
    state.level_game_type = "Botpack.DeathMatchPlus".to_string();

    // Stage 1: Commit Weather
    state.weather_settings.active_config = WeatherArchetype::MorpheusBlizzardDrift.to_config();
    state.commit_weather_to_world();

    // Stage 2: Commit Arena Rules
    state.arena_rules_settings.active_config = ArenaRulesArchetype::RocketArenaHeavySiege.to_config();
    state.commit_arena_rules_to_world();

    // Stage 3: Commit Armor Station
    state.armor_stacking_settings.active_config = ArmorArchetype::HardcoreDuelStandoff.to_config();
    state.armor_stacking_settings.active_config.shield_belt_count = 1;
    state.armor_stacking_settings.active_config.health_vial_count = 0;
    state.commit_armor_stacking_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.level_game_type, "Botpack.TeamGamePlus");

    // Cascading Undo 1: Revert Armor Station
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);

    // Cascading Undo 2: Revert Arena Rules
    state.undo();
    assert_eq!(state.level_game_type, "Botpack.DeathMatchPlus");
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].tag, "InitialInfoTag");

    // Cascading Undo 3: Revert Weather Volume
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1); // Only LevelInfo0 left

    // Cascading Redo 1: Re-apply Weather Volume
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);

    // Cascading Redo 2: Re-apply Arena Rules
    state.redo();
    assert_eq!(state.level_game_type, "Botpack.TeamGamePlus");

    // Cascading Redo 3: Re-apply Armor Station
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    let level_final = state.loaded_unr.as_ref().unwrap();
    assert!(level_final.actors.iter().any(|a| a.class_name == "Botpack.UT_ShieldBelt"));
}

#[test]
fn test_physics_kinematics_damage_and_hazard_audit_matrix() {
    let mut state = EditorState::default();

    // 1. Weather speed-of-sound acoustics
    state.weather_settings.active_config = WeatherArchetype::FrigateOceanicStorm.to_config();
    state.weather_settings.listener_probe_pos = Vec3::new(3400.0, 0.0, 0.0);
    state.scrub_weather_time(0.05); // Strike
    let w_metrics = &state.weather_settings.cached_metrics;
    assert!(w_metrics.is_flashing_now);
    assert!(w_metrics.thunder_delay_at_listener_secs > 0.0);

    // 2. Arena Rules competitive deadlock & TTK calculation
    state.arena_rules_settings.active_config = ArenaRulesArchetype::InstagibShockShowdown.to_config();
    state.recalculate_arena_rules_preview();
    assert_eq!(state.arena_rules_settings.cached_metrics.estimated_first_engagement_ttk_secs, 0.2);

    // 3. Armor Stacking Damage Absorption Hierarchy
    // 100 HP, 100 ShieldBelt, 100 BodyArmor, 50 ThighPads
    state.armor_stacking_settings.simulated_damage_probe_hp = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_shield = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_body = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_thigh = 50.0;

    // Test Shock Combo (200 damage):
    // 100 absorbed by ShieldBelt (shield broken).
    // Remaining 100 hits BodyArmor (75% absorbed = 75, 25% hits HP = 25).
    // Final: Shield=0, BodyArmor=25, Thigh=50, HP=75. Survived!
    state.simulate_armor_test_damage(200.0);
    let res = &state.armor_stacking_settings.cached_damage_result;
    assert_eq!(res.final_shield_belt, 0.0);
    assert_eq!(res.final_body_armor, 25.0);
    assert_eq!(res.final_thigh_pads, 50.0);
    assert_eq!(res.final_health, 75.0);
    assert_eq!(res.total_armor_absorbed, 175.0);
    assert_eq!(res.health_damage_taken, 25.0);
    assert!(res.player_survived);
}
