// crates/ut99-editor/tests/test_teleporter_arch_studio.rs
//
// Integration tests for Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio.

use ut99_bsp::TeleporterArchArchetype;
use ut99_editor::editor_state::EditorState;

#[test]
fn test_teleporter_arch_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is GothicMonolithGate
    assert_eq!(
        state.teleporter_arch_settings.active_config.archetype,
        TeleporterArchArchetype::GothicMonolithGate
    );
    assert_eq!(state.teleporter_arch_settings.active_config.arch_width_uu, 128.0);
    assert_eq!(state.teleporter_arch_settings.active_config.exit_boost_velocity, 300.0);

    // Apply IndustrialSludgeDrainTeleport
    state.apply_teleporter_arch_archetype(TeleporterArchArchetype::IndustrialSludgeDrainTeleport);
    assert_eq!(
        state.teleporter_arch_settings.active_config.archetype,
        TeleporterArchArchetype::IndustrialSludgeDrainTeleport
    );
    assert_eq!(state.teleporter_arch_settings.active_config.arch_width_uu, 96.0);

    // Apply CosmicVoidStargate
    state.apply_teleporter_arch_archetype(TeleporterArchArchetype::CosmicVoidStargate);
    assert_eq!(
        state.teleporter_arch_settings.active_config.archetype,
        TeleporterArchArchetype::CosmicVoidStargate
    );
    assert_eq!(state.teleporter_arch_settings.active_config.exit_boost_velocity, 600.0);

    // Undo back to IndustrialSludgeDrainTeleport
    state.undo();
    assert_eq!(
        state.teleporter_arch_settings.active_config.archetype,
        TeleporterArchArchetype::IndustrialSludgeDrainTeleport
    );

    // Redo to CosmicVoidStargate
    state.redo();
    assert_eq!(
        state.teleporter_arch_settings.active_config.archetype,
        TeleporterArchArchetype::CosmicVoidStargate
    );
}

#[test]
fn test_teleporter_arch_metrics_and_aperture() {
    let mut state = EditorState::default();

    state.apply_teleporter_arch_archetype(TeleporterArchArchetype::GothicMonolithGate);
    state.recalculate_teleporter_arch_preview();

    let metrics = &state.teleporter_arch_settings.cached_metrics;
    // Net width = 128 - 2*24 = 80, net height = 160 - 24 = 136
    assert_eq!(metrics.portal_aperture_area_sq_uu, 80.0 * 136.0);
    assert_eq!(metrics.exit_launch_speed, 300.0);
}

#[test]
fn test_teleporter_arch_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Narrow Portal Aperture (< 48 uu clearance)
    state.teleporter_arch_settings.active_config.arch_width_uu = 64.0;
    state.teleporter_arch_settings.active_config.pillar_thickness_uu = 16.0; // 64 - 32 = 32 uu
    state.recalculate_teleporter_arch_preview();
    assert!(state.teleporter_arch_settings.cached_hazards.iter().any(|h| h.title.contains("Restricted Portal Aperture")));

    // 2. High-Velocity Ejection (> 1000 uu/s)
    state.teleporter_arch_settings.active_config.exit_boost_velocity = 1200.0;
    state.teleporter_arch_settings.active_config.b_changes_velocity = true;
    state.recalculate_teleporter_arch_preview();
    assert!(state.teleporter_arch_settings.cached_hazards.iter().any(|h| h.title.contains("High-Velocity Ejection")));
}

#[test]
fn test_commit_teleporter_arch_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Arch & Teleporter Actor to World
    state.apply_teleporter_arch_archetype(TeleporterArchArchetype::GothicMonolithGate);
    let (brush_idx, tele_idx) = state.commit_teleporter_arch_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(tele_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Engine.Teleporter");

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
    assert_eq!(level_redone.actors[0].class_name, "Engine.Teleporter");
}
