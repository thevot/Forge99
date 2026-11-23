use glam::Vec3;
use ut99_editor::{CompilerBridge, EditorState};
use ut99_editor::editor_state::{PendingDirtyAction, SaveOperationKind, SaveWorkerMessage, LevelAuditFreshness};
use ut99_editor::raycast::GizmoAxis;
mod common;

fn populated() -> EditorState {
    let mut state = EditorState::default();
    state.new_map();
    state.add_mover_from_builder();
    state.movers[0].name = "PreviousDoor".into();
    let mut mesh = state.builder_brush.clone();
    mesh.name = "PreviousMesh".into();
    state.static_meshes.push(mesh);
    state.projectors.push(ut99_bsp::Projector::default());
    state.place_particle_emitter_at_pivot(None);
    state.selected_mover_index = Some(0);
    state.selected_static_mesh_index = Some(0);
    state.selected_projector_index = Some(0);
    state.selected_surfaces.insert((0, 0));
    state.memorized_surfaces.insert((0, 0));
    state.multi_selected_brushes.insert(0);
    state.map = CompilerBridge::export_t3d_map(&state);
    state.run_level_doctor_audit();
    state.cached_map_manifest = Some(state.generate_map_manifest());
    state.set_camera_bookmark(0, Some("Old viewpoint".into()));
    state.background_image_path = Some("old-reference.png".into());
    state.mover_preview_playing = true;
    state.cinematic_settings.is_playing = true;
    state.map_path = Some("Previous.unr".into());
    state
}

#[test]
fn new_map_after_populated_document_matches_a_fresh_export() {
    let mut fresh = EditorState::default();
    fresh.new_map();
    let expected = CompilerBridge::export_t3d_map(&fresh);
    let mut state = populated();
    state.new_map();
    assert_eq!(CompilerBridge::export_t3d_map(&state), expected);
    assert!(state.movers.is_empty() && state.static_meshes.is_empty());
    assert!(state.projectors.is_empty() && state.particle_emitters.is_empty());
    assert!(state.selected_mover_index.is_none() && state.selected_static_mesh_index.is_none());
    assert!(state.selected_projector_index.is_none());
    assert!(state.selected_surfaces.is_empty() && state.memorized_surfaces.is_empty());
    assert!(state.multi_selected_brushes.is_empty());
    assert!(!state.history.can_undo() && !state.history.can_redo());
    assert!(state.map_path.is_none() && !state.map_is_dirty);
    assert!(state.map.actors.iter().all(|actor| !actor.name.starts_with("Previous")));
    assert_eq!(state.level_audit_freshness(), LevelAuditFreshness::NotRun);
    assert!(state.map_audit_issues.is_empty() && state.cached_map_manifest.is_none());
    assert!(state.camera_bookmarks.iter().all(Option::is_none));
    assert!(state.background_image_path.is_none());
    assert!(!state.mover_preview_playing && !state.cinematic_settings.is_playing);
}

fn wav() -> Vec<u8> {
    let mut data = Vec::new();
    data.extend(b"RIFF"); data.extend(40u32.to_le_bytes()); data.extend(b"WAVEfmt ");
    data.extend(16u32.to_le_bytes()); data.extend(1u16.to_le_bytes()); data.extend(1u16.to_le_bytes());
    data.extend(8000u32.to_le_bytes()); data.extend(16000u32.to_le_bytes());
    data.extend(2u16.to_le_bytes()); data.extend(16u16.to_le_bytes());
    data.extend(b"data"); data.extend(4u32.to_le_bytes()); data.extend([0u8,0,1,0]); data
}

#[test]
fn new_map_removes_only_document_assets_and_preserves_shared_resources() {
    let mut state = EditorState::default();
    state.new_map();
    state.default_author = "Preferred Author".into();
    state.grid_step = 32.0;
    state.builder_cube_size = Vec3::splat(333.0);
    let builder_polys = state.builder_brush.polys.len();
    state.import_wav_sound("SharedSound", "SharedScratch", wav()).unwrap();
    state.import_wav_sound("OldEmbedded", "MyLevel", wav()).unwrap();
    state.set_mylevel_screenshot_rgba8(1, 1, &[255, 0, 0, 255]);
    let mut texture = state.texture_manager.find_loaded_texture("MyLevel.Screenshot").unwrap().clone();
    texture.name = "SharedTexture".into();
    state.texture_manager.upsert_texture("SharedLibrary", texture);
    state.texture_manager.pin_package("SharedLibrary");
    let ctx = egui::Context::default();
    assert!(state.texture_manager.get_or_create_egui_texture(&ctx, "MyLevel", "Screenshot").is_some());
    assert!(state.texture_manager.get_or_create_egui_texture(&ctx, "SharedLibrary", "SharedTexture").is_some());
    assert_eq!(state.texture_manager.cached_thumbnail_count(), 2);
    state.active_texture = Some("MyLevel.Screenshot".into());
    state.selected_texture_package = "MyLevel".into();
    state.builder_brush.polys[0].texture = "MyLevel.Screenshot".into();
    state.new_map();
    assert!(state.mylevel_sounds.is_empty());
    assert!(state.texture_manager.get_package("MyLevel").is_none());
    assert!(!state.texture_manager.is_package_pinned("MyLevel"));
    assert!(!state.texture_manager.available_packages().iter().any(|p| p.eq_ignore_ascii_case("MyLevel")));
    assert!(state.active_texture.is_none());
    assert!(!state.sound_packages.iter().any(|p| p.eq_ignore_ascii_case("MyLevel")));
    assert!(state.loaded_sounds.iter().all(|s| !s.package.eq_ignore_ascii_case("MyLevel")));
    assert!(state.builder_brush.polys.iter().all(|p| !p.texture.starts_with("MyLevel.")));
    assert_eq!(state.texture_manager.cached_thumbnail_count(), 1);
    assert!(state.texture_manager.get_package("SharedLibrary").is_some());
    assert!(state.texture_manager.is_package_pinned("SharedLibrary"));
    state.load_sound_package("SharedScratch");
    assert_eq!(state.loaded_sounds[0].name, "SharedSound");
    assert_eq!(state.level_author, "Preferred Author");
    assert_eq!(state.grid_step, 32.0);
    assert_eq!(state.builder_cube_size, Vec3::splat(333.0));
    assert_eq!(state.builder_brush.polys.len(), builder_polys);
    let level = CompilerBridge::build_unr_level_from_state(&state, std::path::Path::new("Fresh.unr")).unwrap();
    assert!(level.embedded_sounds.is_empty() && level.embedded_textures.is_empty());
}

#[test]
fn repeated_new_map_has_stable_t3d_and_native_output() {
    let output = common::TestDir::new();
    let mut state = populated();
    state.new_map();
    let expected = CompilerBridge::export_t3d_map(&state);
    state.new_map();
    assert_eq!(CompilerBridge::export_t3d_map(&state), expected);
    let target = output.path().join("Fresh.unr");
    CompilerBridge::save_unr_native(&state, &target).unwrap();
    let level = ut99_format::UnrLevel::open(&target).unwrap();
    assert!(level.actors.iter().all(|a| !a.name.starts_with("Previous") && a.class_name != "Mover" && a.class_name != "ParticleEmitter"));
    assert_eq!(level.player_starts.len(), 1);
    assert_eq!(level.lights.len(), 1);
}

#[test]
fn pending_or_running_save_keeps_the_old_document_until_completion() {
    let mut state = populated();
    state.pending_save_op = Some(SaveOperationKind::SaveUnrNative("Previous.unr".into()));
    state.new_map();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.map_path.as_deref(), Some(std::path::Path::new("Previous.unr")));
    assert!(state.map_is_dirty);
    assert!(state.pending_save_op.is_some());
    state.pending_save_op = None;
    state.trigger_save_unr_native("Previous.unr");
    state.pending_save_op = None;
    let (_tx, rx) = std::sync::mpsc::channel();
    state.save_worker_rx = Some(rx);
    state.new_map();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.map_path.as_deref(), Some(std::path::Path::new("Previous.unr")));
    assert!(state.save_worker_rx.is_some());
    state.pending_dirty_action = Some(PendingDirtyAction::NewMap);
    state.handle_save_worker_message(SaveWorkerMessage::Success {
        saved_path: "Previous.unr".into(), file_size_bytes: 10, duration: std::time::Duration::ZERO,
    });
    assert!(state.movers.is_empty() && state.map_path.is_none());
    assert!(state.save_feedback.is_none());
}

#[test]
fn failed_save_and_active_transform_do_not_reset_the_document() {
    let mut state = populated();
    state.pending_dirty_action = Some(PendingDirtyAction::NewMap);
    state.handle_save_worker_message(SaveWorkerMessage::Error { message: "injected failure".into() });
    assert_eq!(state.movers.len(), 1);
    assert!(state.map_is_dirty);
    state.active_gizmo_axis = GizmoAxis::X;
    state.new_map();
    assert_eq!(state.movers.len(), 1);
    assert_eq!(state.active_gizmo_axis, GizmoAxis::X);
    state.active_gizmo_axis = GizmoAxis::None;
    state.new_map();
    assert!(state.movers.is_empty());
}

#[test]
fn file_menu_keeps_unsaved_confirmation_and_cancel_preserves_document() {
    fn frame(ctx: &egui::Context, state: &mut EditorState, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0)));
        input.events = events;
        input.time = Some(ctx.input(|i| i.time) + 0.2);
        ctx.run(input, |ctx| ut99_editor::gui::render_ui(ctx, state))
    }
    fn click(ctx: &egui::Context, state: &mut EditorState, label: &str) {
        let output = frame(ctx, state, vec![]);
        let pos = output.shapes.iter().find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                if text.galley.text() == label || (label.starts_with("Discard") && text.galley.text().contains(label)) {
                    return Some(egui::Rect::from_min_size(text.pos, text.galley.size()).intersect(shape.clip_rect).center());
                }
            }
            None
        }).unwrap_or_else(|| panic!("Visible command not found: {label}"));
        for pressed in [true, false] {
            frame(ctx, state, vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {
                pos, button: egui::PointerButton::Primary, pressed, modifiers: egui::Modifiers::NONE,
            }]);
        }
    }
    let ctx = egui::Context::default();
    let mut state = populated();
    frame(&ctx, &mut state, vec![]);
    click(&ctx, &mut state, "File");
    click(&ctx, &mut state, "New Map");
    assert!(state.show_unsaved_exit_modal);
    assert!(matches!(state.pending_dirty_action, Some(PendingDirtyAction::NewMap)));
    assert_eq!(state.movers.len(), 1);
    click(&ctx, &mut state, "Cancel");
    assert!(!state.show_unsaved_exit_modal && state.pending_dirty_action.is_none());
    assert_eq!(state.movers.len(), 1);
    click(&ctx, &mut state, "File");
    click(&ctx, &mut state, "New Map");
    click(&ctx, &mut state, "Discard & New Map");
    assert!(state.movers.is_empty() && state.map_path.is_none());
}
