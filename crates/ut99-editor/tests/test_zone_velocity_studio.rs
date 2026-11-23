// crates/ut99-editor/tests/test_zone_velocity_studio.rs
//
// Integration tests for Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio.

use glam::Vec3;
use ut99_bsp::{ZoneMediumType, ZoneVelocityArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_zone_velocity_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is HydroelectricRiverCurrent
    assert_eq!(
        state.zone_velocity_settings.active_config.archetype,
        ZoneVelocityArchetype::HydroelectricRiverCurrent
    );
    assert_eq!(state.zone_velocity_settings.active_config.medium, ZoneMediumType::SubmergedWater);
    assert_eq!(state.zone_velocity_settings.active_config.zone_velocity, Vec3::new(400.0, 0.0, 0.0));

    // Apply IndustrialUpdraftWindTunnel
    state.apply_zone_velocity_archetype(ZoneVelocityArchetype::IndustrialUpdraftWindTunnel);
    assert_eq!(
        state.zone_velocity_settings.active_config.archetype,
        ZoneVelocityArchetype::IndustrialUpdraftWindTunnel
    );
    assert_eq!(state.zone_velocity_settings.active_config.zone_velocity.z, 850.0);

    // Apply ToxicSlimeGutterFlume
    state.apply_zone_velocity_archetype(ZoneVelocityArchetype::ToxicSlimeGutterFlume);
    assert_eq!(
        state.zone_velocity_settings.active_config.archetype,
        ZoneVelocityArchetype::ToxicSlimeGutterFlume
    );
    assert!(state.zone_velocity_settings.active_config.is_pain_zone);
    assert_eq!(state.zone_velocity_settings.active_config.damage_per_sec, 20.0);

    // Undo back to IndustrialUpdraftWindTunnel
    state.undo();
    assert_eq!(
        state.zone_velocity_settings.active_config.archetype,
        ZoneVelocityArchetype::IndustrialUpdraftWindTunnel
    );

    // Redo to ToxicSlimeGutterFlume
    state.redo();
    assert_eq!(
        state.zone_velocity_settings.active_config.archetype,
        ZoneVelocityArchetype::ToxicSlimeGutterFlume
    );
}

#[test]
fn test_zone_velocity_metrics_and_buoyancy() {
    let mut state = EditorState::default();

    state.apply_zone_velocity_archetype(ZoneVelocityArchetype::IndustrialUpdraftWindTunnel);
    state.recalculate_zone_velocity_preview();

    let metrics = &state.zone_velocity_settings.cached_metrics;
    assert_eq!(metrics.effective_velocity_magnitude, 850.0);
    assert!((metrics.drift_transit_time_secs - (800.0 / 850.0)).abs() < 1e-4);
    assert!(metrics.buoyancy_rating_pct > 80.0); // +850 uu/s vs 950 gravity
}

#[test]
fn test_zone_velocity_safety_hazard_auditing() {
    let mut state = EditorState::default();

    // 1. Extreme Velocity
    state.zone_velocity_settings.active_config.zone_velocity = Vec3::new(2200.0, 0.0, 0.0);
    state.recalculate_zone_velocity_preview();
    assert!(state.zone_velocity_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Excessive Zone Velocity")));

    // 2. Irreversible Ceiling Updraft
    state.zone_velocity_settings.active_config.zone_velocity = Vec3::new(0.0, 0.0, 1400.0);
    state.zone_velocity_settings.active_config.is_zero_gravity = false;
    state.recalculate_zone_velocity_preview();
    assert!(state.zone_velocity_settings.cached_hazards.iter().any(|h| h.title.contains("Irreversible Ceiling Updraft")));

    // 3. Lethal Pain Trap (transit time 10s, DPS = 50 -> lethal in 2s)
    state.zone_velocity_settings.active_config.zone_length_uu = 1000.0;
    state.zone_velocity_settings.active_config.zone_velocity = Vec3::new(100.0, 0.0, 0.0); // 10s transit
    state.zone_velocity_settings.active_config.is_pain_zone = true;
    state.zone_velocity_settings.active_config.damage_per_sec = 50.0;
    state.recalculate_zone_velocity_preview();
    assert!(state.zone_velocity_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Lethal Pain Trap")));

    // 4. Zero-Friction Ice Skidding
    state.zone_velocity_settings.active_config.fluid_friction = 0.02;
    state.zone_velocity_settings.active_config.zone_velocity = Vec3::new(300.0, 0.0, 0.0);
    state.recalculate_zone_velocity_preview();
    assert!(state.zone_velocity_settings.cached_hazards.iter().any(|h| h.title.contains("Zero-Friction Ice Skidding")));
}

#[test]
fn test_commit_zone_velocity_to_world_and_undo_redo() {
    let mut state = EditorState::default();
    let initial_brushes = state.world_brushes.len();

    // 1. Commit Submerged Water Zone
    state.apply_zone_velocity_archetype(ZoneVelocityArchetype::HydroelectricRiverCurrent);
    let (brush_idx, zone_info_idx) = state.commit_zone_velocity_to_world();

    assert_eq!(state.world_brushes.len(), initial_brushes + 1);
    assert_eq!(brush_idx, initial_brushes);
    assert!(zone_info_idx.is_some());

    let level = state.loaded_unr.as_ref().unwrap();
    assert_eq!(level.actors.len(), 1);
    assert_eq!(level.actors[0].class_name, "Botpack.WaterZone");

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
    assert_eq!(level_redone.actors[0].class_name, "Botpack.WaterZone");
}
