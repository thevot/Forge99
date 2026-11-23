use glam::Vec3;
use ut99_bsp::{
    evaluate_player_survival_time, simulate_swimming_trajectory, FluidZonePreset,
};
use ut99_editor::editor_state::EditorState;
use ut99_format::TaggedValue;

#[test]
fn test_fluid_presets_and_properties() {
    let presets = FluidZonePreset::all();
    assert_eq!(presets.len(), 6, "Must provide 6 canonical fluid and hazard presets");

    for &preset in presets {
        assert!(!preset.display_name().is_empty());
        assert!(!preset.description().is_empty());

        let phys = preset.to_physics();
        assert!(phys.zone_terminal_velocity >= 500.0);
        assert!(phys.zone_ground_friction >= 0.1);
        assert!(phys.zone_fluid_friction >= 0.0);

        let col = preset.hazard_color_rgba();
        assert!(col[3] > 0.0 && col[3] <= 1.0);
    }
}

#[test]
fn test_fluid_simulation_and_survival_time() {
    let water = FluidZonePreset::CrystalLakeWater.to_physics();
    let traj = simulate_swimming_trajectory(
        Vec3::ZERO,
        Vec3::new(0.0, 0.0, -100.0),
        &water,
        0.05,
        30,
    );
    assert_eq!(traj.len(), 31);
    // Archimedes buoyancy accelerates upward
    assert!(traj.last().unwrap().1.z > -100.0);

    let lava = FluidZonePreset::LethalMoltenLava.to_physics();
    let surv = evaluate_player_survival_time(&lava, 100);
    assert_eq!(surv, Some(2.5), "100 HP / 40 DPS = 2.5s");

    let slime = FluidZonePreset::ToxicBiohazardSlime.to_physics();
    let surv_slime = evaluate_player_survival_time(&slime, 100);
    assert_eq!(surv_slime, Some(5.0), "100 HP / 20 DPS = 5.0s");

    let safe = FluidZonePreset::CryogenicSlipperyIce.to_physics();
    assert_eq!(evaluate_player_survival_time(&safe, 100), None);
}

#[test]
fn test_editor_fluid_studio_placement_and_undo_redo() {
    let mut state = EditorState::default();
    state.builder_brush.location = Vec3::new(100.0, 200.0, 300.0);

    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    // 1. Place WaterZone at pivot
    let zone_idx = state.place_physics_zone_at_pivot(FluidZonePreset::CrystalLakeWater);

    let unr = state.loaded_unr.as_ref().expect("Level loaded");
    assert_eq!(unr.actors.len(), initial_actors + 1);
    let actor = &unr.actors[zone_idx];
    assert_eq!(actor.class_name, "WaterZone");
    assert_eq!(actor.location, Vec3::new(100.0, 200.0, 300.0));

    let b_water = actor.properties.get("bWaterZone").map(|p| &p.value);
    assert_eq!(b_water, Some(&TaggedValue::Bool(true)));

    // Test Undo placement
    assert!(state.history.can_undo());
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_after_undo.actors.len(), initial_actors);

    // Test Redo placement
    assert!(state.history.can_redo());
    state.redo();
    let unr_after_redo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_after_redo.actors.len(), initial_actors + 1);

    // 2. Select actor and apply Lava preset
    state.selected_actor_index = Some(zone_idx);
    let modified = state.apply_fluid_preset_to_selected(FluidZonePreset::LethalMoltenLava);
    assert_eq!(modified, 1);

    let unr_mod = state.loaded_unr.as_ref().unwrap();
    let dps = unr_mod.actors[zone_idx].properties.get("DamagePerSec").map(|p| &p.value);
    assert_eq!(dps, Some(&TaggedValue::Int(40)));

    // Test Undo preset application
    assert!(state.history.can_undo());
    state.undo();
    let unr_revert = state.loaded_unr.as_ref().unwrap();
    let dps_revert = unr_revert.actors[zone_idx].properties.get("DamagePerSec").map(|p| &p.value);
    assert_eq!(dps_revert, Some(&TaggedValue::Int(0)));

    // Test Redo preset application
    assert!(state.history.can_redo());
    state.redo();
    let unr_redone = state.loaded_unr.as_ref().unwrap();
    let dps_redone = unr_redone.actors[zone_idx].properties.get("DamagePerSec").map(|p| &p.value);
    assert_eq!(dps_redone, Some(&TaggedValue::Int(40)));

    // 3. Collect level zone physics
    let collected = state.collect_level_zone_physics();
    assert_eq!(collected.len(), 1);
    assert!(collected[0].2.b_pain_zone);
    assert_eq!(collected[0].2.damage_per_sec, 40);
    assert_eq!(collected[0].2.damage_type, "Burned");
}
