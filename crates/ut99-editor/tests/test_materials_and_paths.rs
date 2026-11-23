use glam::Vec3;
use ut99_editor::EditorState;

#[test]
fn test_texture_manager_discovery_and_loading() {
    let mut state = EditorState::default();
    let packages = state.texture_manager.available_packages();

    println!("Discovered {} texture packages:", packages.len());
    for p in packages.iter().take(10) {
        println!(" - {}", p);
    }

    if packages.is_empty() {
        eprintln!("No local UT99 installation found; skipping texture discovery test");
        return;
    }

    assert!(!packages.is_empty(), "Should discover UT99 texture packages in Textures folder");
    assert!(
        packages.iter().any(|p| p.eq_ignore_ascii_case("Indus1")),
        "Indus1.utx package should be discovered"
    );

    // Load Indus1 package
    let (first_tex_name, width, height, pixel_count) = {
        let pkg = state.texture_manager.load_package("Indus1").expect("Should load Indus1 package");
        assert_eq!(pkg.package_name, "Indus1");
        assert!(!pkg.textures.is_empty(), "Indus1 should contain textures");

        let first_tex = &pkg.textures[0];
        println!("First texture: {} ({}x{})", first_tex.name, first_tex.width, first_tex.height);
        assert!(first_tex.width > 0 && first_tex.height > 0);
        assert_eq!(first_tex.rgba_pixels.len(), (first_tex.width * first_tex.height * 4) as usize);
        (first_tex.name.clone(), first_tex.width, first_tex.height, first_tex.rgba_pixels.len())
    };

    assert_eq!(pixel_count, (width * height * 4) as usize);

    // Retrieve via helper
    let retrieved = state.texture_manager.get_texture("Indus1", &first_tex_name);
    assert!(retrieved.is_some());
}

#[test]
fn test_material_assignment_and_surface_uv_controls() {
    let mut state = EditorState::default();

    state.active_texture = Some("Indus1.Indus_B01".to_string());

    // 1. Apply to Builder Brush
    state.apply_active_texture_to_builder();
    for poly in &state.builder_brush.polys {
        assert_eq!(poly.texture, "Indus1.Indus_B01");
    }

    // 2. CSG Add inherits active texture
    let initial_brush_count = state.world_brushes.len();
    state.csg_add();
    assert_eq!(state.world_brushes.len(), initial_brush_count + 1);
    let added_brush = state.world_brushes.last().unwrap();
    for poly in &added_brush.polys {
        assert_eq!(poly.texture, "Indus1.Indus_B01");
    }

    // 3. Select the brush and test Pan, Scale, and Align
    let sel_idx = state.world_brushes.len() - 1;
    state.selected_brush_index = Some(sel_idx);

    // Change texture
    state.active_texture = Some("City.wt_wal01".to_string());
    state.apply_active_texture_to_selected_brush();
    for poly in &state.world_brushes[sel_idx].polys {
        assert_eq!(poly.texture, "City.wt_wal01");
    }

    // Pan UV
    let initial_base = state.world_brushes[sel_idx].polys[0].base;
    state.pan_selected_brush_texture(32.0, 16.0);
    let panned_base = state.world_brushes[sel_idx].polys[0].base;
    assert_ne!(initial_base, panned_base, "Base should shift after panning");

    // Scale UV
    let initial_u_len = state.world_brushes[sel_idx].polys[0].texture_u.length();
    state.scale_selected_brush_texture(2.0, 2.0);
    let scaled_u_len = state.world_brushes[sel_idx].polys[0].texture_u.length();
    assert!((scaled_u_len - initial_u_len * 2.0).abs() < 1e-3, "Texture U should scale by 2.0x");

    // Align UV
    state.align_selected_brush_texture("floor");
    for poly in &state.world_brushes[sel_idx].polys {
        assert_eq!(poly.texture_u, Vec3::new(1.0, 0.0, 0.0));
        assert_eq!(poly.texture_v, Vec3::new(0.0, 1.0, 0.0));
    }
}

#[test]
fn test_ai_navigation_path_network() {
    let mut state = EditorState::default();

    // Start with a clean map
    state.new_map();
    // Default new_map creates a PlayerStart at (0, 0, 48)

    // Add a nearby PathNode at (300, 0, 48) -> should form a Walkable link (Green)
    state.builder_brush.location = Vec3::new(300.0, 0.0, 48.0);
    state.add_path_node_at_builder();

    // Add a jump node at (600, 0, 150) -> delta Z = 102 (within jump height) -> Jump link (Magenta)
    state.builder_brush.location = Vec3::new(600.0, 0.0, 150.0);
    state.add_path_node_at_builder();

    // Add a drop node at (900, 0, -200) -> delta Z = -350 (within drop height) -> Drop link (Cyan)
    state.builder_brush.location = Vec3::new(900.0, 0.0, -200.0);
    state.add_path_node_at_builder();

    // Add a weapon at builder
    state.builder_brush.location = Vec3::new(150.0, 0.0, 48.0);
    state.add_weapon_or_pickup_at_builder("ShockRifle");

    state.build_ai_paths();

    println!("Total AI Path Links calculated: {}", state.ai_path_links.len());
    assert!(!state.ai_path_links.is_empty(), "AI navigation path links should be generated");

    // Verify colored link types exist
    let has_walkable = state.ai_path_links.iter().any(|(_, _, c)| (c[1] - 0.95).abs() < 0.1 && c[0] < 0.3);
    let has_jump = state.ai_path_links.iter().any(|(_, _, c)| (c[0] - 0.95).abs() < 0.1 && (c[2] - 0.95).abs() < 0.1);

    assert!(has_walkable, "Should have calculated walkable green links");
    assert!(has_jump, "Should have calculated jump magenta links");
}
