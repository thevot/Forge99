use std::{path::PathBuf, time::{Duration, Instant}};
use ut99_editor::{EditorState, editor_state::{PendingDirtyAction, SaveOperationKind, SavePhase, SaveWorkerMessage}};
mod common;

fn start_export(state: &mut EditorState, path: &std::path::Path, format: &str) {
    match format {
        "gltf" => state.trigger_export_gltf(path),
        "obj" => state.trigger_export_obj(path),
        "t3d" => state.trigger_export_t3d(path),
        _ => unreachable!(),
    }
}
fn success(state: &mut EditorState, path: &std::path::Path) {
    state.pending_save_op = None;
    state.handle_save_worker_message(SaveWorkerMessage::Success {
        saved_path: path.into(), file_size_bytes: 123, duration: Duration::ZERO,
    });
}
#[test]
fn real_exports_preserve_document_identity_dirty_flag_and_recent_maps() {
    let dir = common::TestDir::new();
    let ctx = egui::Context::default();
    for format in ["gltf", "obj", "t3d"] {
        for (named, dirty) in [(true, true), (false, true), (true, false)] {
            let mut state = EditorState::default();
            state.new_map();
            let original = named.then(|| dir.path().join("Original.unr"));
            state.map_path = original.clone();
            state.map_is_dirty = dirty;
            state.recent_maps = vec![dir.path().join("Other.unr")];
            let recent = state.recent_maps.clone();
            let output = dir.path().join(format!("Preview-{named}-{dirty}.{format}"));
            start_export(&mut state, &output, format);
            let deadline = Instant::now() + Duration::from_secs(10);
            while !matches!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Completed | SavePhase::Failed) {
                state.process_pending_save_operation(&ctx);
                assert!(Instant::now() < deadline, "export timed out");
                std::thread::sleep(Duration::from_millis(5));
            }
            assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Completed);
            assert!(output.is_file());
            assert_eq!(state.map_path, original, "{format} changed document path");
            assert_eq!(state.map_is_dirty, dirty, "{format} changed dirty flag");
            assert_eq!(state.recent_maps, recent, "{format} changed recent maps");
            assert!(state.toasts.last().unwrap().message.starts_with("Exported:"));
        }
    }
}
#[test]
fn export_completion_or_failure_cannot_release_a_pending_exit() {
    for fails in [false, true] {
        let mut state = EditorState::default();
        state.map_path = Some(PathBuf::from("Original.unr"));
        state.map_is_dirty = true;
        state.trigger_export_gltf("Preview.gltf");
        state.pending_dirty_action = Some(PendingDirtyAction::ExitApp);
        state.show_unsaved_exit_modal = false;
        if fails {
            state.handle_save_worker_message(SaveWorkerMessage::Error { message: "disk full".into() });
        } else { success(&mut state, std::path::Path::new("Preview.gltf")); }
        assert!(!state.confirm_exit_without_saving);
        assert!(matches!(state.pending_dirty_action, Some(PendingDirtyAction::ExitApp)));
        assert!(state.show_unsaved_exit_modal);
        assert!(state.map_is_dirty);
        assert_eq!(state.map_path, Some(PathBuf::from("Original.unr")));
        if fails { assert!(state.toasts.last().unwrap().message.contains("Export failed")); }
    }
}
#[test]
fn document_saves_still_update_identity_and_release_pending_exit() {
    for kind in [0, 1, 2] {
        let mut state = EditorState::default();
        state.map_is_dirty = true;
        let path = PathBuf::from(if kind == 2 { "Saved.t3d" } else { "Saved.unr" });
        match kind {
            0 => state.trigger_save_unr(&path),
            1 => state.trigger_save_unr_native(&path),
            _ => state.trigger_save_t3d(&path),
        }
        state.pending_dirty_action = Some(PendingDirtyAction::ExitApp);
        success(&mut state, &path);
        assert_eq!(state.map_path, Some(path.clone()));
        assert_eq!(state.recent_maps.first(), Some(&path));
        assert!(!state.map_is_dirty);
        assert!(state.confirm_exit_without_saving);
    }
}
#[test]
fn unclassified_completion_is_not_proof_of_a_document_save() {
    let mut state = EditorState::default();
    state.map_is_dirty = true;
    state.pending_dirty_action = Some(PendingDirtyAction::ExitApp);
    success(&mut state, std::path::Path::new("Unknown.unr"));
    assert!(state.map_path.is_none());
    assert!(state.map_is_dirty);
    assert!(!state.confirm_exit_without_saving);
}
#[test]
fn another_request_cannot_replace_a_pending_or_running_export() {
    for running in [false, true] {
        let mut state = EditorState::default();
        state.trigger_export_gltf("Preview.gltf");
        if running { let (_, rx) = std::sync::mpsc::channel(); state.save_worker_rx = Some(rx); }
        state.pending_dirty_action = Some(PendingDirtyAction::ExitApp);
        state.trigger_save_unr("Original.unr");
        assert!(matches!(state.pending_save_op, Some(SaveOperationKind::ExportGltf(_))));
        assert!(matches!(state.save_feedback.as_ref().unwrap().kind, SaveOperationKind::ExportGltf(_)));
        assert!(state.show_unsaved_exit_modal);
    }
}
#[test]
fn failed_exports_preserve_the_document_and_report_export_errors() {
    let dir = common::TestDir::new();
    let blocker = dir.path().join("not-a-directory");
    std::fs::write(&blocker, "blocker").unwrap();
    let ctx = egui::Context::default();
    for format in ["t3d", "gltf", "obj"] {
        let mut state = EditorState::default();
        state.new_map();
        state.map_path = Some(dir.path().join("Original.unr"));
        state.map_is_dirty = true;
        state.recent_maps = vec![dir.path().join("Other.unr")];
        let original = state.map_path.clone();
        let recent = state.recent_maps.clone();
        start_export(&mut state, &blocker.join(format!("output.{format}")), format);
        let deadline = Instant::now() + Duration::from_secs(10);
        while !matches!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Completed | SavePhase::Failed) {
            state.process_pending_save_operation(&ctx);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(state.save_feedback.as_ref().unwrap().phase, SavePhase::Failed);
        assert_eq!(state.map_path, original);
        assert_eq!(state.recent_maps, recent);
        assert!(state.map_is_dirty);
        assert!(state.toasts.last().unwrap().message.starts_with("Export failed:"));
    }
}

#[test]
fn escape_keeps_active_operation_context_but_dismisses_completed_feedback() {
    for phase in [SavePhase::PendingPreparation, SavePhase::Executing, SavePhase::Completed] {
        let mut state = EditorState::default();
        state.trigger_export_gltf("Preview.gltf");
        state.save_feedback.as_mut().unwrap().phase = phase;
        let ctx = egui::Context::default();
        let input = egui::RawInput {
            events: vec![egui::Event::Key {
                key: egui::Key::Escape, physical_key: None, pressed: true,
                repeat: false, modifiers: egui::Modifiers::NONE,
            }],
            ..Default::default()
        };
        let _ = ctx.run(input, |ctx| ut99_editor::gui::save_feedback_dialog::render_save_feedback_overlay(ctx, &mut state));
        assert_eq!(state.save_feedback.is_some(), phase != SavePhase::Completed);
        assert_eq!(state.pending_save_op.is_some(), phase != SavePhase::Completed);
    }
}
