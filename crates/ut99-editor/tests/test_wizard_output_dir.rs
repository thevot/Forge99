use ut99_editor::editor_state::EditorState;

#[test]
fn wizard_output_dir_is_separate_from_ut_maps_and_overridable() {
    let mut state = EditorState::default();
    let default_dir = state.wizard_output_directory();
    assert!(default_dir.ends_with("GeneratedMaps"), "{default_dir:?}");
    assert_ne!(default_dir, state.default_maps_directory());

    let custom = std::env::temp_dir().join("forge99_wizard_out_test");
    state.wizard_output_dir = Some(custom.clone());
    assert_eq!(state.wizard_output_directory(), custom);
    assert!(custom.is_dir());
    let _ = std::fs::remove_dir_all(&custom);
}
