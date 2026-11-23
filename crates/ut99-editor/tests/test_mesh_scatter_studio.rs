// crates/ut99-editor/tests/test_mesh_scatter_studio.rs
//
// Integration tests for Interactive Surface Clutter, Procedural Foliage & Decoration Scatter Studio.

use glam::Vec3;
use ut99_bsp::{Brush, CsgOperation, Polygon, ScatterAlignmentMode, ScatterArchetype};
use ut99_editor::editor_state::EditorState;
use ut99_format::{ActorRecord, UnrLevel};

fn make_cube_brush(name: &str, center: Vec3, size: f32) -> Brush {
    let half = size * 0.5;
    let min = center - Vec3::splat(half);
    let max = center + Vec3::splat(half);

    let polys = vec![
        // Top floor (+Z)
        Polygon::new(vec![
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
        ]).unwrap(),
        // Bottom (-Z)
        Polygon::new(vec![
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(min.x, min.y, min.z),
        ]).unwrap(),
        // North (+Y, vertical wall)
        Polygon::new(vec![
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, max.y, min.z),
        ]).unwrap(),
        // South (-Y, vertical wall)
        Polygon::new(vec![
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, min.y, min.z),
        ]).unwrap(),
        // East (+X, vertical wall)
        Polygon::new(vec![
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, max.z),
            Vec3::new(max.x, max.y, min.z),
        ]).unwrap(),
        // West (-X, vertical wall)
        Polygon::new(vec![
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, min.y, min.z),
        ]).unwrap(),
    ];

    let mut b = Brush::new(name, CsgOperation::Add);
    b.polys = polys;
    b
}

#[test]
fn test_scatter_archetypes_and_undo_redo_invariance() {
    let mut state = EditorState::default();

    // Default archetype is OutdoorForestFlora
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::OutdoorForestFlora
    );
    assert_eq!(state.scatter_studio_settings.active_config.alignment_mode, ScatterAlignmentMode::WorldZWithSurfaceTilt);

    // Apply GothicWallTorches
    state.apply_scatter_archetype(ScatterArchetype::GothicWallTorches);
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::GothicWallTorches
    );
    assert_eq!(state.scatter_studio_settings.active_config.alignment_mode, ScatterAlignmentMode::AlignToSurfaceNormal);

    // Apply IndustrialCargoClutter
    state.apply_scatter_archetype(ScatterArchetype::IndustrialCargoClutter);
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::IndustrialCargoClutter
    );

    // Apply DungeonCeilingStalactites
    state.apply_scatter_archetype(ScatterArchetype::DungeonCeilingStalactites);
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::DungeonCeilingStalactites
    );

    // Apply CaveRubbleDebris
    state.apply_scatter_archetype(ScatterArchetype::CaveRubbleDebris);
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::CaveRubbleDebris
    );

    // Undo returns to DungeonCeilingStalactites
    state.undo();
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::DungeonCeilingStalactites
    );

    // Redo returns to CaveRubbleDebris
    state.redo();
    assert_eq!(
        state.scatter_studio_settings.active_config.archetype,
        ScatterArchetype::CaveRubbleDebris
    );
}

#[test]
fn test_poisson_disc_and_slope_filtering_across_world_brushes() {
    let mut state = EditorState::default();

    // Add a wide floor arena brush (512x512)
    let arena = make_cube_brush("ArenaFloor", Vec3::new(256.0, 256.0, 0.0), 512.0);
    state.world_brushes.push(arena);

    // 1. Test OutdoorForestFlora on floor (slopes 0° to 35°)
    state.apply_scatter_archetype(ScatterArchetype::OutdoorForestFlora);
    state.scatter_studio_settings.active_config.target_count = 12;
    state.scatter_studio_settings.active_config.min_spacing_radius = 64.0;
    state.recalculate_scatter_preview();

    let instances = &state.scatter_studio_settings.preview_instances;
    assert!(!instances.is_empty(), "Should generate instances on flat floor");
    assert!(instances.len() <= 12);

    for inst in instances {
        // Floor top polygon has normal Z = +1.0
        assert!(inst.surface_normal.z > 0.9, "Instances must be on top floor face, not side walls");
    }

    // Verify Poisson-disc minimum distance
    for i in 0..instances.len() {
        for j in (i + 1)..instances.len() {
            let dist = instances[i].position.distance(instances[j].position);
            assert!(
                dist >= 63.9,
                "Distance ({:.1}) must be >= min_spacing_radius (64.0)",
                dist
            );
        }
    }

    // 2. Test GothicWallTorches (slopes 75° to 105°)
    state.apply_scatter_archetype(ScatterArchetype::GothicWallTorches);
    state.scatter_studio_settings.active_config.target_count = 8;
    state.scatter_studio_settings.active_config.min_spacing_radius = 64.0;
    state.recalculate_scatter_preview();

    let wall_instances = &state.scatter_studio_settings.preview_instances;
    assert!(!wall_instances.is_empty(), "Should generate torch instances on vertical walls");
    for inst in wall_instances {
        // Vertical walls have normal Z near 0.0
        assert!(
            inst.surface_normal.z.abs() < 0.25,
            "Torch instances must be placed on vertical walls, not floors or ceilings"
        );
    }
}

#[test]
fn test_bot_navigation_clearance_and_safety_audit() {
    let mut state = EditorState::default();
    let arena = make_cube_brush("ArenaFloor", Vec3::new(256.0, 256.0, 0.0), 512.0);
    state.world_brushes.push(arena);

    // Create UnrLevel with a PathNode directly on the floor
    let mut unr = UnrLevel::default();
    let mut path_node = ActorRecord::default();
    path_node.name = "PathNode0".to_string();
    path_node.class_name = "Engine.PathNode".to_string();
    path_node.location = Vec3::new(256.0, 256.0, 256.0); // center of top face
    unr.actors.push(path_node);
    state.loaded_unr = Some(unr);

    state.apply_scatter_archetype(ScatterArchetype::OutdoorForestFlora);
    state.scatter_studio_settings.active_config.target_count = 20;
    state.scatter_studio_settings.active_config.min_spacing_radius = 32.0;
    state.scatter_studio_settings.active_config.bot_clearance_radius = 80.0;
    state.recalculate_scatter_preview();

    // Verify audit detects clearance status
    let total = state.scatter_studio_settings.preview_instances.len();
    assert!(total > 0);

    let blocked = state.scatter_studio_settings.preview_instances.iter().filter(|i| !i.is_valid_clearance).count();
    let valid = state.scatter_studio_settings.preview_instances.iter().filter(|i| i.is_valid_clearance).count();
    assert_eq!(total, valid + blocked);

    // If any instances fell within 80 uu of (256, 256, 256), a critical hazard was reported
    if blocked > 0 {
        let has_crit = state.scatter_studio_settings.cached_hazards.iter().any(|h| h.is_critical);
        assert!(has_crit, "Encroaching on PathNode must trigger critical hazard");
    }

    // Re-rolling seed changes coordinates deterministically
    let first_pos = state.scatter_studio_settings.preview_instances[0].position;
    state.reroll_scatter_seed();
    let second_pos = state.scatter_studio_settings.preview_instances[0].position;
    assert_ne!(first_pos, second_pos, "Seed reroll must produce different deterministic scatter layout");
}

#[test]
fn test_commit_scatter_instances_to_world_actors_with_undo_redo() {
    let mut state = EditorState::default();
    let arena = make_cube_brush("ArenaFloor", Vec3::new(256.0, 256.0, 0.0), 512.0);
    state.world_brushes.push(arena);

    let unr = UnrLevel::default();
    state.loaded_unr = Some(unr);

    state.apply_scatter_archetype(ScatterArchetype::IndustrialCargoClutter);
    state.scatter_studio_settings.active_config.target_count = 6;
    state.scatter_studio_settings.active_config.min_spacing_radius = 48.0;
    state.recalculate_scatter_preview();

    let preview_count = state.scatter_studio_settings.preview_instances.len();
    assert!(preview_count > 0);

    // Commit instances to world
    let initial_actor_count = state.loaded_unr.as_ref().unwrap().actors.len();
    let added_indices = state.commit_scatter_instances_to_world();
    assert_eq!(added_indices.len(), preview_count);
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count + preview_count
    );
    assert!(state.map_is_dirty);
    assert!(state.scatter_studio_settings.preview_instances.is_empty(), "Preview cleared after commit");

    // Verify actor attributes
    for &idx in &added_indices {
        let act = &state.loaded_unr.as_ref().unwrap().actors[idx];
        assert_eq!(act.class_name, "UnrealShare.WoodenBox");
        assert_eq!(act.group, "ScatteredClutter");
    }

    // Undo removes all committed clutter actors
    state.undo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count
    );

    // Redo re-inserts all committed clutter actors
    state.redo();
    assert_eq!(
        state.loaded_unr.as_ref().unwrap().actors.len(),
        initial_actor_count + preview_count
    );
}
