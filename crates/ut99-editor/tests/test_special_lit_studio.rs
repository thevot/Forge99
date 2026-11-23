// crates/ut99-editor/tests/test_special_lit_studio.rs
//
// Integration tests for Interactive Special Lit, Zone Lighting Mask & Light Filter Studio.

use ut99_bsp::SpecialLitArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_special_lit_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is ToxicSlimeGlowIsolator
    assert_eq!(
        state.special_lit_settings.active_config.archetype,
        SpecialLitArchetype::ToxicSlimeGlowIsolator
    );
    assert_eq!(state.special_lit_settings.active_config.light_hue, 85);
    assert_eq!(state.special_lit_settings.active_config.light_brightness, 180);

    // Apply CelestialBackdropIlluminator
    state.apply_special_lit_archetype(SpecialLitArchetype::CelestialBackdropIlluminator);
    assert_eq!(
        state.special_lit_settings.active_config.archetype,
        SpecialLitArchetype::CelestialBackdropIlluminator
    );
    assert_eq!(state.special_lit_settings.active_config.light_radius_uu, 1200.0);

    // Apply TeamBaseAccentSpot
    state.apply_special_lit_archetype(SpecialLitArchetype::TeamBaseAccentSpot);
    assert_eq!(
        state.special_lit_settings.active_config.archetype,
        SpecialLitArchetype::TeamBaseAccentSpot
    );
    assert_eq!(state.special_lit_settings.active_config.light_brightness, 200);

    // Undo back to CelestialBackdropIlluminator
    state.undo();
    assert_eq!(
        state.special_lit_settings.active_config.archetype,
        SpecialLitArchetype::CelestialBackdropIlluminator
    );

    // Redo to TeamBaseAccentSpot
    state.redo();
    assert_eq!(
        state.special_lit_settings.active_config.archetype,
        SpecialLitArchetype::TeamBaseAccentSpot
    );
}

#[test]
fn test_special_lit_metrics_and_isolation() {
    let mut state = EditorState::default();

    state.apply_special_lit_archetype(SpecialLitArchetype::ToxicSlimeGlowIsolator);
    state.recalculate_special_lit_preview();

    let metrics = &state.special_lit_settings.cached_metrics;
    assert!(metrics.effective_illumination_flux > 0.0);
}

#[test]
fn test_special_lit_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Extreme Brightness Clipping
    state.special_lit_settings.active_config.light_brightness = 245;
    state.recalculate_special_lit_preview();
    assert!(state.special_lit_settings.cached_hazards.iter().any(|h| h.title.contains("Over-Saturated Light Brightness")));

    // 2. Excessive Radius for Indoor Accent
    state.special_lit_settings.active_config.light_radius_uu = 1100.0;
    state.recalculate_special_lit_preview();
    assert!(state.special_lit_settings.cached_hazards.iter().any(|h| h.title.contains("Excessive Light Radius")));
}

#[test]
fn test_commit_special_lit_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit SpecialLit Light to World
    state.apply_special_lit_archetype(SpecialLitArchetype::ToxicSlimeGlowIsolator);
    let (brush_idx, light_idx) = state.commit_special_lit_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(light_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Light");

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
