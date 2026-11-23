use glam::Vec3;
use ut99_bsp::ObjModel;
use ut99_editor::{EditorState, MeshLibrary};

#[test]
fn test_mesh_library_initialization_and_categories() {
    let library = MeshLibrary::default();

    assert!(library.meshes.len() >= 8, "Default library should contain at least 8 canonical presets");

    let names: Vec<&str> = library.meshes.iter().map(|m| m.name.as_str()).collect();
    assert!(names.contains(&"Pillar_Cylinder_256"), "Must contain Pillar_Cylinder_256");
    assert!(names.contains(&"Torch_Pedestal_96"), "Must contain Torch_Pedestal_96");
    assert!(names.contains(&"Crate_Cargo_64"), "Must contain Crate_Cargo_64");
    assert!(names.contains(&"Barrel_Hazard_80"), "Must contain Barrel_Hazard_80");
    assert!(names.contains(&"Arch_Portal_512"), "Must contain Arch_Portal_512");
    assert!(names.contains(&"Staircase_Spiral_384"), "Must contain Staircase_Spiral_384");
    assert!(names.contains(&"Conduit_Ring_128"), "Must contain Conduit_Ring_128");
    assert!(names.contains(&"Pyramid_Cap_64"), "Must contain Pyramid_Cap_64");

    // Test categories
    let categories = library.categories();
    assert!(categories.contains(&"All".to_string()));
    assert!(categories.contains(&"Architecture".to_string()));
    assert!(categories.contains(&"Decorations".to_string()));
    assert!(categories.contains(&"Props".to_string()));
    assert!(categories.contains(&"Custom".to_string()));

    // Test mesh metrics
    let crate_mesh = library.get_mesh("Crate_Cargo_64").expect("Crate_Cargo_64 must exist");
    assert_eq!(crate_mesh.poly_count(), 6, "Cube has 6 faces");
    let dim = crate_mesh.dimensions();
    assert!((dim.x - 64.0).abs() < 0.1);
    assert!((dim.y - 64.0).abs() < 0.1);
    assert!((dim.z - 64.0).abs() < 0.1);

    let pillar_mesh = library.get_mesh("Pillar_Cylinder_256").expect("Pillar must exist");
    assert!(pillar_mesh.poly_count() >= 14, "12-sided cylinder has 12 sides + 2 caps");
    let p_dim = pillar_mesh.dimensions();
    assert!((p_dim.z - 256.0).abs() < 0.1);
}

#[test]
fn test_mesh_placement_and_undo() {
    let mut state = EditorState::default();
    assert_eq!(state.static_meshes.len(), 0);

    let target_loc = Vec3::new(256.0, 512.0, 128.0);
    let place_res = state.place_mesh_from_library("Crate_Cargo_64", target_loc);
    assert!(place_res.is_ok(), "Placement must succeed: {:?}", place_res.err());

    assert_eq!(state.static_meshes.len(), 1);
    let placed = &state.static_meshes[0];
    assert_eq!(placed.location, target_loc);
    assert_eq!(placed.polys.len(), 6);
    assert!(placed.name.starts_with("Crate_Cargo_64"));

    // Test Undo
    state.undo();
    assert_eq!(state.static_meshes.len(), 0, "Undo should revert placed mesh");

    // Test Redo
    state.redo();
    assert_eq!(state.static_meshes.len(), 1, "Redo should restore placed mesh");
    assert_eq!(state.static_meshes[0].location, target_loc);
}

#[test]
fn test_extract_mesh_to_builder_brush() {
    let mut state = EditorState::default();

    // Set custom builder brush location
    let builder_loc = Vec3::new(100.0, 200.0, 300.0);
    state.builder_brush.location = builder_loc;

    let res = state.extract_mesh_to_builder("Pillar_Cylinder_256");
    assert!(res.is_ok());

    assert_eq!(state.builder_brush.location, builder_loc, "Builder location should be preserved");
    assert_eq!(state.builder_brush.name, "ActiveBuilder");
    assert!(state.builder_brush.polys.len() >= 14);

    let (min, max) = state.builder_brush.world_bounds();
    let height = max.z - min.z;
    assert!((height - 256.0).abs() < 1.0, "Pillar height should be approximately 256 uu");
}

#[test]
fn test_import_and_export_obj_mesh_library() {
    let mut state = EditorState::default();

    // Synthetic Wavefront OBJ (triangular prism / ramp)
    let obj_source = r#"
# Ramp Wavefront OBJ
o RampCustom
v 0.0 0.0 0.0
v 128.0 0.0 0.0
v 128.0 64.0 0.0
v 0.0 64.0 0.0
v 0.0 0.0 64.0
v 0.0 64.0 64.0
f 1 2 3 4
f 1 4 6 5
f 1 5 2
f 3 2 5 6
f 4 3 6
"#;

    let import_res = state.import_obj_to_mesh_library("RampCustom", "Custom", "Custom diagonal ramp", obj_source);
    assert!(import_res.is_ok(), "Import OBJ must succeed: {:?}", import_res.err());

    let imported = state.mesh_library.get_mesh("RampCustom").expect("Imported mesh must exist in library");
    assert_eq!(imported.category, "Custom");
    assert_eq!(imported.poly_count(), 5, "Prism should have 5 faces");

    // Test export to OBJ
    let exported_obj = state.mesh_library.export_obj("RampCustom").expect("Export to OBJ must succeed");
    assert!(exported_obj.contains("v "), "Exported OBJ must contain vertices");
    assert!(exported_obj.contains("f "), "Exported OBJ must contain faces");

    // Verify round-trip parsing
    let parsed_back = ObjModel::parse(&exported_obj).expect("Exported OBJ must be valid and re-parsable");
    assert!(parsed_back.vertices.len() >= 6);
    assert_eq!(parsed_back.faces.len(), 5);
}
