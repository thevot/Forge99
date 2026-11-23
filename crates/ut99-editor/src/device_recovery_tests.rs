use super::*;
use winit::event_loop::EventLoop;
use winit::platform::windows::EventLoopBuilderExtWindows;

#[test]
#[ignore = "creates a hidden native window and destroys its GPU device; run serially"]
fn destroyed_device_recreates_rendering_and_preserves_document_and_ui_textures() {
    struct Probe { app: EditorApp, completed: bool }
    impl ApplicationHandler<EditorEvent> for Probe {
        fn resumed(&mut self, event_loop: &ActiveEventLoop) {
            let window = Arc::new(event_loop.create_window(Window::default_attributes()
                .with_visible(false).with_inner_size(PhysicalSize::new(1024, 768))).unwrap());
            self.app.window = Some(window.clone());
            self.app.initialize_graphics(window).unwrap();
            self.app.state.world_brushes = vec![ut99_bsp::build_cube("Recovery", ut99_bsp::CsgOperation::Add, Vec3::splat(128.0))].into();
            self.app.state.multi_selected_brushes.insert(0);
            self.app.state.map_is_dirty = true;
            self.app.state.show_map_budget_dialog = true;
            let mut thumbnail = self.app.egui_ctx.load_texture("recovery_test_image",
                egui::ColorImage::new([4, 4], egui::Color32::GREEN), egui::TextureOptions::NEAREST);
            self.app.render_frame();
            thumbnail.set_partial([1, 1], egui::ColorImage::new([2, 2], egui::Color32::RED), egui::TextureOptions::NEAREST);
            self.app.render_frame();
            self.app.device.as_ref().unwrap().poll(wgpu::Maintain::Wait);
            self.app.render_frame();
            if self.app.state.graphics_diagnostics.gpu_timing_supported {
                let timings = self.app.state.graphics_diagnostics.gpu_viewport_timing.expect("real 3D pass timestamp sample");
                eprintln!("3D viewport GPU timings (samples, p50 ms, p95 ms): {timings:?}");
                assert!(timings.1.is_finite() && timings.1 >= 0.0);
            }
            assert!(self.app.egui_renderer.as_ref().unwrap().texture(&thumbnail.id()).is_some());
            assert!(self.app.egui_renderer.as_ref().unwrap().texture(&egui::TextureId::Managed(0)).is_some());
            self.app.state.history.push(crate::history::Action::MoveBrush {
                brush_index: 0, old_location: Vec3::ZERO, new_location: Vec3::X,
            });
            let original_undo = self.app.state.history.next_undo_description().map(str::to_owned);
            let original_brushes = self.app.state.world_brushes.clone();
            let original_selection = self.app.state.multi_selected_brushes.clone();
            let pending_path = std::path::PathBuf::from("device-recovery-preservation-only.t3d");
            self.app.state.pending_save_op = Some(crate::editor_state::SaveOperationKind::SaveT3d(pending_path.clone()));
            let old_loss_sink = self.app.device_loss.clone();
            self.app.device.as_ref().unwrap().destroy();
            self.app.device.as_ref().unwrap().poll(wgpu::Maintain::Wait);
            assert!(old_loss_sink.lock().unwrap().is_some(), "real device destruction must trigger callback");
            let now = Instant::now();
            self.app.check_device_recovery(now);
            assert!(self.app.device.is_none());
            self.app.check_device_recovery(now + std::time::Duration::from_secs(2));
            assert!(self.app.device.is_some());
            assert_eq!(self.app.state.graphics_diagnostics.device_losses, 1);
            assert_eq!(self.app.state.graphics_diagnostics.device_recoveries, 1);
            assert_eq!(self.app.state.world_brushes, original_brushes);
            assert_eq!(self.app.state.multi_selected_brushes, original_selection);
            assert!(self.app.state.map_is_dirty);
            assert!(self.app.state.history.can_undo());
            assert_eq!(self.app.state.history.next_undo_description().map(str::to_owned), original_undo);
            assert!(matches!(&self.app.state.pending_save_op,
                Some(crate::editor_state::SaveOperationKind::SaveT3d(path)) if path == &pending_path));
            assert!(self.app.egui_renderer.as_ref().unwrap().texture(&thumbnail.id()).is_some());
            assert!(self.app.egui_renderer.as_ref().unwrap().texture(&egui::TextureId::Managed(0)).is_some());
            // A callback from an obsolete device must never invalidate the replacement.
            *old_loss_sink.lock().unwrap() = Some("obsolete device callback".into());
            self.app.check_device_recovery(Instant::now());
            assert_eq!(self.app.state.graphics_diagnostics.device_losses, 1);
            self.app.state.pending_save_op = None;
            self.app.render_frame();
            assert!(self.app.state.graphics_diagnostics.gpu_vertex_buffer_bytes > 0);
            assert!(self.app.state.graphics_diagnostics.last_render_stats.solid_vertex_count > 0);
            assert!(self.app.state.graphics_diagnostics.last_render_stats.draw_calls > 0);
            assert!(self.app.state.graphics_diagnostics.last_render_stats.uploaded_solid_bytes > 0);
            self.app.render_frame();
            self.app.device.as_ref().unwrap().poll(wgpu::Maintain::Wait);
            self.app.render_frame();
            assert_eq!(self.app.state.graphics_diagnostics.last_render_stats.uploaded_solid_bytes, 0);
            assert!(self.app.state.graphics_diagnostics.last_render_stats.solid_vertex_count > 0);
            assert!(self.app.state.graphics_diagnostics.last_render_stats.draw_calls > 0);
            if self.app.state.graphics_diagnostics.gpu_timing_supported {
                let timings = self.app.state.graphics_diagnostics.gpu_viewport_timing
                    .expect("post-recovery 3D pass timestamp sample");
                eprintln!("Post-recovery 3D viewport GPU timings (samples, p50 ms, p95 ms): {timings:?}");
                assert!(timings.0 >= 1 && timings.1.is_finite() && timings.1 >= 0.0);
            }
            self.app.device.as_ref().unwrap().poll(wgpu::Maintain::Wait);
            eprintln!("Recovered adapter: {:?}", self.app.state.graphics_diagnostics.adapter);
            self.completed = true;
            event_loop.exit();
        }
        fn window_event(&mut self, _: &ActiveEventLoop, _: WindowId, _: WindowEvent) {}
    }
    let event_loop = EventLoop::<EditorEvent>::with_user_event().with_any_thread(true).build().unwrap();
    let mut probe = Probe { app: EditorApp::default(), completed: false };
    probe.app.install_repaint_callback(event_loop.create_proxy());
    event_loop.run_app(&mut probe).unwrap();
    assert!(probe.completed);
}
