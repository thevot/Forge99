// crates/ut99-editor/tests/test_jump_boots_studio.rs
//
// Integration tests for Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio

use glam::Vec3;
use ut99_bsp::{
    evaluate_jump_boots_metrics, JumpBootsArchetype, JumpBootsConfig,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_jump_boots_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    let initial_config = state.jump_boots_settings.active_config.clone();
    assert_eq!(
        initial_config.archetype,
        JumpBootsArchetype::PhobosLowGravityVault
    );

    // Apply FacingWorldsTowerSpireAscent
    state.apply_jump_boots_archetype(JumpBootsArchetype::FacingWorldsTowerSpireAscent);
    assert_eq!(
        state.jump_boots_settings.active_config.archetype,
        JumpBootsArchetype::FacingWorldsTowerSpireAscent
    );
    assert_eq!(
        state.jump_boots_settings.active_config.launch_boost_vz,
        1250.0
    );

    // Undo -> Back to initial
    state.undo();
    assert_eq!(
        state.jump_boots_settings.active_config.archetype,
        JumpBootsArchetype::PhobosLowGravityVault
    );

    // Redo -> Back to FacingWorldsTowerSpireAscent
    state.redo();
    assert_eq!(
        state.jump_boots_settings.active_config.archetype,
        JumpBootsArchetype::FacingWorldsTowerSpireAscent
    );
}

#[test]
fn test_jump_boots_ballistics_and_reachability() {
    let mut cfg = JumpBootsConfig::default();
    cfg.launch_boost_vz = 1000.0;
    cfg.forward_velocity_vx = 400.0;
    cfg.world_gravity_z = -1000.0;
    cfg.target_platform_offset = Vec3::new(800.0, 0.0, 0.0);
    cfg.target_platform_size = Vec3::new(100.0, 100.0, 16.0);

    let metrics = evaluate_jump_boots_metrics(&cfg, &[]);
    // apex = vz^2 / (2g) = 1,000,000 / 2000 = 500 uu
    assert_eq!(metrics.peak_apex_height_uu, 500.0);
    // t_apex = vz / g = 1.0s, total_hang_time = 2.0s
    assert_eq!(metrics.time_to_apex_s, 1.0);
    assert_eq!(metrics.total_hang_time_s, 2.0);
    // max_dist = 400 * 2 = 800 uu
    assert_eq!(metrics.maximum_vault_distance_uu, 800.0);
    assert!(metrics.is_target_reachable);
}

#[test]
fn test_jump_boots_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // Trigger ceiling concussion & stranding
    state.jump_boots_settings.active_config.overhead_ceiling_clearance_uu = 200.0;
    state.jump_boots_settings.active_config.target_platform_offset = Vec3::new(400.0, 0.0, 600.0);
    state.jump_boots_settings.active_config.has_return_route = false;
    state.jump_boots_settings.active_config.target_platform_size = Vec3::new(32.0, 32.0, 16.0);
    state.recalculate_jump_boots_preview();

    let hazards = &state.jump_boots_settings.cached_hazards;
    assert!(hazards.iter().any(|h| h.title.contains("Ceiling Concussion")));
    assert!(hazards.iter().any(|h| h.title.contains("Stranding")));
    assert!(hazards.iter().any(|h| h.title.contains("Insufficient Landing Clearance")));
}

#[test]
fn test_commit_jump_boots_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brush_count = state.world_brushes.len();

    state.jump_boots_settings.active_config.position = Vec3::new(500.0, 500.0, 100.0);
    state.jump_boots_settings.active_config.charge_count = 3;

    let (brush_idx, boots_idx) = state.commit_jump_boots_to_world();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    assert_eq!(brush_idx, initial_brush_count);
    assert!(boots_idx.is_some());

    // Check boots actor in level
    let level = state.loaded_unr.as_ref().unwrap();
    let boots = &level.actors[boots_idx.unwrap()];
    assert_eq!(boots.class_name, "Botpack.UT_JumpBoots");
    assert_eq!(boots.group, "JumpBootsCaches");

    // Undo -> Cradle brush and actor removed
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brush_count);
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 0);

    // Redo -> Restored
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.UT_JumpBoots");
}
