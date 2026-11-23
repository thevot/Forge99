// crates/ut99-editor/tests/test_ux_performance_simplicity.rs
//
// Integration test suite validating the modern UX, performance, and simplicity enhancements:
// 1. UI Experience Modes (Standard vs Architect) and Dialog lifecycle
// 2. Action Wizards: Mover & Lift, Teleporter Pair, Water Basin, Jump Pad, Room Architect
// 3. Visual & Lighting Presets application to Actors and BSP Surfaces
// 4. Planar Gizmo Mathematics & Ray-Plane projection

use glam::Vec3;
use ut99_editor::EditorState;
use ut99_editor::editor_state::{UiExperienceMode, WizardTab};
use ut99_editor::raycast::{GizmoAxis, Ray};

#[test]
fn test_experience_mode_toggle_and_dialog_state() {
    let mut state = EditorState::default();

    // 1. Initial defaults
    assert_eq!(state.ui_experience_mode, UiExperienceMode::Standard);
    assert!(!state.show_action_wizards_dialog);
    assert!(!state.show_visual_presets_dialog);
    assert_eq!(state.active_wizard_tab, WizardTab::MoverLift);

    // 2. Open dialogs
    state.show_action_wizards_dialog = true;
    state.show_visual_presets_dialog = true;

    // 3. Close all floating dialogs
    state.close_all_floating_dialogs();
    assert!(!state.show_action_wizards_dialog);
    assert!(!state.show_visual_presets_dialog);

    // 4. Mode toggle
    state.ui_experience_mode = UiExperienceMode::Architect;
    assert_eq!(state.ui_experience_mode, UiExperienceMode::Architect);
    state.ui_experience_mode = UiExperienceMode::Standard;
    assert_eq!(state.ui_experience_mode, UiExperienceMode::Standard);
}

#[test]
fn test_action_wizards_mover_lift_generation() {
    let mut state = EditorState::default();

    state.wizard_mover_height = 256.0;
    state.wizard_mover_tag = "Elevator_A".to_string();
    state.wizard_mover_delay = 3.5;

    let initial_movers = state.movers.len();
    state.generate_wizard_lift();

    assert_eq!(state.movers.len(), initial_movers + 1);
    let mover = state.movers.last().unwrap();
    assert_eq!(mover.name, "Elevator_A");
    assert_eq!(mover.stay_open_time, 3.5);
    assert!(mover.keyframes[0].is_some());
    assert!(mover.keyframes[1].is_some());

    let kf0 = mover.keyframes[0].as_ref().unwrap();
    let kf1 = mover.keyframes[1].as_ref().unwrap();
    assert_eq!(kf1.location.z - kf0.location.z, 256.0);

    // Trigger actor wiring
    let unr = state.loaded_unr.as_ref().expect("Level should be initialized");
    let trigger = unr.actors.iter().find(|a| a.name == "Elevator_A_Trigger").expect("Trigger must exist");
    assert_eq!(trigger.class_name, "Trigger");
    assert_eq!(trigger.event, "Elevator_A");
    assert!(trigger.properties.contains_key("CollisionRadius"));
    assert!(trigger.properties.contains_key("CollisionHeight"));
}

#[test]
fn test_action_wizards_teleporters_and_basin() {
    let mut state = EditorState::default();

    // 1. Teleporter Pair
    state.wizard_teleport_enter_tag = "Portal_Alpha".to_string();
    state.wizard_teleport_exit_tag = "Portal_Beta".to_string();
    state.wizard_teleport_offset_x = 512.0;
    state.wizard_teleport_offset_z = 128.0;

    state.generate_wizard_teleporters();

    let unr = state.loaded_unr.as_ref().unwrap();
    let ent = unr.actors.iter().find(|a| a.name == "Portal_Alpha").expect("Entrance teleporter exists");
    let exit = unr.actors.iter().find(|a| a.name == "Portal_Beta").expect("Exit teleporter exists");

    assert_eq!(ent.class_name, "Teleporter");
    assert_eq!(exit.class_name, "Teleporter");
    assert_eq!(exit.location - ent.location, Vec3::new(512.0, 0.0, 128.0));

    let url_prop = ent.properties.get("URL").expect("Entrance has URL prop");
    match &url_prop.value {
        ut99_format::TaggedValue::Str(val) => assert_eq!(val, "Portal_Beta"),
        other => panic!("Expected Str tagged value, found {:?}", other),
    }

    // 2. Water Basin
    let initial_brushes = state.world_brushes.len();
    state.wizard_basin_width = 256.0;
    state.wizard_basin_length = 256.0;
    state.wizard_basin_depth = 128.0;
    state.wizard_basin_type = 0; // WaterZone

    state.generate_wizard_water_basin();

    assert!(state.world_brushes.len() > initial_brushes);
    let unr = state.loaded_unr.as_ref().unwrap();
    let water_zone = unr.actors.iter().find(|a| a.class_name == "WaterZone");
    assert!(water_zone.is_some(), "WaterZone actor should be spawned inside basin");
}

#[test]
fn test_action_wizards_jump_pad_physics_and_room() {
    let mut state = EditorState::default();

    // 1. Jump Pad
    state.wizard_jump_height = 200.0;
    state.wizard_jump_dist = 600.0;

    state.generate_wizard_jump_pad();

    let unr = state.loaded_unr.as_ref().unwrap();
    let pad = unr.actors.iter().find(|a| a.class_name == "UT_JumpPad").expect("UT_JumpPad exists");
    let target = unr.actors.iter().find(|a| a.class_name == "PathNode").expect("Target PathNode exists");

    let kick_prop = pad.properties.get("KickVelocity").expect("KickVelocity exists");
    match &kick_prop.value {
        ut99_format::TaggedValue::Vector(v) => {
            assert!(v.x > 0.0, "Horizontal kick velocity must be positive");
            assert!(v.z > 0.0, "Vertical kick velocity must be positive");
        }
        other => panic!("Expected Vector tagged value, found {:?}", other),
    }
    assert_eq!(target.location.x - pad.location.x, 600.0);

    // 2. Room Architect
    let brush_count_before = state.world_brushes.len();
    state.wizard_room_width = 512.0;
    state.wizard_room_length = 512.0;
    state.wizard_room_height = 256.0;
    state.wizard_room_lights = 2;

    state.generate_wizard_room();

    assert!(state.world_brushes.len() > brush_count_before);
    let unr_after = state.loaded_unr.as_ref().unwrap();
    let lights = unr_after.actors.iter().filter(|a| a.class_name == "Light").count();
    assert!(lights >= 4, "Room wizard should spawn at least 4 perimeter lights");
}

#[test]
fn test_visual_presets_lighting_and_materials() {
    let mut state = EditorState::default();

    // 1. Spawn a light actor
    state.add_light_at_builder();
    let unr = state.loaded_unr.as_mut().unwrap();
    let light_idx = unr.actors.len() - 1;
    state.selected_actor_index = Some(light_idx);

    // Apply Neon Cyberpunk preset (hue: 135, sat: 10, bright: 180, rad: 40, effect: 0)
    state.apply_lighting_preset(135, 10, 180, 40, 0);

    let unr = state.loaded_unr.as_ref().unwrap();
    let light = &unr.actors[light_idx];
    assert_eq!(light.light_hue, 135);
    assert_eq!(light.light_saturation, 10);
    assert_eq!(light.light_brightness, 180);
    assert_eq!(light.light_radius, 40);
    assert_eq!(light.light_effect, 0);

    // 2. Surface Material Presets
    state.set_builder_cube(Vec3::splat(128.0));
    state.csg_add();
    let b_idx = state.world_brushes.len() - 1;
    state.selected_surfaces.insert((b_idx, 0));

    // Apply Glass preset (PF_TRANSLUCENT | PF_TWO_SIDED)
    let glass_flags = ut99_bsp::PF_TRANSLUCENT | ut99_bsp::PF_TWO_SIDED;
    state.apply_surface_material_preset(glass_flags);

    assert_eq!(state.world_brushes[b_idx].polys[0].flags, glass_flags);

    // 3. Gameplay Station Preset
    let initial_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    state.spawn_gameplay_station_preset("HealthAndArmor");
    let after_actors = state.loaded_unr.as_ref().unwrap().actors.len();
    assert_eq!(after_actors, initial_actors + 3);
}

#[test]
fn test_raycast_planar_gizmo_math() {
    // 1. Ray-plane intersection test
    let ray = Ray {
        origin: Vec3::new(50.0, 50.0, 100.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let hit_xy = ray.project_onto_plane(Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.0, 0.0, 1.0));
    assert!(hit_xy.is_some());
    let hit_pt = hit_xy.unwrap();
    assert!((hit_pt.x - 50.0).abs() < 1e-4);
    assert!((hit_pt.y - 50.0).abs() < 1e-4);
    assert!((hit_pt.z - 0.0).abs() < 1e-4);

    // 2. Planar Gizmo Hit Testing
    let gizmo_center = Vec3::ZERO;
    let size = 100.0;

    // Ray targeted directly into the XY planar square (x: 25, y: 25, z: 100 pointing down)
    let ray_xy = Ray {
        origin: Vec3::new(25.0, 25.0, 100.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let hit_axis = ray_xy.hit_test_gizmo(gizmo_center, size, 5.0);
    assert_eq!(hit_axis, GizmoAxis::PlaneXY);

    // Ray targeted directly into the XZ planar square (x: 25, y: 100, z: 25 pointing along -Y)
    let ray_xz = Ray {
        origin: Vec3::new(25.0, 100.0, 25.0),
        direction: Vec3::new(0.0, -1.0, 0.0),
    };
    let hit_axis_xz = ray_xz.hit_test_gizmo(gizmo_center, size, 5.0);
    assert_eq!(hit_axis_xz, GizmoAxis::PlaneXZ);
}

#[test]
fn test_gizmo_coordinate_space_toggle_and_matrix() {
    let mut state = EditorState::default();

    // 1. Initial space is World
    assert_eq!(state.gizmo_space, ut99_editor::editor_state::GizmoCoordinateSpace::World);
    assert_eq!(state.get_selected_rotation_matrix(), glam::Mat3::IDENTITY);

    // 2. Toggle space to Local
    state.toggle_gizmo_space();
    assert_eq!(state.gizmo_space, ut99_editor::editor_state::GizmoCoordinateSpace::Local);

    // 3. Set builder brush rotation to Yaw = 90 deg
    state.builder_brush.rotation = Vec3::new(0.0, 90.0, 0.0);
    let mat = state.get_selected_rotation_matrix();

    // Transforming Vec3::X by 90 deg around Z should yield Vec3::Y
    let rotated_x = mat * Vec3::X;
    assert!((rotated_x.x - 0.0).abs() < 1e-4);
    assert!((rotated_x.y - 1.0).abs() < 1e-4);
    assert!((rotated_x.z - 0.0).abs() < 1e-4);

    // 4. Toggle back to World
    state.toggle_gizmo_space();
    assert_eq!(state.gizmo_space, ut99_editor::editor_state::GizmoCoordinateSpace::World);
    assert_eq!(state.get_selected_rotation_matrix(), glam::Mat3::IDENTITY);

    // 5. Test Euler and Unreal Rotator conversion functions
    let euler_mat = ut99_editor::editor_state::euler_degrees_to_mat3(0.0, 90.0, 0.0);
    let unr_mat = ut99_editor::editor_state::unreal_rotator_to_mat3(0, 16384, 0); // 16384 = 90 deg
    assert!((euler_mat.x_axis - unr_mat.x_axis).length() < 1e-3);
    assert!((euler_mat.y_axis - unr_mat.y_axis).length() < 1e-3);
    assert!((euler_mat.z_axis - unr_mat.z_axis).length() < 1e-3);
}

#[test]
fn test_oriented_gizmo_hit_testing() {
    let center = Vec3::ZERO;
    let size = 100.0;
    let hit_rad = 8.0;

    // A ray pointing down from Z=50 towards X=50, Y=0
    let ray_at_x = Ray {
        origin: Vec3::new(50.0, 0.0, 50.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };

    // In identity orientation, this hits the X axis
    let hit_identity = ray_at_x.hit_test_gizmo_oriented(center, size, hit_rad, glam::Mat3::IDENTITY);
    assert_eq!(hit_identity, GizmoAxis::X);

    // If the gizmo is rotated by 90 deg around Z:
    // Local X is now at World Y. Local Y is at World -X.
    let rot_90_z = glam::Mat3::from_rotation_z(90.0_f32.to_radians());

    // In 90 deg orientation, a ray at World (0, 50, 50) hits Local X!
    let ray_at_y = Ray {
        origin: Vec3::new(0.0, 50.0, 50.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let hit_rotated = ray_at_y.hit_test_gizmo_oriented(center, size, hit_rad, rot_90_z);
    assert_eq!(hit_rotated, GizmoAxis::X);

    // Test rotation ring hit test oriented (ring of radius 100 in XY plane with normal Z)
    let ray_ring = Ray {
        origin: Vec3::new(100.0, 0.0, 50.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };
    let hit_rot = ray_ring.hit_test_rotation_gizmo_oriented(center, size, hit_rad, glam::Mat3::IDENTITY);
    assert_eq!(hit_rot, GizmoAxis::Z);
}

#[test]
fn test_view_frustum_culling_math() {
    use ut99_editor::culling::Frustum;

    // Camera at (0, 0, 100) looking along +X with 90° FOV, aspect 1.0, near 10.0, far 1000.0
    let mut camera = ut99_editor::camera::Camera::new(Vec3::new(0.0, 0.0, 100.0), 1.0);
    camera.yaw = 0.0;   // Facing +X
    camera.pitch = 0.0;
    camera.fov_y = 90.0_f32.to_radians();
    camera.z_near = 10.0;
    camera.z_far = 1000.0;

    let vp = camera.view_projection_matrix();
    let frustum = Frustum::from_view_projection(&vp);

    // 1. AABB directly in front of camera at X=200, within view cone
    let aabb_in_front_min = Vec3::new(180.0, -20.0, 80.0);
    let aabb_in_front_max = Vec3::new(220.0, 20.0, 120.0);
    assert!(frustum.intersects_aabb(aabb_in_front_min, aabb_in_front_max), "Box in front must intersect frustum");

    // 2. AABB directly behind camera at X=-200
    let aabb_behind_min = Vec3::new(-220.0, -20.0, 80.0);
    let aabb_behind_max = Vec3::new(-180.0, 20.0, 120.0);
    assert!(!frustum.intersects_aabb(aabb_behind_min, aabb_behind_max), "Box behind camera must be culled");

    // 3. AABB far off to the side at Y=2000 (outside 90° FOV)
    let aabb_side_min = Vec3::new(100.0, 1950.0, 80.0);
    let aabb_side_max = Vec3::new(150.0, 2050.0, 120.0);
    assert!(!frustum.intersects_aabb(aabb_side_min, aabb_side_max), "Box far to side must be culled");

    // 4. AABB beyond far plane at X=1500
    let aabb_far_min = Vec3::new(1450.0, -20.0, 80.0);
    let aabb_far_max = Vec3::new(1550.0, 20.0, 120.0);
    assert!(!frustum.intersects_aabb(aabb_far_min, aabb_far_max), "Box beyond far clipping plane must be culled");

    // 5. Sphere tests
    assert!(frustum.intersects_sphere(Vec3::new(200.0, 0.0, 100.0), 30.0), "Sphere in front must intersect");
    assert!(!frustum.intersects_sphere(Vec3::new(-100.0, 0.0, 100.0), 20.0), "Sphere behind must be culled");
    assert!(!frustum.intersects_sphere(Vec3::new(100.0, 1000.0, 100.0), 20.0), "Sphere to side must be culled");
}

#[test]
fn test_viewport_drag_and_drop_actor_placement() {
    use ut99_editor::editor_state::DragDropPayload;

    let mut state = EditorState::default();
    assert!(state.active_drag_drop.is_none());

    // 1. Initiate Drag & Drop of Actor Class "PathNode"
    state.active_drag_drop = Some(DragDropPayload::ActorClass("PathNode".to_string()));

    // 2. Ray from sky pointing down towards ground at (256, 128, 0)
    let ray = Ray {
        origin: Vec3::new(256.0, 128.0, 500.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };

    // 3. Drop completion on scene
    let success = state.complete_drag_drop((400.0, 300.0), ray);
    assert!(success, "Drag drop completion should succeed");
    assert!(state.active_drag_drop.is_none(), "Active payload should be cleared after drop");
    assert!(state.drag_drop_preview_pos.is_none(), "Preview position cleared");

    // 4. Verify spawned actor in level
    let unr = state.loaded_unr.as_ref().expect("Level must exist");
    let spawned = unr.actors.last().expect("Must have spawned actor");
    assert_eq!(spawned.class_name, "PathNode");
    assert_eq!(spawned.location.x, 256.0);
    assert_eq!(spawned.location.y, 128.0);
    assert!(spawned.location.z >= 0.0, "Actor placed with positive floor offset");
}

#[test]
fn test_viewport_drag_and_drop_texture_assignment() {
    use ut99_editor::editor_state::DragDropPayload;

    let mut state = EditorState::default();

    // Initiate Drag & Drop of Texture
    state.active_drag_drop = Some(DragDropPayload::Texture("GenEarth.Dirt.Floor01".to_string()));

    // Ray pointing down to ground
    let ray = Ray {
        origin: Vec3::new(0.0, 0.0, 500.0),
        direction: Vec3::new(0.0, 0.0, -1.0),
    };

    let success = state.complete_drag_drop((100.0, 100.0), ray);
    assert!(success);
    assert_eq!(state.active_texture.as_deref(), Some("GenEarth.Dirt.Floor01"));
}

#[test]
fn test_drag_and_drop_escape_cancellation() {
    use ut99_editor::editor_state::DragDropPayload;

    let mut state = EditorState::default();
    state.active_drag_drop = Some(DragDropPayload::ActorClass("Light".to_string()));
    state.drag_drop_preview_pos = Some(Vec3::new(100.0, 200.0, 300.0));
    state.drag_drop_preview_normal = Some(Vec3::Z);

    let handled = state.handle_escape();
    assert!(handled, "Escape must consume and handle active drag and drop");
    assert!(state.active_drag_drop.is_none());
    assert!(state.drag_drop_preview_pos.is_none());
    assert!(state.drag_drop_preview_normal.is_none());
}

#[test]
fn test_frustum_culling_toggle_and_stats() {
    let mut state = EditorState::default();

    assert!(state.frustum_culling_enabled, "Frustum culling should be enabled by default for optimal performance");
    assert_eq!(state.last_cull_stats, (0, 0, 0, 0));

    state.toggle_frustum_culling();
    assert!(!state.frustum_culling_enabled);

    state.toggle_frustum_culling();
    assert!(state.frustum_culling_enabled);
}


