// crates/ut99-editor/tests/test_audit_sniping_and_secret_passages.rs
//
// Master Audit Suite for Tactical Sniping & Secret Passages:
// - Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
// - Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
// - Interactive Secret Door, Concealed Passage & Keyhole Mover Studio

use glam::Vec3;
use ut99_bsp::{
    SecretDoorArchetype, SniperVantageArchetype, SpecialLitArchetype,
};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_cross_system_sniping_secret_passages_and_special_lit_commit() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit SpecialLit Fixture & Light
    state.special_lit_settings.active_config = SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config();
    state.special_lit_settings.active_config.position = Vec3::new(100.0, 100.0, 0.0);
    let (light_brush, light_act) = state.commit_special_lit_to_world();

    // 2. Commit Sniper Vantage Balcony & Weapon
    state.sniper_vantage_settings.active_config = SniperVantageArchetype::HighAltitudeTowerSpire.to_config();
    state.sniper_vantage_settings.active_config.position = Vec3::new(400.0, 100.0, 0.0);
    let (sniper_brush, sniper_act) = state.commit_sniper_vantage_to_world();

    // 3. Commit Secret Door Mover & Trigger
    state.secret_door_settings.active_config = SecretDoorArchetype::FlushWallSlidingPanel.to_config();
    state.secret_door_settings.active_config.position = Vec3::new(700.0, 100.0, 0.0);
    let (door_brush, door_act) = state.commit_secret_door_to_world();

    // Verification of brushes: exactly 3 new brushes added atop initial brushes
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(light_brush, initial_brushes);
    assert_eq!(sniper_brush, initial_brushes + 1);
    assert_eq!(door_brush, initial_brushes + 2);

    // Verification of actors
    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 3);
    assert_eq!(light_act, Some(0));
    assert_eq!(sniper_act, Some(1));
    assert_eq!(door_act, Some(2));

    // Verify class names
    assert_eq!(level.actors[0].class_name, "Engine.Light");
    assert_eq!(level.actors[1].class_name, "Botpack.SniperRifle");
    assert_eq!(level.actors[2].class_name, "Engine.Trigger");
}

#[test]
fn test_cascading_three_stage_multi_undo_redo_purity() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // Stage 1: SpecialLit Light
    state.special_lit_settings.active_config = SpecialLitArchetype::ToxicSlimeGlowIsolator.to_config();
    state.commit_special_lit_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Stage 2: Sniper Vantage
    state.sniper_vantage_settings.active_config = SniperVantageArchetype::HighAltitudeTowerSpire.to_config();
    state.commit_sniper_vantage_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Stage 3: Secret Door
    state.secret_door_settings.active_config = SecretDoorArchetype::FlushWallSlidingPanel.to_config();
    state.commit_secret_door_to_world();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);

    // --- Cascade Undo in Reverse Order ---

    // Undo Stage 3 (Secret Door)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Undo Stage 2 (Sniper Vantage)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Undo Stage 1 (SpecialLit)
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // --- Cascade Redo in Forward Order ---

    // Redo Stage 1 (SpecialLit)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);

    // Redo Stage 2 (Sniper Vantage)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);

    // Redo Stage 3 (Secret Door)
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 3);
}

#[test]
fn test_vantage_sightline_and_secret_passage_kinematics() {
    let mut state = EditorState::default();

    // 1. Sniper Headshot Clearance Kinematics
    state.sniper_vantage_settings.active_config.parapet_height_uu = 44.0;
    state.recalculate_sniper_vantage_preview();
    assert_eq!(state.sniper_vantage_settings.cached_metrics.headshot_clearance_uu, 20.0);

    // 2. Secret Door Transit Timing Kinematics
    state.secret_door_settings.active_config.open_time_secs = 1.2;
    state.secret_door_settings.active_config.stay_open_time_secs = 4.0;
    state.recalculate_secret_door_preview();
    // 1.2 * 2 + 4.0 = 6.4s
    assert!((state.secret_door_settings.cached_metrics.transit_duration_secs - 6.4).abs() < 1e-4);
}
