use glam::Vec3;
use ut99_bsp::{build_cube, Brush, CsgOperation, ProjectorBlendMode};
use ut99_editor::editor_state::{EditorMover, EditorState};
use ut99_editor::raycast::Ray;

fn create_cube_brush(name: &str, loc: Vec3, size: f32, tex: &str) -> Brush {
    let mut b = build_cube(name, CsgOperation::Add, Vec3::splat(size));
    b.location = loc;
    for poly in &mut b.polys {
        poly.texture = tex.to_string();
    }
    b
}

#[test]
fn test_eyedropper_picks_from_brush_mover_and_static_mesh() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. World brush at X = 200 with texture "WallTex"
    let b1 = create_cube_brush("WallBrush", Vec3::new(200.0, 0.0, 0.0), 64.0, "WallTex");
    state.world_brushes.push(b1);

    // 2. Mover at Y = 200 with texture "MoverDoorTex"
    let b2 = create_cube_brush("MoverBrush", Vec3::new(0.0, 200.0, 0.0), 64.0, "MoverDoorTex");
    let mover = EditorMover::new(b2);
    state.movers.push(mover);

    // 3. Static mesh at Z = 200 with texture "PillarMeshTex"
    let b3 = create_cube_brush("PillarMesh", Vec3::new(0.0, 0.0, 200.0), 64.0, "PillarMeshTex");
    state.static_meshes.push(b3);

    // Test A: Ray pointing towards world brush (+X)
    let ray_x = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));
    let picked_x = state.pick_texture_at_ray(ray_x);
    assert_eq!(picked_x, Some("WallTex".to_string()));
    assert_eq!(state.active_texture, Some("WallTex".to_string()));

    // Test B: Ray pointing towards mover (+Y)
    let ray_y = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0));
    let picked_y = state.pick_texture_at_ray(ray_y);
    assert_eq!(picked_y, Some("MoverDoorTex".to_string()));
    assert_eq!(state.active_texture, Some("MoverDoorTex".to_string()));

    // Test C: Ray pointing towards static mesh (+Z)
    let ray_z = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    let picked_z = state.pick_texture_at_ray(ray_z);
    assert_eq!(picked_z, Some("PillarMeshTex".to_string()));
    assert_eq!(state.active_texture, Some("PillarMeshTex".to_string()));
}

#[test]
fn test_apply_material_single_and_batch_with_undo_redo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    let b1 = create_cube_brush("FloorBrush", Vec3::new(200.0, 0.0, 0.0), 128.0, "OldStone");
    state.world_brushes.push(b1);

    state.active_texture = Some("NewMarble".to_string());
    let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));

    // 1. Single face paint
    let applied = state.apply_active_texture_at_ray(ray);
    assert!(applied, "Must successfully apply active texture");
    assert!(state.history.can_undo(), "Action must be recorded in history");

    // Check that at least one face received "NewMarble"
    let count_marble = state.world_brushes[0].polys.iter().filter(|p| p.texture == "NewMarble").count();
    assert_eq!(count_marble, 1, "Exactly one face was hit and painted");

    // 2. Undo
    state.undo();
    let count_old = state.world_brushes[0].polys.iter().filter(|p| p.texture == "OldStone").count();
    assert_eq!(count_old, state.world_brushes[0].polys.len(), "Undo must restore OldStone on all faces");

    // 3. Redo
    state.redo();
    let count_marble_redo = state.world_brushes[0].polys.iter().filter(|p| p.texture == "NewMarble").count();
    assert_eq!(count_marble_redo, 1, "Redo must re-apply NewMarble");

    // 4. Batch application to multiple selected surfaces
    state.selected_surfaces.clear();
    state.selected_surfaces.insert((0, 0));
    state.selected_surfaces.insert((0, 1));
    state.selected_surfaces.insert((0, 2));

    state.active_texture = Some("CyberMetal".to_string());
    let batch_applied = state.apply_active_texture_at_ray(ray);
    assert!(batch_applied, "Must apply batch to selected surfaces");

    assert_eq!(state.world_brushes[0].polys[0].texture, "CyberMetal");
    assert_eq!(state.world_brushes[0].polys[1].texture, "CyberMetal");
    assert_eq!(state.world_brushes[0].polys[2].texture, "CyberMetal");

    // 5. Batch Undo
    state.undo();
    assert_ne!(state.world_brushes[0].polys[0].texture, "CyberMetal", "Batch undo must revert all 3 faces");

    // 6. Batch Redo
    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].texture, "CyberMetal", "Batch redo must restore CyberMetal");
}

#[test]
fn test_stamp_decal_at_ray_and_undo_redo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Wall at X = 200 facing -X
    let b1 = create_cube_brush("TargetWall", Vec3::new(200.0, 0.0, 0.0), 128.0, "Concrete");
    state.world_brushes.push(b1);

    state.active_texture = Some("Botpack.BloodSplat".to_string());
    state.decal_width = 64.0;
    state.decal_height = 64.0;
    state.decal_blend_mode = ProjectorBlendMode::Modulated;

    // Ray from origin towards wall (+X)
    let ray = Ray::new(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0));

    let stamped_idx = state.stamp_decal_at_ray(ray);
    assert_eq!(stamped_idx, Some(0), "Projector must be placed and indexed");
    assert_eq!(state.projectors.len(), 1, "Exactly one projector added");
    assert!(state.active_projector_decals.len() > 0, "Decals must be baked immediately onto the wall");

    let proj = &state.projectors[0];
    assert_eq!(proj.texture, "Botpack.BloodSplat");
    assert_eq!(proj.width, 64.0);
    assert_eq!(proj.height, 64.0);
    assert_eq!(proj.blend_mode, ProjectorBlendMode::Modulated);

    // Verify orientation: facing towards wall (+X)
    let (fwd, _, _) = proj.get_axes();
    assert!(fwd.dot(Vec3::new(1.0, 0.0, 0.0)) > 0.95, "Projector must point towards the wall surface");

    // Test Undo: removes stamped projector and clears baked fragments
    assert!(state.history.can_undo());
    state.undo();
    assert_eq!(state.projectors.len(), 0, "Undo must remove stamped projector");
    assert_eq!(state.active_projector_decals.len(), 0, "Undo must clear baked decal fragments");

    // Test Redo: restores projector and re-bakes decal fragments
    assert!(state.history.can_redo());
    state.redo();
    assert_eq!(state.projectors.len(), 1, "Redo must restore projector");
    assert!(state.active_projector_decals.len() > 0, "Redo must re-bake decal fragments");

    // Test removal and clearing
    state.selected_projector_index = Some(0);
    let removed = state.remove_selected_projector();
    assert!(removed, "Must remove selected projector");
    assert_eq!(state.projectors.len(), 0);

    // Test Clear All
    state.stamp_decal_at_ray(ray);
    assert_eq!(state.projectors.len(), 1);
    state.clear_all_decals();
    assert_eq!(state.projectors.len(), 0);
    assert_eq!(state.active_projector_decals.len(), 0);
}
