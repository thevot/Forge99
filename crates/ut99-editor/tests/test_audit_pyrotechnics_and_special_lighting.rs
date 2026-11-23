// crates/ut99-editor/tests/test_audit_pyrotechnics_and_special_lighting.rs
//
// Master Audit Suite for Environmental Hazards, Explosive Pyrotechnics & Special Lighting:
// - Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
// - Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
// - Interactive Special Lit, Zone Lighting Mask & Light Filter Studio

use glam::Vec3;
use ut99_bsp::{
    BarrelHazardType, SpecialLitArchetype, ZoneVelocityArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_pyrotechnics_special_lighting_and_fluid_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Zone Velocity & Portal Sheet
    state.zone_velocity_settings.active_config = ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config();
    state.zone_velocity_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (zone_brush, zone_act) = state.commit_zone_velocity_to_world();

    // 2. Commit Explosive Barrel
    state.hazard_barrel_settings.active_config = BarrelHazardType::VolatileNapalmFuelDrum.to_config();
    state.hazard_barrel_settings.active_config.position = Vec3::new(400.0, 100.0, 0.0);
    let (barrel_brush, barrel_act) = state.commit_hazard_barrel_to_world();

    // 3. Commit SpecialLit Fixture & Light
    state.special_lit_settings.active_config = SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config();
    state.special_lit_settings.active_config.position = Vec3::new(700.0, 100.0, 0.0);
    let (light_brush, light_act) = state.commit_special_lit_to_world();

    // Verification of brushes: exactly 3 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(zone_brush, initial_brushes);
    assert_eq!(barrel_brush, initial_brushes + 1);
    assert_eq!(light_brush, initial_brushes + 2);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(zone_act, Some(0));
    assert_eq!(barrel_act, Some(1));
    assert_eq!(light_act, Some(2));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Botpack.WaterZone");
    assert_eq!(level.actors[1].class_name, "Botpack.UT_SteelBarrel");
    assert_eq!(level.actors[2].class_name, "Engine.Light");
}

#[test]
fn test_cascading_three_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Zone Velocity
    state.zone_velocity_settings.active_config = ZoneVelocityArchetype::HydroelectricRiverCurrent.to_config();
    state.commit_zone_velocity_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Explosive Barrel
    state.hazard_barrel_settings.active_config = BarrelHazardType::VolatileNapalmFuelDrum.to_config();
    state.commit_hazard_barrel_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: SpecialLit Light
    state.special_lit_settings.active_config = SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config();
    state.commit_special_lit_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 3 (SpecialLit)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Barrel)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (Zone Velocity)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (Zone Velocity)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Barrel)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (SpecialLit)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
}

#[test]
fn test_blast_physics_and_special_illumination_isolation_kinematics() {
    let mut state = EditorState::default();

    // 1. Blast Physics Kinematics
    state.hazard_barrel_settings.active_config.blast_damage = 200.0;
    state.hazard_barrel_settings.active_config.blast_radius_uu = 400.0;
    state.recalculate_hazard_barrel_preview();
    // lethal radius: 400 * (1 - 100/200) = 200 uu
    assert_eq!(state.hazard_barrel_settings.cached_metrics.lethal_kill_radius_uu, 200.0);

    // 2. SpecialLit Illumination Flux
    state.special_lit_settings.active_config.light_brightness = 255;
    state.special_lit_settings.active_config.light_radius_uu = 200.0;
    state.recalculate_special_lit_preview();
    // flux: (255/255) * (200/100)^2 * 50 = 1 * 4 * 50 = 200.0
    assert_eq!(state.special_lit_settings.cached_metrics.effective_illumination_flux, 200.0);
}
