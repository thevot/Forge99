use glam::Vec3;
use ut99_bsp::{
    CinematicCameraTrack, CinematicPreset, SplineCameraKeyframe,
};
use ut99_editor::editor_state::EditorState;
use ut99_format::unr::UnrLevel;

#[test]
fn test_cinematic_track_creation_and_catmull_rom_sampling() {
    let mut track = CinematicCameraTrack::new("TestLinearTrack");
    track.keyframes.push(SplineCameraKeyframe {
        position: Vec3::new(0.0, 0.0, 100.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        fov: 90.0,
        speed: 300.0,
        pause_time: 0.0,
        label: "Node0".to_string(),
    });
    track.keyframes.push(SplineCameraKeyframe {
        position: Vec3::new(300.0, 0.0, 100.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        fov: 85.0,
        speed: 300.0,
        pause_time: 0.0,
        label: "Node1".to_string(),
    });
    track.keyframes.push(SplineCameraKeyframe {
        position: Vec3::new(600.0, 0.0, 100.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        fov: 80.0,
        speed: 300.0,
        pause_time: 0.0,
        label: "Node2".to_string(),
    });
    track.keyframes.push(SplineCameraKeyframe {
        position: Vec3::new(900.0, 0.0, 100.0),
        rotation: Vec3::new(0.0, 0.0, 0.0),
        fov: 75.0,
        speed: 300.0,
        pause_time: 0.0,
        label: "Node3".to_string(),
    });

    let total_len = track.total_length();
    assert!((total_len - 900.0).abs() < 20.0, "Total length should be ~900 uu (got {})", total_len);

    // Sample at midpoint t = 0.5
    let (pos, _, fov) = track.sample_at(0.5);
    assert!((pos.x - 450.0).abs() < 15.0, "Midpoint X should be ~450 uu (got {})", pos.x);
    assert!((pos.y - 0.0).abs() < 1e-2);
    assert!((pos.z - 100.0).abs() < 1e-2);
    assert!((fov - 82.5).abs() < 1.0, "Midpoint FOV should be ~82.5° (got {})", fov);
}

#[test]
fn test_canonical_cinematic_presets() {
    for preset in CinematicPreset::all() {
        let track = preset.create_track();
        assert!(track.keyframes.len() >= 3, "Presets must have at least 3 control keyframes");

        let duration = track.estimated_duration();
        assert!(duration > 1.0, "Track duration must be greater than 1 second (got {}s)", duration);

        let poly = track.sample_polyline(8);
        assert!(!poly.is_empty(), "Polyline sampling must produce non-empty path points");
    }
}

#[test]
fn test_cinematic_playback_update_and_pilot_camera() {
    let mut state = EditorState::default();
    state.apply_cinematic_preset(CinematicPreset::ArenaFlybyIntro360);

    state.cinematic_settings.is_playing = true;
    state.cinematic_settings.playback_speed = 1.0;
    state.cinematic_settings.pilot_camera = true;

    let initial_cam_pos = state.camera.position;
    state.update_cinematic_playback(0.5);

    assert!(state.cinematic_settings.playhead_t > 0.0, "Playhead must advance with delta time");
    assert_ne!(state.camera.position, initial_cam_pos, "Camera position must sync to spline during pilot mode");

    // Camera frustum generation test
    let frustum_lines = state.get_camera_frustum_lines_at_playhead();
    assert_eq!(frustum_lines.len(), 8, "Camera frustum must yield exactly 8 wireframe lines");
}

#[test]
fn test_cinematic_export_to_interpolation_points_and_undo_redo() {
    let mut state = EditorState::default();
    state.loaded_unr = Some(UnrLevel::default());

    state.apply_cinematic_preset(CinematicPreset::CorridorTrackingShot);
    let expected_kfs = state.cinematic_settings.active_track.keyframes.len();

    // Export to actors
    let created_indices = state.export_cinematic_to_level_actors();
    assert_eq!(created_indices.len(), expected_kfs);
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), expected_kfs);

    // Verify actor class and properties
    let actor = &state.loaded_unr.as_ref().unwrap().actors[0];
    assert_eq!(actor.class_name, "InterpolationPoint");
    assert_eq!(actor.tag, "CorridorTrackingShot");

    // Test Undo (removes exported actors in reverse)
    for _ in 0..expected_kfs {
        state.undo();
    }
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), 0, "Undo must remove all exported actors");

    // Test Redo
    for _ in 0..expected_kfs {
        state.redo();
    }
    assert_eq!(state.loaded_unr.as_ref().unwrap().actors.len(), expected_kfs, "Redo must restore exported actors");
}
