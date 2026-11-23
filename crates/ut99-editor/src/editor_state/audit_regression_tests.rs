    use super::*;
    use glam::Vec3;

    #[test]
    fn test_autosave_snapshot_lossless_roundtrip_and_dirty_flag() {
        let mut state = EditorState::default();
        state.new_map();

        // Customize a world brush with non-default texture, UVs, and surface flags
        let mut custom_brush = state.builder_brush.clone();
        custom_brush.csg_oper = CsgOperation::Add;
        custom_brush.location = Vec3::new(320.0, -160.0, 64.0);
        for poly in &mut custom_brush.polys {
            poly.texture = "Indus5.Walls.Wall01".to_string();
            poly.texture_u = Vec3::new(2.0, 0.0, 0.0);
            poly.texture_v = Vec3::new(0.0, 0.0, -2.0);
            poly.flags = ut99_bsp::PF_SEMISOLID;
        }
        state.world_brushes.push(custom_brush);

        // Add a Mover with keyframes
        state.add_mover_from_builder();
        assert_eq!(state.movers.len(), 1);
        state.movers[0].keyframes[1] = Some(MoverKeyframe {
            location: Vec3::new(0.0, 0.0, 256.0),
            rotation: Vec3::new(0.0, 90.0, 0.0),
        });

        // Place actors of multiple categories
        state.place_actor_of_class_at("Light", Vec3::new(100.0, 100.0, 100.0));
        state.place_actor_of_class_at("ShockRifle", Vec3::new(200.0, 200.0, 32.0));
        state.map_is_dirty = true;

        let brush_count_before = state.world_brushes.len();

        // Trigger auto-save: must NOT clear map_is_dirty
        state.trigger_auto_save();
        assert!(
            state.map_is_dirty,
            "Auto-save must not clear map_is_dirty so unsaved changes prompt remains active"
        );
        assert_eq!(state.auto_save_snapshots.len(), 1);

        // Wipe the map and restore from the auto-save snapshot
        state.world_brushes.clear();
        state.movers.clear();
        state.loaded_unr = None;

        state
            .restore_auto_save_snapshot(0)
            .expect("restore_auto_save_snapshot must succeed");
        assert_eq!(state.world_brushes.len(), brush_count_before);

        let restored_brush = state.world_brushes.last().unwrap();
        assert_eq!(restored_brush.polys[0].texture, "Indus5.Walls.Wall01");
        assert_eq!(restored_brush.polys[0].flags, ut99_bsp::PF_SEMISOLID);
        assert!((restored_brush.polys[0].texture_u.x - 2.0).abs() < 1e-3);

        assert_eq!(state.movers.len(), 1);
        let kf1 = state.movers[0].keyframes[1].as_ref().expect("Keyframe 1 must be restored");
        assert!((kf1.location.z - 256.0).abs() < 1e-3);

        let unr = state.loaded_unr.as_ref().expect("loaded_unr must be restored");
        assert!(unr.lights.iter().any(|a| a.class_name.eq_ignore_ascii_case("Light")));
        assert!(unr
            .weapons_and_pickups
            .iter()
            .any(|a| a.class_name.eq_ignore_ascii_case("ShockRifle")));
    }

    #[test]
    fn test_actor_sync_views_across_place_delete_undo_redo() {
        let mut state = EditorState::default();
        state.new_map();

        let initial_lights = state.loaded_unr.as_ref().map(|u| u.lights.len()).unwrap_or(0);
        let initial_starts = state.loaded_unr.as_ref().map(|u| u.player_starts.len()).unwrap_or(0);

        state.place_actor_of_class_at("Light", Vec3::new(64.0, 64.0, 64.0));
        state.place_actor_of_class_at("PlayerStart", Vec3::new(-64.0, -64.0, 64.0));
        state.place_actor_of_class_at("PathNode", Vec3::new(128.0, 0.0, 64.0));
        state.place_actor_of_class_at("ShockRifle", Vec3::new(0.0, 128.0, 64.0));

        {
            let unr = state.loaded_unr.as_ref().unwrap();
            assert_eq!(unr.lights.len(), initial_lights + 1);
            assert_eq!(unr.player_starts.len(), initial_starts + 1);
            assert_eq!(unr.path_nodes.len(), 1);
            assert_eq!(unr.weapons_and_pickups.len(), 1);
        }

        // Undo ShockRifle and PathNode placement
        state.undo();
        state.undo();
        {
            let unr = state.loaded_unr.as_ref().unwrap();
            assert_eq!(unr.path_nodes.len(), 0);
            assert_eq!(unr.weapons_and_pickups.len(), 0);
            assert_eq!(unr.lights.len(), initial_lights + 1);
            assert_eq!(unr.player_starts.len(), initial_starts + 1);
        }

        // Redo PathNode and ShockRifle placement
        state.redo();
        state.redo();
        {
            let unr = state.loaded_unr.as_ref().unwrap();
            assert_eq!(unr.path_nodes.len(), 1);
            assert_eq!(unr.weapons_and_pickups.len(), 1);
        }
    }

    #[test]
    fn test_studio_movers_register_and_compile_populated_unr_models() {
        let mut state = EditorState::default();
        state.new_map();

        let initial_movers = state.movers.len();
        state.commit_elevator_to_world();
        state.commit_piston_to_world();
        assert_eq!(
            state.movers.len(),
            initial_movers + 2,
            "Studio mover generators must register full EditorMover instances in state.movers"
        );

        let temp_path = std::env::temp_dir().join("forge99_test_studio_movers.unr");
        crate::compiler::CompilerBridge::save_unr_native(&state, &temp_path)
            .expect("save_unr_native must succeed");

        let pkg = ut99_format::Package::open(&temp_path)
            .expect("Saved .unr must be a valid UE1 package");

        // Verify that the two Studio Movers have Model_{i} exports with populated compiled BSP (Nodes, Surfs, Verts, Vectors, Points)
        let mut populated_mover_models = 0usize;
        for e in &pkg.exports.entries {
            let name = pkg.get_name(e.object_name as usize).unwrap_or("");
            if name.starts_with("Model_") {
                let start = e.serial_offset as usize;
                let end = start + (e.serial_size as usize);
                if let Some((bsp, polys_ref)) = ut99_format::unr::level::parse_compiled_umodel_bsp(
                    &pkg.raw_data[start..end],
                    e.object_flags,
                    &pkg,
                ) {
                    if !bsp.nodes.is_empty()
                        && !bsp.surfs.is_empty()
                        && !bsp.verts.is_empty()
                        && !bsp.vectors.is_empty()
                        && !bsp.points.is_empty()
                        && polys_ref != 0
                    {
                        populated_mover_models += 1;
                    }
                }
            }
        }
        assert!(
            populated_mover_models >= 2,
            "Expected at least 2 Mover Model_{{i}} exports with compiled BSP nodes/surfs/verts, found {}",
            populated_mover_models
        );

        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_scale_brush_undo_redo_and_viewport_shading_cycle() {
        let mut state = EditorState::default();
        state.new_map();
        assert!(!state.world_brushes.is_empty());

        let old_scale = state.world_brushes[0].scale;
        let new_scale = Vec3::new(2.0, 1.5, 0.5);
        state.world_brushes[0].scale = new_scale;
        state.history.push(Action::ScaleBrush {
            brush_index: 0,
            old_scale,
            new_scale,
        });

        state.undo();
        assert_eq!(state.world_brushes[0].scale, old_scale);

        state.redo();
        assert_eq!(state.world_brushes[0].scale, new_scale);

        // Verify ViewportShading cycle includes TextureUsage and BspCuts
        let mut shading = crate::camera::ViewportShading::Wireframe;
        let mut visited = Vec::new();
        for _ in 0..7 {
            visited.push(shading);
            shading = shading.next();
        }
        assert!(visited.contains(&crate::camera::ViewportShading::TextureUsage));
        assert!(visited.contains(&crate::camera::ViewportShading::BspCuts));
        assert_eq!(shading, crate::camera::ViewportShading::Wireframe);
    }

    #[test]
    fn test_vertex_edit_drag_coalescing_and_bsp_doctor_undo_and_pan_uv() {
        let mut state = EditorState::default();
        state.new_map();
        assert!(!state.world_brushes.is_empty());

        // 1. Test Vertex Edit Gizmo Drag Coalescing (REQ-B06)
        state.selected_brush_index = Some(0);
        state.selected_vertex_indices = vec![0];
        let orig_v0 = state.world_brushes[0].polys[0].vertices[0];

        // Simulate active gizmo drag across 3 frames
        state.active_gizmo_axis = crate::raycast::GizmoAxis::X;
        let undo_count_before = state.history.undo_stack.len();
        state.move_selected_vertices(Vec3::new(8.0, 0.0, 0.0));
        state.move_selected_vertices(Vec3::new(8.0, 0.0, 0.0));
        state.move_selected_vertices(Vec3::new(16.0, 0.0, 0.0));
        state.active_gizmo_axis = crate::raycast::GizmoAxis::None;
        state.history.end_interaction();

        // All 3 drag steps must have coalesced into a single Undo action!
        assert_eq!(state.history.undo_stack.len(), undo_count_before + 1);
        assert_eq!(
            state.world_brushes[0].polys[0].vertices[0],
            orig_v0 + Vec3::new(32.0, 0.0, 0.0)
        );
        state.undo();
        assert_eq!(state.world_brushes[0].polys[0].vertices[0], orig_v0);
        state.redo();
        assert_eq!(
            state.world_brushes[0].polys[0].vertices[0],
            orig_v0 + Vec3::new(32.0, 0.0, 0.0)
        );

        // 2. Test BSP Doctor Undo/Redo (REQ-I02)
        state.grid_step = 16.0;
        let off_grid = orig_v0 + Vec3::new(32.02, 0.0, 0.0);
        state.world_brushes[0].polys[0].vertices[0] = off_grid;
        let undo_before_doctor = state.history.undo_stack.len();
        let report = state.run_bsp_doctor();
        assert!(report.snapped_vertices_count > 0);
        assert_eq!(state.history.undo_stack.len(), undo_before_doctor + 1);
        state.undo();
        assert_eq!(state.world_brushes[0].polys[0].vertices[0], off_grid);
        state.redo();
        assert_ne!(state.world_brushes[0].polys[0].vertices[0], off_grid);

        // 3. Test PanU / PanV preservation across UNR roundtrip
        state.world_brushes[0].polys[0].pan_u = 24;
        state.world_brushes[0].polys[0].pan_v = -12;
        let temp_path = std::env::temp_dir().join("test_pan_uv_roundtrip.unr");
        crate::compiler::CompilerBridge::save_unr_native(&state, &temp_path)
            .expect("save_unr_native should succeed");
        let mut reloaded = EditorState::default();
        reloaded
            .load_unr(&temp_path)
            .expect("load_unr should succeed");
        assert_eq!(reloaded.world_brushes[0].polys[0].pan_u, 24);
        assert_eq!(reloaded.world_brushes[0].polys[0].pan_v, -12);
        let _ = std::fs::remove_file(&temp_path);
    }

    #[test]
    fn test_brush_group_reorder_shape2d_undo_uv_eyedropper_and_semisolid_order() {
        let mut state = EditorState::default();
        state.world_brushes.clear();
        state.brush_groups.clear();

        // 1. Brush::group synchronization across reorder + Undo/Redo (REQ-B07 / REQ-H03)
        let b0 = ut99_bsp::build_cube(
            "RoomA",
            ut99_bsp::CsgOperation::Add,
            Vec3::new(128.0, 128.0, 128.0),
        );
        let b1 = ut99_bsp::build_cube(
            "PillarB",
            ut99_bsp::CsgOperation::Add,
            Vec3::new(64.0, 64.0, 64.0),
        );
        state.world_brushes.push(b0);
        state.world_brushes.push(b1);

        state.selected_brush_index = Some(0);
        state.set_selected_objects_group("Architecture");
        state.selected_brush_index = Some(1);
        state.set_selected_objects_group("Pillars");

        assert_eq!(state.world_brushes[0].group.as_deref(), Some("Architecture"));
        assert_eq!(state.world_brushes[1].group.as_deref(), Some("Pillars"));
        assert_eq!(state.brush_groups.get(&0).map(|s| s.as_str()), Some("Architecture"));
        assert_eq!(state.brush_groups.get(&1).map(|s| s.as_str()), Some("Pillars"));

        // Send brush 1 ("Pillars") to First (index 0)
        state.selected_brush_index = Some(1);
        state.send_selected_brush_to_first();
        assert_eq!(state.world_brushes[0].name, "PillarB");
        assert_eq!(state.brush_groups.get(&0).map(|s| s.as_str()), Some("Pillars"));
        assert_eq!(state.brush_groups.get(&1).map(|s| s.as_str()), Some("Architecture"));

        // Undo reorder restores original groups at original indices
        state.undo();
        assert_eq!(state.world_brushes[0].name, "RoomA");
        assert_eq!(state.brush_groups.get(&0).map(|s| s.as_str()), Some("Architecture"));
        assert_eq!(state.brush_groups.get(&1).map(|s| s.as_str()), Some("Pillars"));

        // Redo reorder
        state.redo();
        assert_eq!(state.world_brushes[0].name, "PillarB");
        assert_eq!(state.brush_groups.get(&0).map(|s| s.as_str()), Some("Pillars"));

        // 2. 2D Shape Editor local Undo/Redo stack (REQ-B04)
        let orig_pts = state.shape2d.points.clone();
        state.push_shape2d_undo();
        state.shape2d = ut99_bsp::Shape2D::new_circle(128.0, 8);
        assert_eq!(state.shape2d.points.len(), 8);
        assert!(state.undo_shape2d());
        assert_eq!(state.shape2d.points, orig_pts);
        assert!(state.redo_shape2d());
        assert_eq!(state.shape2d.points.len(), 8);

        // 3. Eyedropper + Ctrl+Alt+LMB UV & Texture alignment (REQ-D01)
        state.world_brushes[0].polys[0].texture = "Indus5.Trim01".to_string();
        state.world_brushes[0].polys[0].texture_u = Vec3::new(2.0, 0.0, 0.0);
        state.world_brushes[0].polys[0].texture_v = Vec3::new(0.0, 0.0, -2.0);
        state.world_brushes[0].polys[0].pan_u = 16;
        state.world_brushes[0].polys[0].pan_v = -8;
        state.sampled_surface_uv = Some((
            state.world_brushes[0].location + state.world_brushes[0].polys[0].base,
            Vec3::new(2.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, -2.0),
            16,
            -8,
        ));
        state.active_texture = Some("Indus5.Trim01".to_string());
        // Ray pointing from outside into the cube at origin
        let ray = crate::raycast::Ray {
            origin: Vec3::new(0.0, 0.0, 256.0),
            direction: Vec3::new(0.0, 0.0, -1.0),
        };
        assert!(state.apply_active_texture_and_uv_at_ray(ray));

        // 4. Semi-Solid chronological CSG order (REQ-B02)
        // A Semi-Solid placed BEFORE a later Subtract brush is NOT cut by that later Subtract brush.
        let room = ut99_bsp::build_cube(
            "Room",
            ut99_bsp::CsgOperation::Subtract,
            Vec3::new(512.0, 512.0, 512.0),
        );

        let mut pillar = ut99_bsp::build_cube(
            "Pillar",
            ut99_bsp::CsgOperation::Add,
            Vec3::new(128.0, 128.0, 128.0),
        );
        for p in &mut pillar.polys {
            p.flags |= ut99_bsp::PF_SEMISOLID;
        }

        let later_sub = ut99_bsp::build_cube(
            "LaterSub",
            ut99_bsp::CsgOperation::Subtract,
            Vec3::new(256.0, 256.0, 256.0),
        );

        let eval = ut99_bsp::evaluate_csg_brushes_indexed(&[room, pillar, later_sub]);
        let semisolid_polys: usize = eval
            .iter()
            .filter(|(b_idx, _)| *b_idx == 1)
            .count();
        assert_eq!(
            semisolid_polys, 6,
            "Semi-Solid brush at index 1 must not be clipped away by Subtract brush at index 2"
        );
    }

    #[test]
    fn test_ux_improvement_plan_ux01_to_ux08() {
        let mut state = EditorState::default();

        // UX-01: Persistent Math Expression Evaluator
        state.math_expression_input = "=512-128".to_string();
        let res = state.evaluate_math_expression_input().expect("math evaluation should succeed");
        assert!((res - 384.0).abs() < 1e-4);
        assert_eq!(state.last_math_result, Some(384.0));

        // UX-05 & UX-07: Grid step synchronization & Shading mode persistence
        state.set_grid_step(64.0);
        assert_eq!(state.grid_step, 64.0);
        assert_eq!(state.grid.step, 64.0);
        state.viewport_shading = crate::camera::ViewportShading::TextureUsage;
        let prefs = crate::preferences::EditorPreferences::from_editor_state(&state);
        let mut restored_state = EditorState::default();
        prefs.apply_to_editor_state(&mut restored_state);
        assert_eq!(restored_state.grid_step, 64.0);
        assert_eq!(restored_state.grid.step, 64.0);
        assert_eq!(restored_state.viewport_shading, crate::camera::ViewportShading::TextureUsage);

        state.viewport_shading = crate::camera::ViewportShading::BspCuts;
        let prefs2 = crate::preferences::EditorPreferences::from_editor_state(&state);
        prefs2.apply_to_editor_state(&mut restored_state);
        assert_eq!(restored_state.viewport_shading, crate::camera::ViewportShading::BspCuts);

        // UX-06: Context menu ray selection preserves existing selection on empty space
        let cube = ut99_bsp::build_cube(
            "TestCube",
            ut99_bsp::CsgOperation::Add,
            Vec3::new(128.0, 128.0, 128.0),
        );
        state.world_brushes.push(cube);
        state.selected_brush_index = Some(0);
        state.multi_selected_brushes.insert(0);
        let miss_ray = crate::raycast::Ray {
            origin: Vec3::new(5000.0, 5000.0, 5000.0),
            direction: Vec3::new(0.0, 1.0, 0.0),
        };
        state.select_at_ray_for_context_menu(miss_ray);
        assert_eq!(
            state.selected_brush_index,
            Some(0),
            "Right-clicking empty space must preserve existing selection for context menu"
        );

        // UX-06: delete_actor_at synchronizes lights/nav_points, undo history, and map_is_dirty
        state.place_actor_of_class("Light");
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 1);
        assert_eq!(state.loaded_unr.as_ref().unwrap().lights.len(), 1);
        state.map_is_dirty = false;
        assert!(state.delete_actor_at(0));
        assert!(state.map_is_dirty);
        assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);
        assert_eq!(state.loaded_unr.as_ref().unwrap().lights.len(), 0);

        // UX-08 & UX-12: PendingDirtyAction state machine & Open Map protection
        state.pending_dirty_action = Some(PendingDirtyAction::NewMap);
        assert_eq!(state.pending_dirty_action, Some(PendingDirtyAction::NewMap));

        state.map_is_dirty = true;
        state.show_unsaved_exit_modal = false;
        state.prompt_or_open_unr_dialog();
        assert!(state.show_unsaved_exit_modal);
        assert_eq!(state.pending_dirty_action, Some(PendingDirtyAction::OpenUnrDialog));

        state.show_unsaved_exit_modal = false;
        state.prompt_or_open_t3d_dialog();
        assert!(state.show_unsaved_exit_modal);
        assert_eq!(state.pending_dirty_action, Some(PendingDirtyAction::OpenT3dDialog));

        let test_path = std::path::PathBuf::from("DM-Morpheus.unr");
        state.show_unsaved_exit_modal = false;
        state.prompt_or_open_map_path(test_path.clone());
        assert!(state.show_unsaved_exit_modal);
        assert_eq!(state.pending_dirty_action, Some(PendingDirtyAction::OpenMapPath(test_path)));

        // UX-10: Preferences persistence for UI Experience Mode, Target Engine, Keymap Profile, and Gizmo Space
        state.ui_experience_mode = UiExperienceMode::Architect;
        state.target_engine = TargetEngine::Unreal227;
        state.keymap_profile = KeymapProfile::ClassicUnrealEd2;
        state.gizmo_space = GizmoCoordinateSpace::Local;
        let prefs_ux10 = crate::preferences::EditorPreferences::from_editor_state(&state);
        let mut restored_ux10 = EditorState::default();
        prefs_ux10.apply_to_editor_state(&mut restored_ux10);
        assert_eq!(restored_ux10.ui_experience_mode, UiExperienceMode::Architect);
        assert_eq!(restored_ux10.target_engine, TargetEngine::Unreal227);
        assert_eq!(restored_ux10.keymap_profile, KeymapProfile::ClassicUnrealEd2);
        assert_eq!(restored_ux10.gizmo_space, GizmoCoordinateSpace::Local);

        // UX-11: Red Builder Brush Reset records Action::ModifyBuilderBrush in undo history
        state.builder_brush.location = Vec3::new(256.0, -128.0, 64.0);
        state.builder_brush.rotation = Vec3::new(45.0, 90.0, 0.0);
        state.reset_builder_brush();
        assert_eq!(state.builder_brush.location, Vec3::ZERO);
        assert_eq!(state.builder_brush.rotation, Vec3::ZERO);
        state.undo();
        assert_eq!(state.builder_brush.location, Vec3::new(256.0, -128.0, 64.0));
        assert_eq!(state.builder_brush.rotation, Vec3::new(45.0, 90.0, 0.0));
    }
