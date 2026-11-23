use glam::Vec3;
use ut99_editor::camera::{Camera, ViewportLayout, ViewportType};
use ut99_editor::editor_state::{EditorMode, EditorState, MarqueeBox, SurfaceFlagsPreset, TextureMatchMode, TextureReplaceScope};
use ut99_editor::gui::command_palette::{fuzzy_match, parse_query};
use ut99_editor::gui::theme::{check_wcag_aa, wcag_contrast_ratio, ThemeColors};
use ut99_format::{ActorRecord, UnrLevel};

#[test]
fn test_viewport_rect_and_aspect_ratio_accuracy() {
    let mut state = EditorState::default();

    // Emulate window size 1440x900, but with side panel (280px), left toolbar (48px) and top/bottom panels
    // leaving a central viewport area of 1112x786
    let avail_x = 48.0;
    let avail_y = 60.0;
    let avail_w = 1112.0;
    let avail_h = 786.0;

    state.viewport_rect = [avail_x, avail_y, avail_w, avail_h];
    state.camera.aspect = avail_w / avail_h;

    assert_eq!(state.viewport_rect[0], 48.0);
    assert_eq!(state.viewport_rect[1], 60.0);
    assert_eq!(state.viewport_rect[2], 1112.0);
    assert_eq!(state.viewport_rect[3], 786.0);

    let expected_aspect = 1112.0 / 786.0;
    assert!((state.camera.aspect - expected_aspect).abs() < 1e-5);
    // Crucial check: aspect ratio must NOT equal whole window 1440/900 (1.60), eliminating distortion!
    assert!((state.camera.aspect - (1440.0 / 900.0)).abs() > 0.1);
}

#[test]
fn test_multi_viewport_quadrant_partitioning() {
    let mut state = EditorState::default();
    state.viewport_layout = ViewportLayout::FourViewports;
    state.viewport_rect = [100.0, 50.0, 1000.0, 800.0];

    let [vx, vy, vw, vh] = state.viewport_rect;
    let hw = vw * 0.5;
    let hh = vh * 0.5;
    let rw = vw - hw;
    let rh = vh - hh;

    // 4 Quadrants: Top (XY), Side (YZ), Front (XZ), Perspective
    let top_quad = (vx, vy, hw, hh);
    let side_quad = (vx + hw, vy, rw, hh);
    let front_quad = (vx, vy + hh, hw, rh);
    let persp_quad = (vx + hw, vy + hh, rw, rh);

    assert_eq!(top_quad, (100.0, 50.0, 500.0, 400.0));
    assert_eq!(side_quad, (600.0, 50.0, 500.0, 400.0));
    assert_eq!(front_quad, (100.0, 450.0, 500.0, 400.0));
    assert_eq!(persp_quad, (600.0, 450.0, 500.0, 400.0));

    let total_area = (top_quad.2 * top_quad.3)
        + (side_quad.2 * side_quad.3)
        + (front_quad.2 * front_quad.3)
        + (persp_quad.2 * persp_quad.3);

    assert_eq!(total_area, vw * vh);
}

#[test]
fn test_texture_eyedropper_and_painting() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create 2 test brushes using real editor CSG pipeline
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();

    state.builder_brush.location = Vec3::new(256.0, 0.0, 0.0);
    state.csg_add();

    assert_eq!(state.world_brushes.len(), 2);

    // Set brush 0 texture
    for p in &mut state.world_brushes[0].polys {
        p.texture = "GenEarth.Rock.rock1".to_string();
    }

    // Eyedropper test: Pick texture from Brush 0
    state.active_texture = Some("GenEarth.Rock.rock1".to_string());
    assert_eq!(state.active_texture.as_deref(), Some("GenEarth.Rock.rock1"));

    // Paint test: Apply active texture to Brush 1
    state.selected_brush_index = Some(1);
    state.apply_active_texture_to_selected_brush();

    for p in &state.world_brushes[1].polys {
        assert_eq!(p.texture, "GenEarth.Rock.rock1");
    }
}

#[test]
fn test_camera_ortho_zoom_and_perspective_dolly() {
    let mut camera = Camera::default();
    let initial_pos = camera.position;
    let initial_zoom = camera.ortho_zoom;

    // 1. Perspective dolly (scroll forward)
    let fwd = camera.forward();
    let scroll_amount = 2.0;
    camera.position += fwd * (scroll_amount * 64.0);
    assert!(camera.position.distance(initial_pos) > 50.0);

    // 2. Orthographic zoom clamping
    let factor = 0.85f32.powf(scroll_amount);
    camera.ortho_zoom = (camera.ortho_zoom * factor).clamp(32.0, 100_000.0);
    assert!(camera.ortho_zoom < initial_zoom);
    assert!(camera.ortho_zoom >= 32.0);
}

#[test]
fn test_camera_speed_presets_and_cycling() {
    let mut state = EditorState::default();

    // Default preset is 4 (600 uu/s)
    assert_eq!(state.camera_speed_preset, 4);
    assert_eq!(state.camera.speed, 600.0);

    // Test setting preset 1 (150 uu/s)
    state.set_camera_speed_preset(1);
    assert_eq!(state.camera_speed_preset, 1);
    assert_eq!(state.camera.speed, 150.0);

    // Test setting preset 8 (2000 uu/s)
    state.set_camera_speed_preset(8);
    assert_eq!(state.camera_speed_preset, 8);
    assert_eq!(state.camera.speed, 2000.0);

    // Out of bounds preset should clamp
    state.set_camera_speed_preset(10);
    assert_eq!(state.camera_speed_preset, 8);
    state.set_camera_speed_preset(0);
    assert_eq!(state.camera_speed_preset, 1);

    // Test cycling from 1 to 8, and wrap-around back to 1
    state.set_camera_speed_preset(7);
    state.cycle_camera_speed();
    assert_eq!(state.camera_speed_preset, 8);
    state.cycle_camera_speed();
    assert_eq!(state.camera_speed_preset, 1);
    assert_eq!(state.camera.speed, 150.0);
}

#[test]
fn test_recent_maps_tracking_and_deduplication() {
    let mut state = EditorState::default();
    state.recent_maps.clear();

    // Add maps
    state.add_recent_map("DM-Deck16][.unr");
    state.add_recent_map("CTF-FacingWorlds.unr");
    state.add_recent_map("DM-Curse][.unr");

    assert_eq!(state.recent_maps.len(), 3);
    assert_eq!(state.recent_maps[0], std::path::PathBuf::from("DM-Curse][.unr"));
    assert_eq!(state.recent_maps[1], std::path::PathBuf::from("CTF-FacingWorlds.unr"));
    assert_eq!(state.recent_maps[2], std::path::PathBuf::from("DM-Deck16][.unr"));

    // Re-adding DM-Deck16][ should move it to the top without duplicating
    state.add_recent_map("DM-Deck16][.unr");
    assert_eq!(state.recent_maps.len(), 3);
    assert_eq!(state.recent_maps[0], std::path::PathBuf::from("DM-Deck16][.unr"));
    assert_eq!(state.recent_maps[1], std::path::PathBuf::from("DM-Curse][.unr"));
    assert_eq!(state.recent_maps[2], std::path::PathBuf::from("CTF-FacingWorlds.unr"));

    // Add more than 10 maps to test capacity limit (max 10)
    for i in 1..=12 {
        state.add_recent_map(format!("Map{i}.unr"));
    }
    assert_eq!(state.recent_maps.len(), 10);
    assert_eq!(state.recent_maps[0], std::path::PathBuf::from("Map12.unr"));
}

#[test]
fn test_viewport_shading_modes() {
    let mut state = EditorState::default();
    assert_eq!(state.viewport_shading, ut99_editor::camera::ViewportShading::Lit);

    // Test label, icon and display_name
    assert_eq!(state.viewport_shading.label(), "Lit");
    assert_eq!(state.viewport_shading.icon(), "☼");
    assert_eq!(state.viewport_shading.display_name(), "☼ Lit");

    // All 5 canonical modes
    state.viewport_shading = ut99_editor::camera::ViewportShading::Wireframe;
    assert_eq!(state.viewport_shading.label(), "Wireframe");
    assert_eq!(state.viewport_shading.icon(), "◫");

    state.viewport_shading = ut99_editor::camera::ViewportShading::ZonePortal;
    assert_eq!(state.viewport_shading.label(), "Zone/Portal");
    assert_eq!(state.viewport_shading.icon(), "▣");

    state.viewport_shading = ut99_editor::camera::ViewportShading::Unlit;
    assert_eq!(state.viewport_shading.label(), "Unlit");
    assert_eq!(state.viewport_shading.icon(), "☀");

    state.viewport_shading = ut99_editor::camera::ViewportShading::TextureUsage;
    assert_eq!(state.viewport_shading.label(), "Texture Usage");
    assert_eq!(state.viewport_shading.icon(), "▦");

    state.viewport_shading = ut99_editor::camera::ViewportShading::BspCuts;
    assert_eq!(state.viewport_shading.label(), "BSP Cuts");
    assert_eq!(state.viewport_shading.icon(), "✂");

    state.viewport_shading = ut99_editor::camera::ViewportShading::DepthComplexity;
    assert_eq!(state.viewport_shading.label(), "Depth Complexity");
    assert_eq!(state.viewport_shading.icon(), "▥");

    // Test cyclic progression via next()
    let mut mode = ut99_editor::camera::ViewportShading::Wireframe;
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::ZonePortal);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::TextureUsage);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::BspCuts);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::Unlit);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::Lit);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::DepthComplexity);
    mode = mode.next();
    assert_eq!(mode, ut99_editor::camera::ViewportShading::Wireframe);
}

#[test]
fn test_adaptive_gizmo_scaling() {
    let mut state = EditorState::default();
    let center = Vec3::ZERO;
    state.camera.position = Vec3::new(0.0, -600.0, 0.0);

    // 1. Perspective scaling at 600uu distance
    let scale_600 = state.get_gizmo_scale(center, ut99_editor::camera::ViewportType::Perspective, 800.0);
    assert!(scale_600 > 60.0 && scale_600 < 100.0, "Scale at 600uu should be ~76uu, got {}", scale_600);

    // 2. Far away at 3000uu distance: gizmo scale must increase to stay constant pixel size
    state.camera.position = Vec3::new(0.0, -3000.0, 0.0);
    let scale_3000 = state.get_gizmo_scale(center, ut99_editor::camera::ViewportType::Perspective, 800.0);
    assert!(scale_3000 > scale_600 * 3.5, "Scale at 3000uu must scale up proportionally");

    // 3. Orthographic scaling with ortho_zoom
    state.camera.ortho_zoom = 512.0;
    let ortho_scale_512 = state.get_gizmo_scale(center, ut99_editor::camera::ViewportType::Top, 800.0);
    state.camera.ortho_zoom = 2048.0;
    let ortho_scale_2048 = state.get_gizmo_scale(center, ut99_editor::camera::ViewportType::Top, 800.0);
    assert_eq!(ortho_scale_2048, ortho_scale_512 * 4.0);

    // 4. Hit radius must be proportional to scale
    let (axis_scale, hit_rad) = state.get_gizmo_scale_and_hit_radius(center, ut99_editor::camera::ViewportType::Perspective, 800.0);
    assert!(hit_rad >= 8.0);
    assert!((hit_rad - axis_scale * 0.22).abs() < 1e-4 || hit_rad == 8.0);
}

#[test]
fn test_brush_texture_pan_and_scale() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.csg_add();
    state.selected_brush_index = Some(0);

    let initial_base = state.world_brushes[0].polys[0].base;
    let initial_u_len = state.world_brushes[0].polys[0].texture_u.length();

    // 1. Pan texture by (32, 64)
    state.pan_selected_brush_texture(32.0, 64.0);
    let panned_base = state.world_brushes[0].polys[0].base;
    assert!(panned_base.distance(initial_base) > 10.0, "Base vector should move upon pan");

    // 2. Scale texture by (2.0, 2.0)
    state.scale_selected_brush_texture(2.0, 2.0);
    let scaled_u_len = state.world_brushes[0].polys[0].texture_u.length();
    assert!((scaled_u_len - initial_u_len * 2.0).abs() < 1e-4, "Texture U length should double");
}

#[test]
fn test_minimap_radar_state() {
    let mut state = EditorState::default();
    assert!(!state.show_minimap);

    state.show_minimap = true;
    assert!(state.show_minimap);
}

#[test]
fn test_floor_snapping_and_normal_alignment() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create a solid floor brush (512x512x32) at origin (top face at Z=16)
    state.set_builder_cube(Vec3::new(512.0, 512.0, 32.0));
    state.builder_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    // Verify find_floor_hit_below
    let hit = state.find_floor_hit_below(Vec3::new(0.0, 0.0, 200.0), 200.0, None);
    assert!(hit.is_some(), "Floor hit should be found beneath (0, 0, 200)");
    let (floor_z, normal) = hit.unwrap();
    assert!((floor_z - 16.0).abs() < 1e-3, "Floor top should be at Z=16");
    assert!((normal.z - 1.0).abs() < 1e-3, "Floor normal should point straight up +Z");

    // 2. Add an actor hovering high above floor
    let mut actor = ut99_format::unr::actor::ActorRecord::default();
    actor.name = "LightTest".into();
    actor.location = Vec3::new(0.0, 0.0, 300.0);
    actor.rotation = (0, 0, 0);

    let mut unr = ut99_format::unr::UnrLevel::default();
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);
    state.selected_actor_index = Some(0);

    // 3. Snap actor to floor aligned
    let snapped = state.snap_selected_to_floor_aligned();
    assert!(snapped, "Actor should successfully snap to floor");
    let snapped_loc = state.loaded_unr.as_ref().unwrap().actors[0].location;
    // Actor default offset is floor_z + 16 = 32
    assert!((snapped_loc.z - 32.0).abs() < 1e-3, "Actor Z should be at 32.0, got {}", snapped_loc.z);

    // 4. Test Undo / Redo for Snap
    state.undo(); // Undo RotateActor
    state.undo(); // Undo MoveActor
    let restored_loc = state.loaded_unr.as_ref().unwrap().actors[0].location;
    assert!((restored_loc.z - 300.0).abs() < 1e-3, "Undo should restore initial location 300.0, got {}", restored_loc.z);

    state.redo(); // Redo MoveActor
    let redone_loc = state.loaded_unr.as_ref().unwrap().actors[0].location;
    assert!((redone_loc.z - 32.0).abs() < 1e-3, "Redo should restore snapped location 32.0, got {}", redone_loc.z);
}

#[test]
fn test_actor_properties_dialog_state_and_editing() {
    let mut state = EditorState::default();
    assert!(!state.show_actor_properties);

    // 1. Toggle dialog state
    state.show_actor_properties = true;
    assert!(state.show_actor_properties);

    // 2. Load actor with lighting and tagged properties
    let mut actor = ut99_format::unr::actor::ActorRecord::default();
    actor.name = "TestSpotlight".into();
    actor.class_name = "Engine.Spotlight".into();
    actor.location = Vec3::new(100.0, 200.0, 300.0);
    actor.rotation = (1024, 2048, 4096);
    actor.light_brightness = 192;
    actor.light_radius = 64;
    actor.light_hue = 150;
    actor.light_saturation = 32;

    actor.properties.insert(
        "bStatic".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "bStatic".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Bool(true),
        },
    );
    actor.properties.insert(
        "DrawScale".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "DrawScale".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Float(2.5),
        },
    );

    let mut unr = ut99_format::unr::UnrLevel::default();
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);
    state.selected_actor_index = Some(0);

    // Verify actor can be queried and edited
    let a = &mut state.loaded_unr.as_mut().unwrap().actors[0];
    assert_eq!(a.name, "TestSpotlight");
    assert_eq!(a.class_name, "Engine.Spotlight");
    assert_eq!(a.light_brightness, 192);
    assert_eq!(a.properties.len(), 2);

    // Modify a property as would happen in the properties window
    a.light_brightness = 255;
    if let Some(prop) = a.properties.get_mut("DrawScale") {
        if let ut99_format::unr::tagged_property::TaggedValue::Float(ref mut val) = prop.value {
            *val = 3.0;
        }
    }

    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].light_brightness, 255);
    if let ut99_format::unr::tagged_property::TaggedValue::Float(val) = state.loaded_unr.as_ref().unwrap().actors[0].properties["DrawScale"].value {
        assert_eq!(val, 3.0);
    } else {
        panic!("DrawScale property should be a Float");
    }
}

#[test]
fn test_radii_view_and_collision_cylinder_bounds() {
    let mut state = EditorState::default();
    assert!(!state.show_actor_radii);

    // Toggle Radii View state
    state.show_actor_radii = true;
    assert!(state.show_actor_radii);

    // Load actors with various collision properties
    let mut player_start = ut99_format::unr::actor::ActorRecord::default();
    player_start.name = "PlayerStart0".into();
    player_start.class_name = "Botpack.TournamentPlayerStart".into();
    player_start.location = Vec3::new(0.0, 0.0, 100.0);

    let mut custom_trigger = ut99_format::unr::actor::ActorRecord::default();
    custom_trigger.name = "Trigger0".into();
    custom_trigger.class_name = "Engine.Trigger".into();
    custom_trigger.location = Vec3::new(200.0, 0.0, 50.0);
    custom_trigger.properties.insert(
        "CollisionRadius".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "CollisionRadius".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Float(80.0),
        },
    );
    custom_trigger.properties.insert(
        "CollisionHeight".to_string(),
        ut99_format::unr::tagged_property::TaggedProperty {
            name: "CollisionHeight".to_string(),
            array_index: 0,
            value: ut99_format::unr::tagged_property::TaggedValue::Float(60.0),
        },
    );

    let mut unr = ut99_format::unr::UnrLevel::default();
    unr.actors.push(player_start);
    unr.actors.push(custom_trigger);
    state.loaded_unr = Some(unr);

    // Verify PlayerStart default collision bounds
    let ps = &state.loaded_unr.as_ref().unwrap().actors[0];
    assert!(ps.class_name.contains("PlayerStart"));

    // Verify Trigger custom collision properties
    let tr = &state.loaded_unr.as_ref().unwrap().actors[1];
    if let ut99_format::unr::tagged_property::TaggedValue::Float(rad) = tr.properties["CollisionRadius"].value {
        assert_eq!(rad, 80.0);
    } else {
        panic!("CollisionRadius should be Float");
    }
    if let ut99_format::unr::tagged_property::TaggedValue::Float(h) = tr.properties["CollisionHeight"].value {
        assert_eq!(h, 60.0);
    } else {
        panic!("CollisionHeight should be Float");
    }
}

#[test]
fn test_level_doctor_diagnostic_markers_and_selection() {
    let mut state = EditorState::default();
    assert!(state.show_diagnostic_markers);
    assert_eq!(state.selected_issue_index, None);

    // Add diagnostic issues with 3D positions
    state.map_audit_issues.push(ut99_bsp::MapIssue {
        severity: ut99_bsp::IssueSeverity::Error,
        category: "Geometry".to_string(),
        description: "Degenerate coplanar face".to_string(),
        object_name: "Brush_3".to_string(),
        location: Some(Vec3::new(128.0, 256.0, 64.0)),
        auto_fixable: true,
    });
    state.map_audit_issues.push(ut99_bsp::MapIssue {
        severity: ut99_bsp::IssueSeverity::Warning,
        category: "Texture".to_string(),
        description: "Missing texture assignment".to_string(),
        object_name: "Brush_4".to_string(),
        location: Some(Vec3::new(512.0, 512.0, 128.0)),
        auto_fixable: true,
    });

    assert_eq!(state.map_audit_issues.len(), 2);

    // Select first issue
    state.selected_issue_index = Some(0);
    assert_eq!(state.selected_issue_index, Some(0));

    // Verify camera focus math for selected issue
    let issue_loc = state.map_audit_issues[0].location.unwrap();
    state.camera.position = issue_loc - state.camera.forward() * 300.0;
    let dist_to_issue = (state.camera.position - issue_loc).length();
    assert!((dist_to_issue - 300.0).abs() < 1e-3);

    // Test toggle of diagnostic markers
    state.show_diagnostic_markers = false;
    assert!(!state.show_diagnostic_markers);

    // Running an audit clears selection
    state.run_level_doctor_audit();
    assert_eq!(state.selected_issue_index, None);
}

#[test]
fn test_clipboard_copy_cut_paste_and_cascading() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create a brush and an actor
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    let mut unr = ut99_format::unr::UnrLevel::default();
    let mut actor = ut99_format::unr::actor::ActorRecord::default();
    actor.name = "Light0".into();
    actor.class_name = "Engine.Light".into();
    actor.location = Vec3::new(100.0, 200.0, 300.0);
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);

    // 2. Select brush and copy
    state.selected_brush_index = Some(0);
    state.selected_actor_index = None;
    let (b_count, a_count) = state.copy_selected_to_clipboard();
    assert_eq!(b_count, 1);
    assert_eq!(a_count, 0);
    assert_eq!(state.clipboard.brushes.len(), 1);

    // 3. Paste brush (with grid offset 64)
    state.grid_step = 64.0;
    state.snap_to_grid = true;
    let (pasted_b, pasted_a) = state.paste_from_clipboard();
    assert_eq!(pasted_b, 1);
    assert_eq!(pasted_a, 0);
    assert_eq!(state.world_brushes.len(), 2);
    assert_eq!(state.world_brushes[1].location, Vec3::new(64.0, 64.0, 0.0));
    assert_eq!(state.selected_brush_index, Some(1));

    // 4. Consecutive paste cascades location (+64 again)
    let (pasted_b2, _) = state.paste_from_clipboard();
    assert_eq!(pasted_b2, 1);
    assert_eq!(state.world_brushes.len(), 3);
    assert_eq!(state.world_brushes[2].location, Vec3::new(128.0, 128.0, 0.0));

    // 5. Test Cut operation
    state.selected_brush_index = Some(2);
    let (cut_b, _) = state.cut_selected_to_clipboard();
    assert_eq!(cut_b, 1);
    assert_eq!(state.world_brushes.len(), 2);

    // 6. Test undoing the cut
    state.undo();
    assert_eq!(state.world_brushes.len(), 3);

    // 7. Multi-selection copy & paste (1 brush + 1 actor)
    state.clear_multi_selection();
    state.multi_selected_brushes.insert(0);
    state.multi_selected_actors.insert(0);
    let (mb, ma) = state.copy_selected_to_clipboard();
    assert_eq!(mb, 1);
    assert_eq!(ma, 1);

    let (pb, pa) = state.paste_from_clipboard();
    assert_eq!(pb, 1);
    assert_eq!(pa, 1);
    assert_eq!(state.world_brushes.len(), 4);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);
}

#[test]
fn test_selection_operations_and_inversion() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create 3 world brushes
    for i in 0..3 {
        state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));
        state.builder_brush.location = Vec3::new(i as f32 * 100.0, 0.0, 0.0);
        state.csg_add();
    }
    assert_eq!(state.world_brushes.len(), 3);

    // Create 2 actors
    let mut unr = ut99_format::unr::UnrLevel::default();
    let mut act0 = ut99_format::unr::actor::ActorRecord::default();
    act0.name = "Light0".into();
    let mut act1 = ut99_format::unr::actor::ActorRecord::default();
    act1.name = "Light1".into();
    unr.actors.push(act0);
    unr.actors.push(act1);
    state.loaded_unr = Some(unr);

    // 1. Select All
    state.select_all();
    assert_eq!(state.multi_selected_brushes.len(), 3);
    assert_eq!(state.multi_selected_actors.len(), 2);
    assert!(state.is_brush_selected(0));
    assert!(state.is_brush_selected(1));
    assert!(state.is_brush_selected(2));
    assert!(state.is_actor_selected(0));
    assert!(state.is_actor_selected(1));

    // 2. Deselect All
    state.deselect_all();
    assert_eq!(state.multi_selected_brushes.len(), 0);
    assert_eq!(state.multi_selected_actors.len(), 0);
    assert_eq!(state.selected_brush_index, None);
    assert_eq!(state.selected_actor_index, None);

    // 3. Select All Brushes only
    state.select_all_brushes();
    assert_eq!(state.multi_selected_brushes.len(), 3);
    assert_eq!(state.multi_selected_actors.len(), 0);

    // 4. Invert Selection -> Should select 0 brushes and 2 actors
    state.invert_selection();
    assert_eq!(state.multi_selected_brushes.len(), 0);
    assert_eq!(state.multi_selected_actors.len(), 2);
    assert!(state.is_actor_selected(0));
    assert!(state.is_actor_selected(1));
}

#[test]
fn test_brush_mirroring_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create brush (Face 4 is Left face with normal -X)
    state.set_builder_cube(Vec3::new(128.0, 64.0, 32.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    state.selected_brush_index = Some(0);
    // Face 4 has outward normal pointing along -X
    let orig_normal = state.world_brushes[0].polys[4].plane.normal;
    assert!((orig_normal.x - (-1.0)).abs() < 1e-4);

    // Make brush slightly asymmetric to test coordinate reflection
    state.world_brushes[0].polys[4].vertices[0].x += 25.0;
    let modified_v0 = state.world_brushes[0].polys[4].vertices[0];

    // Mirror across X
    state.mirror_selected(ut99_editor::editor_state::MirrorAxis::X);

    // After reflection across X and winding reversal, normal now points along +X
    let mirrored_normal = state.world_brushes[0].polys[4].plane.normal;
    assert!((mirrored_normal.x - 1.0).abs() < 1e-4);
    assert!((mirrored_normal.length() - 1.0).abs() < 1e-4);

    // Test Undo of Mirror
    state.undo();
    assert_eq!(state.world_brushes[0].polys[4].vertices[0], modified_v0);
    assert_eq!(state.world_brushes[0].polys[4].plane.normal, orig_normal);

    // Test Redo of Mirror
    state.redo();
    assert_eq!(state.world_brushes[0].polys[4].plane.normal, mirrored_normal);
}

#[test]
fn test_snap_selected_to_grid_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.grid_step = 64.0;

    // 1. Single Brush Snapping
    state.set_builder_cube(Vec3::new(128.0, 128.0, 64.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    let initial_loc = Vec3::new(123.4, 250.0, -15.0);
    state.world_brushes[0].location = initial_loc;
    state.selected_brush_index = Some(0);
    state.selected_actor_index = None;

    let snapped = state.snap_selected_to_grid();
    assert!(snapped);
    let expected_snapped = Vec3::new(128.0, 256.0, 0.0);
    assert_eq!(state.world_brushes[0].location, expected_snapped);

    // Test Undo
    state.undo();
    assert_eq!(state.world_brushes[0].location, initial_loc);

    // Test Redo
    state.redo();
    assert_eq!(state.world_brushes[0].location, expected_snapped);

    // Snapping again when already aligned should return false
    assert!(!state.snap_selected_to_grid());

    // 2. Single Actor Snapping
    let mut actor = ut99_format::unr::actor::ActorRecord::default();
    actor.name = "Light_0".into();
    actor.location = Vec3::new(55.0, 110.0, 200.0);
    let mut unr = ut99_format::unr::UnrLevel::default();
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);

    state.selected_brush_index = None;
    state.selected_actor_index = Some(0);

    let act_snapped = state.snap_selected_to_grid();
    assert!(act_snapped);
    let expected_actor = Vec3::new(64.0, 128.0, 192.0);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, expected_actor);

    // Test Undo of Actor snap
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, Vec3::new(55.0, 110.0, 200.0));

    // 3. Red Builder Brush Snapping when nothing is selected
    state.deselect_all();
    state.builder_brush.location = Vec3::new(17.0, 33.0, 99.0);
    let builder_snapped = state.snap_selected_to_grid();
    assert!(builder_snapped);
    assert_eq!(state.builder_brush.location, Vec3::new(0.0, 64.0, 128.0));

    // 4. Multi-selection Batch Snapping
    state.world_brushes[0].location = Vec3::new(10.0, 20.0, 30.0);
    state.loaded_unr.as_mut().unwrap().actors[0].location = Vec3::new(70.0, 80.0, 90.0);

    state.multi_selected_brushes.insert(0);
    state.multi_selected_actors.insert(0);

    let batch_count = state.snap_multi_selected_to_grid();
    assert_eq!(batch_count, 2);
    assert_eq!(state.world_brushes[0].location, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, Vec3::new(64.0, 64.0, 64.0));
}

#[test]
fn test_actor_and_brush_group_management() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create Brushes
    state.set_builder_cube(Vec3::splat(128.0));
    state.csg_add();
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 2);

    // 2. Create Actors
    let mut act0 = ut99_format::unr::actor::ActorRecord::default();
    act0.name = "Light0".into();
    act0.group = "Lighting".into();

    let mut act1 = ut99_format::unr::actor::ActorRecord::default();
    act1.name = "Path0".into();
    act1.group = "Navigation".into();

    let mut act2 = ut99_format::unr::actor::ActorRecord::default();
    act2.name = "PlayerStart0".into();
    act2.group = "".into();

    let mut unr = ut99_format::unr::UnrLevel::default();
    unr.actors = vec![act0, act1, act2].into();
    state.loaded_unr = Some(unr);

    // 3. Assign Brush 0 to Architecture group
    state.selected_brush_index = Some(0);
    state.selected_actor_index = None;
    let (a_cnt, b_cnt) = state.set_selected_objects_group("Architecture");
    assert_eq!(a_cnt, 0);
    assert_eq!(b_cnt, 1);

    // 4. Verify get_map_groups
    let groups = state.get_map_groups();
    assert_eq!(groups.len(), 3);
    assert_eq!(groups[0], ("Architecture".to_string(), 0, 1));
    assert_eq!(groups[1], ("Lighting".to_string(), 1, 0));
    assert_eq!(groups[2], ("Navigation".to_string(), 1, 0));

    // 5. Select by Group
    state.deselect_all();
    let (sel_a, sel_b) = state.select_objects_by_group("Lighting");
    assert_eq!(sel_a, 1);
    assert_eq!(sel_b, 0);
    assert!(state.is_actor_selected(0));
    assert!(!state.is_actor_selected(1));

    // 6. Deselect by Group
    let (desel_a, desel_b) = state.deselect_objects_by_group("Lighting");
    assert_eq!(desel_a, 1);
    assert_eq!(desel_b, 0);
    assert!(!state.is_actor_selected(0));

    // 7. Group Visibility Toggling
    assert!(state.is_actor_group_visible("Lighting"));
    assert!(state.is_brush_group_visible(0));

    state.toggle_group_visibility("Lighting");
    assert!(!state.is_actor_group_visible("Lighting"));
    assert!(state.is_brush_group_visible(0));

    state.toggle_group_visibility("Lighting");
    assert!(state.is_actor_group_visible("Lighting"));

    state.toggle_group_visibility("Architecture");
    assert!(!state.is_brush_group_visible(0));
    assert!(state.is_brush_group_visible(1)); // Brush 1 has no group, so visible

    // 8. Rename Group
    let (ren_a, ren_b) = state.rename_group("Lighting", "Illumination");
    assert_eq!(ren_a, 1);
    assert_eq!(ren_b, 0);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].group, "Illumination");

    // 9. Delete Group
    let (del_a, del_b) = state.delete_group("Architecture");
    assert_eq!(del_a, 0);
    assert_eq!(del_b, 1);
    assert_eq!(state.brush_groups.get(&0), None);
}

#[test]
fn test_bsp_geometry_rebuild_and_build_all() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Add 3 distinct world brushes
    state.set_builder_cube(Vec3::new(256.0, 256.0, 128.0));
    state.builder_brush.location = Vec3::new(0.0, 0.0, 64.0);
    state.csg_add();

    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.builder_brush.location = Vec3::new(300.0, 0.0, 64.0);
    state.csg_add();

    state.set_builder_cylinder(128.0, 48.0, 8);
    state.builder_brush.location = Vec3::new(-300.0, 0.0, 64.0);
    state.csg_add();

    assert_eq!(state.world_brushes.len(), 3);

    // 2. Build Geometry & BSP
    let report = state.build_geometry();
    assert!(report.node_count > 0, "BSP tree should contain nodes");
    assert!(report.poly_count >= 18, "3 brushes should produce at least 6+6+10 = 22 polys");
    assert!(report.max_depth > 0, "Tree depth must be positive");
    assert!(report.balance_ratio_pct > 0.0, "Balance efficiency must be calculated");
    assert!(state.last_bsp_report.is_some());

    // 3. Test Only Visible Brushes Filter
    state.bsp_config.only_visible_brushes = true;
    state.selected_brush_index = Some(2);
    state.set_selected_objects_group("HiddenPillars");
    state.toggle_group_visibility("HiddenPillars");
    assert!(!state.is_brush_group_visible(2));

    let filtered_report = state.build_geometry();
    assert!(filtered_report.poly_count < report.poly_count, "Filtered build should exclude hidden brush");

    // Restore visibility
    state.bsp_config.only_visible_brushes = false;
    state.toggle_group_visibility("HiddenPillars");

    // 4. Test Build All Pipeline
    assert!(!state.show_build_options_dialog);
    state.show_build_options_dialog = true;
    assert!(state.show_build_options_dialog);

    // Add a light and a navigation actor
    let mut unr = ut99_format::unr::UnrLevel::default();
    let mut light = ut99_format::unr::actor::ActorRecord::default();
    light.name = "Light0".into();
    light.class_name = "Light".into();
    light.location = Vec3::new(0.0, 0.0, 150.0);
    unr.actors.push(light);

    let mut start = ut99_format::unr::actor::ActorRecord::default();
    start.name = "PlayerStart0".into();
    start.class_name = "PlayerStart".into();
    start.location = Vec3::new(0.0, 0.0, 32.0);
    unr.actors.push(start);

    state.loaded_unr = Some(unr);

    state.build_all_complete();
    assert!(state.last_bsp_report.is_some());
    assert!(state.last_lighting_stats.is_some());
}

#[test]
fn test_brush_scaling_dialog_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Dialog state
    assert!(!state.show_scale_brush_dialog);
    state.show_scale_brush_dialog = true;
    assert!(state.show_scale_brush_dialog);

    // 2. Create a test box (100 x 200 x 50)
    state.set_builder_cube(Vec3::new(100.0, 200.0, 50.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    state.selected_brush_index = Some(0);
    let orig_bounds = state.world_brushes[0].world_bounds();
    let orig_dx = orig_bounds.1.x - orig_bounds.0.x;
    let orig_dy = orig_bounds.1.y - orig_bounds.0.y;
    let orig_dz = orig_bounds.1.z - orig_bounds.0.z;
    assert!((orig_dx - 100.0).abs() < 1e-3);
    assert!((orig_dy - 200.0).abs() < 1e-3);
    assert!((orig_dz - 50.0).abs() < 1e-3);

    // 3. Scale geometry: X x2.0, Y x0.5, Z x3.0
    let scale_factor = Vec3::new(2.0, 0.5, 3.0);
    let ok = state.scale_selected_brush_geometry(scale_factor, true);
    assert!(ok);

    let scaled_bounds = state.world_brushes[0].world_bounds();
    let scaled_dx = scaled_bounds.1.x - scaled_bounds.0.x;
    let scaled_dy = scaled_bounds.1.y - scaled_bounds.0.y;
    let scaled_dz = scaled_bounds.1.z - scaled_bounds.0.z;
    assert!((scaled_dx - 200.0).abs() < 1e-3, "dx should be 200, got {}", scaled_dx);
    assert!((scaled_dy - 100.0).abs() < 1e-3, "dy should be 100, got {}", scaled_dy);
    assert!((scaled_dz - 150.0).abs() < 1e-3, "dz should be 150, got {}", scaled_dz);

    // 4. Test Undo & Redo
    state.undo();
    let undone_bounds = state.world_brushes[0].world_bounds();
    let undone_dx = undone_bounds.1.x - undone_bounds.0.x;
    let undone_dy = undone_bounds.1.y - undone_bounds.0.y;
    let undone_dz = undone_bounds.1.z - undone_bounds.0.z;
    assert!((undone_dx - 100.0).abs() < 1e-3, "Undo should restore dx to 100");
    assert!((undone_dy - 200.0).abs() < 1e-3, "Undo should restore dy to 200");
    assert!((undone_dz - 50.0).abs() < 1e-3, "Undo should restore dz to 50");

    state.redo();
    let redone_bounds = state.world_brushes[0].world_bounds();
    let redone_dx = redone_bounds.1.x - redone_bounds.0.x;
    assert!((redone_dx - 200.0).abs() < 1e-3, "Redo should restore scaled dx to 200");

    // 5. Test Red Builder Brush scaling when no world brush is selected
    state.selected_brush_index = None;
    state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));
    let bb_orig_bounds = state.builder_brush.world_bounds();
    assert!(((bb_orig_bounds.1.x - bb_orig_bounds.0.x) - 64.0).abs() < 1e-3);

    state.scale_selected_brush_geometry(Vec3::splat(2.0), false);
    let bb_scaled_bounds = state.builder_brush.world_bounds();
    assert!(((bb_scaled_bounds.1.x - bb_scaled_bounds.0.x) - 128.0).abs() < 1e-3);

    // 6. Test negative factor (mirror reflection + winding reversal)
    let left_normal_before = state.builder_brush.polys[4].plane.normal;
    state.scale_selected_brush_geometry(Vec3::new(-1.0, 1.0, 1.0), false);
    let left_normal_after = state.builder_brush.polys[4].plane.normal;
    assert!((left_normal_after.x + left_normal_before.x).abs() < 1e-4, "Normal X should invert sign");

    // 7. Test transform scaling mode
    state.selected_brush_index = Some(0);
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);
    state.scale_selected_transform(Vec3::new(1.5, 2.0, 0.5));
    assert_eq!(state.world_brushes[0].scale, Vec3::new(1.5, 2.0, 0.5));
}

#[test]
fn test_find_actors_dialog_and_focus() {
    let mut state = EditorState::default();

    // 1. Dialog state
    assert!(!state.show_find_actors_dialog);
    state.show_find_actors_dialog = true;
    assert!(state.show_find_actors_dialog);

    // 2. Populate level with distinct actors
    let mut unr = ut99_format::unr::UnrLevel::default();

    let mut light = ut99_format::unr::actor::ActorRecord::default();
    light.name = "Spotlight_Base".into();
    light.class_name = "Engine.Spotlight".into();
    light.tag = "MainHallLight".into();
    light.group = "RedBase".into();
    light.location = Vec3::new(100.0, 200.0, 300.0);
    unr.actors.push(light);

    let mut node = ut99_format::unr::actor::ActorRecord::default();
    node.name = "PathNode_Corner".into();
    node.class_name = "Engine.PathNode".into();
    node.group = "BotRoutes".into();
    node.location = Vec3::new(500.0, -100.0, 50.0);
    unr.actors.push(node);

    let mut trigger = ut99_format::unr::actor::ActorRecord::default();
    trigger.name = "Trigger_SecretDoor".into();
    trigger.class_name = "Engine.Trigger".into();
    trigger.event = "OpenVaultGate".into();
    trigger.location = Vec3::new(-300.0, 400.0, 20.0);
    unr.actors.push(trigger);

    let mut weapon = ut99_format::unr::actor::ActorRecord::default();
    weapon.name = "UT_FlakCannon_Spawn".into();
    weapon.class_name = "Botpack.UT_FlakCannon".into();
    weapon.tag = "PowerWeapon".into();
    weapon.group = "RedBase".into();
    weapon.location = Vec3::new(120.0, 250.0, 10.0);
    unr.actors.push(weapon);

    state.loaded_unr = Some(unr);

    // 3. Search by substring across various fields
    // By class substring
    let matches_flak = state.find_actors_matching("flak", ut99_editor::editor_state::FindActorCategory::All);
    assert_eq!(matches_flak.len(), 1);
    assert_eq!(matches_flak[0].name, "UT_FlakCannon_Spawn");

    // By event substring
    let matches_vault = state.find_actors_matching("openvault", ut99_editor::editor_state::FindActorCategory::All);
    assert_eq!(matches_vault.len(), 1);
    assert_eq!(matches_vault[0].name, "Trigger_SecretDoor");

    // By group substring
    let matches_group = state.find_actors_matching("redbase", ut99_editor::editor_state::FindActorCategory::All);
    assert_eq!(matches_group.len(), 2);

    // 4. Category filtering
    let lights = state.find_actors_matching("", ut99_editor::editor_state::FindActorCategory::Lights);
    assert_eq!(lights.len(), 1);
    assert_eq!(lights[0].name, "Spotlight_Base");

    let navs = state.find_actors_matching("", ut99_editor::editor_state::FindActorCategory::Navigation);
    assert_eq!(navs.len(), 1);
    assert_eq!(navs[0].name, "PathNode_Corner");

    let invs = state.find_actors_matching("", ut99_editor::editor_state::FindActorCategory::Inventory);
    assert_eq!(invs.len(), 1);
    assert_eq!(invs[0].name, "UT_FlakCannon_Spawn");

    // 5. Select and Focus Actor
    let ok = state.select_and_focus_actor(1);
    assert!(ok);
    assert_eq!(state.selected_actor_index, Some(1));
    // Verify camera focused towards the target location
    let target_loc = Vec3::new(500.0, -100.0, 50.0);
    let cam_dist = state.camera.position.distance(target_loc);
    assert!((cam_dist - 400.0).abs() < 1.0, "Camera should be framed 400uu from target, got {}", cam_dist);

    // 6. Multi-select all matches
    let selected_count = state.select_all_matching_actors("redbase", ut99_editor::editor_state::FindActorCategory::All);
    assert_eq!(selected_count, 2);
    assert!(state.is_actor_selected(0));
    assert!(state.is_actor_selected(3));
}

#[test]
fn test_brush_shearing_dialog_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Dialog state
    assert!(!state.show_shear_brush_dialog);
    state.show_shear_brush_dialog = true;
    assert!(state.show_shear_brush_dialog);

    // 2. Create a test cube (128 x 128 x 128)
    state.set_builder_cube(Vec3::splat(128.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    state.selected_brush_index = Some(0);

    let (min_before, max_before) = state.world_brushes[0].world_bounds();
    assert!((min_before.z - (-64.0)).abs() < 1e-3);
    assert!((max_before.z - 64.0).abs() < 1e-3);

    // 3. Shear X based on Z height (factor 1.0 -> 45° slope), anchored to base
    let ok = state.shear_selected(ut99_editor::editor_state::ShearAxis::XZ, 1.0, true);
    assert!(ok);

    let (min_after, max_after) = state.world_brushes[0].world_bounds();
    // At bottom (z = -64), shift is 0, so min.x remains -64
    assert!((min_after.x - (-64.0)).abs() < 1e-3, "Base X min should stay -64, got {}", min_after.x);
    // At top (z = +64), delta_z is 128, shift is +128, so max.x becomes +64 + 128 = 192
    assert!((max_after.x - 192.0).abs() < 1e-3, "Top X max should become 192, got {}", max_after.x);

    // 4. Test Undo / Redo
    state.undo();
    let (_, max_undone) = state.world_brushes[0].world_bounds();
    assert!((max_undone.x - 64.0).abs() < 1e-3, "Undo should restore X max to 64, got {}", max_undone.x);

    state.redo();
    let (_, max_redone) = state.world_brushes[0].world_bounds();
    assert!((max_redone.x - 192.0).abs() < 1e-3, "Redo should restore X max to 192, got {}", max_redone.x);

    // 5. Test Shearing Active Red Builder Brush
    state.selected_brush_index = None;
    state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));
    let ok_bb = state.shear_selected(ut99_editor::editor_state::ShearAxis::YZ, 0.5, true);
    assert!(ok_bb);
    let (_, max_bb) = state.builder_brush.world_bounds();
    // Height is 64, shift at top is 64 * 0.5 = 32, max.y becomes 32 + 32 = 64
    assert!((max_bb.y - 64.0).abs() < 1e-3);
}

#[test]
fn test_actor_alignment_and_distribution() {
    let mut state = EditorState::default();

    // 1. Dialog state
    assert!(!state.show_actor_align_dialog);
    state.show_actor_align_dialog = true;
    assert!(state.show_actor_align_dialog);

    // 2. Setup level with 3 actors at distinct positions
    let mut unr = ut99_format::unr::UnrLevel::default();

    let mut a0 = ut99_format::unr::actor::ActorRecord::default();
    a0.name = "Light0".into();
    a0.location = Vec3::new(100.0, 50.0, 10.0);
    unr.actors.push(a0);

    let mut a1 = ut99_format::unr::actor::ActorRecord::default();
    a1.name = "Light1".into();
    a1.location = Vec3::new(300.0, 150.0, 10.0);
    unr.actors.push(a1);

    let mut a2 = ut99_format::unr::actor::ActorRecord::default();
    a2.name = "Light2".into();
    a2.location = Vec3::new(500.0, -20.0, 10.0);
    unr.actors.push(a2);

    state.loaded_unr = Some(unr);

    // Multi-select all 3 actors
    state.clear_multi_selection();
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);
    state.multi_selected_actors.insert(2);

    // 3. Test Align to Center (Average) on Y: (50 + 150 - 20) / 3 = 60.0
    let aligned = state.align_selected_actors(ut99_editor::editor_state::AlignAxis::Y, ut99_editor::editor_state::AlignMode::Center);
    assert_eq!(aligned, 3);

    let actors = &state.loaded_unr.as_ref().unwrap().actors;
    assert!((actors[0].location.y - 60.0).abs() < 1e-3);
    assert!((actors[1].location.y - 60.0).abs() < 1e-3);
    assert!((actors[2].location.y - 60.0).abs() < 1e-3);

    // 4. Test Undo / Redo for Alignment
    state.undo();
    state.undo();
    state.undo();
    let restored = &state.loaded_unr.as_ref().unwrap().actors;
    assert!((restored[0].location.y - 50.0).abs() < 1e-3);
    assert!((restored[1].location.y - 150.0).abs() < 1e-3);
    assert!((restored[2].location.y - (-20.0)).abs() < 1e-3);

    // 5. Test Distribute Evenly along X: endpoints are 100 and 500
    // Intermediate actor 1 should move to 300.0
    state.loaded_unr.as_mut().unwrap().actors[1].location.x = 220.0; // Intentionally offset
    let distributed = state.distribute_selected_actors_evenly(ut99_editor::editor_state::AlignAxis::X);
    assert!(distributed > 0);

    let dist_actors = &state.loaded_unr.as_ref().unwrap().actors;
    assert!((dist_actors[0].location.x - 100.0).abs() < 1e-3);
    assert!((dist_actors[1].location.x - 300.0).abs() < 1e-3);
    assert!((dist_actors[2].location.x - 500.0).abs() < 1e-3);

    // 6. Test Fixed Spacing along X (spacing 64.0)
    let spaced = state.distribute_selected_actors_spacing(ut99_editor::editor_state::AlignAxis::X, 64.0);
    assert!(spaced > 0);
    let spaced_actors = &state.loaded_unr.as_ref().unwrap().actors;
    assert!((spaced_actors[0].location.x - 100.0).abs() < 1e-3);
    assert!((spaced_actors[1].location.x - 164.0).abs() < 1e-3);
    assert!((spaced_actors[2].location.x - 228.0).abs() < 1e-3);
}

#[test]
fn test_map_statistics_computation() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Dialog state
    assert!(!state.show_map_statistics_dialog);
    state.show_map_statistics_dialog = true;
    assert!(state.show_map_statistics_dialog);

    // 2. Add 2 solid brushes and 1 subtractive brush
    state.set_builder_cube(Vec3::splat(256.0));
    state.csg_add();
    state.csg_add();
    state.csg_subtract();
    assert_eq!(state.world_brushes.len(), 3);

    // 3. Add actors across categories
    let mut unr = ut99_format::unr::UnrLevel::default();

    let mut l0 = ut99_format::unr::actor::ActorRecord::default();
    l0.name = "Light0".into();
    l0.class_name = "Light".into();
    unr.actors.push(l0);

    let mut l1 = ut99_format::unr::actor::ActorRecord::default();
    l1.name = "Light1".into();
    l1.class_name = "Engine.Spotlight".into();
    unr.actors.push(l1);

    let mut p0 = ut99_format::unr::actor::ActorRecord::default();
    p0.name = "PathNode0".into();
    p0.class_name = "Engine.PathNode".into();
    unr.actors.push(p0);

    let mut p1 = ut99_format::unr::actor::ActorRecord::default();
    p1.name = "PlayerStart0".into();
    p1.class_name = "Engine.PlayerStart".into();
    unr.actors.push(p1);

    let mut w0 = ut99_format::unr::actor::ActorRecord::default();
    w0.name = "ShockRifle0".into();
    w0.class_name = "Botpack.ShockRifle".into();
    unr.actors.push(w0);

    let mut t0 = ut99_format::unr::actor::ActorRecord::default();
    t0.name = "Trigger0".into();
    t0.class_name = "Engine.Trigger".into();
    unr.actors.push(t0);

    state.loaded_unr = Some(unr);

    // 4. Compute statistics
    let stats = state.compute_map_statistics();
    assert_eq!(stats.total_brushes, 3);
    assert_eq!(stats.solid_brushes, 2);
    assert_eq!(stats.subtract_brushes, 1);
    assert_eq!(stats.total_actors, 6);
    assert_eq!(stats.lights_count, 2);
    assert_eq!(stats.nav_points_count, 2);
    assert_eq!(stats.weapons_pickups_count, 1);
    assert_eq!(stats.triggers_count, 1);
    assert!(stats.total_polys >= 18);
    assert!(stats.total_vertices >= 72);
}

#[test]
fn test_surface_selection_shortcuts_and_filtering() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Create two adjacent cubes:
    // Cube 0: location (-128, 0, 128), size (256, 256, 256) -> X spans [-256, 0]
    // Cube 1: location (128, 0, 128), size (256, 256, 256)  -> X spans [0, 256]
    // They share an edge/boundary at X=0.
    let mut cube0 = ut99_bsp::build_cube("Cube0", ut99_bsp::CsgOperation::Add, Vec3::splat(256.0));
    cube0.location = Vec3::new(-128.0, 0.0, 128.0);
    let mut cube1 = ut99_bsp::build_cube("Cube1", ut99_bsp::CsgOperation::Add, Vec3::splat(256.0));
    cube1.location = Vec3::new(128.0, 0.0, 128.0);

    state.world_brushes.push(cube0);
    state.world_brushes.push(cube1);

    assert_eq!(state.world_brushes.len(), 2);
    assert_eq!(state.selected_surfaces.len(), 0);

    // 2. Select All Surfaces (Shift+S)
    state.select_all_surfaces();
    assert_eq!(state.selected_surfaces.len(), 12); // 6 faces * 2 cubes

    // 3. Deselect All Surfaces (Shift+N)
    state.deselect_all_surfaces();
    assert_eq!(state.selected_surfaces.len(), 0);

    // 4. Select single surface and invert selection
    state.select_surface(0, 0, false);
    assert_eq!(state.selected_surfaces.len(), 1);
    assert!(state.is_surface_selected(0, 0));

    state.invert_surface_selection();
    assert_eq!(state.selected_surfaces.len(), 11);
    assert!(!state.is_surface_selected(0, 0));
    assert!(state.is_surface_selected(0, 1));

    // 5. Select Surfaces on Brush
    state.deselect_all_surfaces();
    state.selected_brush_index = Some(0);
    state.select_surfaces_on_selected_brushes();
    assert_eq!(state.selected_surfaces.len(), 6);
    for p in 0..6 {
        assert!(state.is_surface_selected(0, p));
    }

    // 6. Matching Texture Selection (Shift+T)
    state.deselect_all_surfaces();
    state.world_brushes[0].polys[2].texture = "Indus1.SpecialFloor".to_string();
    state.world_brushes[1].polys[4].texture = "Indus1.SpecialFloor".to_string();

    state.select_surface(0, 2, false);
    assert_eq!(state.selected_surfaces.len(), 1);

    state.select_surfaces_matching_texture();
    assert!(state.is_surface_selected(0, 2));
    assert!(state.is_surface_selected(1, 4));
    assert_eq!(state.selected_surfaces.len(), 2);

    // 7. Coplanar Adjacent Selection (Shift+B)
    state.deselect_all_surfaces();
    let w_polys0 = state.world_brushes[0].to_world_polys();
    let top_idx0 = w_polys0.iter().position(|p| p.plane.normal.z > 0.9).expect("top face on cube 0");
    let w_polys1 = state.world_brushes[1].to_world_polys();
    let top_idx1 = w_polys1.iter().position(|p| p.plane.normal.z > 0.9).expect("top face on cube 1");

    state.select_surface(0, top_idx0, false);
    assert_eq!(state.selected_surfaces.len(), 1);

    state.select_surfaces_coplanar_adjacent();
    assert!(state.is_surface_selected(0, top_idx0));
    assert!(state.is_surface_selected(1, top_idx1), "Touching top face of Cube 1 must be selected via Shift+B");

    // 8. Adjacent Walls Selection (Shift+W)
    state.deselect_all_surfaces();
    let wall_idx0 = w_polys0.iter().position(|p| p.plane.normal.z.abs() < 0.1).expect("wall face on cube 0");
    state.select_surface(0, wall_idx0, false);
    state.select_surfaces_adjacent_walls();
    let selected_wall_count = (0..6).filter(|&p| state.is_surface_selected(0, p) && w_polys0[p].plane.normal.z.abs() < 0.1).count();
    assert!(selected_wall_count >= 3, "Adjacent vertical walls should be selected via Shift+W");
    assert!(!state.is_surface_selected(0, top_idx0), "Horizontal top face must not be selected as wall");

    // 9. Multi-surface manipulation with Undo/Redo
    state.deselect_all_surfaces();
    state.select_surface(0, 0, false);
    state.select_surface(0, 1, true);
    assert_eq!(state.selected_surfaces.len(), 2);

    let initial_base0 = state.world_brushes[0].polys[0].base;
    let initial_base1 = state.world_brushes[0].polys[1].base;
    let initial_base2 = state.world_brushes[0].polys[2].base;

    state.pan_selected_brush_texture(64.0, 32.0);
    assert_ne!(state.world_brushes[0].polys[0].base, initial_base0);
    assert_ne!(state.world_brushes[0].polys[1].base, initial_base1);
    assert_eq!(state.world_brushes[0].polys[2].base, initial_base2, "Unselected face must remain untouched");

    // Undo
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].base, initial_base0);
    assert_eq!(state.world_brushes[0].polys[1].base, initial_base1);

    // Redo
    state.redo();
    assert_ne!(state.world_brushes[0].polys[0].base, initial_base0);
    assert_ne!(state.world_brushes[0].polys[1].base, initial_base1);
}

#[test]
fn test_find_and_replace_textures_and_undo() {
    let mut state = EditorState::default();

    // Setup 2 world brushes with diverse textures
    let mut cube0 = ut99_bsp::build_cube("Cube0", ut99_bsp::CsgOperation::Add, Vec3::splat(128.0));
    for (i, p) in cube0.polys.iter_mut().enumerate() {
        if i < 4 {
            p.texture = "GenEarth.Dirt1".to_string();
        } else {
            p.texture = "GenEarth.Rock2".to_string();
        }
    }

    let mut cube1 = ut99_bsp::build_cube("Cube1", ut99_bsp::CsgOperation::Add, Vec3::splat(128.0));
    for (i, p) in cube1.polys.iter_mut().enumerate() {
        if i < 3 {
            p.texture = "SpaceFX.Metal_Wall_01".to_string();
        } else {
            p.texture = "SpaceFX.Metal_Floor_02".to_string();
        }
    }

    state.world_brushes = vec![cube0, cube1].into();

    // 1. Preview Counts
    let preview_exact = state.preview_replace_textures_count(
        "Dirt1",
        TextureReplaceScope::EntireLevel,
        TextureMatchMode::ExactName,
    );
    assert_eq!(preview_exact, 4, "Should match 4 surfaces by terminal exact name");

    let preview_full = state.preview_replace_textures_count(
        "GenEarth.Dirt1",
        TextureReplaceScope::EntireLevel,
        TextureMatchMode::ExactName,
    );
    assert_eq!(preview_full, 4, "Should match 4 surfaces by full package.texture name");

    let preview_none = state.preview_replace_textures_count(
        "NonExistent",
        TextureReplaceScope::EntireLevel,
        TextureMatchMode::ExactName,
    );
    assert_eq!(preview_none, 0);

    // 2. EntireLevel ExactName Replacement & Undo / Redo
    let replaced_count = state.replace_textures(
        "GenEarth.Dirt1",
        "Ancient.Sand",
        TextureReplaceScope::EntireLevel,
        TextureMatchMode::ExactName,
    );
    assert_eq!(replaced_count, 4);
    assert_eq!(state.world_brushes[0].polys[0].texture, "Ancient.Sand");
    assert_eq!(state.world_brushes[0].polys[3].texture, "Ancient.Sand");
    assert_eq!(state.world_brushes[0].polys[4].texture, "GenEarth.Rock2");

    // Undo
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture, "GenEarth.Dirt1");
    assert_eq!(state.world_brushes[0].polys[3].texture, "GenEarth.Dirt1");

    // Redo
    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].texture, "Ancient.Sand");
    assert_eq!(state.world_brushes[0].polys[3].texture, "Ancient.Sand");

    // 3. SelectedBrushes Scope with Substring Match
    state.selected_brush_index = Some(1);
    let preview_brush1 = state.preview_replace_textures_count(
        "Metal",
        TextureReplaceScope::SelectedBrushes,
        TextureMatchMode::ContainsSubstring,
    );
    assert_eq!(preview_brush1, 6, "All 6 faces of Cube1 contain 'Metal'");

    let rep_brush1 = state.replace_textures(
        "Metal",
        "Alloy",
        TextureReplaceScope::SelectedBrushes,
        TextureMatchMode::ContainsSubstring,
    );
    assert_eq!(rep_brush1, 6);
    assert_eq!(state.world_brushes[1].polys[0].texture, "SpaceFX.Alloy_Wall_01");
    assert_eq!(state.world_brushes[1].polys[3].texture, "SpaceFX.Alloy_Floor_02");

    // 4. SelectedSurfaces Scope
    state.deselect_all();
    state.select_surface(0, 4, false); // Rock2 on Cube0 Face 4
    assert_eq!(state.selected_surfaces.len(), 1);

    let rep_surf = state.replace_textures(
        "Rock2",
        "Lava.Fire01",
        TextureReplaceScope::SelectedSurfaces,
        TextureMatchMode::ExactName,
    );
    assert_eq!(rep_surf, 1);
    assert_eq!(state.world_brushes[0].polys[4].texture, "Lava.Fire01");
    assert_eq!(state.world_brushes[0].polys[5].texture, "GenEarth.Rock2", "Face 5 must remain Rock2");

    // 5. PackageOnly Mode
    let rep_pkg = state.replace_textures(
        "SpaceFX",
        "SciFi",
        TextureReplaceScope::EntireLevel,
        TextureMatchMode::PackageOnly,
    );
    assert_eq!(rep_pkg, 6);
    assert_eq!(state.world_brushes[1].polys[0].texture, "SciFi.Alloy_Wall_01");
    assert_eq!(state.world_brushes[1].polys[3].texture, "SciFi.Alloy_Floor_02");

    // 6. Dialog and Swap state
    state.show_replace_textures_dialog = true;
    state.replace_texture_find = "OldTexture".to_string();
    state.replace_texture_with = "NewTexture".to_string();
    std::mem::swap(&mut state.replace_texture_find, &mut state.replace_texture_with);
    assert_eq!(state.replace_texture_find, "NewTexture");
    assert_eq!(state.replace_texture_with, "OldTexture");
}

#[test]
fn test_continuous_wall_strip_alignment_and_pivot_snapping() {
    let mut state = EditorState::default();

    // 1. Setup brush with location (100.0, 200.0, 300.0) and size 128 (half-extent 64)
    // Bounds will be: X in [36, 164], Y in [136, 264], Z in [236, 364]
    let mut cube0 = ut99_bsp::build_cube("Cube0", ut99_bsp::CsgOperation::Add, Vec3::splat(128.0));
    cube0.location = Vec3::new(100.0, 200.0, 300.0);
    state.world_brushes = vec![cube0].into();
    state.selected_brush_index = Some(0);

    let (orig_min, orig_max) = state.world_brushes[0].world_bounds();
    assert!((orig_min.x - 36.0).abs() < 1e-3);
    assert!((orig_max.x - 164.0).abs() < 1e-3);

    // 2. Snap Pivot to Nearest Vertex (Corner at 36, 136, 236)
    state.snap_selected_brush_pivot_to_nearest_vertex(Some(Vec3::new(36.0, 136.0, 236.0)));
    assert_eq!(state.world_brushes[0].location, Vec3::new(36.0, 136.0, 236.0));

    // Verify world geometry did NOT shift
    let (after_min, after_max) = state.world_brushes[0].world_bounds();
    assert!((after_min.x - orig_min.x).abs() < 1e-3);
    assert!((after_max.x - orig_max.x).abs() < 1e-3);

    // Undo Pivot Snap
    state.undo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(100.0, 200.0, 300.0));
    let (undo_min, _undo_max) = state.world_brushes[0].world_bounds();
    assert!((undo_min.x - orig_min.x).abs() < 1e-3);

    // Redo Pivot Snap
    state.redo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(36.0, 136.0, 236.0));

    // 3. Center Pivot to Bounding Box
    state.snap_brush_pivot_to_center();
    assert_eq!(state.world_brushes[0].location, Vec3::new(100.0, 200.0, 300.0));
    let (center_min, _center_max) = state.world_brushes[0].world_bounds();
    assert!((center_min.x - orig_min.x).abs() < 1e-3);

    // 4. Continuous Wall Strip Alignment across surfaces
    state.deselect_all();
    // Select 2 vertical wall faces on Cube0
    let w_polys = state.world_brushes[0].to_world_polys();
    let wall_indices: Vec<usize> = (0..w_polys.len())
        .filter(|&i| w_polys[i].plane.normal.z.abs() < 0.1)
        .take(2)
        .collect();
    assert_eq!(wall_indices.len(), 2);

    state.select_surface(0, wall_indices[0], false);
    state.select_surface(0, wall_indices[1], true);
    assert_eq!(state.selected_surfaces.len(), 2);

    state.align_selected_surfaces_continuous(1.0, 1.0);

    let poly0 = &state.world_brushes[0].polys[wall_indices[0]];
    let poly1 = &state.world_brushes[0].polys[wall_indices[1]];

    assert!(poly0.texture_u.length() > 1e-4);
    assert!(poly1.texture_u.length() > 1e-4);
    // Vertical V axis must be aligned with -Z (normalized scale: -1.0)
    assert!((poly0.texture_v.z + 1.0).abs() < 0.05);
    assert!((poly1.texture_v.z + 1.0).abs() < 0.05);

    // Undo surface alignment
    state.undo();
}

#[test]
fn test_surface_flags_presets_and_tag_event_wiring() {
    let mut state = EditorState::default();

    // ==========================================
    // 1. Surface Flags Presets
    // ==========================================
    let cube = ut99_bsp::build_cube("Cube0", ut99_bsp::CsgOperation::Add, Vec3::splat(128.0));
    state.world_brushes = vec![cube].into();
    state.selected_brush_index = Some(0);

    // Initial state: all flags are 0
    for poly in &state.world_brushes[0].polys {
        assert_eq!(poly.flags, 0);
    }

    // A. Apply Water Preset to whole selected brush
    state.apply_surface_flags_preset(SurfaceFlagsPreset::Water);
    for poly in &state.world_brushes[0].polys {
        assert_eq!(poly.flags, SurfaceFlagsPreset::Water.flags());
    }

    // B. Undo Water Preset
    state.undo();
    for poly in &state.world_brushes[0].polys {
        assert_eq!(poly.flags, 0);
    }

    // C. Redo Water Preset
    state.redo();
    for poly in &state.world_brushes[0].polys {
        assert_eq!(poly.flags, SurfaceFlagsPreset::Water.flags());
    }

    // D. Apply Lava/Slime Preset to selected single surface #0
    state.deselect_all();
    state.select_surface(0, 0, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::LavaSlime);
    assert_eq!(state.world_brushes[0].polys[0].flags, SurfaceFlagsPreset::LavaSlime.flags());
    // Face #1 retains Water flags
    assert_eq!(state.world_brushes[0].polys[1].flags, SurfaceFlagsPreset::Water.flags());

    // E. Apply GlassWindow to surface #1
    state.select_surface(0, 1, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::GlassWindow);
    assert_eq!(state.world_brushes[0].polys[1].flags, SurfaceFlagsPreset::GlassWindow.flags());

    // F. Apply MaskedGrate to surface #2
    state.select_surface(0, 2, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::MaskedGrate);
    assert_eq!(state.world_brushes[0].polys[2].flags, SurfaceFlagsPreset::MaskedGrate.flags());

    // G. Apply SkyBackdrop to surface #3
    state.select_surface(0, 3, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::SkyBackdrop);
    assert_eq!(state.world_brushes[0].polys[3].flags, SurfaceFlagsPreset::SkyBackdrop.flags());

    // H. Apply InvisibleBarrier to surface #4
    state.select_surface(0, 4, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::InvisibleBarrier);
    assert_eq!(state.world_brushes[0].polys[4].flags, SurfaceFlagsPreset::InvisibleBarrier.flags());

    // I. Apply ClearAll to surface #0
    state.select_surface(0, 0, false);
    state.apply_surface_flags_preset(SurfaceFlagsPreset::ClearAll);
    assert_eq!(state.world_brushes[0].polys[0].flags, 0);

    // Undo ClearAll returns face 0 to LavaSlime
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].flags, SurfaceFlagsPreset::LavaSlime.flags());

    // ==========================================
    // 2. Tag / Event Selection & Wiring
    // ==========================================
    let mut level = UnrLevel::default();

    // Actor 0: Trigger with Event "DoorOpen"
    let mut trigger1 = ActorRecord::default();
    trigger1.name = "Trigger0".to_string();
    trigger1.class_name = "Trigger".to_string();
    trigger1.tag = "EntranceTrigger".to_string();
    trigger1.event = "DoorOpen".to_string();
    level.actors.push(trigger1);

    // Actor 1: Mover Door with Tag "DoorOpen", Event "ActivateLights"
    let mut mover = ActorRecord::default();
    mover.name = "Mover0".to_string();
    mover.class_name = "Mover".to_string();
    mover.tag = "DoorOpen".to_string();
    mover.event = "ActivateLights".to_string();
    level.actors.push(mover);

    // Actor 2: Light 1 with Tag "ActivateLights"
    let mut light1 = ActorRecord::default();
    light1.name = "Light0".to_string();
    light1.class_name = "Light".to_string();
    light1.tag = "ActivateLights".to_string();
    level.actors.push(light1);

    // Actor 3: Light 2 with Tag "ActivateLights"
    let mut light2 = ActorRecord::default();
    light2.name = "Light1".to_string();
    light2.class_name = "Light".to_string();
    light2.tag = "ActivateLights".to_string();
    level.actors.push(light2);

    // Actor 4: Secondary Button with Event "DoorOpen"
    let mut trigger2 = ActorRecord::default();
    trigger2.name = "Trigger1".to_string();
    trigger2.class_name = "Trigger".to_string();
    trigger2.tag = "ConsoleButton".to_string();
    trigger2.event = "DoorOpen".to_string();
    level.actors.push(trigger2);

    // Actor 5: Unrelated Sound with Tag "Ambience"
    let mut sound = ActorRecord::default();
    sound.name = "SpecialEvent0".to_string();
    sound.class_name = "SpecialEvent".to_string();
    sound.tag = "Ambience".to_string();
    sound.event = "PlaySound".to_string();
    level.actors.push(sound);

    state.loaded_unr = Some(level);

    // A. Select Actors Matching Tag
    // Select Actor 2 (Light 1 with Tag "ActivateLights")
    state.selected_actor_index = Some(2);
    let matched_tags = state.select_actors_matching_tag();
    assert_eq!(matched_tags, 2, "Should match 2 actors with Tag 'ActivateLights'");
    assert!(state.multi_selected_actors.contains(&2));
    assert!(state.multi_selected_actors.contains(&3));
    assert_eq!(state.multi_selected_actors.len(), 2);

    // B. Select Actors Matching Event
    // Select Actor 0 (Trigger with Event "DoorOpen")
    state.deselect_all();
    state.selected_actor_index = Some(0);
    let matched_events = state.select_actors_matching_event();
    assert_eq!(matched_events, 2, "Should match 2 actors with Event 'DoorOpen' (0 and 4)");
    assert!(state.multi_selected_actors.contains(&0));
    assert!(state.multi_selected_actors.contains(&4));
    assert_eq!(state.multi_selected_actors.len(), 2);

    // C. Select Actors Targeted by Event (Event -> Tag)
    // Select Actor 0 (Trigger whose Event is "DoorOpen") -> Should select Mover 1 (whose Tag is "DoorOpen")
    state.deselect_all();
    state.selected_actor_index = Some(0);
    let targets_count = state.select_actors_targeted_by_event();
    assert_eq!(targets_count, 1, "Should find 1 target actor whose Tag is 'DoorOpen'");
    assert_eq!(state.selected_actor_index, Some(1));
    assert!(state.multi_selected_actors.contains(&1));

    // Follow the chain: Select Actor 1 (Mover whose Event is "ActivateLights") -> Targets should be Lights 2 & 3
    let next_targets = state.select_actors_targeted_by_event();
    assert_eq!(next_targets, 2, "Should find 2 lights whose Tag is 'ActivateLights'");
    assert!(state.multi_selected_actors.contains(&2));
    assert!(state.multi_selected_actors.contains(&3));

    // D. Select Actors Triggering Tag (Event -> Tag, reverse direction)
    // Select Actor 1 (Mover with Tag "DoorOpen") -> Triggers are Actor 0 and Actor 4 (both have Event "DoorOpen")
    state.deselect_all();
    state.selected_actor_index = Some(1);
    let triggers_count = state.select_actors_triggering_tag();
    assert_eq!(triggers_count, 2, "Should find 2 triggers whose Event is 'DoorOpen'");
    assert!(state.multi_selected_actors.contains(&0));
    assert!(state.multi_selected_actors.contains(&4));
}

#[test]
fn test_extract_surface_to_builder_and_hollow_brush_and_uv_snapping() {
    let mut state = EditorState::default();

    // ==========================================
    // 1. Extract Selected Surface to Builder Brush
    // ==========================================
    let mut cube0 = ut99_bsp::build_cube("CubeWorld", ut99_bsp::CsgOperation::Add, Vec3::splat(256.0));
    cube0.location = Vec3::new(500.0, 500.0, 200.0);
    cube0.polys[0].texture = "WallTexture.Bricks01".to_string();
    state.world_brushes = vec![cube0].into();

    // Select surface 0 on brush 0
    state.select_surface(0, 0, false);
    let orig_builder_polys_count = state.builder_brush.polys.len();

    let success = state.extract_selected_surfaces_to_builder_brush();
    assert!(success);
    assert_eq!(state.builder_brush.polys.len(), 1, "Extracted sheet should have 1 polygon");
    assert_eq!(state.builder_brush.polys[0].texture, "WallTexture.Bricks01");

    // The builder brush must be positioned at the world location of the extracted surface
    assert!((state.builder_brush.location.z - 200.0).abs() < 256.0);

    // Undo extraction
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), orig_builder_polys_count);

    // Redo extraction
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), 1);
    assert_eq!(state.builder_brush.polys[0].texture, "WallTexture.Bricks01");

    // ==========================================
    // 2. Make Builder Brush Hollow
    // ==========================================
    state.builder_brush = ut99_bsp::build_cube("RedBuilder", ut99_bsp::CsgOperation::Add, Vec3::splat(256.0));
    assert_eq!(state.builder_brush.polys.len(), 6, "Standard cube has 6 outer faces");

    let hollow_success = state.make_builder_brush_hollow(16.0);
    assert!(hollow_success);
    assert_eq!(state.builder_brush.polys.len(), 12, "Hollow cube must have 12 faces (6 outer + 6 inner)");

    // Inner faces must have normals pointing inwards (opposite to outer faces)
    let outer_normal = state.builder_brush.polys[0].plane.normal;
    let inner_normal = state.builder_brush.polys[6].plane.normal;
    assert!((outer_normal.dot(inner_normal) + 1.0).abs() < 1e-3, "Inner normal must be opposite to outer normal");

    // Distance between corresponding outer and inner face plane should be 16.0
    let outer_v = state.builder_brush.polys[0].vertices[0];
    let inner_v = state.builder_brush.polys[6].vertices.last().unwrap();
    let dist = (outer_v - *inner_v).length();
    assert!((dist - 16.0).abs() < 1e-2, "Wall thickness between corresponding outer and inner vertices must be 16 uu");

    // Undo hollow
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), 6);

    // Redo hollow
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), 12);

    // ==========================================
    // 3. Reset Pan & Snap UV to Grid
    // ==========================================
    state.selected_brush_index = Some(0);
    state.deselect_all_surfaces();
    state.select_surface(0, 0, false);

    let v0 = state.world_brushes[0].polys[0].vertices[0];
    // Offset base by arbitrary non-grid amounts
    let u_dir = state.world_brushes[0].polys[0].texture_u.normalize();
    let v_dir = state.world_brushes[0].polys[0].texture_v.normalize();
    state.world_brushes[0].polys[0].base = v0 + u_dir * 17.3 + v_dir * 63.8;

    // Snap to 16px grid
    state.snap_selected_surfaces_uv_to_grid(16.0);
    let diff = state.world_brushes[0].polys[0].base - v0;
    let u_proj = diff.dot(u_dir);
    let v_proj = diff.dot(v_dir);
    assert!((u_proj - 16.0).abs() < 1e-2, "U projection must snap to 16.0");
    assert!((v_proj - 64.0).abs() < 1e-2, "V projection must snap to 64.0 (nearest multiple of 16)");

    // Reset Pan to Origin
    state.reset_selected_surfaces_pan();
    assert_eq!(state.world_brushes[0].polys[0].base, v0, "Base must match first vertex on reset pan");
}

#[test]
fn test_auto_wall_direction_alignment_and_multi_fit() {
    let mut state = EditorState::default();

    // 1. Build a diagonal wall at 45 degrees:
    // Vertices in plane X + Y = 0 (Normal: (0.7071, 0.7071, 0.0))
    let v0 = Vec3::new(0.0, 0.0, 0.0);
    let v1 = Vec3::new(100.0, -100.0, 0.0);
    let v2 = Vec3::new(100.0, -100.0, 200.0);
    let v3 = Vec3::new(0.0, 0.0, 200.0);

    let poly = ut99_bsp::Polygon::new(vec![v0, v1, v2, v3]).expect("Valid 45 deg wall poly");
    let mut brush = ut99_bsp::Brush::new("DiagonalWall", ut99_bsp::CsgOperation::Add);
    brush.polys = vec![poly];
    state.world_brushes = vec![brush].into();
    state.selected_brush_index = Some(0);

    // Apply WallAuto
    state.align_selected_brush_texture("wall_auto");

    let p = &state.world_brushes[0].polys[0];
    let norm = p.plane.normal.normalize();

    // U must be coplanar with the wall (n . u == 0)
    assert!(p.texture_u.dot(norm).abs() < 1e-4, "U vector must lie on wall plane");
    // U must be strictly horizontal (Z component == 0)
    assert!(p.texture_u.z.abs() < 1e-4, "U vector must be strictly horizontal");
    // U magnitude must be 1.0 (un-distorted natural 1:1 scale)
    assert!((p.texture_u.length() - 1.0).abs() < 1e-3, "U vector length must be 1.0");

    // V must point straight down along world Z (-1.0)
    assert_eq!(p.texture_v, Vec3::new(0.0, 0.0, -1.0));

    // Base must be at polygon origin
    assert_eq!(p.base, v0);

    // 2. Test Fit 2x2 and Fit 4x4
    state.fit_selected_brush_texture(2.0, 2.0);
    let fit2_u_len = state.world_brushes[0].polys[0].texture_u.length();
    let fit2_v_len = state.world_brushes[0].polys[0].texture_v.length();
    assert!(fit2_u_len > 0.01);
    assert!(fit2_v_len > 0.01);

    state.fit_selected_brush_texture(4.0, 4.0);
    let fit4_u_len = state.world_brushes[0].polys[0].texture_u.length();
    let fit4_v_len = state.world_brushes[0].polys[0].texture_v.length();
    // 4x4 repeats means twice as dense texture as 2x2 repeats
    assert!((fit4_u_len - fit2_u_len * 2.0).abs() < 1e-3);
    assert!((fit4_v_len - fit2_v_len * 2.0).abs() < 1e-3);
}

#[test]
fn test_mover_8_keyframes_cycling_and_duplication() {
    let mut state = EditorState::default();
    let loc = Vec3::new(100.0, 200.0, 0.0);
    state.builder_brush.location = loc;
    state.add_mover_from_builder();
    state.selected_mover_index = Some(0);

    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.selected_mover_index, Some(0));
    assert_eq!(state.movers[0].current_key, 0);
    assert_eq!(state.movers[0].brush.location, loc);

    // Cycle forward: 0 -> 1
    let next1 = state.cycle_selected_mover_key(true);
    assert_eq!(next1, Some(1));
    assert_eq!(state.movers[0].current_key, 1);
    assert_eq!(state.movers[0].brush.location, loc + Vec3::new(0.0, 0.0, 128.0));

    // Cycle forward: 1 -> 2 (uninitialized, auto-clones from Key 0)
    let next2 = state.cycle_selected_mover_key(true);
    assert_eq!(next2, Some(2));
    assert_eq!(state.movers[0].current_key, 2);
    assert!(state.movers[0].keyframes[2].is_some());
    assert_eq!(state.movers[0].brush.location, loc);

    // Save custom position for Key 2
    let custom_pos_2 = Vec3::new(500.0, 600.0, 300.0);
    state.save_selected_mover_current_key_pos(custom_pos_2, Vec3::ZERO);
    assert_eq!(state.movers[0].brush.location, custom_pos_2);

    // Duplicate Key 2 to Key 6
    let dup_ok = state.duplicate_selected_mover_key(6);
    assert!(dup_ok);
    assert_eq!(state.movers[0].keyframes[6].as_ref().unwrap().location, custom_pos_2);

    // Switch directly to Key 6
    state.set_selected_mover_key(6);
    assert_eq!(state.movers[0].current_key, 6);
    assert_eq!(state.movers[0].brush.location, custom_pos_2);

    // Cycle to Key 7
    state.cycle_selected_mover_key(true);
    assert_eq!(state.movers[0].current_key, 7);

    // Cycle forward from 7 wraps to 0
    state.cycle_selected_mover_key(true);
    assert_eq!(state.movers[0].current_key, 0);

    // Cycle backward from 0 wraps to 7
    state.cycle_selected_mover_key(false);
    assert_eq!(state.movers[0].current_key, 7);

    // Cycle backward from 7 goes to 6
    state.cycle_selected_mover_key(false);
    assert_eq!(state.movers[0].current_key, 6);
    assert_eq!(state.movers[0].brush.location, custom_pos_2);
}

#[test]
fn test_vertex_edit_edge_split_and_collapse_with_undo() {
    let mut state = EditorState::default();
    // Default builder brush is a 256x256x256 cube
    state.editor_mode = EditorMode::VertexEdit;

    let unique_before = state.get_active_brush_unique_vertices();
    assert_eq!(unique_before.len(), 8); // 8 corners of cube

    // Find two vertices that form a real edge on the builder brush
    let poly = &state.builder_brush.polys[0];
    let v0 = poly.vertices[0] + state.builder_brush.location;
    let v1 = poly.vertices[1] + state.builder_brush.location;

    let idx0 = unique_before.iter().position(|&v| v.distance_squared(v0) < 0.01).unwrap();
    let idx1 = unique_before.iter().position(|&v| v.distance_squared(v1) < 0.01).unwrap();

    state.selected_vertex_indices = vec![idx0, idx1];

    // 1. Split Edge on Builder Brush
    let split_ok = state.split_selected_edge();
    assert!(split_ok);

    let unique_after_split = state.get_active_brush_unique_vertices();
    assert_eq!(unique_after_split.len(), 9); // +1 new vertex added
    assert_eq!(state.selected_vertex_indices.len(), 1); // auto-selected new vertex!

    let expected_midpoint = (v0 + v1) * 0.5;
    let new_vert = unique_after_split[state.selected_vertex_indices[0]];
    assert!(new_vert.distance_squared(expected_midpoint) < 0.01);

    // Undo Split on Builder Brush
    state.undo();
    let unique_undone = state.get_active_brush_unique_vertices();
    assert_eq!(unique_undone.len(), 8);

    // Redo Split
    state.redo();
    let unique_redone = state.get_active_brush_unique_vertices();
    assert_eq!(unique_redone.len(), 9);

    // 2. Collapse Edge on Builder Brush
    // Let's select the newly inserted midpoint and one of the original edge endpoints
    let mid_idx = unique_redone.iter().position(|&v| v.distance_squared(expected_midpoint) < 0.01).unwrap();
    let end_idx = unique_redone.iter().position(|&v| v.distance_squared(v0) < 0.01).unwrap();

    state.selected_vertex_indices = vec![mid_idx, end_idx];
    let collapse_ok = state.collapse_selected_edge();
    assert!(collapse_ok);

    let unique_after_collapse = state.get_active_brush_unique_vertices();
    assert_eq!(unique_after_collapse.len(), 8); // merged 2 vertices into 1!

    // Verify Undo Collapse on Builder Brush
    state.undo();
    assert_eq!(state.get_active_brush_unique_vertices().len(), 9);

    // Verify Redo Collapse on Builder Brush
    state.redo();
    assert_eq!(state.get_active_brush_unique_vertices().len(), 8);

    // 3. Test Edge Split and Collapse on a World Brush
    let world_b = state.builder_brush.clone();
    state.world_brushes.push(world_b);
    state.selected_brush_index = Some(0);

    let world_unique = state.get_active_brush_unique_vertices();
    assert_eq!(world_unique.len(), 8);

    let w_poly = &state.world_brushes[0].polys[0];
    let wv0 = w_poly.vertices[0] + state.world_brushes[0].location;
    let wv1 = w_poly.vertices[1] + state.world_brushes[0].location;

    let w_idx0 = world_unique.iter().position(|&v| v.distance_squared(wv0) < 0.01).unwrap();
    let w_idx1 = world_unique.iter().position(|&v| v.distance_squared(wv1) < 0.01).unwrap();

    state.selected_vertex_indices = vec![w_idx0, w_idx1];
    let w_split_ok = state.split_selected_edge();
    assert!(w_split_ok);
    assert_eq!(state.world_brushes[0].polys[0].vertices.len(), 5); // 4 + 1

    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].vertices.len(), 4);

    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].vertices.len(), 5);
}

#[test]
fn test_canonical_grid_step_cycling_and_camera_fov() {
    let mut state = EditorState::default();

    // Default grid is 64 uu
    assert_eq!(state.grid_step, 64.0);
    assert_eq!(state.grid.step, 64.0);

    // Step up: 64 -> 128 -> 256 -> 512 -> 1024 -> 2048 -> 4096
    assert_eq!(state.step_grid_up(), 128.0);
    assert_eq!(state.grid_step, 128.0);
    assert_eq!(state.grid.step, 128.0);

    assert_eq!(state.step_grid_up(), 256.0);
    assert_eq!(state.step_grid_up(), 512.0);
    assert_eq!(state.step_grid_up(), 1024.0);
    assert_eq!(state.step_grid_up(), 2048.0);
    assert_eq!(state.step_grid_up(), 4096.0);
    // At maximum 4096, stepping up remains 4096
    assert_eq!(state.step_grid_up(), 4096.0);

    // Step down: 4096 -> 2048 -> 1024 -> 512 -> 256 -> 128 -> 64 -> 32 -> 16 -> 8 -> 4 -> 2 -> 1
    assert_eq!(state.step_grid_down(), 2048.0);
    assert_eq!(state.step_grid_down(), 1024.0);
    assert_eq!(state.step_grid_down(), 512.0);
    assert_eq!(state.step_grid_down(), 256.0);
    assert_eq!(state.step_grid_down(), 128.0);
    assert_eq!(state.step_grid_down(), 64.0);
    assert_eq!(state.step_grid_down(), 32.0);
    assert_eq!(state.step_grid_down(), 16.0);
    assert_eq!(state.step_grid_down(), 8.0);
    assert_eq!(state.step_grid_down(), 4.0);
    assert_eq!(state.step_grid_down(), 2.0);
    assert_eq!(state.step_grid_down(), 1.0);
    // At minimum 1, stepping down remains 1
    assert_eq!(state.step_grid_down(), 1.0);

    // Direct assignment with set_grid_step
    state.set_grid_step(32.0);
    assert_eq!(state.grid_step, 32.0);
    assert_eq!(state.grid.step, 32.0);

    // Camera FOV testing
    state.set_camera_fov_degrees(90.0);
    assert!((state.get_camera_fov_degrees() - 90.0).abs() < 1e-3);

    // Clamp testing
    state.set_camera_fov_degrees(10.0);
    assert!((state.get_camera_fov_degrees() - 30.0).abs() < 1e-3);

    state.set_camera_fov_degrees(160.0);
    assert!((state.get_camera_fov_degrees() - 120.0).abs() < 1e-3);
}

#[test]
fn test_builder_brush_obj_roundtrip_and_undo() {
    let mut state = EditorState::default();

    // Default builder brush has 6 cube faces
    assert_eq!(state.builder_brush.polys.len(), 6);

    let obj_text = state.export_builder_brush_to_obj();
    assert!(obj_text.contains("o BuilderBrush"));
    assert!(obj_text.contains("v "));
    assert!(obj_text.contains("f "));

    // Clear builder brush polygons
    state.builder_brush.polys.clear();
    assert_eq!(state.builder_brush.polys.len(), 0);

    // Re-import from OBJ
    let res = state.import_obj_to_builder_brush(&obj_text);
    assert!(res.is_ok());
    assert_eq!(state.builder_brush.polys.len(), 6);

    // Undo should restore the empty state
    state.undo();
    assert_eq!(state.builder_brush.polys.len(), 0);

    // Redo should restore the 6 polygons
    state.redo();
    assert_eq!(state.builder_brush.polys.len(), 6);
}

#[test]
fn test_brush_solidity_conversion_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Add world brush (default is Solid)
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);
    state.selected_brush_index = Some(0);

    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::Solid);
    for p in &state.world_brushes[0].polys {
        assert_eq!(p.flags & (ut99_bsp::PF_SEMISOLID | ut99_bsp::PF_NOT_SOLID), 0);
    }

    // 2. Convert to Semi-Solid
    let changed = state.set_selected_brush_solidity(ut99_editor::editor_state::BrushSolidity::SemiSolid);
    assert!(changed);
    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::SemiSolid);
    for p in &state.world_brushes[0].polys {
        assert_ne!(p.flags & ut99_bsp::PF_SEMISOLID, 0);
        assert_eq!(p.flags & ut99_bsp::PF_NOT_SOLID, 0);
    }

    // 3. Convert to Non-Solid
    let changed2 = state.set_selected_brush_solidity(ut99_editor::editor_state::BrushSolidity::NonSolid);
    assert!(changed2);
    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::NonSolid);
    for p in &state.world_brushes[0].polys {
        assert_eq!(p.flags & ut99_bsp::PF_SEMISOLID, 0);
        assert_ne!(p.flags & ut99_bsp::PF_NOT_SOLID, 0);
    }

    // 4. Undo back to Semi-Solid
    state.undo();
    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::SemiSolid);

    // 5. Undo back to Solid
    state.undo();
    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::Solid);

    // 6. Redo to Semi-Solid
    state.redo();
    assert_eq!(state.get_brush_solidity(0), ut99_editor::editor_state::BrushSolidity::SemiSolid);

    // 7. Builder Brush Solidity when no world brush is selected
    state.selected_brush_index = None;
    state.set_selected_brush_solidity(ut99_editor::editor_state::BrushSolidity::SemiSolid);
    for p in &state.builder_brush.polys {
        assert_ne!(p.flags & ut99_bsp::PF_SEMISOLID, 0);
    }
    state.undo();
    for p in &state.builder_brush.polys {
        assert_eq!(p.flags & ut99_bsp::PF_SEMISOLID, 0);
    }
}

#[test]
fn test_brush_ordering_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create 3 brushes with distinct locations
    state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));

    state.builder_brush.location = Vec3::new(10.0, 0.0, 0.0);
    state.csg_add();

    state.builder_brush.location = Vec3::new(20.0, 0.0, 0.0);
    state.csg_add();

    state.builder_brush.location = Vec3::new(30.0, 0.0, 0.0);
    state.csg_add();

    assert_eq!(state.world_brushes.len(), 3);
    assert_eq!(state.world_brushes[0].location.x, 10.0);
    assert_eq!(state.world_brushes[1].location.x, 20.0);
    assert_eq!(state.world_brushes[2].location.x, 30.0);

    // Select brush 0 (x=10) and send to last
    state.selected_brush_index = Some(0);
    state.send_selected_brush_to_last();
    assert_eq!(state.selected_brush_index, Some(2));
    assert_eq!(state.world_brushes[0].location.x, 20.0);
    assert_eq!(state.world_brushes[1].location.x, 30.0);
    assert_eq!(state.world_brushes[2].location.x, 10.0);

    // Undo send to last
    state.undo();
    assert_eq!(state.selected_brush_index, Some(0));
    assert_eq!(state.world_brushes[0].location.x, 10.0);
    assert_eq!(state.world_brushes[1].location.x, 20.0);
    assert_eq!(state.world_brushes[2].location.x, 30.0);

    // Redo send to last
    state.redo();
    assert_eq!(state.selected_brush_index, Some(2));
    assert_eq!(state.world_brushes[2].location.x, 10.0);

    // Send brush 2 (x=10) to first
    state.send_selected_brush_to_first();
    assert_eq!(state.selected_brush_index, Some(0));
    assert_eq!(state.world_brushes[0].location.x, 10.0);
    assert_eq!(state.world_brushes[1].location.x, 20.0);
    assert_eq!(state.world_brushes[2].location.x, 30.0);

    // Test Move Up / Down
    state.selected_brush_index = Some(1); // x=20
    state.move_selected_brush_up(); // swap with index 0
    assert_eq!(state.selected_brush_index, Some(0));
    assert_eq!(state.world_brushes[0].location.x, 20.0);
    assert_eq!(state.world_brushes[1].location.x, 10.0);

    state.move_selected_brush_down(); // swap back with index 1
    assert_eq!(state.selected_brush_index, Some(1));
    assert_eq!(state.world_brushes[0].location.x, 10.0);
    assert_eq!(state.world_brushes[1].location.x, 20.0);

    // Undo swap down
    state.undo();
    assert_eq!(state.selected_brush_index, Some(0));
    assert_eq!(state.world_brushes[0].location.x, 20.0);
    assert_eq!(state.world_brushes[1].location.x, 10.0);
}

#[test]
fn test_brush_invert_normals_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);
    state.selected_brush_index = Some(0);

    // Initial normal for polygon 0
    let orig_normal = state.world_brushes[0].polys[0].plane.normal;
    let orig_v0 = state.world_brushes[0].polys[0].vertices[0];
    let orig_v_last = *state.world_brushes[0].polys[0].vertices.last().unwrap();

    // 1. Invert brush normals and winding
    state.invert_selected_brush();
    let inverted_normal = state.world_brushes[0].polys[0].plane.normal;
    let inv_v0 = state.world_brushes[0].polys[0].vertices[0];
    let inv_v_last = *state.world_brushes[0].polys[0].vertices.last().unwrap();

    // Verify normal is negated and winding reversed
    assert!((inverted_normal - (-orig_normal)).length() < 1e-3);
    assert_eq!(inv_v0, orig_v_last);
    assert_eq!(inv_v_last, orig_v0);

    // 2. Undo
    state.undo();
    let restored_normal = state.world_brushes[0].polys[0].plane.normal;
    assert!((restored_normal - orig_normal).length() < 1e-3);
    assert_eq!(state.world_brushes[0].polys[0].vertices[0], orig_v0);

    // 3. Redo
    state.redo();
    assert!((state.world_brushes[0].polys[0].plane.normal - inverted_normal).length() < 1e-3);

    // 4. Invert Builder Brush when no brush is selected
    state.selected_brush_index = None;
    let bb_orig_normal = state.builder_brush.polys[0].plane.normal;
    state.invert_selected_brush();
    assert!((state.builder_brush.polys[0].plane.normal - (-bb_orig_normal)).length() < 1e-3);
    state.undo();
    assert!((state.builder_brush.polys[0].plane.normal - bb_orig_normal).length() < 1e-3);

    // 5. Invert per-surface
    state.selected_brush_index = Some(0);
    state.selected_surfaces.insert((0, 2)); // Select face 2
    let face2_orig_n = state.world_brushes[0].polys[2].plane.normal;
    let count = state.invert_selected_surfaces();
    assert_eq!(count, 1);
    assert!((state.world_brushes[0].polys[2].plane.normal - (-face2_orig_n)).length() < 1e-3);
    state.undo();
    assert!((state.world_brushes[0].polys[2].plane.normal - face2_orig_n).length() < 1e-3);
}

#[test]
fn test_selective_brush_and_actor_filters() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Additive Brush
    state.set_builder_cube(Vec3::new(64.0, 64.0, 64.0));
    state.builder_brush.csg_oper = ut99_bsp::CsgOperation::Add;
    state.csg_add();

    // 2. Subtractive Brush
    state.builder_brush.csg_oper = ut99_bsp::CsgOperation::Subtract;
    state.csg_subtract();

    // 3. Semi-Solid Brush
    state.builder_brush.csg_oper = ut99_bsp::CsgOperation::Add;
    state.csg_add();
    state.selected_brush_index = Some(2);
    state.set_selected_brush_solidity(ut99_editor::editor_state::BrushSolidity::SemiSolid);

    // 4. Non-Solid Brush
    state.csg_add();
    state.selected_brush_index = Some(3);
    state.set_selected_brush_solidity(ut99_editor::editor_state::BrushSolidity::NonSolid);

    assert_eq!(state.world_brushes.len(), 4);

    // Test select_additive_brushes (Brushes 0, 2, 3 have CsgOperation::Add)
    let add_count = state.select_additive_brushes();
    assert_eq!(add_count, 3);
    assert!(state.is_brush_selected(0));
    assert!(!state.is_brush_selected(1));
    assert!(state.is_brush_selected(2));
    assert!(state.is_brush_selected(3));

    // Test select_subtractive_brushes (Brush 1)
    let sub_count = state.select_subtractive_brushes();
    assert_eq!(sub_count, 1);
    assert!(!state.is_brush_selected(0));
    assert!(state.is_brush_selected(1));

    // Test select_semisolid_brushes (Brush 2)
    let semi_count = state.select_semisolid_brushes();
    assert_eq!(semi_count, 1);
    assert!(state.is_brush_selected(2));

    // Test select_nonsolid_brushes (Brush 3)
    let non_count = state.select_nonsolid_brushes();
    assert_eq!(non_count, 1);
    assert!(state.is_brush_selected(3));

    // Test Actor Filters
    let mut unr = ut99_format::UnrLevel::default();
    let mut a1 = ut99_format::ActorRecord::default();
    a1.class_name = "Light".to_string();
    a1.name = "Light0".to_string();

    let mut a2 = ut99_format::ActorRecord::default();
    a2.class_name = "PathNode".to_string();
    a2.name = "PathNode0".to_string();

    let mut a3 = ut99_format::ActorRecord::default();
    a3.class_name = "PlayerStart".to_string();
    a3.name = "PlayerStart0".to_string();

    let mut a4 = ut99_format::ActorRecord::default();
    a4.class_name = "Trigger".to_string();
    a4.name = "Trigger0".to_string();

    unr.actors = vec![a1, a2, a3, a4].into();
    state.loaded_unr = Some(unr);

    // Test select_all_lights
    let light_count = state.select_all_lights();
    assert_eq!(light_count, 1);
    assert!(state.is_actor_selected(0));
    assert!(!state.is_actor_selected(1));

    // Test select_all_pathnodes (PathNode + PlayerStart)
    let nav_count = state.select_all_pathnodes();
    assert_eq!(nav_count, 2);
    assert!(!state.is_actor_selected(0));
    assert!(state.is_actor_selected(1));
    assert!(state.is_actor_selected(2));
    assert!(!state.is_actor_selected(3));
}

#[test]
fn test_brush_transform_permanently_and_reset_suite() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    state.set_builder_cube(Vec3::new(100.0, 100.0, 100.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);
    state.selected_brush_index = Some(0);

    // 1. Set rotation and scale on brush 0
    state.world_brushes[0].rotation = Vec3::new(0.0, 90.0, 0.0);
    state.world_brushes[0].scale = Vec3::new(2.0, 1.0, 0.5);

    let (min_before, max_before) = state.world_brushes[0].world_bounds();

    // 2. Transform permanently (bake)
    let baked = state.transform_permanently_selected();
    assert!(baked);
    assert_eq!(state.world_brushes[0].rotation, Vec3::ZERO);
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);

    let (min_after, max_after) = state.world_brushes[0].world_bounds();
    assert!((min_before - min_after).length() < 1e-2, "World bounds must remain invariant after bake");
    assert!((max_before - max_after).length() < 1e-2, "World bounds must remain invariant after bake");

    // 3. Test Undo of Transform Permanently
    state.undo();
    assert_eq!(state.world_brushes[0].rotation, Vec3::new(0.0, 90.0, 0.0));
    assert_eq!(state.world_brushes[0].scale, Vec3::new(2.0, 1.0, 0.5));

    // 4. Test Redo
    state.redo();
    assert_eq!(state.world_brushes[0].rotation, Vec3::ZERO);
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);

    // 5. Test Reset Rotation
    state.world_brushes[0].rotation = Vec3::new(15.0, 45.0, 30.0);
    let rot_reset = state.reset_rotation_selected();
    assert!(rot_reset);
    assert_eq!(state.world_brushes[0].rotation, Vec3::ZERO);
    state.undo();
    assert_eq!(state.world_brushes[0].rotation, Vec3::new(15.0, 45.0, 30.0));
    state.redo();
    assert_eq!(state.world_brushes[0].rotation, Vec3::ZERO);

    // 6. Test Reset Scaling
    state.world_brushes[0].scale = Vec3::new(3.0, 4.0, 5.0);
    let scale_reset = state.reset_scaling_selected();
    assert!(scale_reset);
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);
    state.undo();
    assert_eq!(state.world_brushes[0].scale, Vec3::new(3.0, 4.0, 5.0));
    state.redo();
    assert_eq!(state.world_brushes[0].scale, Vec3::ONE);

    // 7. Test on Builder Brush when no world brush is selected
    state.selected_brush_index = None;
    state.builder_brush.rotation = Vec3::new(0.0, 180.0, 0.0);
    state.builder_brush.scale = Vec3::new(1.5, 1.5, 1.5);
    let (bb_min_bef, bb_max_bef) = state.builder_brush.world_bounds();

    let bb_baked = state.transform_permanently_selected();
    assert!(bb_baked);
    assert_eq!(state.builder_brush.rotation, Vec3::ZERO);
    assert_eq!(state.builder_brush.scale, Vec3::ONE);
    let (bb_min_aft, bb_max_aft) = state.builder_brush.world_bounds();
    assert!((bb_min_bef - bb_min_aft).length() < 1e-2);
    assert!((bb_max_bef - bb_max_aft).length() < 1e-2);

    state.undo();
    assert_eq!(state.builder_brush.rotation, Vec3::new(0.0, 180.0, 0.0));
    assert_eq!(state.builder_brush.scale, Vec3::new(1.5, 1.5, 1.5));
}

#[test]
fn test_level_properties_and_compiler_roundtrip() {
    let mut state = EditorState::default();

    // Check defaults
    assert_eq!(state.level_title, "Untitled Arena");
    assert_eq!(state.level_author, "Unknown Mapper");
    assert_eq!(state.level_ideal_player_count, "4-8");
    assert_eq!(state.level_game_type, "Botpack.DeathMatchPlus");

    // Update level properties
    state.update_level_properties(
        "DM-Morpheus][".to_string(),
        "Dave Ewing".to_string(),
        "3-6".to_string(),
        "Firebr.umx".to_string(),
        "Botpack.CTFGame".to_string(),
        "Welcome to Space".to_string(),
    );
    state.level_ambient_brightness = 32;
    state.level_ambient_hue = 150;
    state.level_ambient_saturation = 200;

    assert_eq!(state.level_title, "DM-Morpheus][");
    assert_eq!(state.level_author, "Dave Ewing");

    // Export map and verify LevelInfo actor properties
    let t3d_map = ut99_editor::compiler::CompilerBridge::export_t3d_map(&state);
    let level_info = t3d_map.actors.iter().find(|a| a.class == "LevelInfo").expect("LevelInfo actor must exist in exported map");

    assert_eq!(level_info.properties.get("Title").unwrap(), "\"DM-Morpheus][\"");
    assert_eq!(level_info.properties.get("Author").unwrap(), "\"Dave Ewing\"");
    assert_eq!(level_info.properties.get("IdealPlayerCount").unwrap(), "\"3-6\"");
    assert_eq!(level_info.properties.get("Song").unwrap(), "\"Firebr.umx\"");
    assert_eq!(level_info.properties.get("DefaultGameType").unwrap(), "Class'Botpack.CTFGame'");
    assert_eq!(level_info.properties.get("LevelEnterText").unwrap(), "\"Welcome to Space\"");
    assert_eq!(level_info.properties.get("AmbientBrightness").unwrap(), "32");
    assert_eq!(level_info.properties.get("AmbientHue").unwrap(), "150");
    assert_eq!(level_info.properties.get("AmbientSaturation").unwrap(), "200");

    // Test new_map resets properties
    state.new_map();
    assert_eq!(state.level_title, "Untitled Arena");
    assert_eq!(state.level_author, "Unknown Mapper");
    assert_eq!(state.level_song, "");
}

#[test]
fn test_scale_entire_map_and_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.movers.clear();

    // 1. Add a world brush
    let mut brush = ut99_bsp::build_cube("Box0", ut99_bsp::CsgOperation::Add, Vec3::new(100.0, 100.0, 100.0));
    brush.location = Vec3::new(100.0, 200.0, 300.0);
    state.world_brushes.push(brush);

    // 2. Add an EditorMover
    let mover_brush = ut99_bsp::build_cube("MoverBrush0", ut99_bsp::CsgOperation::Add, Vec3::new(64.0, 64.0, 64.0));
    let mut mover = ut99_editor::editor_state::EditorMover::new(mover_brush);
    mover.brush.location = Vec3::new(50.0, 50.0, 50.0);
    mover.keyframes[0] = Some(ut99_editor::editor_state::MoverKeyframe { location: Vec3::new(50.0, 50.0, 50.0), rotation: Vec3::ZERO });
    mover.keyframes[1] = Some(ut99_editor::editor_state::MoverKeyframe { location: Vec3::new(50.0, 50.0, 178.0), rotation: Vec3::ZERO });
    state.movers.push(mover);

    // 3. Setup loaded_unr with a Light actor
    let mut unr = ut99_format::UnrLevel::default();
    let mut light = ut99_format::ActorRecord::default();
    light.name = "Light0".to_string();
    light.class_name = "Light".to_string();
    light.location = Vec3::new(10.0, 20.0, 30.0);
    light.light_radius = 32;
    unr.actors.push(light.clone());
    unr.lights.push(light);
    state.loaded_unr = Some(unr);

    // 4. Apply scale: 2.0x uniform
    let scale_success = state.scale_entire_map(Vec3::new(2.0, 2.0, 2.0), true, true);
    assert!(scale_success);

    // Check world brush scaled
    assert_eq!(state.world_brushes[0].location, Vec3::new(200.0, 400.0, 600.0));
    // Check mover scaled
    assert_eq!(state.movers[0].brush.location, Vec3::new(100.0, 100.0, 100.0));
    assert_eq!(state.movers[0].keyframes[0].as_ref().unwrap().location, Vec3::new(100.0, 100.0, 100.0));
    assert_eq!(state.movers[0].keyframes[1].as_ref().unwrap().location, Vec3::new(100.0, 100.0, 356.0));
    // Check light scaled
    let unr_ref = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_ref.actors[0].location, Vec3::new(20.0, 40.0, 60.0));
    assert_eq!(unr_ref.actors[0].light_radius, 64);
    assert_eq!(unr_ref.lights[0].location, Vec3::new(20.0, 40.0, 60.0));
    assert_eq!(unr_ref.lights[0].light_radius, 64);

    // 5. Test Undo
    state.undo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(100.0, 200.0, 300.0));
    assert_eq!(state.movers[0].brush.location, Vec3::new(50.0, 50.0, 50.0));
    let unr_restored = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_restored.actors[0].location, Vec3::new(10.0, 20.0, 30.0));
    assert_eq!(unr_restored.actors[0].light_radius, 32);

    // 6. Test Redo
    state.redo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(200.0, 400.0, 600.0));
    assert_eq!(state.movers[0].brush.location, Vec3::new(100.0, 100.0, 100.0));
    let unr_re_scaled = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_re_scaled.actors[0].location, Vec3::new(20.0, 40.0, 60.0));
}

#[test]
fn test_fase_a_console_drawer_toggle_and_defaults() {
    let mut state = EditorState::default();

    // Default: console drawer collapsed to give maximum viewport vertical space (+96px)
    assert!(!state.show_bottom_console, "Bottom console must be hidden by default to maximize viewport");

    // Toggle on
    state.show_bottom_console = true;
    assert!(state.show_bottom_console);

    // Toggle off
    state.show_bottom_console = false;
    assert!(!state.show_bottom_console);
}

#[test]
fn test_fase_a_progressive_escape_hierarchy() {
    let mut state = EditorState::default();

    // Setup: command palette open, build options open, multi-selection present, and single selection
    state.show_command_palette = true;
    state.show_build_options_dialog = true;
    state.multi_selected_brushes.insert(1);
    state.multi_selected_brushes.insert(2);
    state.selected_brush_index = Some(1);

    // Step 1: Escape closes command palette first
    let handled1 = state.handle_escape();
    assert!(handled1);
    assert!(!state.show_command_palette, "Escape must close command palette first");
    assert!(state.show_build_options_dialog, "Other dialogs must remain open");
    assert!(state.has_multi_selection(), "Selection must not be dropped yet");

    // Step 2: Escape closes build options dialog next
    let handled2 = state.handle_escape();
    assert!(handled2);
    assert!(!state.show_build_options_dialog, "Escape must close active floating dialog");
    assert!(state.has_multi_selection(), "Selection must still remain intact");

    // Step 3: Escape clears multi-selection next
    let handled3 = state.handle_escape();
    assert!(handled3);
    assert!(!state.has_multi_selection(), "Escape must clear multi-selection before single selection");

    // Step 4: Escape clears single selection
    let handled4 = state.handle_escape();
    assert!(handled4);
    assert_eq!(state.selected_brush_index, None, "Escape must deselect single selection");

    // Step 5: With everything clear, Escape returns false
    let handled5 = state.handle_escape();
    assert!(!handled5, "Escape returns false when nothing is open or selected");
}

#[test]
fn test_fase_b_toast_notification_lifecycle_and_triggers() {
    let mut state = EditorState::default();
    assert_eq!(state.toasts.len(), 0);

    // 1. Explicit toast creation
    state.toast_info("Information toast");
    state.toast_success("Success toast");
    state.toast_warning("Warning toast");
    state.toast_error("Error toast");
    assert_eq!(state.toasts.len(), 4);
    assert_eq!(state.toasts[0].kind, ut99_editor::editor_state::ToastKind::Info);
    assert_eq!(state.toasts[1].kind, ut99_editor::editor_state::ToastKind::Success);
    assert_eq!(state.toasts[2].kind, ut99_editor::editor_state::ToastKind::Warning);
    assert_eq!(state.toasts[3].kind, ut99_editor::editor_state::ToastKind::Error);
    assert_eq!(state.toasts[0].id, 1);
    assert_eq!(state.toasts[3].id, 4);

    // 2. Specific dismiss by id
    state.dismiss_toast(2); // dismiss success toast
    assert_eq!(state.toasts.len(), 3);
    assert!(!state.toasts.iter().any(|t| t.id == 2));

    // 3. Expiration pruning
    // Artificially expire the first toast by setting its created_at back
    state.toasts[0].created_at = std::time::Instant::now() - std::time::Duration::from_secs(10);
    state.prune_expired_toasts();
    assert_eq!(state.toasts.len(), 2);

    // 4. Workflow triggers: CSG Add creates a Success toast
    let count_before = state.toasts.len();
    state.csg_add();
    assert_eq!(state.toasts.len(), count_before + 1);
    let last_toast = state.toasts.last().unwrap();
    assert_eq!(last_toast.kind, ut99_editor::editor_state::ToastKind::Success);
    assert!(last_toast.message.contains("CSG Add"));

    // 5. Workflow triggers: Undo on empty history creates a Warning toast
    state.history.undo_stack.clear();
    state.undo();
    let undo_toast = state.toasts.last().unwrap();
    assert_eq!(undo_toast.kind, ut99_editor::editor_state::ToastKind::Warning);
    assert_eq!(undo_toast.message, "Nothing to undo");
}

#[test]
fn test_fase_b_shortcut_cheatsheet_search_and_escape() {
    let mut state = EditorState::default();
    assert!(!state.show_shortcut_help_dialog);
    assert!(state.shortcut_help_search.is_empty());

    // 1. Toggle cheatsheet
    state.show_shortcut_help_dialog = true;
    assert!(state.show_shortcut_help_dialog);

    // 2. Verify SHORTCUTS catalog is populated and accessible
    assert!(ut99_editor::gui::shortcut_help_dialog::SHORTCUTS.len() > 30);
    let has_csg = ut99_editor::gui::shortcut_help_dialog::SHORTCUTS.iter().any(|s| s.category.contains("CSG"));
    let has_viewports = ut99_editor::gui::shortcut_help_dialog::SHORTCUTS.iter().any(|s| s.category.contains("Viewport"));
    let has_palette = ut99_editor::gui::shortcut_help_dialog::SHORTCUTS.iter().any(|s| s.title.contains("Command Palette"));
    assert!(has_csg);
    assert!(has_viewports);
    assert!(has_palette);

    // 3. Escape priority closes shortcut cheatsheet overlay first
    let handled = state.handle_escape();
    assert!(handled);
    assert!(!state.show_shortcut_help_dialog, "Escape must close shortcut cheatsheet");
}

#[test]
fn test_fase_b_command_palette_navigation_state() {
    let mut state = EditorState::default();
    state.show_command_palette = true;
    state.command_palette_selected_index = 0;

    // Simulate list of 10 items
    let list_len = 10;

    // Down navigation (cyclic)
    state.command_palette_selected_index = (state.command_palette_selected_index + 1) % list_len;
    assert_eq!(state.command_palette_selected_index, 1);

    // Up navigation (cyclic from 1 to 0, then 0 to 9)
    state.command_palette_selected_index = (state.command_palette_selected_index + list_len - 1) % list_len;
    assert_eq!(state.command_palette_selected_index, 0);
    state.command_palette_selected_index = (state.command_palette_selected_index + list_len - 1) % list_len;
    assert_eq!(state.command_palette_selected_index, 9);

    // Reset when search query updates
    state.command_palette_search = "tex".to_string();
    state.command_palette_selected_index = 0;
    assert_eq!(state.command_palette_selected_index, 0);
}

#[test]
fn test_fase_c_independent_orthographic_cameras() {
    let mut camera = ut99_editor::camera::Camera::default();

    // 1. Initial independent coordinates match default position
    assert_eq!(camera.ortho_pos_top, glam::Vec2::new(camera.position.x, camera.position.y));
    assert_eq!(camera.ortho_pos_front, glam::Vec2::new(camera.position.x, camera.position.z));
    assert_eq!(camera.ortho_pos_side, glam::Vec2::new(camera.position.y, camera.position.z));

    // 2. Pan Top viewport independently (MOU-03)
    let initial_front = camera.ortho_pos_front;
    let initial_side = camera.ortho_pos_side;
    let initial_persp = camera.position;

    camera.pan_ortho(ut99_editor::camera::ViewportType::Top, 150.0, -75.0);
    assert_eq!(camera.ortho_pos_top.x, initial_persp.x + 150.0);
    assert_eq!(camera.ortho_pos_top.y, initial_persp.y - 75.0);
    // Other cameras must NOT have moved
    assert_eq!(camera.ortho_pos_front, initial_front);
    assert_eq!(camera.ortho_pos_side, initial_side);
    assert_eq!(camera.position, initial_persp);

    // 3. Pan Front and Side independently
    camera.pan_ortho(ut99_editor::camera::ViewportType::Front, 50.0, 30.0);
    assert_eq!(camera.ortho_pos_front.x, initial_front.x + 50.0);
    assert_eq!(camera.ortho_pos_front.y, initial_front.y + 30.0);
    assert_eq!(camera.ortho_pos_side, initial_side);

    // 4. Independent zoom levels (MOU-03)
    camera.set_ortho_zoom(ut99_editor::camera::ViewportType::Top, 512.0);
    camera.set_ortho_zoom(ut99_editor::camera::ViewportType::Front, 2048.0);
    assert_eq!(camera.get_ortho_zoom(ut99_editor::camera::ViewportType::Top), 512.0);
    assert_eq!(camera.get_ortho_zoom(ut99_editor::camera::ViewportType::Front), 2048.0);
    assert_eq!(camera.get_ortho_zoom(ut99_editor::camera::ViewportType::Side), 1024.0);

    // 5. Perspective Pan (MOU-02)
    let pos_before_pan = camera.position;
    camera.pan_perspective(10.0, -15.0);
    assert_ne!(camera.position, pos_before_pan);

    // 6. Focus On synchronizes all views to target
    let target = glam::Vec3::new(1000.0, 2000.0, 500.0);
    camera.focus_on(target, 400.0);
    assert_eq!(camera.ortho_pos_top, glam::Vec2::new(1000.0, 2000.0));
    assert_eq!(camera.ortho_pos_front, glam::Vec2::new(1000.0, 500.0));
    assert_eq!(camera.ortho_pos_side, glam::Vec2::new(2000.0, 500.0));
}

#[test]
fn test_fase_c_viewport_context_menu_state_and_escape() {
    let mut state = EditorState::default();
    assert!(!state.show_viewport_context_menu);
    assert_eq!(state.context_menu_world_pos, None);

    // 1. Open context menu
    state.show_viewport_context_menu = true;
    state.context_menu_screen_pos = egui::pos2(450.0, 300.0);
    state.context_menu_world_pos = Some(glam::Vec3::new(128.0, 256.0, 64.0));
    assert!(state.show_viewport_context_menu);

    // 2. Escape priority hierarchy: context menu is dismissed first
    let handled = state.handle_escape();
    assert!(handled, "Escape must be handled");
    assert!(!state.show_viewport_context_menu, "Context menu must close on Escape");
}

#[test]
fn test_fase_c_resource_browser_quick_actions() {
    let mut state = EditorState::default();

    // 1. Test place_actor_of_class_at places actor at explicit coordinate
    let target_loc = glam::Vec3::new(512.0, -256.0, 128.0);
    state.place_actor_of_class_at("Light", target_loc);
    let sel_idx = state.selected_actor_index.expect("Actor must be selected after placement");
    let placed_actor = &state.loaded_unr.as_ref().unwrap().actors[sel_idx];
    assert_eq!(placed_actor.class_name, "Light");
    assert_eq!(placed_actor.location, target_loc);
    assert_eq!(placed_actor.light_brightness, 64);
    assert!(state.toasts.iter().any(|t| t.message.contains("Placed Light")));

    // 2. Test apply_active_texture_to_current_selection
    state.active_texture = Some("GenEarth.Ground.dirt".to_string());
    state.apply_active_texture_to_current_selection();
    assert!(state.toasts.iter().any(|t| t.message.contains("Applied texture")));
}

#[test]
fn test_fase_d_texture_browser_dynamic_thumbnail_zoom() {
    let mut state = EditorState::default();

    // 1. Initial default size must be 110.0 (Medium)
    assert_eq!(state.texture_thumbnail_size, 110.0);

    // 2. Preset sizes: Small (64.0), Medium (110.0), Large (180.0)
    state.texture_thumbnail_size = 64.0;
    assert_eq!(state.texture_thumbnail_size, 64.0);

    state.texture_thumbnail_size = 180.0;
    assert_eq!(state.texture_thumbnail_size, 180.0);

    // 3. Continuous slider bounds: clamped to 48.0..=256.0
    let too_small = 20.0_f32;
    state.texture_thumbnail_size = too_small.clamp(48.0, 256.0);
    assert_eq!(state.texture_thumbnail_size, 48.0);

    let too_large = 500.0_f32;
    state.texture_thumbnail_size = too_large.clamp(48.0, 256.0);
    assert_eq!(state.texture_thumbnail_size, 256.0);
}

#[test]
fn test_fase_d_surface_properties_continuous_uv_differential_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Create a cube brush
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.csg_add();
    state.selected_brush_index = Some(0);

    let initial_base = state.world_brushes[0].polys[0].base;
    let initial_u = state.world_brushes[0].polys[0].texture_u;
    let initial_v = state.world_brushes[0].polys[0].texture_v;
    let history_len_before = state.history.undo_stack.len();

    // --- 1. Continuous UV Pan with Snapshot & Differential Commit ---
    // Start continuous drag: take snapshot
    let initial_pan = state.snapshot_selected_brush_polys();
    state.uv_edit_initial_polys = Some(initial_pan.clone());
    assert!(state.uv_edit_initial_polys.is_some());

    // Continuous frames: raw transformations without pushing to history
    state.pan_selected_brush_texture_raw(16.5, 0.0);
    state.pan_selected_brush_texture_raw(8.0, -12.5);
    state.pan_selected_brush_texture_raw(-4.5, 2.5);
    // History should not be polluted by intermediate frames
    assert_eq!(state.history.undo_stack.len(), history_len_before);

    // Stop drag: atomic differential commit
    state.commit_uv_modification(&initial_pan, "Pan UV Continuous".to_string());
    state.uv_edit_initial_polys = None;
    assert_eq!(state.history.undo_stack.len(), history_len_before + 1);
    assert!(state.uv_edit_initial_polys.is_none());

    let panned_base = state.world_brushes[0].polys[0].base;
    assert_ne!(panned_base, initial_base);

    // Test Undo restores exact initial base
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].base, initial_base);

    // Test Redo restores panned base
    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].base, panned_base);

    // --- 2. Continuous UV Scale ---
    let history_len_before_scale = state.history.undo_stack.len();
    let initial_scale = state.snapshot_selected_brush_polys();
    state.scale_selected_brush_texture_raw(1.5, 1.5);
    state.scale_selected_brush_texture_raw(1.2, 1.2);
    assert_eq!(state.history.undo_stack.len(), history_len_before_scale);

    state.commit_uv_modification(&initial_scale, "Scale UV Continuous".to_string());
    assert_eq!(state.history.undo_stack.len(), history_len_before_scale + 1);

    let scaled_u = state.world_brushes[0].polys[0].texture_u;
    assert_ne!(scaled_u, initial_u);

    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, initial_u);
    assert_eq!(state.world_brushes[0].polys[0].texture_v, initial_v);

    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, scaled_u);

    // --- 3. Continuous UV Rotate ---
    let history_len_before_rot = state.history.undo_stack.len();
    let initial_rot = state.snapshot_selected_brush_polys();
    state.rotate_selected_brush_texture_raw(45.0);
    assert_eq!(state.history.undo_stack.len(), history_len_before_rot);

    state.commit_uv_modification(&initial_rot, "Rotate UV Continuous".to_string());
    assert_eq!(state.history.undo_stack.len(), history_len_before_rot + 1);

    let rotated_u = state.world_brushes[0].polys[0].texture_u;
    assert_ne!(rotated_u, scaled_u);

    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, scaled_u);

    state.redo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, rotated_u);
}

#[test]
fn test_fase_d_inspector_numeric_edit_atomic_undo() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Brush Position & Rotation Continuous Editing with Atomic Undo
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.csg_add();
    state.selected_brush_index = Some(0);

    let initial_loc = state.world_brushes[0].location;

    // Simulate drag start
    state.numeric_edit_brush_loc_initial = Some((0, initial_loc));

    // Simulate frames of DragValue
    state.world_brushes[0].location.x += 16.0;
    state.world_brushes[0].location.x += 32.0;
    state.world_brushes[0].location.z -= 64.0;
    let final_loc = state.world_brushes[0].location;

    // History is still pristine
    let history_len = state.history.undo_stack.len();

    // Simulate drag stop / commit
    if let Some((b_idx, init_loc)) = state.numeric_edit_brush_loc_initial.take() {
        if b_idx == 0 && init_loc != final_loc {
            state.history.push(ut99_editor::history::Action::MoveBrush {
                brush_index: 0,
                old_location: init_loc,
                new_location: final_loc,
            });
        }
    }

    assert_eq!(state.history.undo_stack.len(), history_len + 1);

    // Undo should restore initial location
    state.undo();
    assert_eq!(state.world_brushes[0].location, initial_loc);

    // Redo restores final location
    state.redo();
    assert_eq!(state.world_brushes[0].location, final_loc);

    // 2. Actor Position Continuous Editing with Atomic Undo
    let mut unr = ut99_format::unr::UnrLevel::default();
    let mut actor = ut99_format::unr::actor::ActorRecord::default();
    actor.name = "PathNode_0".into();
    actor.location = Vec3::new(100.0, 200.0, 300.0);
    unr.actors.push(actor);
    state.loaded_unr = Some(unr);
    state.selected_actor_index = Some(0);

    let initial_actor_loc = state.loaded_unr.as_ref().unwrap().actors[0].location;
    state.numeric_edit_actor_loc_initial = Some((0, initial_actor_loc));

    // Drag intermediate steps
    state.loaded_unr.as_mut().unwrap().actors[0].location.y += 50.0;
    state.loaded_unr.as_mut().unwrap().actors[0].location.y += 50.0;
    let final_actor_loc = state.loaded_unr.as_ref().unwrap().actors[0].location;

    let h_len_actor = state.history.undo_stack.len();

    // Commit on drag_stopped
    if let Some((a_idx, init_loc)) = state.numeric_edit_actor_loc_initial.take() {
        if a_idx == 0 && init_loc != final_actor_loc {
            state.history.push(ut99_editor::history::Action::MoveActor {
                actor_index: 0,
                old_location: init_loc,
                new_location: final_actor_loc,
            });
        }
    }

    assert_eq!(state.history.undo_stack.len(), h_len_actor + 1);

    // Undo restores initial actor location
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, initial_actor_loc);

    // Redo restores moved actor location
    state.redo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors[0].location, final_actor_loc);
}

#[test]
fn test_fase_e_preferences_roundtrip_json() {
    let mut state = EditorState::default();

    // Configure custom preferences
    state.ui_scale = 1.25;
    state.grid_step = 32.0;
    state.snap_to_grid = true;
    state.rotation_snap_deg = 45.0;
    state.snap_to_rotation = true;
    state.set_camera_speed_preset(6);
    state.camera.fov_y = 85.0_f32.to_radians();
    state.viewport_shading = ut99_editor::camera::ViewportShading::ZonePortal;
    state.show_minimap = true;
    state.show_actor_radii = true;
    state.texture_thumbnail_size = 180.0;
    state.add_recent_map("DM-Phobos.unr");
    state.add_recent_map("CTF-Command.unr");

    // Save to temp JSON file
    let temp_path = std::env::temp_dir().join("test_forge99_prefs.json");
    let saved = state.save_preferences_to_path(&temp_path);
    assert!(saved, "Preferences should successfully save to temp file");
    assert!(temp_path.exists());

    // Create a pristine default editor state and load
    let mut new_state = EditorState::default();
    assert_ne!(new_state.ui_scale, 1.25);
    assert_ne!(new_state.grid_step, 32.0);
    assert_ne!(new_state.rotation_snap_deg, 45.0);
    assert_ne!(new_state.camera_speed_preset, 6);
    assert_ne!(new_state.texture_thumbnail_size, 180.0);

    let loaded = new_state.load_preferences_from_path(&temp_path);
    assert!(loaded, "Preferences should successfully load from temp file");

    // Verify all preferences are restored accurately
    assert_eq!(new_state.ui_scale, 1.25);
    assert_eq!(new_state.grid_step, 32.0);
    assert!(new_state.snap_to_grid);
    assert_eq!(new_state.rotation_snap_deg, 45.0);
    assert!(new_state.snap_to_rotation);
    assert_eq!(new_state.camera_speed_preset, 6);
    assert!((new_state.camera.fov_y.to_degrees() - 85.0).abs() < 1e-3);
    assert_eq!(new_state.viewport_shading, ut99_editor::camera::ViewportShading::ZonePortal);
    assert!(new_state.show_minimap);
    assert!(new_state.show_actor_radii);
    assert_eq!(new_state.texture_thumbnail_size, 180.0);
    assert_eq!(new_state.recent_maps.len(), 2);
    assert_eq!(new_state.recent_maps[0], std::path::PathBuf::from("CTF-Command.unr"));
    assert_eq!(new_state.recent_maps[1], std::path::PathBuf::from("DM-Phobos.unr"));

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);
}

#[test]
fn test_fase_e_accessibility_wcag_contrast_ratios() {
    use ut99_editor::gui::theme::{check_wcag_aa, wcag_contrast_ratio, ThemeColors};

    // 1. Text Primary (Light on Dark Panels): must exceed WCAG AAA (7.0:1)
    let ratio_canvas = wcag_contrast_ratio(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_CANVAS);
    let ratio_panel = wcag_contrast_ratio(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_PANEL);
    let ratio_widget = wcag_contrast_ratio(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_WIDGET);

    assert!(ratio_canvas > 15.0, "TEXT_PRIMARY on BG_CANVAS should have >15:1 contrast, got {:.2}", ratio_canvas);
    assert!(ratio_panel > 14.0, "TEXT_PRIMARY on BG_PANEL should have >14:1 contrast, got {:.2}", ratio_panel);
    assert!(ratio_widget > 10.0, "TEXT_PRIMARY on BG_WIDGET should have >10:1 contrast, got {:.2}", ratio_widget);
    assert!(check_wcag_aa(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_PANEL, false));

    // 2. Text Secondary: must exceed WCAG AA normal text (4.5:1)
    let ratio_sec_panel = wcag_contrast_ratio(ThemeColors::TEXT_SECONDARY, ThemeColors::BG_PANEL);
    assert!(ratio_sec_panel >= 4.5, "TEXT_SECONDARY on BG_PANEL should meet WCAG AA (>= 4.5), got {:.2}", ratio_sec_panel);
    assert!(check_wcag_aa(ThemeColors::TEXT_SECONDARY, ThemeColors::BG_PANEL, false));

    // 3. Interactive Accents and Focus Ring: must exceed WCAG AA component contrast (3.0:1)
    let ratio_focus = wcag_contrast_ratio(ThemeColors::BORDER_FOCUS, ThemeColors::BG_PANEL);
    let ratio_azure = wcag_contrast_ratio(ThemeColors::ACCENT_AZURE, ThemeColors::BG_PANEL);
    let ratio_gold = wcag_contrast_ratio(ThemeColors::ACCENT_GOLD, ThemeColors::BG_PANEL);
    let ratio_csg_add = wcag_contrast_ratio(ThemeColors::CSG_ADD, ThemeColors::BG_PANEL);
    let ratio_csg_sub = wcag_contrast_ratio(ThemeColors::CSG_SUB, ThemeColors::BG_PANEL);

    assert!(ratio_focus >= 3.0, "BORDER_FOCUS must meet UI component contrast (>= 3.0), got {:.2}", ratio_focus);
    assert!(ratio_azure >= 3.0, "ACCENT_AZURE must meet UI component contrast (>= 3.0), got {:.2}", ratio_azure);
    assert!(ratio_gold >= 3.0, "ACCENT_GOLD must meet UI component contrast (>= 3.0), got {:.2}", ratio_gold);
    assert!(ratio_csg_add >= 3.0, "CSG_ADD must meet UI component contrast (>= 3.0), got {:.2}", ratio_csg_add);
    assert!(ratio_csg_sub >= 3.0, "CSG_SUB must meet UI component contrast (>= 3.0), got {:.2}", ratio_csg_sub);

    assert!(check_wcag_aa(ThemeColors::BORDER_FOCUS, ThemeColors::BG_PANEL, true));
    assert!(check_wcag_aa(ThemeColors::ACCENT_AZURE, ThemeColors::BG_PANEL, true));
}

#[test]
fn test_fase_e_global_consolidation_and_invariance() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // 1. Geometry CSG Pipeline
    state.set_builder_cube(Vec3::splat(256.0));
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 1);

    // 2. Continuous UV editing with differential commit
    state.selected_brush_index = Some(0);
    let initial_uv = state.snapshot_selected_brush_polys();
    state.pan_selected_brush_texture_raw(32.0, 16.0);
    state.commit_uv_modification(&initial_uv, "Pan Consolidated".to_string());
    assert_eq!(state.history.undo_stack.len(), 2); // 1 CSG + 1 UV Commit

    // 3. Multi-viewport independent camera state
    state.camera.pan_ortho(ut99_editor::camera::ViewportType::Top, 100.0, 50.0);
    state.camera.pan_ortho(ut99_editor::camera::ViewportType::Side, -25.0, 40.0);
    assert_ne!(state.camera.ortho_pos_top, state.camera.ortho_pos_side);

    // 4. Toast notifications and escape hierarchy
    state.show_command_palette = true;
    assert!(state.handle_escape(), "Escape should close Command Palette first");
    assert!(!state.show_command_palette);

    state.show_shortcut_help_dialog = true;
    assert!(state.handle_escape(), "Escape should close Shortcut Dialog second");
    assert!(!state.show_shortcut_help_dialog);

    // 5. Complete Undo/Redo cycle verification
    state.undo(); // Undo UV
    state.undo(); // Undo CSG Add
    assert_eq!(state.world_brushes.len(), 0);

    state.redo(); // Redo CSG Add
    state.redo(); // Redo UV
    assert_eq!(state.world_brushes.len(), 1);

    // 6. System integrity check
    assert!(state.toasts.len() >= 2);
}

#[test]
fn test_fase_g_window_manager_close_all() {
    let mut state = EditorState::default();

    // 1. Open a mixed set of studios, tool dialogs, and resource browsers
    state.show_lighting_studio_dialog = true;
    state.show_stairway_studio_dialog = true;
    state.show_water_surface_studio_dialog = true;
    state.show_turret_dialog = true;
    state.show_level_properties_dialog = true;
    state.show_find_actors_dialog = true;
    state.show_scale_map_dialog = true;
    state.show_texture_browser = true;
    state.show_actor_properties = true;

    // 2. Execute close_all_floating_dialogs
    let closed_count = state.close_all_floating_dialogs();
    assert_eq!(closed_count, 9, "Must close all 9 open floating windows");

    // 3. Verify all target flags are reset
    assert!(!state.show_lighting_studio_dialog);
    assert!(!state.show_stairway_studio_dialog);
    assert!(!state.show_water_surface_studio_dialog);
    assert!(!state.show_turret_dialog);
    assert!(!state.show_level_properties_dialog);
    assert!(!state.show_find_actors_dialog);
    assert!(!state.show_scale_map_dialog);
    assert!(!state.show_texture_browser);
    assert!(!state.show_actor_properties);

    // 4. Verify toast notification was registered
    let toast = state.toasts.last().expect("Toast should be emitted");
    assert!(toast.message.contains("Closed 9 floating window(s)"));

    // 5. Successive call closes 0 windows
    assert_eq!(state.close_all_floating_dialogs(), 0);
}

#[test]
fn test_fase_g_universal_escape_hierarchy_for_all_studios() {
    let mut state = EditorState::default();

    // Setup active scene selection
    state.selected_brush_index = Some(0);
    state.selected_actor_index = Some(1);

    // Step 1: Open a specialized studio (e.g. Stairway Studio)
    state.show_stairway_studio_dialog = true;

    // Escape must dismiss the studio window first without dropping the selection
    let handled = state.handle_escape();
    assert!(handled, "Escape must be handled");
    assert!(!state.show_stairway_studio_dialog, "Stairway studio must be closed");
    assert_eq!(state.selected_brush_index, Some(0), "Brush selection must be preserved");
    assert_eq!(state.selected_actor_index, Some(1), "Actor selection must be preserved");

    // Step 2: Open a tool dialog (e.g. Find Actors Dialog)
    state.show_find_actors_dialog = true;

    // Escape must dismiss the tool dialog next
    let handled = state.handle_escape();
    assert!(handled);
    assert!(!state.show_find_actors_dialog, "Find actors dialog must be closed");
    assert_eq!(state.selected_brush_index, Some(0), "Brush selection must still be preserved");

    // Step 3: With no dialogs open, Escape deselects the scene items
    let handled = state.handle_escape();
    assert!(handled);
    assert_eq!(state.selected_brush_index, None, "Brush selection cleared");
    assert_eq!(state.selected_actor_index, None, "Actor selection cleared");

    // Step 4: With nothing open or selected, Escape returns false
    assert!(!state.handle_escape(), "No-op escape returns false");
}

#[test]
fn test_fase_g_workspace_presets() {
    let mut state = EditorState::default();

    // 1. Test Default Preset
    state.apply_workspace_preset(ut99_editor::WorkspacePreset::Default);
    assert_eq!(state.viewport_layout, ut99_editor::camera::ViewportLayout::Single);
    assert_eq!(state.active_viewport_type, ut99_editor::camera::ViewportType::Perspective);
    assert_eq!(state.viewport_shading, ut99_editor::camera::ViewportShading::Lit);
    assert!(state.show_texture_browser);
    assert!(!state.show_bottom_console);

    // 2. Open an extra dialog, then apply LevelDesign preset
    state.show_hazard_chamber_dialog = true;
    state.apply_workspace_preset(ut99_editor::WorkspacePreset::LevelDesign);
    assert!(!state.show_hazard_chamber_dialog, "Preset switch must close previous floating windows");
    assert_eq!(state.viewport_layout, ut99_editor::camera::ViewportLayout::FourViewports);
    assert!(state.show_texture_browser);
    assert!(state.show_surface_properties);

    // 3. Test Lighting & Atmosphere Preset
    state.apply_workspace_preset(ut99_editor::WorkspacePreset::LightingAndAtmosphere);
    assert_eq!(state.viewport_layout, ut99_editor::camera::ViewportLayout::Single);
    assert!(state.show_lighting_studio_dialog);
    assert!(state.show_skybox_studio_dialog);
    assert!(state.render_lux_heatmap);

    // 4. Test Bot & Gameplay Preset
    state.apply_workspace_preset(ut99_editor::WorkspacePreset::BotAndGameplay);
    assert_eq!(state.viewport_layout, ut99_editor::camera::ViewportLayout::FourViewports);
    assert_eq!(state.viewport_shading, ut99_editor::camera::ViewportShading::ZonePortal);
    assert!(state.show_bot_nav_studio_dialog);
    assert!(state.show_nav_coverage);
    assert!(state.show_level_properties_dialog);
}

#[test]
fn test_fase_h_hover_focused_quad_navigation() {
    let mut state = EditorState::default();

    // 1. Single Viewport layout
    state.viewport_layout = ViewportLayout::Single;
    state.active_viewport_type = ViewportType::Perspective;
    state.viewport_rect = [50.0, 50.0, 1000.0, 800.0];

    // Inside rect returns active viewport type
    assert_eq!(state.viewport_type_at(100.0, 100.0), Some(ViewportType::Perspective));
    assert_eq!(state.viewport_type_at(500.0, 400.0), Some(ViewportType::Perspective));
    // Outside rect returns None
    assert_eq!(state.viewport_type_at(20.0, 100.0), None);
    assert_eq!(state.viewport_type_at(1100.0, 400.0), None);

    // 2. FourViewports (Quad 2x2) layout
    state.viewport_layout = ViewportLayout::FourViewports;
    // Canvas: x: 100..900 (w=800, mid=500), y: 100..700 (h=600, mid=400)
    state.viewport_rect = [100.0, 100.0, 800.0, 600.0];

    // Quadrant Top-Left: Top (XY)
    assert_eq!(state.viewport_type_at(250.0, 200.0), Some(ViewportType::Top));
    // Quadrant Top-Right: Side (YZ)
    assert_eq!(state.viewport_type_at(650.0, 200.0), Some(ViewportType::Side));
    // Quadrant Bottom-Left: Front (XZ)
    assert_eq!(state.viewport_type_at(250.0, 500.0), Some(ViewportType::Front));
    // Quadrant Bottom-Right: Perspective (3D)
    assert_eq!(state.viewport_type_at(650.0, 500.0), Some(ViewportType::Perspective));

    // Outside bounds
    assert_eq!(state.viewport_type_at(50.0, 200.0), None);
    assert_eq!(state.viewport_type_at(950.0, 200.0), None);
    assert_eq!(state.viewport_type_at(250.0, 50.0), None);
    assert_eq!(state.viewport_type_at(250.0, 750.0), None);

    // 3. sync_hovered_viewport
    state.active_viewport_type = ViewportType::Perspective;
    let res = state.sync_hovered_viewport(250.0, 200.0);
    assert_eq!(res, Some(ViewportType::Top));
    assert_eq!(state.active_viewport_type, ViewportType::Top, "Active viewport must update to hovered quadrant");

    let res2 = state.sync_hovered_viewport(650.0, 500.0);
    assert_eq!(res2, Some(ViewportType::Perspective));
    assert_eq!(state.active_viewport_type, ViewportType::Perspective);

    // Out of bounds sync does not modify active viewport
    let res_out = state.sync_hovered_viewport(10.0, 10.0);
    assert_eq!(res_out, None);
    assert_eq!(state.active_viewport_type, ViewportType::Perspective);
}

#[test]
fn test_fase_h_independent_ortho_zoom_and_pan() {
    let mut cam = Camera::default();

    // Initial default zoom is 1024.0 for all ortho cameras
    assert_eq!(cam.get_ortho_zoom(ViewportType::Top), 1024.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Front), 1024.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Side), 1024.0);

    // Zooming Top viewport must NOT affect Front or Side
    cam.set_ortho_zoom(ViewportType::Top, 512.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Top), 512.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Front), 1024.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Side), 1024.0);

    // Zooming Front viewport must NOT affect Top or Side
    cam.set_ortho_zoom(ViewportType::Front, 2048.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Top), 512.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Front), 2048.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Side), 1024.0);

    // Zoom clamping (32.0 to 100_000.0)
    cam.set_ortho_zoom(ViewportType::Side, 10.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Side), 32.0);
    cam.set_ortho_zoom(ViewportType::Side, 200_000.0);
    assert_eq!(cam.get_ortho_zoom(ViewportType::Side), 100_000.0);

    // Panning Front must only move ortho_pos_front
    let initial_top_pos = cam.ortho_pos_top;
    let initial_side_pos = cam.ortho_pos_side;
    let initial_front_pos = cam.ortho_pos_front;

    cam.pan_ortho(ViewportType::Front, 128.0, 64.0);
    assert_ne!(cam.ortho_pos_front, initial_front_pos);
    assert_eq!(cam.ortho_pos_top, initial_top_pos, "Top camera position must remain unchanged");
    assert_eq!(cam.ortho_pos_side, initial_side_pos, "Side camera position must remain unchanged");
}

#[test]
fn test_fase_i_fuzzy_match_and_category_prefixes() {
    // 1. Test Subsequence Matching
    assert!(fuzzy_match("bop", "Open Build Options & Compilers").is_some());
    assert!(fuzzy_match("bop", "Build Options").is_some());
    assert!(fuzzy_match("bop", "Camera Bookmarks").is_none());

    assert!(fuzzy_match("selwat", "Surface Preset: Water (Translucent, Wavy, NotSolid)").is_some());
    assert!(fuzzy_match("csgadd", "CSG Add Builder Brush to World").is_some());

    // 2. Exact & Prefix Match Score Ordering
    let score_exact = fuzzy_match("Build", "Build").unwrap();
    let score_prefix = fuzzy_match("Build", "Build All Geometry").unwrap();
    let score_subseq = fuzzy_match("Build", "Rebuild All Geometry").unwrap();
    assert!(score_exact > score_prefix, "Exact match must outscore prefix match");
    assert!(score_prefix > score_subseq, "Prefix match must outscore interior subsequence match");

    // 3. Category Prefixes Parsing
    let (cats_act, q_act) = parse_query("> Build");
    assert!(cats_act.is_none());
    assert_eq!(q_act, "Build");

    let (cats_win, q_win) = parse_query("@ Lighting");
    assert_eq!(cats_win, Some(&["Window", "Tools", "Browser"][..]));
    assert_eq!(q_win, "Lighting");

    let (cats_tex, q_tex) = parse_query("# Water");
    assert_eq!(cats_tex, Some(&["Texture", "Surface", "Brush", "Geometry"][..]));
    assert_eq!(q_tex, "Water");

    let (cats_help, q_help) = parse_query("? Shortcuts");
    assert_eq!(cats_help, Some(&["Help", "System", "File"][..]));
    assert_eq!(q_help, "Shortcuts");

    // 4. MRU History Tracking in EditorState
    let mut state = EditorState::default();
    assert!(state.recent_palette_commands.is_empty());

    state.record_recent_palette_command("win_build_options");
    state.record_recent_palette_command("build_all");
    state.record_recent_palette_command("win_build_options"); // Re-recording must promote to top

    assert_eq!(state.recent_palette_commands.len(), 2);
    assert_eq!(state.recent_palette_commands[0], "win_build_options");
    assert_eq!(state.recent_palette_commands[1], "build_all");

    // Test capping at 8 items
    for i in 0..12 {
        state.record_recent_palette_command(format!("cmd_{i}"));
    }
    assert_eq!(state.recent_palette_commands.len(), 8);
}

#[test]
fn test_fase_i_marquee_box_selection_2d() {
    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Setup 2 world brushes:
    let mut b0 = ut99_bsp::build_cube("Cube0", ut99_bsp::CsgOperation::Add, Vec3::splat(64.0));
    b0.location = Vec3::new(100.0, 100.0, 0.0);
    let mut b1 = ut99_bsp::build_cube("Cube1", ut99_bsp::CsgOperation::Add, Vec3::splat(64.0));
    b1.location = Vec3::new(500.0, 500.0, 0.0);
    state.world_brushes.push(b0);
    state.world_brushes.push(b1);

    // Setup 2 actors:
    let mut unr = UnrLevel::default();
    let mut a0 = ActorRecord::default();
    a0.name = "Light0".into();
    a0.location = Vec3::new(120.0, 110.0, 0.0);
    let mut a1 = ActorRecord::default();
    a1.name = "Light1".into();
    a1.location = Vec3::new(520.0, 490.0, 0.0);
    unr.actors.push(a0);
    unr.actors.push(a1);
    state.loaded_unr = Some(unr);

    // Single layout viewport: 800x600, center at (300, 300), zoom 1000
    state.viewport_layout = ViewportLayout::Single;
    state.viewport_rect = [0.0, 0.0, 800.0, 600.0];
    state.camera.ortho_pos_top = glam::Vec2::new(300.0, 300.0);
    state.camera.ortho_zoom_top = 1000.0;

    // Verify center maps correctly
    let center = state.camera.screen_to_ortho_world(ViewportType::Top, 400.0, 300.0, [0.0, 0.0, 800.0, 600.0]);
    assert!(center.is_some());
    let (cx, cy) = center.unwrap();
    assert!((cx - 300.0).abs() < 1e-3);
    assert!((cy - 300.0).abs() < 1e-3);

    // World to screen helper
    fn world_to_screen_top(wx: f32, wy: f32, cx: f32, cy: f32, zoom: f32, rect: [f32; 4]) -> (f32, f32) {
        let [vx, vy, vw, vh] = rect;
        let aspect = vw / vh;
        let half_w = zoom * aspect;
        let half_h = zoom;
        let ndc_x = (wx - cx) / half_w;
        let ndc_y = (wy - cy) / half_h;
        let sx = (ndc_x + 1.0) * 0.5 * vw + vx;
        let sy = (1.0 - ndc_y) * 0.5 * vh + vy;
        (sx, sy)
    }

    let p1 = world_to_screen_top(50.0, 50.0, 300.0, 300.0, 1000.0, [0.0, 0.0, 800.0, 600.0]);
    let p2 = world_to_screen_top(200.0, 200.0, 300.0, 300.0, 1000.0, [0.0, 0.0, 800.0, 600.0]);

    let marquee_first = MarqueeBox {
        start_screen: p1,
        current_screen: p2,
        viewport_type: ViewportType::Top,
    };

    let (sel_b, sel_a) = state.select_within_marquee(marquee_first, false);
    assert_eq!(sel_b, 1, "Only Brush 0 should be selected");
    assert_eq!(sel_a, 1, "Only Actor 0 should be selected");
    assert!(state.multi_selected_brushes.contains(&0));
    assert!(!state.multi_selected_brushes.contains(&1));
    assert!(state.multi_selected_actors.contains(&0));
    assert!(!state.multi_selected_actors.contains(&1));

    // Second area with additive = true
    let p3 = world_to_screen_top(450.0, 450.0, 300.0, 300.0, 1000.0, [0.0, 0.0, 800.0, 600.0]);
    let p4 = world_to_screen_top(550.0, 550.0, 300.0, 300.0, 1000.0, [0.0, 0.0, 800.0, 600.0]);
    let marquee_second = MarqueeBox {
        start_screen: p3,
        current_screen: p4,
        viewport_type: ViewportType::Top,
    };

    let (sel_b2, sel_a2) = state.select_within_marquee(marquee_second, true);
    assert_eq!(sel_b2, 1);
    assert_eq!(sel_a2, 1);
    assert_eq!(state.multi_selected_brushes.len(), 2, "Both brushes selected when additive");
    assert_eq!(state.multi_selected_actors.len(), 2, "Both actors selected when additive");
}

#[test]
fn test_fase_i_standardized_tooltips_and_wcag() {
    // 1. Text Primary vs Canvas Background (WCAG AA text requirement >= 4.5)
    let contrast_text = wcag_contrast_ratio(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_CANVAS);
    assert!(contrast_text >= 4.5, "Primary text contrast ratio must be >= 4.5, got {}", contrast_text);
    assert!(check_wcag_aa(ThemeColors::TEXT_PRIMARY, ThemeColors::BG_CANVAS, false));

    // 2. Interactive Accent Azure vs Canvas Background (WCAG AA UI requirement >= 3.0)
    let contrast_ui = wcag_contrast_ratio(ThemeColors::ACCENT_AZURE, ThemeColors::BG_CANVAS);
    assert!(contrast_ui >= 3.0, "Accent azure UI contrast ratio must be >= 3.0, got {}", contrast_ui);
    assert!(check_wcag_aa(ThemeColors::ACCENT_AZURE, ThemeColors::BG_CANVAS, true));
}

#[test]
fn test_fase_j_compound_batch_undo_redo_multi_selection() {
    use ut99_editor::editor_state::{AlignAxis, AlignMode};
    use ut99_editor::history::Action;

    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut a1 = ActorRecord::default();
    a1.location = Vec3::new(100.0, 20.0, 0.0);
    let mut a2 = ActorRecord::default();
    a2.location = Vec3::new(300.0, 40.0, 0.0);
    let mut a3 = ActorRecord::default();
    a3.location = Vec3::new(500.0, 60.0, 0.0);

    level.actors.push(a1);
    level.actors.push(a2);
    level.actors.push(a3);
    state.loaded_unr = Some(level);

    // Multi-select all 3 actors
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);
    state.multi_selected_actors.insert(2);

    // 1. Align selected actors to Min X (target = 100.0)
    let count = state.align_selected_actors(AlignAxis::X, AlignMode::Min);
    assert_eq!(count, 2, "Actors 1 and 2 moved to align with Actor 0");

    // Check locations
    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors[0].location.x, 100.0);
    assert_eq!(unr.actors[1].location.x, 100.0);
    assert_eq!(unr.actors[2].location.x, 100.0);

    // Ensure it was recorded as a SINGLE Action::Compound, not multiple scattered actions!
    assert_eq!(state.history.undo_stack.len(), 1, "Must be grouped in a single Action::Compound");
    match &state.history.undo_stack[0] {
        Action::Compound { actions, description } => {
            assert_eq!(actions.len(), 2);
            assert!(description.contains("Align"));
        }
        other => panic!("Expected Action::Compound, got {:?}", other),
    }

    // 2. Perform a single Undo
    state.undo();
    let unr_after_undo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_after_undo.actors[0].location.x, 100.0);
    assert_eq!(unr_after_undo.actors[1].location.x, 300.0, "Actor 1 restored in single undo");
    assert_eq!(unr_after_undo.actors[2].location.x, 500.0, "Actor 2 restored in single undo");
    assert_eq!(state.history.undo_stack.len(), 0);
    assert_eq!(state.history.redo_stack.len(), 1);

    // 3. Perform a single Redo
    state.redo();
    let unr_after_redo = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr_after_redo.actors[0].location.x, 100.0);
    assert_eq!(unr_after_redo.actors[1].location.x, 100.0, "Actor 1 re-aligned in single redo");
    assert_eq!(unr_after_redo.actors[2].location.x, 100.0, "Actor 2 re-aligned in single redo");
    assert_eq!(state.history.undo_stack.len(), 1);
    assert_eq!(state.history.redo_stack.len(), 0);
}

#[test]
fn test_fase_j_compound_batch_delete_and_grid_snap() {
    use ut99_editor::history::Action;

    let mut state = EditorState::default();
    let mut level = UnrLevel::default();

    let mut a1 = ActorRecord::default();
    a1.location = Vec3::new(12.3, 45.6, 78.9);
    let mut a2 = ActorRecord::default();
    a2.location = Vec3::new(103.4, 205.1, 307.8);
    level.actors.push(a1);
    level.actors.push(a2);
    state.loaded_unr = Some(level);

    state.grid_step = 64.0;
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);

    // Snap both actors to 64 unit grid
    let snapped = state.snap_multi_selected_to_grid();
    assert_eq!(snapped, 2);

    let unr = state.loaded_unr.as_ref().unwrap();
    assert_eq!(unr.actors[0].location, Vec3::new(0.0, 64.0, 64.0));
    assert_eq!(unr.actors[1].location, Vec3::new(128.0, 192.0, 320.0));

    // Must be a compound action in undo stack
    assert_eq!(state.history.undo_stack.len(), 1);
    match &state.history.undo_stack[0] {
        Action::Compound { actions, .. } => assert_eq!(actions.len(), 2),
        other => panic!("Expected Action::Compound, got {:?}", other),
    }

    // Single undo restores both
    state.undo();
    let unr2 = state.loaded_unr.as_ref().unwrap();
    assert!((unr2.actors[0].location - Vec3::new(12.3, 45.6, 78.9)).length() < 1e-4);
    assert!((unr2.actors[1].location - Vec3::new(103.4, 205.1, 307.8)).length() < 1e-4);

    // Delete batch
    state.multi_selected_actors.insert(0);
    state.multi_selected_actors.insert(1);
    let (_deleted_b, deleted_a) = state.delete_selected_batch();
    assert_eq!(deleted_a, 2);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0);

    // Ensure batch deletion is also a single Compound action
    assert_eq!(state.history.undo_stack.len(), 1);
    match &state.history.undo_stack[0] {
        Action::Compound { actions, .. } => assert_eq!(actions.len(), 2),
        other => panic!("Expected Action::Compound for batch deletion, got {:?}", other),
    }

    // Undo restore both deleted actors
    state.undo();
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 2);
}

#[test]
fn test_fase_j_texture_manager_memory_tracking_and_lru_pruning() {
    use ut99_editor::TextureManager;
    use ut99_format::{UtxPackage, UtxTexture};

    let mut tm = TextureManager::default();

    // Create 3 packages with 512x512 RGBA textures (512*512*4 = 1,048,576 bytes each)
    let make_pkg = |name: &str| -> UtxPackage {
        UtxPackage {
            package_name: name.to_string(),
            textures: vec![UtxTexture {
                name: format!("{}_Tex1", name),
                width: 512,
                height: 512,
                rgba_pixels: vec![255u8; 512 * 512 * 4],
                format: 0,
                masked: false,
                group: "None".to_string(),
            }],
        }
    };

    tm.insert_package("AlphaPkg".to_string(), make_pkg("AlphaPkg"));
    tm.insert_package("BetaPkg".to_string(), make_pkg("BetaPkg"));
    tm.insert_package("GammaPkg".to_string(), make_pkg("GammaPkg"));

    assert_eq!(tm.loaded_package_count(), 3);
    let total_bytes = tm.total_memory_usage_bytes();
    assert!(total_bytes >= 3 * 1024 * 1024, "Total usage must be >= 3 MB, got {}", total_bytes);

    // Pin AlphaPkg to protect it from eviction
    tm.pin_package("AlphaPkg");
    assert!(tm.is_package_pinned("AlphaPkg"));
    assert!(!tm.is_package_pinned("BetaPkg"));

    // Touch BetaPkg so that GammaPkg is the least recently used
    tm.touch_package("BetaPkg");

    // Set budget small enough to hold only 2 packages (~2.5 MB)
    let budget = 2 * 1024 * 1024 + 512 * 1024;
    let (evicted, freed) = tm.set_memory_budget_bytes(budget);
    assert!(evicted >= 1, "Must evict at least 1 package");
    assert!(freed >= 1024 * 1024, "Must free at least 1 MB");
    assert_eq!(tm.loaded_package_count(), 2);

    // AlphaPkg (pinned) and BetaPkg (recently touched) must still exist
    assert!(tm.get_package("AlphaPkg").is_some(), "Pinned AlphaPkg must NOT be evicted");
    assert!(tm.get_package("BetaPkg").is_some(), "Recently used BetaPkg must remain");
    assert!(tm.get_package("GammaPkg").is_none(), "Oldest GammaPkg must have been evicted");

    // Unpin AlphaPkg, then prune everything down to 0 target bytes
    tm.unpin_package("AlphaPkg");
    let (evicted_all, _) = tm.prune_lru(0);
    assert_eq!(evicted_all, 2, "Remaining 2 unpinned packages evicted");
    assert_eq!(tm.loaded_package_count(), 0);
    assert_eq!(tm.total_memory_usage_bytes(), 0);
}

#[test]
fn test_fase_q_inspector_panel_mode_transitions() {
    use ut99_editor::{EditorState, InspectorPanelMode};

    let mut state = EditorState::default();

    // Default inspector panel mode must be Split
    assert_eq!(state.inspector_panel_mode, InspectorPanelMode::Split);

    // Switch to OutlinerOnly mode (maximized scene hierarchy)
    state.inspector_panel_mode = InspectorPanelMode::OutlinerOnly;
    assert_eq!(state.inspector_panel_mode, InspectorPanelMode::OutlinerOnly);
    assert_ne!(state.inspector_panel_mode, InspectorPanelMode::Split);
    assert_ne!(state.inspector_panel_mode, InspectorPanelMode::InspectorOnly);

    // Switch to InspectorOnly mode (maximized selection properties)
    state.inspector_panel_mode = InspectorPanelMode::InspectorOnly;
    assert_eq!(state.inspector_panel_mode, InspectorPanelMode::InspectorOnly);
    assert_ne!(state.inspector_panel_mode, InspectorPanelMode::OutlinerOnly);

    // Cycle back to Split mode
    state.inspector_panel_mode = InspectorPanelMode::Split;
    assert_eq!(state.inspector_panel_mode, InspectorPanelMode::Split);

    // Verify Clone, Copy, PartialEq, Eq, and Debug
    let mode_copy = state.inspector_panel_mode;
    assert_eq!(mode_copy, InspectorPanelMode::Split);
    assert_eq!(format!("{:?}", mode_copy), "Split");
    assert_eq!(format!("{:?}", InspectorPanelMode::OutlinerOnly), "OutlinerOnly");
    assert_eq!(format!("{:?}", InspectorPanelMode::InspectorOnly), "InspectorOnly");
}

#[test]
fn test_fase_q_responsive_toolbar_metrics() {
    use ut99_editor::EditorState;

    let mut state = EditorState::default();

    // Verify UI Scale default is 1.0 (100%)
    assert!((state.ui_scale - 1.0).abs() < 1e-4);

    // Test scaling to 125% (typical high-DPI 1080p laptop display)
    state.ui_scale = 1.25;
    let pct_125 = (state.ui_scale * 100.0).round() as i32;
    assert_eq!(pct_125, 125);

    // Test scaling to 150% (typical high-DPI 4K display)
    state.ui_scale = 1.50;
    let pct_150 = (state.ui_scale * 100.0).round() as i32;
    assert_eq!(pct_150, 150);

    // Test scaling down to 75% (compact layout)
    state.ui_scale = 0.75;
    let pct_75 = (state.ui_scale * 100.0).round() as i32;
    assert_eq!(pct_75, 75);

    // Verify responsive threshold logic:
    // When remaining width >= 900.0, desktop mode shows individual buttons.
    // When remaining width < 900.0, compact mode collapses into Browse and Build dropdowns.
    // When remaining width < 350.0, ultra-compact mode shows only Browse dropdown.
    let is_desktop_mode = |w: f32| w >= 900.0;
    let has_build_dropdown = |w: f32| w >= 350.0 && w < 900.0;
    let has_browse_dropdown = |w: f32| w < 900.0;

    // 1440p / 4K full desktop available width (e.g. 1200px remaining)
    assert!(is_desktop_mode(1200.0));
    assert!(!has_build_dropdown(1200.0));
    assert!(!has_browse_dropdown(1200.0));

    // 1080p with 125% scale available width (e.g. 700px remaining)
    assert!(!is_desktop_mode(700.0));
    assert!(has_build_dropdown(700.0));
    assert!(has_browse_dropdown(700.0));

    // 720p with 150% scale or narrow window (e.g. 280px remaining)
    assert!(!is_desktop_mode(280.0));
    assert!(!has_build_dropdown(280.0));
    assert!(has_browse_dropdown(280.0));
}

#[test]
fn test_vector_icon_enum_properties_and_hashability() {
    use std::collections::HashSet;
    use ut99_editor::gui::icons::Icon;

    let icons = [
        Icon::GizmoTranslate,
        Icon::GizmoRotate,
        Icon::GizmoScale,
        Icon::ViewPerspective,
        Icon::ViewTop,
        Icon::ViewFront,
        Icon::ViewSide,
        Icon::LayoutSingle,
        Icon::LayoutQuad,
        Icon::ShadingWireframe,
        Icon::ShadingZonePortal,
        Icon::ShadingUnlit,
        Icon::ShadingLit,
        Icon::ShadingDepthComplexity,
        Icon::CsgAdd,
        Icon::CsgSubtract,
        Icon::CameraSpeed,
        Icon::Radar,
        Icon::SnapGrid,
        Icon::SnapRotate,
        Icon::Outliner,
        Icon::Inspector,
        Icon::Split,
        Icon::TargetFocus,
        Icon::Duplicate,
        Icon::FloorSnap,
        Icon::Delete,
        Icon::BuildLighting,
        Icon::BuildPaths,
        Icon::BuildAll,
        Icon::TextureBrowser,
        Icon::ActorBrowser,
        Icon::MeshBrowser,
        Icon::SoundBrowser,
        Icon::MusicBrowser,
        Icon::GroupBrowser,
        Icon::SurfaceProperties,
        Icon::ActorProperties,
        Icon::RadiiView,
        Icon::CommandPalette,
    ];

    // Verify all 40 icons are distinct
    let mut set = HashSet::new();
    for &icon in &icons {
        assert!(set.insert(icon), "Duplicate icon enum variant found: {:?}", icon);
        // Verify Copy, Clone, Debug
        let cloned = icon;
        assert_eq!(icon, cloned);
        assert!(!format!("{:?}", icon).is_empty());
    }
    assert_eq!(set.len(), 40);
}

#[test]
fn test_vector_icon_rendering_pipeline() {
    use ut99_editor::gui::icons::{self, Icon};

    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (rect, _response) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());

            // Paint every single icon variant to verify procedural painter math & no panics
            let all_icons = [
                Icon::GizmoTranslate,
                Icon::GizmoRotate,
                Icon::GizmoScale,
                Icon::ViewPerspective,
                Icon::ViewTop,
                Icon::ViewFront,
                Icon::ViewSide,
                Icon::LayoutSingle,
                Icon::LayoutQuad,
                Icon::ShadingWireframe,
                Icon::ShadingZonePortal,
                Icon::ShadingUnlit,
                Icon::ShadingLit,
                Icon::ShadingDepthComplexity,
                Icon::CsgAdd,
                Icon::CsgSubtract,
                Icon::CameraSpeed,
                Icon::Radar,
                Icon::SnapGrid,
                Icon::SnapRotate,
                Icon::Outliner,
                Icon::Inspector,
                Icon::Split,
                Icon::TargetFocus,
                Icon::Duplicate,
                Icon::FloorSnap,
                Icon::Delete,
                Icon::BuildLighting,
                Icon::BuildPaths,
                Icon::BuildAll,
                Icon::TextureBrowser,
                Icon::ActorBrowser,
                Icon::MeshBrowser,
                Icon::SoundBrowser,
                Icon::MusicBrowser,
                Icon::GroupBrowser,
                Icon::SurfaceProperties,
                Icon::ActorProperties,
                Icon::RadiiView,
                Icon::CommandPalette,
            ];

            for icon in all_icons {
                icons::paint_icon(ui.painter(), rect, icon, egui::Color32::from_rgb(200, 210, 225));
            }

            // Test icon helper widgets
            let icon_widget = icons::icon(ui, Icon::GizmoTranslate, egui::vec2(16.0, 16.0), egui::Color32::WHITE);
            assert_eq!(icon_widget.rect.width(), 16.0);
            assert_eq!(icon_widget.rect.height(), 16.0);

            // Test standalone icon_button
            let btn_resp = icons::icon_button(ui, Icon::CsgAdd, true, 20.0);
            assert!(btn_resp.rect.width() >= 16.0);

            // Test icon_label_button with and without text label
            let btn_labeled = icons::icon_label_button(ui, Icon::Split, Some("Split"), false);
            assert!(btn_labeled.rect.width() > 20.0);

            let btn_unlabeled = icons::icon_label_button(ui, Icon::Radar, None, true);
            assert!(btn_unlabeled.rect.width() >= 13.0);
        });
    });
}

#[test]
fn test_pro_301_and_304_compound_undo_redo_and_batch_operations() {
    let mut state = EditorState::default();
    state.world_brushes.clear();
    state.history.undo_stack.clear();
    state.history.redo_stack.clear();

    // Add 2 brushes
    state.set_builder_cube(Vec3::new(128.0, 128.0, 128.0));
    state.builder_brush.location = Vec3::new(0.0, 0.0, 0.0);
    state.csg_add();
    state.builder_brush.location = Vec3::new(256.0, 0.0, 0.0);
    state.csg_add();
    assert_eq!(state.world_brushes.len(), 2);

    // Select both brushes
    state.multi_selected_brushes.insert(0);
    state.multi_selected_brushes.insert(1);
    state.selected_brush_index = Some(0);
    assert!(state.has_multi_selection());

    // Translate multi-selection with atomic compound undo
    let moved = state.translate_multi_selection_with_undo(Vec3::new(64.0, 32.0, 16.0));
    assert_eq!(moved, 2);
    assert_eq!(state.world_brushes[0].location, Vec3::new(64.0, 32.0, 16.0));
    assert_eq!(state.world_brushes[1].location, Vec3::new(320.0, 32.0, 16.0));

    // Single Undo reverts both brushes atomically!
    state.undo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(0.0, 0.0, 0.0));
    assert_eq!(state.world_brushes[1].location, Vec3::new(256.0, 0.0, 0.0));

    // Single Redo re-applies both brushes atomically!
    state.redo();
    assert_eq!(state.world_brushes[0].location, Vec3::new(64.0, 32.0, 16.0));
    assert_eq!(state.world_brushes[1].location, Vec3::new(320.0, 32.0, 16.0));

    // Duplicate batch via duplicate_selected()
    state.duplicate_selected();
    assert_eq!(state.world_brushes.len(), 4);
    assert_eq!(state.multi_selected_brushes.len(), 2);

    // Undo batch duplication in a single undo step
    state.undo();
    assert_eq!(state.world_brushes.len(), 2);
}

#[test]
fn test_pro_302_305_310_and_pro_tip_01_ui_and_camera_framing() {
    use ut99_editor::gui::theme::ProTooltipExt;

    let mut state = EditorState::default();
    state.world_brushes.clear();

    // Add 2 distant brushes and multi-select them for AABB camera framing
    state.set_builder_cube(Vec3::new(256.0, 256.0, 256.0));
    state.builder_brush.location = Vec3::new(-500.0, 0.0, 0.0);
    state.csg_add();
    state.builder_brush.location = Vec3::new(500.0, 0.0, 0.0);
    state.csg_add();

    state.multi_selected_brushes.insert(0);
    state.multi_selected_brushes.insert(1);
    state.selected_brush_index = Some(0);

    state.focus_camera_on_selection();
    // Distance from center (0,0,0) must be larger than default 400.0 to enclose the 1256-wide AABB
    assert!(
        state.camera.position.length() > 900.0,
        "multi-selection camera framing should zoom out to fit entire AABB, got {:?}",
        state.camera.position
    );

    // Test 01, HUD badge, and batch context menu rendering in egui
    state.active_gizmo_axis = ut99_editor::raycast::GizmoAxis::X;
    state.active_gizmo_delta = Vec3::new(128.0, 0.0, -32.0);
    state.show_viewport_context_menu = true;

    let ctx = egui::Context::default();
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        ut99_editor::gui::context_menu::render_viewport_context_menu(ctx, &mut state);
        egui::CentralPanel::default().show(ctx, |ui| {
            let _ = ui.button("Gizmo Move").pro_tooltip_ext(
                "Translate Gizmo",
                Some("W"),
                "Translate selected objects along X, Y, or Z axes",
                Some("Alt+Drag: Instant Duplicate"),
                Some("Grid Snap: 16 UU"),
            );
        });
    });
}



