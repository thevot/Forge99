// crates/ut99-editor/tests/test_shape_loft_studio.rs
//
// Integration tests for Interactive 2D Shape Loft, Curved Pipe & Architectural Molding Studio.

use glam::Vec3;
use ut99_bsp::{CsgOperation, LoftPathKeyframe, LoftProfileArchetype};
use ut99_editor::editor_state::EditorState;

#[test]
fn test_loft_archetypes_and_profile_transitions() {
    let mut state = EditorState::default();

    // Default archetype is IndustrialRoundPipe
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::IndustrialRoundPipe
    );
    assert!(state.loft_studio_settings.cached_preview_brush.is_some());
    let default_polys = state.loft_studio_settings.cached_preview_brush.as_ref().unwrap().polys.len();
    assert!(default_polys > 0);

    // Apply GothicVaultRib
    state.apply_loft_archetype(LoftProfileArchetype::GothicVaultRib);
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::GothicVaultRib
    );
    assert!(state.loft_studio_settings.custom_profile.points.len() >= 6);

    // Apply RomanOgeeMolding
    state.apply_loft_archetype(LoftProfileArchetype::RomanOgeeMolding);
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::RomanOgeeMolding
    );

    // Apply HollowTunnelArch
    state.apply_loft_archetype(LoftProfileArchetype::HollowTunnelArch);
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::HollowTunnelArch
    );

    // Undo returns to RomanOgeeMolding
    state.undo();
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::RomanOgeeMolding
    );

    // Redo re-applies HollowTunnelArch
    state.redo();
    assert_eq!(
        state.loft_studio_settings.active_config.archetype,
        LoftProfileArchetype::HollowTunnelArch
    );
}

#[test]
fn test_curved_pipe_rmf_frames_and_hazard_audit() {
    let mut state = EditorState::default();

    // Smooth 90-degree bend over 512 uu
    state.loft_studio_settings.path_keyframes = vec![
        LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
        LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 256.0)),
        LoftPathKeyframe::new(Vec3::new(256.0, 0.0, 512.0)),
    ];
    state.loft_studio_settings.active_config.tessellation_steps = 12;
    state.recalculate_loft_mesh();

    assert_eq!(state.loft_studio_settings.cached_frames.len(), 12);
    // Smooth bend should produce zero critical self-intersection hazards
    let critical_hazards = state.loft_studio_settings.cached_hazards.iter().filter(|h| h.is_critical).count();
    assert_eq!(critical_hazards, 0, "Smooth 90 degree curve should have 0 critical hazards");

    // Introduce sharp pinch hairpin bend (180 degree snap over 16 uu)
    state.loft_studio_settings.path_keyframes = vec![
        LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
        LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 16.0)),
        LoftPathKeyframe::new(Vec3::new(0.0, 0.0, 0.0)),
    ];
    state.recalculate_loft_mesh();
    assert!(!state.loft_studio_settings.cached_hazards.is_empty(), "Sharp hairpin bend must trigger hazard audit");
}

#[test]
fn test_hollow_tunnel_conduit_mesh_generation() {
    let mut state = EditorState::default();

    state.apply_loft_archetype(LoftProfileArchetype::IndustrialRoundPipe);
    state.loft_studio_settings.active_config.tessellation_steps = 8;

    // Solid brush poly count
    state.loft_studio_settings.active_config.is_hollow = false;
    state.recalculate_loft_mesh();
    let solid_count = state.loft_studio_settings.cached_preview_brush.as_ref().unwrap().polys.len();

    // Hollow conduit brush poly count (outer walls + inner walls + start rim + end rim)
    state.loft_studio_settings.active_config.is_hollow = true;
    state.loft_studio_settings.active_config.wall_thickness = 6.0;
    state.recalculate_loft_mesh();
    let hollow_count = state.loft_studio_settings.cached_preview_brush.as_ref().unwrap().polys.len();

    assert!(hollow_count > solid_count, "Hollow conduit must contain additional inner walls and rim caps");
}

#[test]
fn test_commit_loft_to_builder_and_world_with_undo_redo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    state.apply_loft_archetype(LoftProfileArchetype::IndustrialRoundPipe);
    state.loft_studio_settings.active_config.tessellation_steps = 6;
    state.recalculate_loft_mesh();

    // 1. Commit to Builder Brush
    let initial_builder_polys = state.builder_brush.polys.len();
    state.commit_loft_to_builder_brush();
    assert_ne!(state.builder_brush.polys.len(), initial_builder_polys);
    assert_eq!(state.builder_brush.csg_oper, CsgOperation::Active);

    // Undo builder commit
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), initial_builder_polys);

    // Redo builder commit
    state.redo();
    assert_ne!(state.builder_brush.polys.len(), initial_builder_polys);

    // 2. Commit as Additive World Brush
    let brush_idx = state.commit_loft_to_world_brush(CsgOperation::Add);
    assert_eq!(state.world_brushes.len(), 1);
    assert_eq!(state.world_brushes[brush_idx].csg_oper, CsgOperation::Add);
    assert!(state.world_brushes[brush_idx].polys.len() > 10);

    // Undo world commit
    state.undo();
    assert_eq!(state.world_brushes.len(), 0, "Undo must remove added world brush");

    // Redo world commit
    state.redo();
    assert_eq!(state.world_brushes.len(), 1, "Redo must restore added world brush");
}
