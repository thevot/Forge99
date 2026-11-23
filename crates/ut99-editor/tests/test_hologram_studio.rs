// crates/ut99-editor/tests/test_hologram_studio.rs
//
// Integration tests for Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio.

use ut99_bsp::{HoloDisplayShape, HologramArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_hologram_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is LiandriCorporationEmblem
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::LiandriCorporationEmblem
    );
    assert_eq!(state.hologram_settings.active_config.display_shape, HoloDisplayShape::PlanarSheet);
    assert_eq!(state.hologram_settings.active_config.display_width, 192.0);
    assert_eq!(state.hologram_settings.active_config.rotation_speed_deg_s, 20.0);

    // Apply TournamentLiveScoreboard
    state.apply_hologram_archetype(HologramArchetype::TournamentLiveScoreboard);
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::TournamentLiveScoreboard
    );
    assert_eq!(state.hologram_settings.active_config.display_shape, HoloDisplayShape::CurvedCylinderArc);
    assert_eq!(state.hologram_settings.active_config.display_width, 384.0);

    // Apply DirectionalWaypointArrow
    state.apply_hologram_archetype(HologramArchetype::DirectionalWaypointArrow);
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::DirectionalWaypointArrow
    );
    assert!(!state.hologram_settings.active_config.has_emitter_pedestal);

    // Apply HazardWarningHoloAlert
    state.apply_hologram_archetype(HologramArchetype::HazardWarningHoloAlert);
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::HazardWarningHoloAlert
    );

    // Apply TacticalTargetingReticle
    state.apply_hologram_archetype(HologramArchetype::TacticalTargetingReticle);
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::TacticalTargetingReticle
    );
    assert_eq!(state.hologram_settings.active_config.display_shape, HoloDisplayShape::VolumetricOctagon);

    // Undo returns to HazardWarningHoloAlert
    state.undo();
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::HazardWarningHoloAlert
    );

    // Redo returns to TacticalTargetingReticle
    state.redo();
    assert_eq!(
        state.hologram_settings.active_config.archetype,
        HologramArchetype::TacticalTargetingReticle
    );
}

#[test]
fn test_hologram_kinematics_and_time_scrubbing() {
    let mut state = EditorState::default();

    state.apply_hologram_archetype(HologramArchetype::LiandriCorporationEmblem);

    let m_init = &state.hologram_settings.cached_metrics;
    assert_eq!(m_init.current_yaw_deg, 0.0);
    assert!(m_init.surface_area_sq_uu > 10000.0);

    // Scrub time to 4.5 seconds (rotation speed = 20 deg/s -> 90 degrees)
    state.scrub_hologram_time(4.5);
    let m_4_5 = &state.hologram_settings.cached_metrics;
    assert!((m_4_5.current_yaw_deg - 90.0).abs() < 0.1);

    // Scrub time to 9.0 seconds (rotation speed = 20 deg/s -> 180 degrees)
    state.scrub_hologram_time(9.0);
    let m_9 = &state.hologram_settings.cached_metrics;
    assert!((m_9.current_yaw_deg - 180.0).abs() < 0.1);
}

#[test]
fn test_commit_hologram_to_world_undo_redo() {
    let mut state = EditorState::default();

    state.apply_hologram_archetype(HologramArchetype::LiandriCorporationEmblem);

    let initial_world_brushes = state.world_brushes.len();
    let initial_actors = state.loaded_unr.as_ref().map(|u| u.actors.len()).unwrap_or(0);

    let (disp_idx, ped_idx, cone_idx, mover_idx, light_idx) = state.commit_hologram_to_world();

    // Check brushes
    assert_eq!(disp_idx, initial_world_brushes);
    assert!(ped_idx.is_some());
    assert!(cone_idx.is_some());
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 3);

    let disp_b = &state.world_brushes[disp_idx];
    assert!(!disp_b.polys.is_empty());
    // Translucent unlit flags
    assert_eq!(disp_b.polys[0].flags & 0x00000004, 0x00000004);
    assert_eq!(disp_b.polys[0].flags & 0x00000002, 0x00000002);

    // Check actors: 1 Mover (rotating) + 1 Light (projector lens)
    assert!(mover_idx.is_some());
    assert!(light_idx.is_some());
    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors.len(), initial_actors + 2);

    let mover_act = &unr.actors[mover_idx.unwrap()];
    assert_eq!(mover_act.class_name, "Engine.Mover");
    assert_eq!(mover_act.tag, "HoloLiandri01");

    let light_act = &unr.actors[light_idx.unwrap()];
    assert_eq!(light_act.class_name, "Engine.Light");

    // Undo removes all brushes and actors
    state.undo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors);

    // Redo restores all brushes and actors
    state.redo();
    assert_eq!(state.world_brushes.len(), initial_world_brushes + 3);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), initial_actors + 2);
}

#[test]
fn test_hologram_safety_audit() {
    let mut state = EditorState::default();

    // 1. Sub-miniature display flags critical hazard
    state.hologram_settings.active_config.display_width = 16.0;
    state.recalculate_hologram_preview();
    let has_mini = state.hologram_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Sub-Miniature"));
    assert!(has_mini, "Must flag critical hazard for display dimensions < 32 uu");

    // 2. Floor clipping flags critical hazard
    state.hologram_settings.active_config.display_width = 192.0;
    state.hologram_settings.active_config.elevation_offset_uu = 20.0; // Half height is 96!
    state.recalculate_hologram_preview();
    let has_clip = state.hologram_settings.cached_hazards.iter().any(|h| h.is_critical && h.title.contains("Floor Clipping"));
    assert!(has_clip, "Must flag critical hazard for floor clipping");

    // 3. Disorienting rotation flags warning
    state.hologram_settings.active_config.elevation_offset_uu = 128.0;
    state.hologram_settings.active_config.rotation_speed_deg_s = 100.0;
    state.recalculate_hologram_preview();
    let has_rot = state.hologram_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Disorienting Hologram Rotation"));
    assert!(has_rot, "Must flag warning for rotation speed > 90 deg/s");

    // 4. Excessive glitch strobe flags warning
    state.hologram_settings.active_config.rotation_speed_deg_s = 20.0;
    state.hologram_settings.active_config.glitch_flicker_probability = 0.30;
    state.recalculate_hologram_preview();
    let has_glitch = state.hologram_settings.cached_hazards.iter().any(|h| !h.is_critical && h.title.contains("Excessive Glitch Strobe"));
    assert!(has_glitch, "Must flag warning for glitch probability > 25%");
}
