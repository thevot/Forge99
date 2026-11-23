// crates/ut99-editor/tests/test_armor_stacking_studio.rs
//
// Integration tests for Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio.

use glam::Vec3;
use ut99_bsp::ArmorArchetype;
use ut99_editor::editor_state::EditorState;
use ut99_format::ActorRecord;

#[test]
fn test_armor_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is ClassicDeck16Stack
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::ClassicDeck16Stack
    );
    assert_eq!(state.armor_stacking_settings.active_config.shield_belt_count, 1);
    assert_eq!(state.armor_stacking_settings.active_config.body_armor_count, 1);
    assert_eq!(state.armor_stacking_settings.active_config.keg_of_health_count, 1);

    // Apply HardcoreDuelStandoff
    state.apply_armor_archetype(ArmorArchetype::HardcoreDuelStandoff);
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::HardcoreDuelStandoff
    );
    assert_eq!(state.armor_stacking_settings.active_config.thigh_pads_count, 0);
    assert_eq!(state.armor_stacking_settings.active_config.keg_of_health_count, 0);

    // Apply TeamFortressHeavyDefense
    state.apply_armor_archetype(ArmorArchetype::TeamFortressHeavyDefense);
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::TeamFortressHeavyDefense
    );
    assert_eq!(state.armor_stacking_settings.active_config.shield_belt_count, 2);
    assert_eq!(state.armor_stacking_settings.active_config.body_armor_count, 2);

    // Apply InstagibCleanSweep
    state.apply_armor_archetype(ArmorArchetype::InstagibCleanSweep);
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::InstagibCleanSweep
    );
    assert_eq!(state.armor_stacking_settings.active_config.shield_belt_count, 0);
    assert_eq!(state.armor_stacking_settings.active_config.max_health_overstack, 100);

    // Apply RelicRegenPowerSurge
    state.apply_armor_archetype(ArmorArchetype::RelicRegenPowerSurge);
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::RelicRegenPowerSurge
    );
    assert_eq!(state.armor_stacking_settings.active_config.regeneration_rate_hp_per_sec, 2.0);

    // Undo returns to InstagibCleanSweep
    state.undo();
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::InstagibCleanSweep
    );

    // Redo returns to RelicRegenPowerSurge
    state.redo();
    assert_eq!(
        state.armor_stacking_settings.active_config.archetype,
        ArmorArchetype::RelicRegenPowerSurge
    );
}

#[test]
fn test_armor_absorption_damage_simulation() {
    let mut state = EditorState::default();

    state.armor_stacking_settings.simulated_damage_probe_hp = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_shield = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_body = 100.0;
    state.armor_stacking_settings.simulated_damage_probe_thigh = 50.0;

    // 1. Moderate hit (50 damage, fully absorbed by ShieldBelt)
    state.simulate_armor_test_damage(50.0);
    let res1 = &state.armor_stacking_settings.cached_damage_result;
    assert_eq!(res1.final_health, 100.0);
    assert_eq!(res1.final_shield_belt, 50.0);
    assert_eq!(res1.final_body_armor, 100.0);
    assert!(res1.player_survived);

    // 2. Heavy hit (120 damage, breaks shield, 20 dmg into body armor: 15 absorbed, 5 into HP)
    state.simulate_armor_test_damage(120.0);
    let res2 = &state.armor_stacking_settings.cached_damage_result;
    assert_eq!(res2.final_shield_belt, 0.0);
    assert_eq!(res2.final_body_armor, 85.0);
    assert_eq!(res2.final_health, 95.0);
    assert_eq!(res2.total_armor_absorbed, 115.0);
    assert!(res2.player_survived);

    // 3. Lethal Overkill (500 damage from Redeemer)
    state.simulate_armor_test_damage(500.0);
    let res3 = &state.armor_stacking_settings.cached_damage_result;
    assert_eq!(res3.final_health, 0.0);
    assert!(!res3.player_survived);
}

#[test]
fn test_armor_stacking_safety_audit_and_clustering() {
    let mut state = EditorState::default();

    // 1. Default setup has no hazards
    state.recalculate_armor_stacking_preview();
    assert!(state.armor_stacking_settings.cached_hazards.is_empty());

    // 2. Setup clustered major items in UNR (< 400 uu apart)
    let mut unr = ut99_format::UnrLevel::default();
    let mut sb = ActorRecord::default();
    sb.name = "ShieldBelt0".to_string();
    sb.class_name = "Botpack.UT_ShieldBelt".to_string();
    sb.location = Vec3::new(100.0, 0.0, 0.0);
    unr.actors.push(sb);

    let mut keg = ActorRecord::default();
    keg.name = "Keg0".to_string();
    keg.class_name = "Botpack.HealthPack".to_string();
    keg.location = Vec3::new(250.0, 0.0, 0.0); // 150 uu away!
    unr.actors.push(keg);

    state.loaded_unr = Some(unr);
    state.recalculate_armor_stacking_preview();
    assert!(state.armor_stacking_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Clustering")));

    // 3. ShieldBelt over-saturation (> 2)
    state.armor_stacking_settings.active_config.shield_belt_count = 3;
    state.recalculate_armor_stacking_preview();
    assert!(state.armor_stacking_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Over-Saturation")));
}

#[test]
fn test_commit_armor_station_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    state.apply_armor_archetype(ArmorArchetype::ClassicDeck16Stack);
    state.armor_stacking_settings.active_config.shield_belt_count = 1;
    state.armor_stacking_settings.active_config.health_vial_count = 2;

    let (brush_idx, pickup_indices) = state.commit_armor_stacking_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert_eq!(pickup_indices.len(), 3); // 1 ShieldBelt + 2 Flanking Vials

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors[pickup_indices[0]].class_name, "Botpack.UT_ShieldBelt");
    assert_eq!(level.actors[pickup_indices[1]].class_name, "Botpack.HealthVial");
    assert_eq!(level.actors[pickup_indices[2]].class_name, "Botpack.HealthVial");

    // Undo commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // Redo commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 3);
    assert_eq!(level_redone.actors[0].class_name, "Botpack.UT_ShieldBelt");
}
