// crates/ut99-editor/tests/test_audit_portals_and_hazardous_fluids.rs
//
// Master Audit Suite for Portals & Hazardous Fluids:
// - Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
// - Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
// - Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio

use glam::Vec3;
use ut99_bsp::{
    FluidBasinArchetype, SecretDoorArchetype, TeleporterArchArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_portals_hazardous_fluids_and_secret_doors_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Secret Door
    state.secret_door_settings.active_config = SecretDoorArchetype::FlushWallSlidingPanel.to_config();
    state.secret_door_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (door_brush, door_act) = state.commit_secret_door_to_world();

    // 2. Commit Teleporter Arch
    state.teleporter_arch_settings.active_config = TeleporterArchArchetype::GothicMonolithGate.to_config();
    state.teleporter_arch_settings.active_config.position = Vec3::new(400.0, 100.0, 0.0);
    let (tele_brush, tele_act) = state.commit_teleporter_arch_to_world();

    // 3. Commit Fluid Basin
    state.fluid_basin_settings.active_config = FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config();
    state.fluid_basin_settings.active_config.position = Vec3::new(700.0, 100.0, 0.0);
    let (basin_brush, basin_act) = state.commit_fluid_basin_to_world();

    // Verification of brushes: exactly 3 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(door_brush, initial_brushes);
    assert_eq!(tele_brush, initial_brushes + 1);
    assert_eq!(basin_brush, initial_brushes + 2);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(door_act, Some(0));
    assert_eq!(tele_act, Some(1));
    assert_eq!(basin_act, Some(2));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Engine.Trigger");
    assert_eq!(level.actors[1].class_name, "Engine.Teleporter");
    assert_eq!(level.actors[2].class_name, "Botpack.PainZone");
}

#[test]
fn test_cascading_three_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: Secret Door
    state.secret_door_settings.active_config = SecretDoorArchetype::FlushWallSlidingPanel.to_config();
    state.commit_secret_door_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Teleporter Arch
    state.teleporter_arch_settings.active_config = TeleporterArchArchetype::GothicMonolithGate.to_config();
    state.commit_teleporter_arch_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: Fluid Basin
    state.fluid_basin_settings.active_config = FluidBasinArchetype::CorrosiveDeckSlimeVat.to_config();
    state.commit_fluid_basin_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 3 (Fluid Basin)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Teleporter Arch)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (Secret Door)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (Secret Door)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Teleporter Arch)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (Fluid Basin)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
}

#[test]
fn test_portal_aperture_and_fluid_survival_kinematics() {
    let mut state = EditorState::default();

    // 1. Portal Aperture Geometry
    state.teleporter_arch_settings.active_config.arch_width_uu = 128.0;
    state.teleporter_arch_settings.active_config.arch_height_uu = 160.0;
    state.teleporter_arch_settings.active_config.pillar_thickness_uu = 24.0;
    state.recalculate_teleporter_arch_preview();
    // Net w = 128 - 48 = 80, net h = 160 - 24 = 136 -> 10880.0
    assert_eq!(state.teleporter_arch_settings.cached_metrics.portal_aperture_area_sq_uu, 10880.0);

    // 2. Fluid Survival Dynamics
    state.fluid_basin_settings.active_config.dps = 40.0;
    state.recalculate_fluid_basin_preview();
    assert_eq!(state.fluid_basin_settings.cached_metrics.player_survival_time_secs, 2.5);
}
