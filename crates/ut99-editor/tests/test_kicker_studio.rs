// crates/ut99-editor/tests/test_kicker_studio.rs
//
// Integration tests for Interactive Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio.

use glam::Vec3;
use ut99_bsp::{KickerArchetype, KickerHazardSeverity};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_kicker_archetypes_and_trajectory_metrics() {
    let mut state = EditorState::default();

    // Default archetype is FaceTowerPerchLob
    assert_eq!(
        state.kicker_studio_settings.selected_archetype,
        KickerArchetype::FaceTowerPerchLob
    );
    let default_analysis = &state.kicker_studio_settings.trajectory_analysis;
    assert!(default_analysis.apex_position.z > 500.0);
    assert!(default_analysis.total_flight_time_secs > 1.0);
    assert!(default_analysis.horizontal_distance_uu > 300.0);

    // Apply MorpheusSkyscraperAscent (Low-Gravity space leap)
    state.apply_kicker_archetype(KickerArchetype::MorpheusSkyscraperAscent);
    assert_eq!(
        state.kicker_studio_settings.selected_archetype,
        KickerArchetype::MorpheusSkyscraperAscent
    );
    assert_eq!(state.kicker_studio_settings.active_config.gravity_z, -200.0);

    let morpheus_analysis = &state.kicker_studio_settings.trajectory_analysis;
    // Low gravity must result in extended hang time (> 3.0s)
    assert!(
        morpheus_analysis.total_flight_time_secs > 3.0,
        "Low-G flight time was only {:.2}s",
        morpheus_analysis.total_flight_time_secs
    );
    assert!(morpheus_analysis.apex_position.z > 600.0);

    // Apply Deck16SlimeEscapePad
    state.apply_kicker_archetype(KickerArchetype::Deck16SlimeEscapePad);
    assert_eq!(
        state.kicker_studio_settings.selected_archetype,
        KickerArchetype::Deck16SlimeEscapePad
    );
    assert_eq!(state.kicker_studio_settings.active_config.gravity_z, -950.0);
    assert!(state.kicker_studio_settings.trajectory_analysis.total_flight_time_secs < 2.5);
}

#[test]
fn test_inverse_target_solver_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    let initial_vel = state.kicker_studio_settings.active_config.kick_velocity;

    // Set custom target ledge
    state.kicker_studio_settings.active_config.pad_location = Vec3::new(100.0, 100.0, 64.0);
    state.kicker_studio_settings.active_config.target_location = Vec3::new(850.0, 500.0, 320.0);
    state.kicker_studio_settings.target_apex_clearance_z = 200.0;
    state.kicker_studio_settings.active_config.ceiling_height_z = 2500.0;

    // Run inverse target solver
    state.solve_kicker_velocity_to_target();
    let solved_vel = state.kicker_studio_settings.active_config.kick_velocity;
    assert_ne!(initial_vel, solved_vel);
    assert!(solved_vel.z > 0.0);

    // Solved trajectory must hit target platform with < 35 uu discrepancy
    let analysis = &state.kicker_studio_settings.trajectory_analysis;
    assert!(
        analysis.target_miss_distance_uu < 35.0,
        "Target miss distance was {:.1} uu",
        analysis.target_miss_distance_uu
    );

    // Test Undo
    state.undo();
    assert_eq!(
        state.kicker_studio_settings.active_config.kick_velocity,
        initial_vel
    );

    // Test Redo
    state.redo();
    assert_eq!(
        state.kicker_studio_settings.active_config.kick_velocity,
        solved_vel
    );
    assert!(state.kicker_studio_settings.trajectory_analysis.target_miss_distance_uu < 35.0);
}

#[test]
fn test_generate_kicker_and_jumpspot_setup_with_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(ut99_format::UnrLevel::default());

    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();

    state.kicker_studio_settings.active_config.auto_place_jump_spot = true;
    let (kicker_idx, jump_spot_idx) = state.generate_kicker_setup();

    assert_eq!(kicker_idx, initial_actor_count);
    assert_eq!(jump_spot_idx, Some(initial_actor_count + 1));

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actor_count + 2);

    let kicker = &unr.actors[kicker_idx];
    assert_eq!(kicker.class_name, "Kicker");
    assert!(kicker.properties.contains_key("KickVelocity"));

    let jump_spot = &unr.actors[jump_spot_idx.unwrap()];
    assert_eq!(jump_spot.class_name, "JumpSpot");
    assert_eq!(jump_spot.tag, kicker.event);

    let b_special_cost = jump_spot.properties.get("bSpecialCost")
        .and_then(|p| match &p.value {
            ut99_format::unr::tagged_property::TaggedValue::Bool(b) => Some(*b),
            _ => None,
        })
        .unwrap();
    assert!(b_special_cost);

    // Test Undo
    state.undo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count
    );

    // Test Redo
    state.redo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count + 2
    );
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors[kicker_idx].class_name,
        "Kicker"
    );
}

#[test]
fn test_kicker_safety_hazard_audits() {
    let mut state = EditorState::default();

    // 1. Ceiling Concussion Collision
    state.kicker_studio_settings.active_config.ceiling_height_z = 200.0;
    state.recalculate_kicker_trajectory();

    let hazards = &state.kicker_studio_settings.cached_hazards;
    let ceiling_hazard = hazards.iter().find(|h| h.title.contains("Ceiling Concussion"));
    assert!(ceiling_hazard.is_some());
    assert_eq!(ceiling_hazard.unwrap().severity, KickerHazardSeverity::Critical);

    // 2. Lethal Fall Damage
    // Restore high ceiling, but launch very high with target far below
    state.kicker_studio_settings.active_config.ceiling_height_z = 5000.0;
    state.kicker_studio_settings.active_config.pad_location = Vec3::new(0.0, 0.0, 1000.0);
    state.kicker_studio_settings.active_config.kick_velocity = Vec3::new(200.0, 0.0, 1500.0);
    state.kicker_studio_settings.active_config.target_location = Vec3::new(600.0, 0.0, -1000.0);
    state.recalculate_kicker_trajectory();

    let fall_hazards = &state.kicker_studio_settings.cached_hazards;
    let lethal_hazard = fall_hazards.iter().find(|h| h.title.contains("Lethal Landing Impact"));
    assert!(lethal_hazard.is_some());
    assert_eq!(lethal_hazard.unwrap().severity, KickerHazardSeverity::Critical);

    // 3. Missing companion JumpSpot warning
    state.kicker_studio_settings.active_config.auto_place_jump_spot = false;
    state.recalculate_kicker_trajectory();

    let spot_hazards = &state.kicker_studio_settings.cached_hazards;
    let missing_spot = spot_hazards.iter().find(|h| h.title.contains("Missing Companion JumpSpot"));
    assert!(missing_spot.is_some());
    assert_eq!(missing_spot.unwrap().severity, KickerHazardSeverity::Warning);
}
