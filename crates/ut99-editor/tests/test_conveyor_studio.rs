// crates/ut99-editor/tests/test_conveyor_studio.rs
//
// Integration tests for Interactive Industrial Conveyor Belt, Treadmill & Surface Velocity Studio.

use ut99_bsp::ConveyorArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_conveyor_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is MiningOreConveyor
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::MiningOreConveyor
    );
    assert_eq!(state.conveyor_settings.active_config.kinematics.speed_uu_per_sec, 256.0);

    // Apply HighSpeedPackageChute
    state.apply_conveyor_archetype(ConveyorArchetype::HighSpeedPackageChute);
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::HighSpeedPackageChute
    );
    assert_eq!(state.conveyor_settings.active_config.kinematics.speed_uu_per_sec, 512.0);

    // Apply SubterraneanSludgeTreadmill
    state.apply_conveyor_archetype(ConveyorArchetype::SubterraneanSludgeTreadmill);
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::SubterraneanSludgeTreadmill
    );
    assert_eq!(state.conveyor_settings.active_config.deck.width, 160.0);

    // Apply AscendingInclineBelt
    state.apply_conveyor_archetype(ConveyorArchetype::AscendingInclineBelt);
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::AscendingInclineBelt
    );
    assert_eq!(state.conveyor_settings.active_config.deck.incline_angle_deg, 20.0);

    // Apply ReversibleSortingTurntable
    state.apply_conveyor_archetype(ConveyorArchetype::ReversibleSortingTurntable);
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::ReversibleSortingTurntable
    );
    assert!(state.conveyor_settings.active_config.kinematics.is_reversible);

    // Undo returns to AscendingInclineBelt
    state.undo();
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::AscendingInclineBelt
    );

    // Redo returns to ReversibleSortingTurntable
    state.redo();
    assert_eq!(
        state.conveyor_settings.active_config.archetype,
        ConveyorArchetype::ReversibleSortingTurntable
    );
}

#[test]
fn test_conveyor_sync_metrics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_conveyor_archetype(ConveyorArchetype::AscendingInclineBelt);

    // Check synchronized metrics: 320 uu/s forward at 20° climb
    let metrics = &state.conveyor_settings.cached_metrics;
    assert_eq!(metrics.texture_pan_rate_pixels_sec, 320.0);
    assert!(metrics.incline_rise_height > 0.0);
    assert!(metrics.roller_rpm > 0.0);

    // Scrub time and verify timeline updates
    state.scrub_conveyor_time(2.5);
    assert_eq!(state.conveyor_settings.simulated_time_seconds, 2.5);
}

#[test]
fn test_commit_conveyor_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_conveyor_archetype(ConveyorArchetype::MiningOreConveyor);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (deck_idx, rollers_idx, frame_idx, zone_idx, waypoint_indices) = state.commit_conveyor_to_world();

    assert_eq!(deck_idx, initial_world_brushes);
    assert_eq!(rollers_idx, initial_world_brushes + 1);
    assert_eq!(frame_idx, initial_world_brushes + 2);
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 3);

    // Deck brush: 6 polys, top has PF_TEX_PAN (0x08) and PF_SEMISOLID (0x200000)
    let deck_brush = &state.world_brushes[deck_idx];
    assert_eq!(deck_brush.polys.len(), 6);
    assert_eq!(deck_brush.polys[0].flags & 0x00000008, 0x00000008);
    assert_eq!(deck_brush.polys[0].flags & 0x00200000, 0x00200000);

    // Rollers & Frame brushes exist
    assert!(!state.world_brushes[rollers_idx].polys.is_empty());
    assert!(!state.world_brushes[frame_idx].polys.is_empty());

    // Actors created: ZoneInfo + 2 PathNodes = 3 actors
    assert!(zone_idx.is_some());
    assert_eq!(waypoint_indices.len(), 2);

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 3);

    let zone_actor = &unr.actors[zone_idx.unwrap()];
    assert_eq!(zone_actor.class_name, "Engine.ZoneInfo");

    let in_node = &unr.actors[waypoint_indices[0]];
    assert_eq!(in_node.class_name, "Engine.PathNode");

    let out_node = &unr.actors[waypoint_indices[1]];
    assert_eq!(out_node.class_name, "Engine.PathNode");

    // Undo removes all 3 brushes and all 3 actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores all 3 brushes and all 3 actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 3);
}

#[test]
fn test_conveyor_safety_audit() {
    let mut state = EditorState::default();

    // 1. Extreme velocity (>1000 uu/s) flags critical hazard
    state.conveyor_settings.active_config.kinematics.speed_uu_per_sec = 1200.0;
    state.recalculate_conveyor_preview();
    let has_excess = state.conveyor_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive"));
    assert!(has_excess, "Must flag critical hazard for speed > 1000 uu/s");

    // 2. Steep incline (>30°) flags warning
    state.conveyor_settings.active_config.kinematics.speed_uu_per_sec = 256.0;
    state.conveyor_settings.active_config.deck.incline_angle_deg = 35.0;
    state.recalculate_conveyor_preview();
    let has_steep = state.conveyor_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Steep Incline"));
    assert!(has_steep, "Must flag warning for incline > 30°");

    // 3. Narrow width (<64 uu) flags critical hazard
    state.conveyor_settings.active_config.deck.incline_angle_deg = 0.0;
    state.conveyor_settings.active_config.deck.width = 48.0;
    state.recalculate_conveyor_preview();
    let has_narrow = state.conveyor_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Clearance"));
    assert!(has_narrow, "Must flag critical hazard for width < 64 uu");

    // 4. Missing end rollers flags warning
    state.conveyor_settings.active_config.deck.width = 128.0;
    state.conveyor_settings.active_config.rollers.has_end_rollers = false;
    state.recalculate_conveyor_preview();
    let has_no_rollers = state.conveyor_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Missing Intake"));
    assert!(has_no_rollers, "Must flag warning for missing end rollers");
}
