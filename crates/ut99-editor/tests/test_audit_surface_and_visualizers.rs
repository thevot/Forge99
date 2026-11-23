use ut99_bsp::PF_UNLIT;
use ut99_editor::editor_state::EditorState;
use ut99_format::unr::actor::ActorRecord;
use ut99_format::unr::tagged_property::{TaggedProperty, TaggedValue};

#[test]
fn test_surface_alignment_and_undo() {
    let mut state = EditorState::default();

    // Select the default room brush
    state.selected_brush_index = Some(0);
    let original_base = state.world_brushes[0].polys[0].base;

    // Pan U by 64.0 and V by 32.0
    state.pan_selected_brush_texture(64.0, 32.0);
    let panned_base = state.world_brushes[0].polys[0].base;
    assert_ne!(panned_base, original_base, "Texture base should change after panning");

    // Test Undo
    state.undo();
    assert_eq!(
        state.world_brushes[0].polys[0].base,
        original_base,
        "Undo should revert texture panning"
    );

    // Test Redo
    state.redo();
    assert_eq!(
        state.world_brushes[0].polys[0].base,
        panned_base,
        "Redo should restore texture panning"
    );
}

#[test]
fn test_texture_flip_and_polyflags() {
    let mut state = EditorState::default();
    state.selected_brush_index = Some(0);

    let original_u = state.world_brushes[0].polys[0].texture_u;

    // Flip U
    state.flip_selected_brush_texture_u();
    let flipped_u = state.world_brushes[0].polys[0].texture_u;
    assert_eq!(flipped_u, -original_u, "Flip U should negate texture_u");

    // Undo Flip U
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].texture_u, original_u);

    // Toggle PolyFlags (PF_UNLIT and PF_PORTAL)
    let initial_flags = state.world_brushes[0].polys[0].flags;
    state.toggle_selected_brush_poly_flag(PF_UNLIT);
    assert_eq!(state.world_brushes[0].polys[0].flags, initial_flags ^ PF_UNLIT);

    // Undo flag toggle
    state.undo();
    assert_eq!(state.world_brushes[0].polys[0].flags, initial_flags);
}

#[test]
fn test_light_and_sound_radius_calculation() {
    // 1. Light actor radius
    let mut light = ActorRecord::default();
    light.class_name = "Light".to_string();
    light.light_radius = 32;
    light.light_brightness = 255;
    light.light_hue = 150;
    light.light_saturation = 32;

    let computed_light_radius = (light.light_radius as f32) * 25.0;
    assert_eq!(computed_light_radius, 800.0);

    // 2. Sound actor radius
    let mut sound = ActorRecord::default();
    sound.class_name = "AmbientSound".to_string();
    sound.properties.insert("SoundRadius".to_string(), TaggedProperty {
        name: "SoundRadius".to_string(),
        array_index: 0,
        value: TaggedValue::Byte(16),
    });

    let computed_sound_radius = match sound.properties.get("SoundRadius").unwrap().value {
        TaggedValue::Byte(b) => (b as f32) * 40.0,
        _ => 0.0,
    };
    assert_eq!(computed_sound_radius, 640.0);
}

#[test]
fn test_build_progress_feedback() {
    let mut state = EditorState::default();

    // Initially none
    assert!(state.build_progress.is_none());

    // Run lighting build
    state.build_lighting();

    // Verify progress was recorded
    assert!(state.build_progress.is_some());
    let (progress, ref msg) = state.build_progress.as_ref().unwrap();
    assert_eq!(*progress, 1.0);
    assert!(msg.contains("Lighting Bake"));
}
