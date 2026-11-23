// crates/ut99-editor/tests/test_team_objectives_studio.rs
//
// Integration tests for Interactive Team Base Objective, CTF Flag Run Route & Defense AI Studio.

use glam::Vec3;
use ut99_editor::editor_state::EditorState;
use ut99_bsp::{
    DefenseAuditSeverity, GameObjectiveType, TeamColor, TeamObjectivePreset,
};

#[test]
fn test_ctf_routes_and_asymmetry_analysis() {
    let mut state = EditorState::default();
    state.scan_and_refresh_team_objectives();

    // Default synthesis should provide Red and Blue flags
    let objs = &state.team_objective_settings.detected_objectives;
    assert_eq!(objs.len(), 2);
    assert_eq!(objs[0].team, Some(TeamColor::Red));
    assert_eq!(objs[1].team, Some(TeamColor::Blue));

    // Routes should be computed
    let routes = &state.team_objective_settings.routes;
    assert_eq!(routes.len(), 3);
    assert!(routes[0].is_primary);
    assert_eq!(routes[0].total_distance_uu, 3000.0);
    assert_eq!(routes[0].estimated_run_time_secs, 7.5);

    // Asymmetry analysis should exist and be fair
    let asym = state.team_objective_settings.asymmetry.as_ref().unwrap();
    assert!(asym.is_competitively_fair);
    assert_eq!(asym.delta_time_secs, 0.0);
    assert_eq!(asym.asymmetry_percentage, 0.0);
}

#[test]
fn test_defense_points_deployment_and_undo_redo_invariance() {
    let mut state = EditorState::default();
    let mut unr = ut99_format::UnrLevel::default();

    // Pre-populate with a RedFlagBase actor
    let mut flag = ut99_format::ActorRecord::default();
    flag.class_name = "RedFlagBase".to_string();
    flag.name = "RedFlagBase0".to_string();
    flag.location = Vec3::new(-1200.0, 0.0, 64.0);
    unr.actors.push(flag);

    state.loaded_unr = Some(unr);
    state.scan_and_refresh_team_objectives();

    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(initial_actor_count, 1);

    // Deploy 3 coordinated defense points
    let count = state.generate_defense_points_for_active_objective();
    assert_eq!(count, 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count + 3);

    // Check newly added defense points
    let unr_after = state.loaded_unr.as_ref().unwrap();
    let dp_sniper = &unr_after.actors[1];
    assert_eq!(dp_sniper.class_name, "DefensePoint");
    assert!(dp_sniper.name.contains("SniperPerch"));

    let b_sniping = dp_sniper.properties.get("bSniping")
        .and_then(|p| match &p.value {
            ut99_format::unr::tagged_property::TaggedValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap();
    assert!(b_sniping);

    // Test Undo
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count);

    // Test Redo
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actor_count + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[1].class_name, "DefensePoint");
}

#[test]
fn test_domination_triangle_and_preset_transitions() {
    let mut state = EditorState::default();

    // Apply Domination preset
    state.apply_team_objective_preset(TeamObjectivePreset::CrypticEquilateralDom);
    assert_eq!(state.team_objective_settings.active_game_mode, GameObjectiveType::Domination);
    assert_eq!(state.team_objective_settings.selected_preset, Some(TeamObjectivePreset::CrypticEquilateralDom));

    // Domination triangle analysis must be active
    let tri = state.team_objective_settings.dom_triangle.as_ref().unwrap();
    assert!(tri.triangle_balance_factor > 0.90);
    assert_eq!(tri.point_a_name, "ControlPoint_Alpha");
    assert_eq!(tri.point_b_name, "ControlPoint_Beta");
    assert_eq!(tri.point_c_name, "ControlPoint_Gamma");

    // Test Undo/Redo on preset
    state.undo();
    assert_eq!(state.team_objective_settings.active_game_mode, GameObjectiveType::CaptureTheFlag);

    state.redo();
    assert_eq!(state.team_objective_settings.active_game_mode, GameObjectiveType::Domination);
    assert!(state.team_objective_settings.dom_triangle.is_some());
}

#[test]
fn test_team_objectives_audit_diagnostics() {
    let mut state = EditorState::default();

    // Only Red flag base in level
    let mut unr = ut99_format::UnrLevel::default();
    let mut flag = ut99_format::ActorRecord::default();
    flag.class_name = "RedFlagBase".to_string();
    flag.name = "RedFlagBase0".to_string();
    flag.location = Vec3::new(-1000.0, 0.0, 0.0);
    unr.actors.push(flag);

    state.loaded_unr = Some(unr);
    state.scan_and_refresh_team_objectives();

    let issues = &state.team_objective_settings.audit_issues;
    assert!(issues.iter().any(|i| i.severity == DefenseAuditSeverity::Critical && i.title.contains("Missing Blue FlagBase")));
}
