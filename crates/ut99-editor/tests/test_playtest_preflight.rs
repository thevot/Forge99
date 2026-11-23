use std::{path::PathBuf, sync::Mutex, time::{Duration, Instant}};
use ut99_editor::{EditorState, CompilerBridge};
use ut99_editor::editor_state::{SaveOperationKind, SaveWorkerMessage, ToastKind};
mod common;
static TEST_LOCK: Mutex<()> = Mutex::new(());

struct Game { dir: common::TestDir, executable: PathBuf }
impl Game {
    fn new(compile: bool) -> Self {
        let dir = common::TestDir::new();
        let root = dir.path().join("Game Folder");
        for folder in ["System", "Maps", "Sounds", "Music"] { std::fs::create_dir_all(root.join(folder)).unwrap(); }
        let executable = root.join("System").join(if cfg!(windows) { "UnrealTournament.exe" } else { "ut" });
        if compile {
            let source = dir.path().join("mock_game.rs");
            std::fs::write(&source, r#"fn main() {
                let args: Vec<String> = std::env::args().skip(1).collect();
                let capture = args.iter().find_map(|a| a.strip_prefix("-capture=")).expect("capture argument");
                let text = format!("cwd={}\n{}", std::env::current_dir().unwrap().display(), args.join("\n"));
                std::fs::write(capture, text).unwrap();
            }"#).unwrap();
            let output = std::process::Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
                .arg("--edition=2021").arg(&source).arg("-o").arg(&executable).output().unwrap();
            assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        } else {
            std::fs::write(&executable, b"invalid executable").unwrap();
            #[cfg(unix)] { use std::os::unix::fs::PermissionsExt; std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap(); }
        }
        Self { dir, executable }
    }
    fn state(&self) -> EditorState {
        let mut state = EditorState::default();
        state.new_map();
        state.ut99_executable_path = self.executable.to_string_lossy().to_string();
        state.default_export_dir = Some(self.dir.path().join("Editor Outputs"));
        state
    }
    fn capture(&self) -> PathBuf { self.dir.path().join("arguments with spaces.txt") }
    fn read_capture(&self) -> String {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Ok(text) = std::fs::read_to_string(self.capture()) { if !text.is_empty() { return text; } }
            assert!(Instant::now() < deadline, "mock game did not record its arguments");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}
impl Drop for Game { fn drop(&mut self) { CompilerBridge::set_custom_ut_dir(None); } }

#[test]
fn missing_executable_does_not_save_or_change_document_state() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = common::TestDir::new();
    let target = dir.path().join("Unwritten.unr");
    let mut state = EditorState::default();
    state.new_map();
    state.map_path = Some(target.clone());
    state.map_is_dirty = true;
    state.ut99_executable_path = dir.path().join("Missing.exe").display().to_string();
    assert!(state.play_map_in_ut99().is_err());
    assert!(!target.exists());
    assert!(state.map_is_dirty);
    assert_eq!(state.map_path.as_ref(), Some(&target));
    assert!(state.save_feedback.is_none());
    assert!(matches!(state.toasts.last().unwrap().kind, ToastKind::Error));
}

#[test]
fn pending_and_running_saves_reject_playtest_without_writing() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    let mut state = game.state();
    let target = game.dir.path().join("MustNotExist.unr");
    state.map_path = Some(target.clone());
    state.pending_save_op = Some(SaveOperationKind::SaveUnrNative(target.clone()));
    assert!(state.play_map_in_ut99().unwrap_err().contains("progress"));
    assert!(!target.exists());
    assert!(state.pending_save_op.is_some());
    state.pending_save_op = None;
    let (_tx, rx) = std::sync::mpsc::channel::<SaveWorkerMessage>();
    state.save_worker_rx = Some(rx);
    assert!(state.play_map_in_ut99().unwrap_err().contains("progress"));
    assert!(!target.exists());
    assert!(state.save_worker_rx.is_some());
}

#[test]
fn t3d_source_is_not_overwritten_by_binary_playtest_save() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    let mut state = game.state();
    let source = game.dir.path().join("Original.t3d");
    std::fs::write(&source, b"Begin Map\nEnd Map\n").unwrap();
    state.map_path = Some(source.clone());
    state.map_is_dirty = true;
    assert!(state.play_map_in_ut99().is_err());
    assert_eq!(std::fs::read(&source).unwrap(), b"Begin Map\nEnd Map\n");
    assert!(state.map_is_dirty);
}

#[test]
fn missing_map_does_not_start_the_game_process() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(true);
    let mut state = game.state();
    state.ut99_launch_args = format!("-capture=\"{}\"", game.capture().display());
    assert!(state.launch_saved_map_in_ut99(&game.dir.path().join("Missing.unr")).is_err());
    assert!(!game.capture().exists());
    assert!(matches!(state.toasts.last().unwrap().kind, ToastKind::Error));
}

#[test]
fn root_folder_launch_preserves_absolute_map_url_options_and_quoted_flags() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(true);
    let mut state = game.state();
    state.ut99_executable_path = game.executable.parent().unwrap().parent().unwrap().display().to_string();
    let saved = game.dir.path().join("External Map With Spaces.unr");
    std::fs::write(&saved, b"mock map").unwrap();
    state.ut99_launch_args = format!("?game=Botpack.CTFGame ?mutator=Botpack.InstaGibDM -windowed -capture=\"{}\" -ini=\"C:\\Games Folder\\UT.ini\"", game.capture().display());
    state.launch_saved_map_in_ut99(&saved).unwrap();
    let capture = game.read_capture();
    let lines: Vec<_> = capture.lines().collect();
    assert_eq!(lines[0], format!("cwd={}", game.executable.parent().unwrap().display()));
    assert_eq!(lines[1], format!("{}?game=Botpack.CTFGame?mutator=Botpack.InstaGibDM", saved.display()));
    assert!(lines.contains(&"-windowed"));
    assert!(lines.contains(&r"-ini=C:\Games Folder\UT.ini"));
    assert!(matches!(state.toasts.last().unwrap().kind, ToastKind::Success));
}

#[test]
fn spawn_failure_reports_that_the_map_was_saved() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    let mut state = game.state();
    let target = game.dir.path().join("SavedButNotStarted.unr");
    state.map_path = Some(target.clone());
    state.map_is_dirty = true;
    let error = state.play_map_in_ut99().unwrap_err();
    assert!(target.is_file());
    assert!(!state.map_is_dirty);
    assert!(error.contains("saved") && error.contains("start"));
    assert_eq!(state.toasts.last().unwrap().message, error);
}

#[test]
fn failed_save_keeps_dirty_state_and_emits_one_playtest_error() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    let mut state = game.state();
    let blocker = game.dir.path().join("NotADirectory");
    std::fs::write(&blocker, b"preserve").unwrap();
    state.map_path = Some(blocker.join("CannotSave.unr"));
    state.map_is_dirty = true;
    let first_toast = state.next_toast_id;
    let error = state.play_map_in_ut99().unwrap_err();
    assert!(error.contains("build/save failed"));
    assert!(state.map_is_dirty);
    assert_eq!(std::fs::read(blocker).unwrap(), b"preserve");
    let errors: Vec<_> = state.toasts.iter().filter(|t| t.id >= first_toast && matches!(t.kind, ToastKind::Error)).collect();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].message, error);
}

#[test]
fn malformed_arguments_fail_before_saving_and_unnamed_playtests_use_unique_files() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(true);
    let mut state = game.state();
    state.ut99_launch_args = "-ini=\"unclosed".into();
    assert!(state.play_map_in_ut99().is_err());
    assert!(!state.default_export_dir.as_ref().unwrap().exists());
    state.ut99_launch_args = format!("-capture=\"{}\"", game.capture().display());
    state.play_map_in_ut99().unwrap();
    game.read_capture();
    let first = state.map_path.clone().unwrap();
    assert!(first.is_file());
    assert_ne!(first.file_name().unwrap(), "EditorTest.unr");
    std::fs::remove_file(game.capture()).unwrap();
    state.new_map();
    state.play_map_in_ut99().unwrap();
    game.read_capture();
    assert_ne!(state.map_path.as_ref().unwrap(), &first);
    assert!(first.is_file());
}

fn ui_frame(ctx: &egui::Context, state: &mut EditorState, wizard: bool, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut input = egui::RawInput::default();
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0)));
    input.events = events;
    input.time = Some(ctx.input(|i| i.time) + 0.2);
    ctx.run(input, |ctx| {
        if wizard { ut99_editor::gui::map_wizard_dialog::show_map_wizard_dialog(ctx, state); }
        else { ut99_editor::gui::play_settings_dialog::render_play_settings_dialog(ctx, state); }
    })
}

fn click_ui_text(ctx: &egui::Context, state: &mut EditorState, wizard: bool, needle: &str) {
    ui_frame(ctx, state, wizard, vec![]);
    let output = ui_frame(ctx, state, wizard, vec![]);
    let pos = output.shapes.iter().find_map(|shape| {
        if let egui::epaint::Shape::Text(text) = &shape.shape {
            if text.galley.text().contains(needle) {
                let rect = egui::Rect::from_min_size(text.pos, text.galley.size()).intersect(shape.clip_rect);
                if rect.is_positive() { return Some(rect.center()); }
            }
        }
        None
    }).unwrap_or_else(|| panic!("Visible command not found: {needle}"));
    for pressed in [true, false] {
        ui_frame(ctx, state, wizard, vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
            pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
        }]);
    }
}

#[test]
fn launch_settings_stays_open_and_reports_invalid_configuration() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    let mut state = game.state();
    state.ut99_executable_path = game.dir.path().join("Missing.exe").display().to_string();
    let target = game.dir.path().join("Untouched.unr");
    state.map_path = Some(target.clone());
    state.map_is_dirty = true;
    state.show_play_settings_dialog = true;
    let ctx = egui::Context::default();
    click_ui_text(&ctx, &mut state, false, "Save & Launch Playtest");
    assert!(state.show_play_settings_dialog);
    assert!(!target.exists() && state.map_is_dirty);
    assert!(matches!(state.toasts.last().unwrap().kind, ToastKind::Error));
    let empty_folder = game.dir.path().join("Empty folder");
    std::fs::create_dir(&empty_folder).unwrap();
    state.ut99_executable_path = empty_folder.display().to_string();
    assert!(state.resolved_playtest_executable().is_err());
}

#[test]
fn wizard_checks_configuration_before_generating_or_saving() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let game = Game::new(false);
    CompilerBridge::set_custom_ut_dir(Some(game.executable.parent().unwrap().parent().unwrap().to_path_buf()));
    let mut state = game.state();
    state.ut99_executable_path = game.dir.path().join("Missing.exe").display().to_string();
    state.show_map_wizard_dialog = true;
    state.map_wizard_step = 4;
    state.wizard_output_dir = Some(game.dir.path().join("Wizard Outputs"));
    state.map_is_dirty = true;
    let before = CompilerBridge::export_t3d_map(&state);
    let ctx = egui::Context::default();
    click_ui_text(&ctx, &mut state, true, "Generate, Save & Playtest");
    assert_eq!(CompilerBridge::export_t3d_map(&state), before);
    assert!(state.map_path.is_none() && state.map_is_dirty);
    assert!(matches!(state.toasts.last().unwrap().kind, ToastKind::Error));
    assert_eq!(std::fs::read_dir(game.executable.parent().unwrap().parent().unwrap().join("Maps")).unwrap().count(), 0);
}
