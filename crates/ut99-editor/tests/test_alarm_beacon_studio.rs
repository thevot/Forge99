// crates/ut99-editor/tests/test_alarm_beacon_studio.rs
//
// Integration tests for Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio.

use ut99_bsp::AlarmBeaconArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_alarm_beacon_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is NuclearReactorBreachStrobe
    assert_eq!(
        state.alarm_beacon_settings.active_config.archetype,
        AlarmBeaconArchetype::NuclearReactorBreachStrobe
    );
    assert_eq!(state.alarm_beacon_settings.active_config.light_hue, 0); // Red
    assert_eq!(state.alarm_beacon_settings.active_config.strobe_frequency_hz, 6.0);

    // Apply PerimeterSecurityRevolvingSiren
    state.apply_alarm_beacon_archetype(AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren);
    assert_eq!(
        state.alarm_beacon_settings.active_config.archetype,
        AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren
    );
    assert_eq!(state.alarm_beacon_settings.active_config.rotor_rpm, 120.0);
    assert_eq!(state.alarm_beacon_settings.active_config.light_hue, 28); // Amber

    // Apply HazardDecompressionPulse
    state.apply_alarm_beacon_archetype(AlarmBeaconArchetype::HazardDecompressionPulse);
    assert_eq!(
        state.alarm_beacon_settings.active_config.archetype,
        AlarmBeaconArchetype::HazardDecompressionPulse
    );
    assert_eq!(state.alarm_beacon_settings.active_config.light_hue, 140); // Cyan

    // Undo back to PerimeterSecurityRevolvingSiren
    state.undo();
    assert_eq!(
        state.alarm_beacon_settings.active_config.archetype,
        AlarmBeaconArchetype::PerimeterSecurityRevolvingSiren
    );

    // Redo to HazardDecompressionPulse
    state.redo();
    assert_eq!(
        state.alarm_beacon_settings.active_config.archetype,
        AlarmBeaconArchetype::HazardDecompressionPulse
    );
}

#[test]
fn test_alarm_beacon_metrics_and_audible_radius() {
    let mut state = EditorState::default();

    state.apply_alarm_beacon_archetype(AlarmBeaconArchetype::NuclearReactorBreachStrobe);
    state.recalculate_alarm_beacon_preview();

    let metrics = &state.alarm_beacon_settings.cached_metrics;
    assert_eq!(metrics.photometric_reach_uu, 320.0 * (240.0 / 255.0));
    assert_eq!(metrics.acoustic_audible_radius_uu, 90.0 * 20.0); // 1800 uu
    assert_eq!(metrics.flash_cycle_duration_secs, 1.0 / 6.0);
}

#[test]
fn test_alarm_beacon_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. High frequency strobe epilepsy risk (> 12 Hz)
    state.alarm_beacon_settings.active_config.strobe_frequency_hz = 18.0;
    state.recalculate_alarm_beacon_preview();
    assert!(state.alarm_beacon_settings.cached_hazards.iter().any(|h| h.title.contains("Stroboscopic Photosensitivity Risk")));

    // 2. Unlinked event
    state.alarm_beacon_settings.active_config.event = "".to_string();
    state.recalculate_alarm_beacon_preview();
    assert!(state.alarm_beacon_settings.cached_hazards.iter().any(|h| h.title.contains("Unlinked Alarm Activation Event")));
}

#[test]
fn test_commit_alarm_beacon_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Beacon Brush & Light Actor to World
    state.apply_alarm_beacon_archetype(AlarmBeaconArchetype::NuclearReactorBreachStrobe);
    let (brush_idx, light_idx) = state.commit_alarm_beacon_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(light_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Light");
    assert_eq!(level.actors[0].light_effect, 3); // LE_Strobe

    // 2. Undo Commit
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    let level_undone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_undone.actors.len(), 0);

    // 3. Redo Commit
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    let level_redone = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level_redone.actors.len(), 1);
    assert_eq!(level_redone.actors[0].class_name, "Engine.Light");
}
