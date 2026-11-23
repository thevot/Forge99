// crates/ut99-editor/tests/test_bot_jump_routes_studio.rs
//
// Integration tests for Interactive Bot AI Tactical JumpSpot & Translocator Route Studio.

use glam::Vec3;
use ut99_bsp::{BotJumpArchetype, BotJumpType};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_jump_route_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is Deck16SlimeBridgeJump
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::Deck16SlimeBridgeJump
    );
    assert_eq!(
        state.bot_jump_routes_settings.active_config.jump_type,
        BotJumpType::ImpactHammerJump
    );
    assert!(state.bot_jump_routes_settings.active_config.b_impact_jump);

    // Apply FaceSniperTowerClimb
    state.apply_jump_route_archetype(BotJumpArchetype::FaceSniperTowerClimb);
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::FaceSniperTowerClimb
    );
    assert_eq!(
        state.bot_jump_routes_settings.active_config.jump_type,
        BotJumpType::TranslocatorToss
    );
    assert!(state.bot_jump_routes_settings.active_config.b_always_translocate);

    // Apply PhobosGravityLeap
    state.apply_jump_route_archetype(BotJumpArchetype::PhobosGravityLeap);
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::PhobosGravityLeap
    );
    assert_eq!(
        state.bot_jump_routes_settings.active_config.gravity_uu_s2,
        300.0
    );

    // Apply CoretFlagRunAscent
    state.apply_jump_route_archetype(BotJumpArchetype::CoretFlagRunAscent);
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::CoretFlagRunAscent
    );
    assert_eq!(
        state.bot_jump_routes_settings.active_config.jump_type,
        BotJumpType::JumpBootsJump
    );

    // Apply TurbineVentVault
    state.apply_jump_route_archetype(BotJumpArchetype::TurbineVentVault);
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::TurbineVentVault
    );
    assert_eq!(
        state.bot_jump_routes_settings.active_config.jump_type,
        BotJumpType::StandardJump
    );

    // Undo returns to CoretFlagRunAscent
    state.undo();
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::CoretFlagRunAscent
    );

    // Redo returns to TurbineVentVault
    state.redo();
    assert_eq!(
        state.bot_jump_routes_settings.active_config.archetype,
        BotJumpArchetype::TurbineVentVault
    );
}

#[test]
fn test_jump_trajectory_ballistic_solving_and_metrics() {
    let mut state = EditorState::default();

    state.apply_jump_route_archetype(BotJumpArchetype::Deck16SlimeBridgeJump);
    let sol = &state.bot_jump_routes_settings.cached_solution;
    let metrics = &state.bot_jump_routes_settings.cached_metrics;

    assert!(sol.is_reachable);
    assert!(sol.apex_height_uu > 100.0);
    assert!(metrics.is_physically_viable);
    assert_eq!(metrics.self_damage_penalty, 36.0);
    assert!(metrics.min_bot_skill_required >= 4);

    // Test Low-Gravity leap
    state.apply_jump_route_archetype(BotJumpArchetype::PhobosGravityLeap);
    let sol_low_g = &state.bot_jump_routes_settings.cached_solution;
    assert!(sol_low_g.is_reachable);
    assert!(sol_low_g.total_flight_time_secs >= 0.9);
}

#[test]
fn test_jump_route_hazard_detection_and_suicide_prevention() {
    let mut state = EditorState::default();

    // 1. Initial Deck16 hammer setup has self-damage warning
    state.recalculate_jump_route_preview();
    assert!(state.bot_jump_routes_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Self-Damage")));

    // 2. Set target out of physical reach (3000 uu away horizontally, 1000 uu up)
    state.bot_jump_routes_settings.active_config.target_pos = Vec3::new(3000.0, 0.0, 1000.0);
    state.recalculate_jump_route_preview();
    assert!(state.bot_jump_routes_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Unreachable")));

    // 3. Set translocator toss without bAlwaysTranslocate
    state.apply_jump_route_archetype(BotJumpArchetype::FaceSniperTowerClimb);
    state.bot_jump_routes_settings.active_config.b_always_translocate = false;
    state.recalculate_jump_route_preview();
    assert!(state.bot_jump_routes_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("bAlwaysTranslocate")));
}

#[test]
fn test_commit_jump_route_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    state.apply_jump_route_archetype(BotJumpArchetype::FaceSniperTowerClimb);
    let (brush_idx, actor_indices) = state.commit_jump_route_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert_eq!(actor_indices.len(), 3); // PathNode + JumpSpot + TranslocTarget

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors[actor_indices[0]].class_name, "Engine.PathNode");
    assert_eq!(level.actors[actor_indices[1]].class_name, "Botpack.JumpSpot");
    assert_eq!(level.actors[actor_indices[2]].class_name, "Botpack.TranslocTarget");

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
    assert_eq!(level_redone.actors[1].class_name, "Botpack.JumpSpot");
}
