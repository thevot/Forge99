use crate::editor_state::EditorState;
use crate::gui::render_ui;
use crate::raycast::{GizmoAxis, Ray};
use crate::renderer::Renderer;
use glam::Vec3;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoopProxy};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

#[derive(Debug)]
pub enum EditorEvent {
    RepaintAt(Instant),
}

pub struct EditorApp {
    frame_schedule: crate::frame_schedule::FrameSchedule,
    surface_recovery: crate::surface_recovery::SurfaceRecovery,
    next_maintenance: Instant,
    event_proxy: Option<EventLoopProxy<EditorEvent>>,
    device_loss: Arc<std::sync::Mutex<Option<String>>>,
    device_retry_at: Option<Instant>,
    gui_texture_backup: crate::gui_texture_backup::GuiTextureBackup,
    state: EditorState,
    window: Option<Arc<Window>>,
    surface: Option<wgpu::Surface<'static>>,
    device: Option<wgpu::Device>,
    queue: Option<wgpu::Queue>,
    surface_config: Option<wgpu::SurfaceConfiguration>,
    renderer: Option<Renderer>,
    egui_ctx: egui::Context,
    egui_winit_state: Option<egui_winit::State>,
    egui_renderer: Option<egui_wgpu::Renderer>,
    last_time: Instant,
    keys_down: HashSet<KeyCode>,
    rmb_down: bool,
    rmb_dragged: bool,
    lmb_down: bool,
    mmb_down: bool,
    mouse_pos: (f64, f64),
    last_mouse_pos: Option<(f64, f64)>,
    drag_start_object_loc: Option<Vec3>,
    drag_axis_offset: f32,
    drag_plane_offset: Vec3,
    drag_rot_mat: glam::Mat3,
    scale_drag_s0: Option<f32>,
    transform_snapshot: Option<crate::transform_session::TransformSnapshot>,
    drag_start_angle: f32,
    lmb_drag_start: Option<(f64, f64)>,
    lmb_drag_viewport: Option<crate::camera::ViewportType>,
    direct_drag_eligible: bool,
    shift_drag_tracking: bool,
    clicked_empty_space: bool,
    last_pointer_over_egui: bool,
    pub dirty_redraw: bool,
}

impl Default for EditorApp {
    fn default() -> Self {
        let mut state = EditorState::default();
        state.load_preferences();
        Self {
            frame_schedule: crate::frame_schedule::FrameSchedule::new(Instant::now()),
            surface_recovery: Default::default(),
            next_maintenance: Instant::now(),
            event_proxy: None,
            device_loss: Default::default(),
            device_retry_at: None,
            gui_texture_backup: Default::default(),
            state,
            window: None,
            surface: None,
            device: None,
            queue: None,
            surface_config: None,
            renderer: None,
            egui_ctx: egui::Context::default(),
            egui_winit_state: None,
            egui_renderer: None,
            last_time: Instant::now(),
            keys_down: HashSet::new(),
            rmb_down: false,
            rmb_dragged: false,
            lmb_down: false,
            mmb_down: false,
            mouse_pos: (0.0, 0.0),
            last_mouse_pos: None,
            drag_start_object_loc: None,
            drag_axis_offset: 0.0,
            drag_plane_offset: Vec3::ZERO,
            drag_rot_mat: glam::Mat3::IDENTITY,
            scale_drag_s0: None,
            transform_snapshot: None,
            drag_start_angle: 0.0,
            lmb_drag_start: None,
            lmb_drag_viewport: None,
            direct_drag_eligible: false,
            shift_drag_tracking: false,
            clicked_empty_space: false,
            last_pointer_over_egui: false,
            dirty_redraw: true,
        }
    }
}


impl ApplicationHandler<EditorEvent> for EditorApp {
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, event: EditorEvent) {
        match event {
            EditorEvent::RepaintAt(deadline) => self.frame_schedule.request(deadline),
        }
    }

    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window_attrs = Window::default_attributes()
            .with_title(format!("Forge99 — 64-bit Level Editor for Unreal Tournament 99 v{}", env!("CARGO_PKG_VERSION")))
            .with_inner_size(PhysicalSize::new(1440, 900))
            .with_maximized(self.state.start_maximized);

        let window = match event_loop.create_window(window_attrs) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                tracing::error!("Failed to create editor window: {error}");
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());

        if let Err(message) = self.initialize_graphics(window) {
            tracing::error!("{message}");
            rfd::MessageDialog::new().set_title("Forge99 graphics initialization")
                .set_description(&message).set_level(rfd::MessageLevel::Error).show();
            event_loop.exit();
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        // Forward event to egui first, but keep inert CursorMoved events from scheduling redraws.
        let is_cursor_moved = matches!(event, WindowEvent::CursorMoved { .. });
        if let (Some(w), Some(ref mut egui_winit)) = (&self.window, &mut self.egui_winit_state) {
            let res = egui_winit.on_window_event(w, &event);
            if res.consumed {
                if res.repaint {
                    self.dirty_redraw = true;
                }
                return;
            }
            if res.repaint && !is_cursor_moved {
                self.dirty_redraw = true;
            }
        }

        match event {
            WindowEvent::CloseRequested => {
                if self.state.map_is_dirty && !self.state.confirm_exit_without_saving {
                    self.state.pending_dirty_action = Some(crate::editor_state::PendingDirtyAction::ExitApp);
                    self.state.show_unsaved_exit_modal = true;
                    self.dirty_redraw = true;
                    if let Some(w) = &self.window {
                        w.request_redraw();
                    }
                } else {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(new_size) => {
                if let (Some(surface), Some(device), Some(config), Some(renderer)) = (
                    &self.surface,
                    &self.device,
                    &mut self.surface_config,
                    &mut self.renderer,
                ) {
                    if new_size.width > 0 && new_size.height > 0 {
                        config.width = new_size.width;
                        config.height = new_size.height;
                        surface.configure(device, config);
                        renderer.resize(device, new_size.width, new_size.height);
                        self.dirty_redraw = true;
                        if let Some(w) = &self.window {
                            w.request_redraw();
                        }
                    }
                }
                self.sync_monitor_refresh_rate();
            }
            WindowEvent::Moved(_) => {
                self.sync_monitor_refresh_rate();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                self.dirty_redraw = true;
                if let PhysicalKey::Code(key) = event.physical_key {
                    if event.state == ElementState::Pressed {
                        let shift = self.keys_down.contains(&KeyCode::ShiftLeft)
                            || self.keys_down.contains(&KeyCode::ShiftRight);
                        let alt = self.keys_down.contains(&KeyCode::AltLeft)
                            || self.keys_down.contains(&KeyCode::AltRight);
                        let ctrl = self.keys_down.contains(&KeyCode::ControlLeft)
                            || self.keys_down.contains(&KeyCode::ControlRight);

                        // Fullscreen shortcuts (F11 or Alt+Enter) toggle regardless of egui focus
                        if key == KeyCode::F11 && !shift {
                            self.toggle_fullscreen();
                            return;
                        }
                        if (key == KeyCode::Enter || key == KeyCode::NumpadEnter) && alt && !ctrl {
                            self.toggle_fullscreen();
                            return;
                        }

                        if self.egui_ctx.wants_keyboard_input() {
                            return;
                        }
                        self.keys_down.insert(key);

                        let ctrl = self.keys_down.contains(&KeyCode::ControlLeft)
                            || self.keys_down.contains(&KeyCode::ControlRight);
                        let shift = self.keys_down.contains(&KeyCode::ShiftLeft)
                            || self.keys_down.contains(&KeyCode::ShiftRight);
                        let alt = self.keys_down.contains(&KeyCode::AltLeft)
                            || self.keys_down.contains(&KeyCode::AltRight);

                        match key {
                            KeyCode::KeyX | KeyCode::KeyY | KeyCode::KeyZ
                                if !ctrl && !shift && !alt && self.state.active_gizmo_axis != GizmoAxis::None => {
                                let ax = match key { KeyCode::KeyX => GizmoAxis::X, KeyCode::KeyY => GizmoAxis::Y, _ => GizmoAxis::Z };
                                self.constrain_drag_axis(ax);
                            }
                            KeyCode::KeyW if !ctrl => {
                                self.state.gizmo_mode = crate::raycast::GizmoMode::Translate;
                                self.state.log("Gizmo Mode: Translate (W)");
                            }
                            KeyCode::KeyE if !ctrl => {
                                self.state.gizmo_mode = crate::raycast::GizmoMode::Rotate;
                                self.state.log("Gizmo Mode: Rotate (E)");
                            }
                            KeyCode::KeyR if !ctrl => {
                                self.state.gizmo_mode = crate::raycast::GizmoMode::Scale;
                                self.state.log("Gizmo Mode: Scale (R)");
                            }
                            KeyCode::KeyS if !ctrl && !alt && !shift => {
                                self.state.surface_magnet_mode = !self.state.surface_magnet_mode;
                                self.state.log(format!("Surface Magnet Mode: {}", if self.state.surface_magnet_mode { "ON (Auto-drop to surfaces)" } else { "OFF" }));
                            }
                            KeyCode::KeyV if !ctrl && !alt && !shift => {
                                self.state.vertex_magnet_mode = !self.state.vertex_magnet_mode;
                                self.state.log(format!("Vertex Magnet Snapping: {}", if self.state.vertex_magnet_mode { "ON" } else { "OFF" }));
                            }
                            KeyCode::KeyL if ctrl && shift && !alt => {
                                self.state.build_lighting();
                            }
                            KeyCode::KeyL if ctrl => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    if let Some(ref path) = self.state.map_path.clone() {
                                        self.state.trigger_save_unr(path);
                                    } else {
                                        let default_dir = self.state.default_maps_directory();
                                        let mut dialog = rfd::FileDialog::new().add_filter("Unreal Map (*.unr)", &["unr"]);
                                        if default_dir.exists() {
                                            dialog = dialog.set_directory(&default_dir);
                                        }
                                        if let Some(path) = dialog.save_file() {
                                            self.state.trigger_save_unr(&path);
                                        }
                                    }
                                } else {
                                    self.state.toggle_gizmo_space();
                                }
                            }
                            KeyCode::Comma if ctrl && !shift && !alt => {
                                self.state.show_settings_dialog = true;
                            }
                            KeyCode::Space if shift && !ctrl && !alt => {
                                self.state.toggle_selection_lock();
                            }
                            KeyCode::Space if !ctrl => {
                                self.state.viewport_layout = match self.state.viewport_layout {
                                    crate::camera::ViewportLayout::Single => crate::camera::ViewportLayout::FourViewports,
                                    crate::camera::ViewportLayout::FourViewports => crate::camera::ViewportLayout::Single,
                                };
                                self.state.log(format!("Viewport Layout: {:?}", self.state.viewport_layout));
                            }
                            KeyCode::F1 if shift => {
                                self.state.open_help_tab(crate::editor_state::HelpTab::Shortcuts);
                            }
                            KeyCode::Slash if shift => {
                                self.state.open_help_tab(crate::editor_state::HelpTab::Shortcuts);
                            }
                            KeyCode::F1 => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::Modern {
                                    self.state.open_help_tab(crate::editor_state::HelpTab::QuickStart);
                                } else {
                                    self.state.active_viewport_type = crate::camera::ViewportType::Perspective;
                                    self.state.log("Viewport: 3D Perspective (F1)");
                                }
                            }
                            KeyCode::F2 => {
                                self.state.active_viewport_type = crate::camera::ViewportType::Top;
                                self.state.log("Viewport: Top XY (F2)");
                            }
                            KeyCode::F3 => {
                                self.state.active_viewport_type = crate::camera::ViewportType::Front;
                                self.state.log("Viewport: Front XZ (F3)");
                            }
                            KeyCode::F4 if shift => {
                                self.state.active_viewport_type = crate::camera::ViewportType::Side;
                                self.state.log("Viewport: Side YZ (Shift+F4)");
                            }
                            KeyCode::KeyZ if ctrl && !shift => {
                                self.state.undo();
                            }
                            KeyCode::KeyY if ctrl => {
                                self.state.redo();
                            }
                            KeyCode::KeyZ if ctrl && shift => {
                                self.state.redo();
                            }
                            KeyCode::KeyC if ctrl && !shift && !alt => {
                                self.state.copy_selected_to_clipboard();
                                if !self.state.clipboard.t3d_text.is_empty() {
                                    self.egui_ctx.copy_text(self.state.clipboard.t3d_text.clone());
                                }
                            }
                            KeyCode::KeyX if ctrl && !shift && !alt => {
                                self.state.cut_selected_to_clipboard();
                                if !self.state.clipboard.t3d_text.is_empty() {
                                    self.egui_ctx.copy_text(self.state.clipboard.t3d_text.clone());
                                }
                            }
                            KeyCode::KeyV if ctrl && !shift && !alt => {
                                self.state.paste_from_clipboard();
                            }
                            KeyCode::KeyA if ctrl && shift && !alt => {
                                self.state.show_action_wizards_dialog = !self.state.show_action_wizards_dialog;
                            }
                            KeyCode::KeyA if ctrl && !shift && !alt => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    self.state.csg_add();
                                } else {
                                    self.state.select_all();
                                }
                            }
                            KeyCode::KeyG if ctrl && shift && !alt => {
                                self.state.build_geometry();
                            }
                            KeyCode::KeyI if ctrl && !shift && !alt => {
                                self.state.invert_selection();
                            }
                            KeyCode::Escape => {
                                if !self.cancel_transform() {
                                    self.state.handle_escape();
                                }
                            }
                            KeyCode::KeyN if ctrl && shift && !alt => {
                                self.state.show_map_wizard_dialog = !self.state.show_map_wizard_dialog;
                                if self.state.show_map_wizard_dialog && self.state.available_official_maps.is_empty() {
                                    self.state.scan_official_maps_for_wizard();
                                }
                            }
                            KeyCode::KeyN if ctrl && !shift && !alt => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    self.state.csg_intersect();
                                } else if self.state.map_is_dirty {
                                    self.state.pending_dirty_action = Some(crate::editor_state::PendingDirtyAction::NewMap);
                                    self.state.show_unsaved_exit_modal = true;
                                } else {
                                    self.state.new_map();
                                }
                            }
                            KeyCode::KeyO if ctrl && !shift && !alt => {
                                self.state.prompt_or_open_unr_dialog();
                            }
                            KeyCode::KeyP if ctrl && shift && !alt => {
                                self.state.build_ai_paths();
                            }
                            KeyCode::KeyP if ctrl && !alt => {
                                self.state.show_command_palette = !self.state.show_command_palette;
                            }
                            KeyCode::Backquote if ctrl => {
                                self.state.show_bottom_console = !self.state.show_bottom_console;
                            }
                            KeyCode::KeyC if ctrl && shift && !alt => {
                                self.state.show_bottom_console = !self.state.show_bottom_console;
                            }
                            KeyCode::KeyS if ctrl && !shift && !alt => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    self.state.csg_subtract();
                                } else if let Some(ref path) = self.state.map_path.clone() {
                                    self.state.trigger_save_unr(path);
                                } else {
                                    let default_dir = self.state.default_maps_directory();
                                    let mut dialog = rfd::FileDialog::new().add_filter("Unreal Map (*.unr)", &["unr"]);
                                    if default_dir.exists() {
                                        dialog = dialog.set_directory(&default_dir);
                                    }
                                    if let Some(path) = dialog.save_file() {
                                        self.state.trigger_save_unr(&path);
                                    }
                                }
                            }
                            KeyCode::KeyS if ctrl && shift && !alt => {
                                let default_dir = self.state.default_maps_directory();
                                let mut dialog = rfd::FileDialog::new().add_filter("Unreal Map (*.unr)", &["unr"]);
                                if default_dir.exists() {
                                    dialog = dialog.set_directory(&default_dir);
                                }
                                if let Some(path) = dialog.save_file() {
                                    self.state.trigger_save_unr(&path);
                                }
                            }
                            KeyCode::KeyW if ctrl && shift && !alt => {
                                self.state.close_all_floating_dialogs();
                            }
                            KeyCode::F4 => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2
                                    || self.state.selected_actor_index.is_some()
                                {
                                    self.state.show_actor_properties = !self.state.show_actor_properties;
                                } else {
                                    self.state.show_texture_browser = !self.state.show_texture_browser;
                                }
                            }
                            KeyCode::F5 => {
                                self.state.show_surface_properties = !self.state.show_surface_properties;
                            }
                            KeyCode::F6 => {
                                if shift {
                                    self.state.render_lux_heatmap = !self.state.render_lux_heatmap;
                                    let status = if self.state.render_lux_heatmap { "ENABLED" } else { "DISABLED" };
                                    self.state.log(format!("Luxmeter Lighting Heatmap (Shift+F6) {}", status));
                                } else {
                                    self.state.show_level_properties_dialog = !self.state.show_level_properties_dialog;
                                }
                            }
                            KeyCode::F7 => {
                                if self.state.show_uc_viewer_dialog {
                                    let _ = self.state.compile_active_uc_script();
                                } else {
                                    self.state.show_map_budget_dialog = !self.state.show_map_budget_dialog;
                                }
                            }
                            KeyCode::F8 if ctrl && !shift && !alt => {
                                self.state.build_all_complete();
                            }
                            KeyCode::F8 => {
                                let _ = self.state.play_map_in_ut99();
                            }
                            KeyCode::F9 => {
                                self.state.show_nav_coverage = !self.state.show_nav_coverage;
                                let status = if self.state.show_nav_coverage { "ENABLED" } else { "DISABLED" };
                                self.state.log(format!("Bot Navigation Coverage Heatmap (F9) {}", status));
                            }
                            KeyCode::F10 => {
                                self.state.distance_fog_enabled = !self.state.distance_fog_enabled;
                                let status = if self.state.distance_fog_enabled { "ENABLED" } else { "DISABLED" };
                                self.state.log(format!("Real-Time Distance Fog (F10) {}", status));
                            }
                            KeyCode::F11 if shift => {
                                self.state.show_acoustics_dialog = !self.state.show_acoustics_dialog;
                            }
                            KeyCode::F11 => {
                                self.toggle_fullscreen();
                            }
                            KeyCode::F12 => {
                                self.state.show_gameplay_balance_dialog = !self.state.show_gameplay_balance_dialog;
                            }
                            KeyCode::KeyB if ctrl && shift => {
                                self.state.show_build_options_dialog = !self.state.show_build_options_dialog;
                            }
                            KeyCode::KeyB if ctrl => {
                                self.state.show_camera_bookmarks_dialog = !self.state.show_camera_bookmarks_dialog;
                            }
                            KeyCode::KeyR if ctrl && shift && !alt => {
                                self.state.rotate_selected_brush_texture(45.0);
                            }
                            KeyCode::KeyR if ctrl && !shift && !alt => {
                                self.state.show_replace_textures_dialog = !self.state.show_replace_textures_dialog;
                            }
                            KeyCode::End if ctrl && !shift && !alt => {
                                self.state.snap_selected_brush_pivot_to_nearest_vertex(None);
                            }
                            KeyCode::End if shift && !ctrl && !alt => {
                                if self.state.has_multi_selection() {
                                    self.state.snap_multi_selected_to_floor_aligned();
                                } else {
                                    self.state.snap_selected_to_floor_aligned();
                                }
                            }
                            KeyCode::End if !shift && !ctrl && !alt => {
                                if self.state.has_multi_selection() {
                                    self.state.snap_multi_selected_to_floor();
                                } else {
                                    self.state.snap_selected_to_floor();
                                }
                            }
                            KeyCode::Digit1 if ctrl => { self.state.set_camera_bookmark(0, None); }
                            KeyCode::Digit2 if ctrl => { self.state.set_camera_bookmark(1, None); }
                            KeyCode::Digit3 if ctrl => { self.state.set_camera_bookmark(2, None); }
                            KeyCode::Digit4 if ctrl => { self.state.set_camera_bookmark(3, None); }
                            KeyCode::Digit5 if ctrl => { self.state.set_camera_bookmark(4, None); }
                            KeyCode::Digit6 if ctrl => { self.state.set_camera_bookmark(5, None); }
                            KeyCode::Digit7 if ctrl => { self.state.set_camera_bookmark(6, None); }
                            KeyCode::Digit8 if ctrl => { self.state.set_camera_bookmark(7, None); }
                            KeyCode::Digit9 if ctrl => { self.state.set_camera_bookmark(8, None); }
                            KeyCode::Digit0 if ctrl => { self.state.set_camera_bookmark(9, None); }

                            // Canonical UnrealEd Shading Modes (Alt+1..Alt+7)
                            KeyCode::Digit1 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::Wireframe;
                                self.state.log("Viewport Shading: Wireframe (Alt+1)");
                            }
                            KeyCode::Digit2 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::ZonePortal;
                                self.state.log("Viewport Shading: Zone/Portal View (Alt+2)");
                            }
                            KeyCode::Digit3 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::Unlit;
                                self.state.log("Viewport Shading: Unlit (Alt+3)");
                            }
                            KeyCode::Digit4 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::Lit;
                                self.state.log("Viewport Shading: Lit (Alt+4)");
                            }
                            KeyCode::Digit5 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::DepthComplexity;
                                self.state.log("Viewport Shading: Depth Complexity Heatmap (Alt+5)");
                            }
                            KeyCode::Digit6 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::TextureUsage;
                                self.state.log("Viewport Shading: Texture Usage Color-Coding (Alt+6)");
                            }
                            KeyCode::Digit7 if alt => {
                                self.state.viewport_shading = crate::camera::ViewportShading::BspCuts;
                                self.state.log("Viewport Shading: BSP Cuts Diagnostic (Alt+7)");
                            }
                            KeyCode::Digit8 if alt => { self.state.goto_camera_bookmark(7); }
                            KeyCode::Digit9 if alt => { self.state.goto_camera_bookmark(8); }
                            KeyCode::Digit0 if alt => { self.state.goto_camera_bookmark(9); }

                            // Camera Bookmarks Goto (Shift+1..0)
                            KeyCode::Digit1 if shift => { self.state.goto_camera_bookmark(0); }
                            KeyCode::Digit2 if shift => { self.state.goto_camera_bookmark(1); }
                            KeyCode::Digit3 if shift => { self.state.goto_camera_bookmark(2); }
                            KeyCode::Digit4 if shift => { self.state.goto_camera_bookmark(3); }
                            KeyCode::Digit5 if shift => { self.state.goto_camera_bookmark(4); }
                            KeyCode::Digit6 if shift => { self.state.goto_camera_bookmark(5); }
                            KeyCode::Digit7 if shift => { self.state.goto_camera_bookmark(6); }
                            KeyCode::Digit8 if shift => { self.state.goto_camera_bookmark(7); }
                            KeyCode::Digit9 if shift => { self.state.goto_camera_bookmark(8); }
                            KeyCode::Digit0 if shift => { self.state.goto_camera_bookmark(9); }

                            KeyCode::Digit1 if !ctrl && !alt => { self.state.set_camera_speed_preset(1); }
                            KeyCode::Digit2 if !ctrl && !alt => { self.state.set_camera_speed_preset(2); }
                            KeyCode::Digit3 if !ctrl && !alt => { self.state.set_camera_speed_preset(3); }
                            KeyCode::Digit4 if !ctrl && !alt => { self.state.set_camera_speed_preset(4); }
                            KeyCode::Digit5 if !ctrl && !alt => { self.state.set_camera_speed_preset(5); }
                            KeyCode::Digit6 if !ctrl && !alt => { self.state.set_camera_speed_preset(6); }
                            KeyCode::Digit7 if !ctrl && !alt => { self.state.set_camera_speed_preset(7); }
                            KeyCode::Digit8 if !ctrl && !alt => { self.state.set_camera_speed_preset(8); }
                            KeyCode::KeyS if ctrl && alt => {
                                self.state.snap_selected_vertices_to_grid();
                            }
                            KeyCode::KeyW if ctrl && alt => {
                                self.state.weld_selected_vertices_to_nearest(16.0);
                            }
                            KeyCode::KeyE if ctrl && alt => {
                                self.state.split_selected_edge();
                            }
                            KeyCode::KeyM if ctrl && alt => {
                                self.state.collapse_selected_edge();
                            }
                            KeyCode::KeyG if ctrl && alt => {
                                self.state.snap_selected_to_grid();
                            }
                            KeyCode::KeyI if ctrl && alt => {
                                self.state.invert_selected_brush();
                            }
                            KeyCode::BracketLeft => {
                                self.state.step_grid_down();
                            }
                            KeyCode::BracketRight => {
                                self.state.step_grid_up();
                            }
                            KeyCode::KeyF if ctrl && !shift => {
                                self.state.show_find_actors_dialog = !self.state.show_find_actors_dialog;
                            }
                            KeyCode::KeyD if ctrl => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    self.state.csg_deintersect();
                                } else if self.state.has_multi_selection() {
                                    self.state.duplicate_selected_batch(glam::Vec3::new(self.state.grid_step, self.state.grid_step, 0.0));
                                } else {
                                    self.state.duplicate_selected();
                                }
                            }
                            KeyCode::KeyW if ctrl && !shift => {
                                if self.state.has_multi_selection() {
                                    self.state.duplicate_selected_batch(glam::Vec3::new(self.state.grid_step, self.state.grid_step, 0.0));
                                } else {
                                    self.state.duplicate_selected();
                                }
                            }
                            KeyCode::Enter | KeyCode::NumpadEnter => {
                                if self.state.editor_mode == crate::editor_state::EditorMode::FreehandPolygon && self.state.freehand_points.len() >= 3 {
                                    let depth = self.state.freehand_extrude_depth;
                                    let _ = self.state.extrude_freehand_polygon_to_builder(depth);
                                } else if self.state.selected_actor_index.is_some() {
                                    self.state.show_actor_properties = true;
                                } else if self.state.selected_brush_index.is_some() {
                                    self.state.show_surface_properties = true;
                                }
                            }
                            KeyCode::KeyF if ctrl && shift => {
                                self.state.fit_selected_brush_texture(1.0, 1.0);
                            }
                            KeyCode::ArrowUp if shift => {
                                self.state.pan_selected_brush_texture(0.0, self.state.grid_step);
                            }
                            KeyCode::ArrowDown if shift => {
                                self.state.pan_selected_brush_texture(0.0, -self.state.grid_step);
                            }
                            KeyCode::ArrowLeft if shift => {
                                self.state.pan_selected_brush_texture(-self.state.grid_step, 0.0);
                            }
                            KeyCode::ArrowRight if shift => {
                                self.state.pan_selected_brush_texture(self.state.grid_step, 0.0);
                            }
                            KeyCode::KeyR if shift && !ctrl && !alt => {
                                if !self.state.memorized_surfaces.is_empty() {
                                    self.state.recall_surface_selection();
                                } else {
                                    self.state.rotate_selected_brush_texture(45.0);
                                }
                            }
                            KeyCode::PageUp if shift => {
                                self.state.scale_selected_brush_texture(1.25, 1.25);
                            }
                            KeyCode::PageDown if shift => {
                                self.state.scale_selected_brush_texture(0.8, 0.8);
                            }
                            KeyCode::KeyB if shift && !ctrl && !alt => {
                                self.state.select_surfaces_on_selected_brushes();
                            }
                            KeyCode::KeyC if shift && !ctrl && !alt => {
                                self.state.select_surfaces_coplanar_adjacent();
                            }
                            KeyCode::KeyT if shift && !ctrl && !alt => {
                                self.state.select_surfaces_matching_texture();
                            }
                            KeyCode::KeyG if shift && !ctrl && !alt => {
                                self.state.select_surfaces_matching_groups();
                            }
                            KeyCode::KeyI if shift && !ctrl && !alt => {
                                self.state.select_surfaces_matching_items();
                            }
                            KeyCode::KeyI if ctrl && shift && !alt => {
                                self.state.invert_surface_selection();
                            }
                            KeyCode::KeyJ if shift && !ctrl && !alt => {
                                self.state.select_surfaces_adjacent_all();
                            }
                            KeyCode::KeyW if shift && !ctrl && !alt => {
                                self.state.select_surfaces_adjacent_walls();
                            }
                            KeyCode::KeyQ if shift && !ctrl && !alt => {
                                self.state.select_surfaces_adjacent_slants();
                            }
                            KeyCode::KeyF if shift && !ctrl && !alt => {
                                if !self.state.selected_surfaces.is_empty() {
                                    self.state.select_surfaces_adjacent_floors();
                                } else {
                                    self.state.fit_selected_brush_texture(1.0, 1.0);
                                }
                            }
                            KeyCode::KeyS if shift && !ctrl && !alt => {
                                self.state.select_all_surfaces();
                            }
                            KeyCode::KeyN if shift && !ctrl && !alt => {
                                self.state.deselect_all_surfaces();
                            }
                            KeyCode::KeyZ if shift && !ctrl && !alt => {
                                if self.state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2 {
                                    self.state.select_surfaces_matching_zone();
                                } else {
                                    self.state.deselect_all_surfaces();
                                }
                            }
                            KeyCode::KeyM if shift && !ctrl && !alt => {
                                self.state.memorize_surface_selection();
                            }
                            KeyCode::KeyO if shift && !ctrl && !alt => {
                                self.state.or_surface_selection();
                            }
                            KeyCode::KeyU if shift && !ctrl && !alt => {
                                self.state.and_surface_selection();
                            }
                            KeyCode::KeyX if shift && !ctrl && !alt => {
                                self.state.xor_surface_selection();
                            }
                            KeyCode::KeyM if !shift && !ctrl && !alt => {
                                self.state.show_minimap = !self.state.show_minimap;
                                let status = if self.state.show_minimap { "ENABLED" } else { "DISABLED" };
                                self.state.log(format!("Map Navigation Radar (M) {}", status));
                            }
                            KeyCode::KeyF if !shift => {
                                self.state.focus_camera_on_selection();
                            }
                            KeyCode::Delete => {
                                self.state.delete_selected_batch();
                            }
                            _ => {}
                        }
                    } else {
                        self.keys_down.remove(&key);
                    }
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                self.dirty_redraw = true;
                let alt = self.keys_down.contains(&KeyCode::AltLeft)
                    || self.keys_down.contains(&KeyCode::AltRight);
                let shift = self.keys_down.contains(&KeyCode::ShiftLeft)
                    || self.keys_down.contains(&KeyCode::ShiftRight);
                let ctrl = self.keys_down.contains(&KeyCode::ControlLeft)
                    || self.keys_down.contains(&KeyCode::ControlRight);

                if button == MouseButton::Right {
                    if state == ElementState::Pressed {
                        self.rmb_down = true;
                        self.rmb_dragged = false;
                        self.last_mouse_pos = Some(self.mouse_pos);
                        if self.cancel_transform() {
                            self.rmb_dragged = true; // suppress context menu on release
                        }
                    } else {
                        self.rmb_down = false;
                        self.last_mouse_pos = None;
                        // Single click without dragging camera
                        if !self.rmb_dragged {
                            if let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) {
                                if alt {
                                    if self.state.keymap_profile
                                        == crate::editor_state::KeymapProfile::ClassicUnrealEd2
                                    {
                                        // Canonical UnrealEd 2.0 Alt+RMB: Eyedropper / Pick texture
                                        self.state.pick_texture_at_ray(ray);
                                    } else {
                                        // Modern profile Alt+RMB: Apply active texture
                                        self.state.apply_active_texture_at_ray(ray);
                                    }
                                } else {
                                    // Plain RMB click on scene: select object and open 3D Context Menu (preserving compatible multi-selection)
                                    self.state.select_at_ray_for_context_menu(ray);
                                    self.state.show_viewport_context_menu = true;
                                    self.state.context_menu_screen_pos = egui::pos2(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
                                    let world_pos = ray.origin + ray.direction * 350.0;
                                    self.state.context_menu_world_pos = Some(world_pos);
                                }
                            }
                        }
                    }
                } else if button == MouseButton::Left {
                    if state == ElementState::Pressed {
                        self.lmb_down = true;
                        let (mx, my) = self.mouse_pos;
                        self.last_mouse_pos = Some((mx, my));
                        self.lmb_drag_start = Some((mx, my));
                        self.lmb_drag_viewport = self.state.viewport_type_at(mx as f32, my as f32);

                        if let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) {
                            if shift && alt {
                                // Shift+Alt+LMB: Stamp Decal at clicked surface
                                self.state.stamp_decal_at_ray(ray);
                            } else if self.state.decal_paint_mode {
                                // Decal Paint Mode active: Stamp Decal
                                self.state.stamp_decal_at_ray(ray);
                            } else if ctrl && alt {
                                // Canonical UnrealEd 2.0 Ctrl+Alt+LMB: Apply active texture AND sampled UV alignment
                                self.state.apply_active_texture_and_uv_at_ray(ray);
                            } else if alt {
                                if self.state.keymap_profile
                                    == crate::editor_state::KeymapProfile::ClassicUnrealEd2
                                {
                                    // Canonical UnrealEd 2.0 Alt+LMB: Apply active texture
                                    self.state.apply_active_texture_at_ray(ray);
                                } else {
                                    // Modern profile Alt+LMB: Pick / Eyedropper texture
                                    self.state.pick_texture_at_ray(ray);
                                }
                            } else {
                                // 1. Check gizmo hit first
                                let gizmo_center = crate::transform_session::gizmo_pivot(&self.state);

                                let mut gizmo_hit = GizmoAxis::None;
                                let rot_mat = self.state.get_selected_rotation_matrix();
                                if let Some(center) = gizmo_center {
                                    let vh = self.state.viewport_rect[3];
                                    let (axis_scale, hit_rad) = self.state.get_gizmo_scale_and_hit_radius(center, self.state.active_viewport_type, vh);
                                    let rot_scale = axis_scale * 0.875;
                                    gizmo_hit = match self.state.gizmo_mode {
                                        crate::raycast::GizmoMode::Translate => ray.hit_test_gizmo_oriented(center, axis_scale, hit_rad, rot_mat),
                                        crate::raycast::GizmoMode::Rotate => ray.hit_test_rotation_gizmo_oriented(center, rot_scale, hit_rad * 0.75, rot_mat),
                                        crate::raycast::GizmoMode::Scale => ray.hit_test_gizmo_oriented(center, axis_scale, hit_rad, rot_mat),
                                    };
                                }

                                gizmo_hit = crate::transform_session::constrain_hit_for_view(gizmo_hit, self.state.gizmo_mode, rot_mat, self.view_dir());
                                if gizmo_hit != GizmoAxis::None {
                                    if alt {
                                        // Alt + Drag Gizmo instantly duplicates selection
                                        if self.state.has_multi_selection() {
                                            self.state.duplicate_selected_batch(glam::Vec3::ZERO);
                                        } else {
                                            self.state.duplicate_selected();
                                        }
                                        self.state.log("Duplicated selection via Alt+Drag");
                                    }
                                    self.state.active_gizmo_axis = gizmo_hit;
                                    self.transform_snapshot = crate::transform_session::TransformSnapshot::capture(&self.state);
                                    self.state.active_gizmo_delta = Vec3::ZERO;
                                    self.state.active_gizmo_angle_deg = 0.0;
                                    self.state.gizmo_drag_initial_brushes.clear();
                                    self.state.gizmo_drag_initial_actors.clear();
                                    if self.state.has_multi_selection() {
                                        for &b_idx in &self.state.multi_selected_brushes {
                                            if let Some(b) = self.state.world_brushes.get(b_idx) {
                                                self.state.gizmo_drag_initial_brushes.push((b_idx, b.location));
                                            }
                                        }
                                        if let Some(ref unr) = self.state.loaded_unr {
                                            for &a_idx in &self.state.multi_selected_actors {
                                                if let Some(a) = unr.actors.get(a_idx) {
                                                    self.state.gizmo_drag_initial_actors.push((a_idx, a.location));
                                                }
                                            }
                                        }
                                    }
                                    self.drag_start_object_loc = gizmo_center;
                                    self.drag_rot_mat = rot_mat;
                                    self.scale_drag_s0 = None;
                                    if let Some(center) = gizmo_center {
                                        if self.state.gizmo_mode == crate::raycast::GizmoMode::Rotate {
                                            self.drag_start_angle = ray.get_rotation_angle_oriented(center, gizmo_hit, rot_mat).unwrap_or(0.0);
                                        } else {
                                            match gizmo_hit {
                                                GizmoAxis::PlaneXY | GizmoAxis::PlaneXZ | GizmoAxis::PlaneYZ | GizmoAxis::ScreenPlane => {
                                                    let local_normal = match gizmo_hit {
                                                        GizmoAxis::PlaneXY => Vec3::Z,
                                                        GizmoAxis::PlaneXZ => Vec3::Y,
                                                        GizmoAxis::PlaneYZ => Vec3::X,
                                                        GizmoAxis::ScreenPlane => match self.state.active_viewport_type {
                                                            crate::camera::ViewportType::Top => Vec3::Z,
                                                            crate::camera::ViewportType::Front => Vec3::Y,
                                                            crate::camera::ViewportType::Side => Vec3::X,
                                                            crate::camera::ViewportType::Perspective => self.state.camera.forward(),
                                                        },
                                                        _ => Vec3::Z,
                                                    };
                                                    let world_normal = if gizmo_hit == GizmoAxis::ScreenPlane { local_normal } else { rot_mat * local_normal };
                                                    if let Some(hit) = ray.project_onto_plane(center, world_normal) {
                                                        self.drag_plane_offset = hit - center;
                                                    } else {
                                                        self.drag_plane_offset = Vec3::ZERO;
                                                    }
                                                }
                                                _ => {
                                                    let local_axis_dir = match gizmo_hit {
                                                        GizmoAxis::X => Vec3::X,
                                                        GizmoAxis::Y => Vec3::Y,
                                                        GizmoAxis::Z => Vec3::Z,
                                                        _ => Vec3::ZERO,
                                                    };
                                                    let world_axis_dir = rot_mat * local_axis_dir;
                                                    self.drag_axis_offset = ray.project_onto_axis(center, world_axis_dir).unwrap_or(0.0);
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    if self.state.editor_mode == crate::editor_state::EditorMode::VertexEdit {
                                        let unique_verts = self.state.get_active_brush_unique_vertices();
                                        let shift = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
                                        let mut hit_idx = None;
                                        let mut min_dist = 20.0;
                                        for (i, &v) in unique_verts.iter().enumerate() {
                                            let to_v = v - ray.origin;
                                            let proj = to_v.dot(ray.direction);
                                            if proj > 0.0 {
                                                let closest_on_ray = ray.origin + ray.direction * proj;
                                                let d = closest_on_ray.distance(v);
                                                if d < min_dist {
                                                    min_dist = d;
                                                    hit_idx = Some(i);
                                                }
                                            }
                                        }

                                        if let Some(idx) = hit_idx {
                                            if shift {
                                                if let Some(pos) = self.state.selected_vertex_indices.iter().position(|&x| x == idx) {
                                                    self.state.selected_vertex_indices.remove(pos);
                                                } else {
                                                    self.state.selected_vertex_indices.push(idx);
                                                }
                                            } else {
                                                self.state.selected_vertex_indices = vec![idx];
                                            }
                                        } else if !shift {
                                            self.state.selected_vertex_indices.clear();
                                        }
                                    } else if self.state.editor_mode == crate::editor_state::EditorMode::BrushClip {
                                        let ref_z = self.state.builder_brush.location.z;
                                        if ray.direction.z.abs() > 0.001 {
                                            let t = (ref_z - ray.origin.z) / ray.direction.z;
                                            if t > 0.0 && t < 10000.0 {
                                                let mut pt = ray.origin + ray.direction * t;
                                                if self.state.snap_to_grid && self.state.grid_step > 0.0 {
                                                    pt.x = (pt.x / self.state.grid_step).round() * self.state.grid_step;
                                                    pt.y = (pt.y / self.state.grid_step).round() * self.state.grid_step;
                                                }
                                                self.state.add_clip_point(pt);
                                            }
                                        }
                                    } else if self.state.editor_mode == crate::editor_state::EditorMode::FreehandPolygon {
                                        let ref_loc = self.state.builder_brush.location;
                                        let plane_normal = match self.state.active_viewport_type {
                                            crate::camera::ViewportType::Front => glam::Vec3::Y,
                                            crate::camera::ViewportType::Side => glam::Vec3::X,
                                            _ => glam::Vec3::Z,
                                        };
                                        if let Some(pt) = ray.project_onto_plane(ref_loc, plane_normal) {
                                            self.state.add_freehand_vertex(pt);
                                        }
                                    } else {
                                        // Hit test scene objects for selection
                                        if self.state.selection_locked {
                                            self.direct_drag_eligible = false;
                                            self.clicked_empty_space = false;
                                        } else {
                                            let shift = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
                                            let ctrl = self.keys_down.contains(&KeyCode::ControlLeft) || self.keys_down.contains(&KeyCode::ControlRight);
                                            let hit_something = self.state.select_at_ray_with_modifier(ray, shift || ctrl);
                                            if hit_something {
                                                self.direct_drag_eligible = true;
                                                self.shift_drag_tracking = shift;
                                                self.drag_start_object_loc = crate::transform_session::primary_location(&self.state);
                                                self.clicked_empty_space = false;
                                            } else {
                                                self.direct_drag_eligible = false;
                                                self.clicked_empty_space = !shift && !ctrl;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        self.lmb_down = false;
                        let drag_start = self.lmb_drag_start.take();
                        self.lmb_drag_viewport = None;
                        self.direct_drag_eligible = false;
                        self.shift_drag_tracking = false;
                        self.state.cursor_delta_text = None;

                        // Deselect on click on empty space (without dragging)
                        if self.clicked_empty_space && !self.state.selection_locked {
                            if let Some((sx, sy)) = drag_start {
                                let dist_sq = (self.mouse_pos.0 - sx).powi(2) + (self.mouse_pos.1 - sy).powi(2);
                                let thresh = self.state.drag_threshold_px as f64;
                                if dist_sq <= thresh * thresh && self.state.active_marquee.is_none() && self.state.active_gizmo_axis == GizmoAxis::None {
                                    self.state.deselect_all();
                                }
                            }
                        }
                        self.clicked_empty_space = false;

                        // Complete Viewport 3D Drag & Drop on mouse release
                        if self.state.active_drag_drop.is_some() {
                            if let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) {
                                let screen_pos = (self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
                                self.state.complete_drag_drop(screen_pos, ray);
                            } else {
                                self.state.active_drag_drop = None;
                                self.state.drag_drop_preview_pos = None;
                                self.state.drag_drop_preview_normal = None;
                            }
                            self.dirty_redraw = true;
                        }

                        // Complete 2D Marquee Box Selection on mouse release
                        if let Some(marquee) = self.state.active_marquee.take() {
                            let shift = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
                            let ctrl = self.keys_down.contains(&KeyCode::ControlLeft) || self.keys_down.contains(&KeyCode::ControlRight);
                            let (cnt_b, cnt_a) = self.state.select_within_marquee(marquee, shift || ctrl);
                            self.state.log(format!("Marquee Selection: {} brush(es), {} actor(s) selected", cnt_b, cnt_a));
                        }

                        if self.state.active_gizmo_axis != GizmoAxis::None {
                            // Phase 1: single transactional commit (one undo entry per gesture)
                            if let Some(snapshot) = self.transform_snapshot.take() {
                                let n = snapshot.commit(&mut self.state);
                                if n > 0 {
                                    self.state.log(format!("Transformed via 3D Gizmo ({} change(s))", n));
                                }
                            }
                            if let Some(m_idx) = self.state.selected_mover_index {
                                if let Some(m) = self.state.movers.get(m_idx) {
                                    self.state.log(format!("Mover '{}': Key {} set to loc ({:.0}, {:.0}, {:.0}) rot ({:.1}, {:.1}, {:.1})", m.name, m.current_key, m.brush.location.x, m.brush.location.y, m.brush.location.z, m.brush.rotation.x, m.brush.rotation.y, m.brush.rotation.z));
                                }
                            }
                        }
                        self.state.active_gizmo_axis = GizmoAxis::None;
                        self.state.active_gizmo_delta = Vec3::ZERO;
                        self.state.active_gizmo_angle_deg = 0.0;
                        self.state.gizmo_drag_initial_brushes.clear();
                        self.state.gizmo_drag_initial_actors.clear();
                        self.drag_start_object_loc = None;
                        self.state.history.end_interaction();
                        self.last_mouse_pos = None;
                    }
                } else if button == MouseButton::Middle {
                    if state == ElementState::Pressed {
                        self.mmb_down = true;
                        self.last_mouse_pos = Some(self.mouse_pos);
                        self.state.sync_hovered_viewport(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32);
                    } else {
                        self.mmb_down = false;
                        self.last_mouse_pos = None;
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.handle_cursor_moved((position.x, position.y)) {
                    self.dirty_redraw = true;
                } else if let Some(ref mut egui_winit) = self.egui_winit_state {
                    coalesce_trailing_pointer_moved(&mut egui_winit.egui_input_mut().events);
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let scroll_amount = match delta {
                    MouseScrollDelta::LineDelta(_x, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y as f32) / 20.0,
                };
                if scroll_amount.abs() > 0.001 {
                    self.dirty_redraw = true;
                    let ctrl = self.keys_down.contains(&KeyCode::ControlLeft) || self.keys_down.contains(&KeyCode::ControlRight);
                    let shift = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
                    let alt = self.keys_down.contains(&KeyCode::AltLeft) || self.keys_down.contains(&KeyCode::AltRight);

                    if self.lmb_down && (self.state.active_gizmo_axis != GizmoAxis::None || self.direct_drag_eligible) {
                        if ctrl && alt {
                            let steps = [1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0];
                            let cur_step = self.state.grid_step;
                            let mut idx = steps.iter().position(|&s| (s - cur_step).abs() < 0.1).unwrap_or(4);
                            if scroll_amount > 0.0 && idx + 1 < steps.len() {
                                idx += 1;
                            } else if scroll_amount < 0.0 && idx > 0 {
                                idx -= 1;
                            }
                            self.state.grid_step = steps[idx];
                            self.state.log(format!("Grid Step: {:.0} UU", steps[idx]));
                        } else if shift {
                            let step = if self.state.snap_to_grid && self.state.grid_step > 0.0 { self.state.grid_step } else { 16.0 };
                            let delta_z = if scroll_amount > 0.0 { step } else { -step };
                            if let Some(sel_idx) = self.state.selected_actor_index {
                                if let Some(ref mut unr) = self.state.loaded_unr {
                                    if let Some(a) = unr.actors.get_mut(sel_idx) {
                                        a.location.z += delta_z;
                                    }
                                    unr.sync_actor_views();
                                }
                            } else if let Some(b_idx) = self.state.selected_brush_index {
                                if let Some(b) = self.state.world_brushes.get_mut(b_idx) {
                                    b.location.z += delta_z;
                                }
                            }
                            self.state.map_is_dirty = true;
                        } else {
                            let yaw_step_deg = if ctrl { 45.0 } else if alt { 5.0 } else { 15.0 };
                            let yaw_units = (yaw_step_deg * 65536.0 / 360.0) as i32;
                            let rot_delta = if scroll_amount > 0.0 { yaw_units } else { -yaw_units };
                            if let Some(sel_idx) = self.state.selected_actor_index {
                                if let Some(ref mut unr) = self.state.loaded_unr {
                                    if let Some(a) = unr.actors.get_mut(sel_idx) {
                                        a.rotation.1 = (a.rotation.1 + rot_delta) & 0xFFFF;
                                    }
                                    unr.sync_actor_views();
                                }
                            } else if let Some(b_idx) = self.state.selected_brush_index {
                                if let Some(b) = self.state.world_brushes.get_mut(b_idx) {
                                    let deg_delta = if scroll_amount > 0.0 { yaw_step_deg } else { -yaw_step_deg };
                                    b.rotation.z = (b.rotation.z + deg_delta) % 360.0;
                                }
                            }
                            self.state.map_is_dirty = true;
                        }
                    } else {
                        // Zoom the viewport quadrant directly under cursor without requiring prior left-click
                        let target_vtype = self.state.sync_hovered_viewport(self.mouse_pos.0 as f32, self.mouse_pos.1 as f32)
                            .unwrap_or(self.state.active_viewport_type);
                        if target_vtype == crate::camera::ViewportType::Perspective {
                            self.state.camera.position += self.state.camera.forward() * (scroll_amount * 64.0);
                        } else {
                            let factor = if scroll_amount > 0.0 {
                                0.85f32.powf(scroll_amount)
                            } else {
                                1.15f32.powf(-scroll_amount)
                            };
                            let cur_zoom = self.state.camera.get_ortho_zoom(target_vtype);
                            self.state.camera.set_ortho_zoom(target_vtype, cur_zoom * factor);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                self.dirty_redraw = false;
                self.frame_schedule.frame_started(Instant::now());
                self.render_frame();
                if self.state.pending_toggle_fullscreen {
                    self.state.pending_toggle_fullscreen = false;
                    self.toggle_fullscreen();
                }
                if self.state.confirm_exit_without_saving {
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(false) => {
                self.cancel_transform();
                self.keys_down.clear();
                self.rmb_down = false;
                self.lmb_down = false;
                self.mmb_down = false;
                self.last_mouse_pos = None;
                self.dirty_redraw = true;
            }
            WindowEvent::Focused(true)
            | WindowEvent::ScaleFactorChanged { .. } => {
                self.sync_monitor_refresh_rate();
                self.dirty_redraw = true;
            }
            WindowEvent::Occluded(false)
            | WindowEvent::ThemeChanged(_) => {
                self.dirty_redraw = true;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.state.confirm_exit_without_saving {
            event_loop.exit();
            return;
        }
        if self.state.pending_graphics_reinit {
            self.state.pending_graphics_reinit = false;
            if let Some(window) = self.window.clone() {
                match self.initialize_graphics(window) {
                    Ok(()) => {
                        let name = self
                            .state
                            .graphics_diagnostics
                            .adapter
                            .as_ref()
                            .map(|a| a.name.clone())
                            .unwrap_or_else(|| "adapter".to_string());
                        self.state.log(format!("Reinitialized graphics on '{name}'"));
                    }
                    Err(err) => {
                        self.state.log(format!("Graphics reinitialization failed: {err}"));
                        self.state.toast_error(format!("Graphics reinit failed: {err}"));
                    }
                }
                self.dirty_redraw = true;
            }
        }
        let now = Instant::now();
        if now >= self.next_maintenance {
            if let Some(device) = &self.device {
                // Deliver asynchronous device callbacks without blocking the UI.
                device.poll(wgpu::Maintain::Poll);
            }
        }
        self.check_device_recovery(now);
        let elapsed = now.saturating_duration_since(self.last_time);
        let effective_dt = elapsed.min(std::time::Duration::from_millis(100));
        self.last_time = now;

        // Camera flight controls if RMB is held
        if self.rmb_down {
            let mut fwd = 0.0f32;
            let mut rgt = 0.0f32;
            let mut up = 0.0f32;

            if self.keys_down.contains(&KeyCode::KeyW) { fwd += 1.0; }
            if self.keys_down.contains(&KeyCode::KeyS) { fwd -= 1.0; }
            if self.keys_down.contains(&KeyCode::KeyD) { rgt += 1.0; }
            if self.keys_down.contains(&KeyCode::KeyA) { rgt -= 1.0; }
            if self.keys_down.contains(&KeyCode::KeyE) { up += 1.0; }
            if self.keys_down.contains(&KeyCode::KeyQ) { up -= 1.0; }

            let fast = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
            self.state.camera.update_movement(fwd, rgt, up, effective_dt, fast);
        }

        let wake_cb = self.make_repaint_wake();
        let autosave_tick = self.state.tick_async_auto_save(elapsed.as_secs_f32(), wake_cb);
        if autosave_tick.dispatched {
            self.state.graphics_diagnostics.cpu_autosave_ms = autosave_tick.snapshot_ms;
        }
        if autosave_tick.completed {
            self.state.graphics_diagnostics.bg_autosave_serialize_ms = autosave_tick.bg_serialize_ms;
            self.state.graphics_diagnostics.bg_autosave_io_ms = autosave_tick.bg_io_ms;
            self.dirty_redraw = true;
        }

        let active = self.rmb_down || self.lmb_down || self.mmb_down
            || !self.keys_down.is_empty() || self.state.mover_preview_playing;
        if self.dirty_redraw || active {
            self.frame_schedule.request(now);
        }
        // Save stages are advanced by the UI, including receipt of worker results.
        let saving = self.state.pending_save_op.is_some() || self.state.save_worker_rx.is_some();
        if now >= self.next_maintenance {
            if (self.surface_recovery.is_recovering() || self.device_retry_at.is_some()) && saving {
                // The normal UI pass cannot advance save stages while acquisition fails.
                self.state.process_pending_save_operation(&self.egui_ctx);
            }
            if saving { self.frame_schedule.request(now); }
            self.next_maintenance = now + std::time::Duration::from_millis(if saving { 50 } else { 1000 });
        } else if saving {
            self.next_maintenance = self.next_maintenance.min(now + std::time::Duration::from_millis(50));
        }
        let mut wake_at = self.next_maintenance;
        if let Some(deadline) = self.frame_schedule.deadline() {
            if deadline <= now {
                self.dirty_redraw = false;
                if let Some(window) = &self.window { window.request_redraw(); }
            } else {
                wake_at = wake_at.min(deadline);
            }
        }
        event_loop.set_control_flow(ControlFlow::WaitUntil(wake_at));
    }
}

impl EditorApp {
    fn initialize_graphics(&mut self, window: Arc<Window>) -> Result<(), String> {
        let options = crate::graphics::GraphicsSelectionOptions {
            preferred_backend: self.state.graphics_backend,
            preferred_adapter_name: self.state.preferred_adapter_name.clone(),
            hardware_profile: self.state.hardware_profile,
        };
        let init = pollster::block_on(
            crate::graphics::initialize_window_graphics(window.clone(), &options),
        )?;
        if self.state.graphics_diagnostics.available_adapters.is_empty() {
            self.state.graphics_diagnostics.available_adapters =
                crate::graphics::enumerate_available_adapters();
        }
        let surface = init.surface;
        let adapter = init.adapter;
        let device = init.device;
        let queue = init.queue;
        let diagnostics = init.diagnostics;
        let budgets = init.budgets;

        self.state.graphics_diagnostics.adapter = diagnostics.adapter;
        self.state.graphics_diagnostics.adapter_max_texture_array_layers = diagnostics.adapter_max_texture_array_layers;
        self.state.graphics_diagnostics.adapter_max_texture_dimension_2d = diagnostics.adapter_max_texture_dimension_2d;
        self.state.graphics_diagnostics.negotiated_max_texture_dimension_2d = diagnostics.negotiated_max_texture_dimension_2d;
        self.state.graphics_diagnostics.negotiated_max_texture_array_layers = diagnostics.negotiated_max_texture_array_layers;
        self.state.graphics_diagnostics.gpu_timing_supported = diagnostics.gpu_timing_supported;
        self.state.graphics_diagnostics.gpu_viewport_timing = None;
        self.state.graphics_diagnostics.failed_attempts = diagnostics.failed_attempts;
        self.state.apply_hardware_budgets(budgets);

        let size = window.inner_size();
        let width = size.width.max(1);
        let height = size.height.max(1);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        self.state.graphics_diagnostics.supported_present_modes = surface_caps.present_modes.clone();
        let present_mode = crate::graphics::resolve_present_mode(
            self.state.present_mode,
            &surface_caps.present_modes,
        );
        let desired_latency = self.state.max_frame_latency.clamp(1, 2);
        self.state.graphics_diagnostics.active_present_mode = Some(present_mode);
        self.state.graphics_diagnostics.active_max_frame_latency = desired_latency;
        self.state.graphics_diagnostics.active_render_scale =
            self.state.viewport_render_scale.clamp(0.5, 1.0);
        self.state.graphics_diagnostics.active_render_resolution = (width, height);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: desired_latency,
        };
        surface.configure(&device, &surface_config);

        let mut renderer = Renderer::new_with_budgets(&device, surface_format, width, height, budgets);
        renderer.enable_gpu_timing(&device, &queue);
        renderer.enable_async_csg(self.make_repaint_wake());
        self.state.graphics_diagnostics.diffuse_texture_layer_cap = renderer.max_diffuse_texture_layers();
        self.state.graphics_diagnostics.diffuse_texture_array_bytes = renderer.diffuse_texture_array_bytes();
        self.state.graphics_diagnostics.lightmap_atlas_dim = renderer.lightmap_atlas.dim();
        self.state.graphics_diagnostics.lightmap_atlas_bytes = renderer.lightmap_atlas.vram_bytes();

        // Initialize egui
        let egui_winit_state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );

        let mut egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1, false);
        self.gui_texture_backup.restore(&mut egui_renderer, &device, &queue);
        let device_loss = Arc::new(std::sync::Mutex::new(None));
        let loss_sink = device_loss.clone();
        let proxy = self.event_proxy.clone();
        device.set_device_lost_callback(move |reason, message| {
            *loss_sink.lock().unwrap_or_else(|poison| poison.into_inner()) = Some(format!("{reason:?}: {message}"));
            if let Some(proxy) = &proxy {
                let _ = proxy.send_event(EditorEvent::RepaintAt(Instant::now()));
            }
        });

        // Apply configured theme variant
        crate::gui::theme::apply_theme_variant(&self.egui_ctx, self.state.theme);

        // Drop old presentation resources before replacing their device.
        self.egui_renderer = None;
        self.renderer = None;
        self.surface = None;
        self.queue = None;
        self.device = None;
        self.device_loss = device_loss;
        self.surface = Some(surface);
        self.device = Some(device);
        self.queue = Some(queue);
        self.surface_config = Some(surface_config);
        self.renderer = Some(renderer);
        self.egui_winit_state = Some(egui_winit_state);
        self.egui_renderer = Some(egui_renderer);
        self.last_time = Instant::now();
        self.surface_recovery = Default::default();
        self.frame_schedule.resume();
        self.device_retry_at = None;
        self.state.graphics_diagnostics.cpu_vertex_storage_bytes = 0;
        self.state.graphics_diagnostics.cpu_grid_cache_bytes = 0;
        self.state.graphics_diagnostics.gpu_vertex_buffer_bytes = 0;
        self.sync_monitor_refresh_rate();
        window.request_redraw();
        Ok(())
    }

    fn sync_monitor_refresh_rate(&mut self) {
        let mhz = self
            .window
            .as_ref()
            .and_then(|w| w.current_monitor())
            .and_then(|m| m.refresh_rate_millihertz());
        self.frame_schedule.set_refresh_rate_millihertz(mhz);
        self.state.graphics_diagnostics.monitor_refresh_hz = self.frame_schedule.refresh_rate_hz();
    }

    fn check_device_recovery(&mut self, now: Instant) {
        let loss = self.device_loss.lock().unwrap_or_else(|poison| poison.into_inner()).take();
        if let Some(message) = loss {
            tracing::warn!("Graphics device lost: {message}");
            self.state.graphics_diagnostics.device_losses = self.state.graphics_diagnostics.device_losses.saturating_add(1);
            self.state.graphics_diagnostics.last_device_error = Some(message);
            let retry = now + std::time::Duration::from_secs(2);
            self.device_retry_at = Some(retry);
            self.frame_schedule.defer_until(retry);
            self.frame_schedule.request(retry);
            // Never submit new rendering work against a device known to be lost.
            self.egui_renderer = None;
            self.renderer = None;
            self.surface = None;
            self.queue = None;
            self.device = None;
        }
        if self.device_retry_at.is_some_and(|deadline| deadline <= now) {
            if let Some(window) = self.window.clone() {
                match self.initialize_graphics(window) {
                    Ok(()) => {
                        tracing::info!("Graphics device recreated; document preserved");
                        self.state.graphics_diagnostics.device_recoveries = self.state.graphics_diagnostics.device_recoveries.saturating_add(1);
                        self.dirty_redraw = true;
                    }
                    Err(message) => {
                        if self.state.graphics_diagnostics.last_device_error.as_ref() != Some(&message) {
                            tracing::warn!("Graphics device recovery failed: {message}");
                        }
                        self.state.graphics_diagnostics.last_device_error = Some(message);
                        let retry = Instant::now() + std::time::Duration::from_secs(5);
                        self.device_retry_at = Some(retry);
                        self.frame_schedule.defer_until(retry);
                        self.frame_schedule.request(retry);
                    }
                }
            }
        }
    }

    fn make_repaint_wake(&self) -> Option<Arc<dyn Fn() + Send + Sync>> {
        let proxy = self.event_proxy.clone()?;
        Some(Arc::new(move || {
            let _ = proxy.send_event(EditorEvent::RepaintAt(Instant::now()));
        }))
    }

    /// Wake the event loop for delayed UI work and repaint requests from other threads.
    pub fn install_repaint_callback(&mut self, proxy: EventLoopProxy<EditorEvent>) {
        self.event_proxy = Some(proxy.clone());
        let wake = self.make_repaint_wake();
        if let Some(ref mut renderer) = self.renderer {
            renderer.enable_async_csg(wake);
        }
        self.egui_ctx.set_request_repaint_callback(move |info| {
            if let Some(deadline) = Instant::now().checked_add(info.delay) {
                let _ = proxy.send_event(EditorEvent::RepaintAt(deadline));
            }
        });
    }

    /// Toggles between borderless fullscreen and restored/maximized window.
    pub fn toggle_fullscreen(&mut self) {
        if let Some(w) = &self.window {
            let is_fs = w.fullscreen().is_some();
            if is_fs {
                w.set_fullscreen(None);
                self.state.is_fullscreen = false;
                self.state.log("Exited Fullscreen mode (Maximized / Windowed)");
            } else {
                w.set_fullscreen(Some(Fullscreen::Borderless(None)));
                self.state.is_fullscreen = true;
                self.state.log("Entered Borderless Fullscreen mode (F11 / Alt+Enter to exit)");
            }
            self.dirty_redraw = true;
            w.request_redraw();
        }
    }

    /// Direction the active viewport looks along (used to detect gizmo handles that are degenerate in this view).
    fn view_dir(&self) -> Vec3 {
        match self.state.active_viewport_type {
            crate::camera::ViewportType::Top => Vec3::Z,
            crate::camera::ViewportType::Front => Vec3::Y,
            crate::camera::ViewportType::Side => Vec3::X,
            crate::camera::ViewportType::Perspective => self.state.camera.forward(),
        }
    }

    /// While translating: X / Y / Z constrains the drag to that axis, pressing it again returns to free (screen-plane) movement.
    /// Offsets are rebased so the object does not jump along the constrained axis.
    fn constrain_drag_axis(&mut self, axis: GizmoAxis) {
        if self.state.gizmo_mode != crate::raycast::GizmoMode::Translate
            || self.state.editor_mode == crate::editor_state::EditorMode::VertexEdit
        {
            return;
        }
        let Some(start_loc) = self.drag_start_object_loc else { return };
        let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) else { return };
        let new_axis = if self.state.active_gizmo_axis == axis { GizmoAxis::ScreenPlane } else { axis };
        let rot_mat = self.drag_rot_mat;
        let view = self.view_dir();
        if crate::transform_session::constrain_hit_for_view(new_axis, crate::raycast::GizmoMode::Translate, rot_mat, view) != new_axis {
            self.state.log("Axis is perpendicular to the view: switch viewport to constrain along it");
            return;
        }
        let delta = self.state.active_gizmo_delta;
        match new_axis {
            GizmoAxis::ScreenPlane => {
                let Some(hit) = ray.project_onto_plane(start_loc, view) else { return };
                self.drag_plane_offset = hit - start_loc - delta;
            }
            _ => {
                let local = match new_axis { GizmoAxis::X => Vec3::X, GizmoAxis::Y => Vec3::Y, _ => Vec3::Z };
                let dir = rot_mat * local;
                let Some(s) = ray.project_onto_axis(start_loc, dir) else { return };
                self.drag_axis_offset = s - delta.dot(dir);
            }
        }
        self.state.active_gizmo_axis = new_axis;
        self.state.log(format!("Drag constraint: {:?}", new_axis));
    }

    /// Aborts an in-progress gizmo / direct drag and restores every affected object to its exact
    /// original state. Returns `true` if a drag was actually cancelled.
    fn cancel_transform(&mut self) -> bool {
        if self.state.active_gizmo_axis == GizmoAxis::None || self.transform_snapshot.is_none() {
            return false;
        }
        if let Some(snapshot) = self.transform_snapshot.take() {
            snapshot.cancel(&mut self.state);
        }
        self.state.active_gizmo_axis = GizmoAxis::None;
        self.state.active_gizmo_delta = Vec3::ZERO;
        self.state.active_gizmo_angle_deg = 0.0;
        self.state.cursor_delta_text = None;
        self.drag_start_object_loc = None;
        self.direct_drag_eligible = false;
        self.shift_drag_tracking = false;
        self.clicked_empty_space = false;
        self.lmb_drag_start = None;
        self.lmb_drag_viewport = None;
        self.lmb_down = false;
        self.last_mouse_pos = None;
        self.state.history.end_interaction();
        self.state.log("Transform cancelled: original state restored");
        self.dirty_redraw = true;
        true
    }
    fn handle_cursor_moved(&mut self, pos: (f64, f64)) -> bool {
        let (old_x, old_y) = self.mouse_pos;
        self.mouse_pos = pos;
        let dx = (pos.0 - old_x) as f32;
        let dy = (pos.1 - old_y) as f32;

        let over_egui_now = pointer_hits_egui_ui(&self.egui_ctx, self.state.viewport_rect, pos);
        let was_over_egui = self.last_pointer_over_egui;
        self.last_pointer_over_egui = over_egui_now;
        let mut needs_redraw = self.egui_ctx.is_using_pointer() || over_egui_now || was_over_egui;

        // Synchronize active viewport type to hovered quadrant in 2x2 multi-viewport mode
        let prev_active_vp = self.state.active_viewport_type;
        self.state.sync_hovered_viewport(pos.0 as f32, pos.1 as f32);
        if self.state.active_viewport_type != prev_active_vp {
            needs_redraw = true;
        }

        // Active 3D Drag & Drop Raycast Preview Update
        if self.state.active_drag_drop.is_some() {
            if let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) {
                if let Some((hit_pos, hit_norm)) = self.state.raycast_world(ray) {
                    self.state.drag_drop_preview_pos = Some(hit_pos);
                    self.state.drag_drop_preview_normal = Some(hit_norm);
                } else if let Some(hit) = ray.project_onto_plane(glam::Vec3::ZERO, glam::Vec3::Z) {
                    self.state.drag_drop_preview_pos = Some(hit);
                    self.state.drag_drop_preview_normal = Some(glam::Vec3::Z);
                } else {
                    self.state.drag_drop_preview_pos = None;
                    self.state.drag_drop_preview_normal = None;
                }
                needs_redraw = true;
            }
        }

        // Gizmo Hover Pre-Highlighting when idle
        if !self.lmb_down && !self.rmb_down && !self.mmb_down {
            let prev_hover = self.state.hovered_gizmo_axis;
            let mut new_hover = GizmoAxis::None;

            if let Some(ray) = self.get_ray_at_cursor(self.mouse_pos) {
                let gizmo_center = crate::transform_session::gizmo_pivot(&self.state);

                if let Some(center) = gizmo_center {
                    let vh = self.state.viewport_rect[3];
                    let (axis_scale, hit_rad) = self.state.get_gizmo_scale_and_hit_radius(center, self.state.active_viewport_type, vh);
                    let rot_scale = axis_scale * 0.875;
                    let rot_mat = self.state.get_selected_rotation_matrix();
                    new_hover = match self.state.gizmo_mode {
                        crate::raycast::GizmoMode::Translate => ray.hit_test_gizmo_oriented(center, axis_scale, hit_rad, rot_mat),
                        crate::raycast::GizmoMode::Rotate => ray.hit_test_rotation_gizmo_oriented(center, rot_scale, hit_rad * 0.75, rot_mat),
                        crate::raycast::GizmoMode::Scale => ray.hit_test_gizmo_oriented(center, axis_scale, hit_rad, rot_mat),
                    };
                }
            }

            new_hover = crate::transform_session::constrain_hit_for_view(new_hover, self.state.gizmo_mode, self.state.get_selected_rotation_matrix(), self.view_dir());
            if new_hover != prev_hover {
                self.state.hovered_gizmo_axis = new_hover;
                needs_redraw = true;
            }
        }

        if self.state.active_viewport_type != prev_active_vp
            || self.lmb_down
            || self.rmb_down
            || self.mmb_down
            || self.state.active_gizmo_axis != GizmoAxis::None
            || self.state.cursor_delta_text.is_some()
        {
            needs_redraw = true;
        }

        if self.rmb_down && (dx.abs() > 1.5 || dy.abs() > 1.5) {
            self.rmb_dragged = true;
        }

        let ctrl = self.keys_down.contains(&KeyCode::ControlLeft) || self.keys_down.contains(&KeyCode::ControlRight);
        let shift = self.keys_down.contains(&KeyCode::ShiftLeft) || self.keys_down.contains(&KeyCode::ShiftRight);
        let alt = self.keys_down.contains(&KeyCode::AltLeft) || self.keys_down.contains(&KeyCode::AltRight);
        if ctrl && shift && self.lmb_down {
            if let Some((last_x, _last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                self.state.rotate_selected_brush_texture(dx * 0.5);
            }
            self.last_mouse_pos = Some(pos);
        } else if shift && self.rmb_down {
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                let dy = (pos.1 - last_y) as f32;
                let su = 1.0 + (dx * 0.005);
                let sv = 1.0 + (dy * 0.005);
                self.state.scale_selected_brush_texture(su, sv);
            }
            self.last_mouse_pos = Some(pos);
        } else if shift && !self.state.selected_surfaces.is_empty() && self.lmb_down && self.state.active_gizmo_axis == GizmoAxis::None {
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                let dy = (pos.1 - last_y) as f32;
                self.state.pan_selected_brush_texture(dx * 0.5, dy * 0.5);
            }
            self.last_mouse_pos = Some(pos);
        } else if self.lmb_down && self.rmb_down && self.state.active_gizmo_axis == GizmoAxis::None {
            // Classic UnrealEd 2.0 LMB + RMB drag: pan camera vertically along world Z and strafe horizontally
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                let dy = (pos.1 - last_y) as f32;
                let speed_mult = if shift { 3.0 } else { 1.0 };
                let right = self.state.camera.right();
                self.state.camera.position += right * (dx * 1.2 * speed_mult);
                self.state.camera.position.z -= dy * 1.2 * speed_mult;
            }
            self.last_mouse_pos = Some(pos);
        } else if self.lmb_down && self.state.active_gizmo_axis == GizmoAxis::None {
            let thresh = self.state.drag_threshold_px.max(2.0) as f64;
            let thresh_sq = thresh * thresh;
            if self.direct_drag_eligible {
                if let Some((sx, sy)) = self.lmb_drag_start {
                    let dist_sq = (pos.0 - sx).powi(2) + (pos.1 - sy).powi(2);
                    if dist_sq > thresh_sq {
                        self.state.active_gizmo_axis = GizmoAxis::ScreenPlane;
                        self.drag_rot_mat = self.state.get_selected_rotation_matrix();
                        self.transform_snapshot = crate::transform_session::TransformSnapshot::capture(&self.state);
                        self.state.active_gizmo_delta = Vec3::ZERO;
                        if let (Some(ray), Some(start_loc)) = (self.get_ray_at_cursor(pos), self.drag_start_object_loc) {
                            let plane_normal = match self.state.active_viewport_type {
                                crate::camera::ViewportType::Top => Vec3::Z,
                                crate::camera::ViewportType::Front => Vec3::Y,
                                crate::camera::ViewportType::Side => Vec3::X,
                                crate::camera::ViewportType::Perspective => self.state.camera.forward(),
                            };
                            if let Some(hit) = ray.project_onto_plane(start_loc, plane_normal) {
                                self.drag_plane_offset = hit - start_loc;
                            } else {
                                self.drag_plane_offset = Vec3::ZERO;
                            }
                        }
                    }
                }
            } else {
                // Update Rubber-Band Marquee Selection in 2D and 3D Frustum
                if let Some((sx, sy)) = self.lmb_drag_start {
                    let dist_sq = (pos.0 - sx).powi(2) + (pos.1 - sy).powi(2);
                    if dist_sq > thresh_sq {
                        if let Some(vp_type) = self.lmb_drag_viewport {
                            self.state.active_marquee = Some(crate::editor_state::MarqueeBox {
                                start_screen: (sx as f32, sy as f32),
                                current_screen: (pos.0 as f32, pos.1 as f32),
                                viewport_type: vp_type,
                            });
                        }
                    }
                }
            }
            self.last_mouse_pos = Some(pos);
        } else if self.mmb_down {
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                let dy = (pos.1 - last_y) as f32;
                let speed_mult = if shift { 3.0 } else { 1.0 };
                let target_vtype = self.state.viewport_type_at(pos.0 as f32, pos.1 as f32)
                    .unwrap_or(self.state.active_viewport_type);
                if target_vtype == crate::camera::ViewportType::Perspective {
                    let sensitivity = 0.5 * speed_mult;
                    self.state.camera.pan_perspective(-dx * sensitivity, dy * sensitivity);
                } else {
                    let vh = self.state.viewport_rect[3].max(100.0);
                    let zoom = self.state.camera.get_ortho_zoom(target_vtype);
                    let units_per_pixel = (zoom * 2.0) / vh;
                    let world_dx = -dx * units_per_pixel * speed_mult;
                    let world_dy = dy * units_per_pixel * speed_mult;
                    self.state.camera.pan_ortho(target_vtype, world_dx, world_dy);
                }
            }
            self.last_mouse_pos = Some(pos);
        } else if self.rmb_down {
            if let Some((last_x, last_y)) = self.last_mouse_pos {
                let dx = (pos.0 - last_x) as f32;
                let dy = (pos.1 - last_y) as f32;
                self.state.camera.rotate(dx, dy, 0.003);
            }
            self.last_mouse_pos = Some(pos);
        } else if self.lmb_down && self.state.active_gizmo_axis != GizmoAxis::None {
            if let (Some(ray), Some(start_loc)) = (self.get_ray_at_cursor(self.mouse_pos), self.drag_start_object_loc) {
                let rot_mat = self.drag_rot_mat;
                // Ctrl = invert snapping, Shift = fine (precision) snapping steps.
                let snap_inv = ctrl && !shift;
                let snap_grid = self.state.snap_to_grid != snap_inv;
                let snap_rot = self.state.snap_to_rotation != snap_inv;
                let precise = shift && !ctrl && !self.shift_drag_tracking;
                let gs_eff = if precise { self.state.grid_step / 4.0 } else { self.state.grid_step };
                let rot_step_eff = if precise { self.state.rotation_snap_deg / 3.0 } else { self.state.rotation_snap_deg };
                match self.state.gizmo_mode {
                    crate::raycast::GizmoMode::Translate => {
                        let new_loc = match self.state.active_gizmo_axis {
                            GizmoAxis::PlaneXY | GizmoAxis::PlaneXZ | GizmoAxis::PlaneYZ => {
                                let local_normal = match self.state.active_gizmo_axis {
                                    GizmoAxis::PlaneXY => Vec3::Z,
                                    GizmoAxis::PlaneXZ => Vec3::Y,
                                    GizmoAxis::PlaneYZ => Vec3::X,
                                    _ => Vec3::Z,
                                };
                                let world_normal = rot_mat * local_normal;
                                if let Some(hit) = ray.project_onto_plane(start_loc, world_normal) {
                                    let delta_world = hit - self.drag_plane_offset - start_loc;
                                    let mut delta_local = rot_mat.transpose() * delta_world;
                                    match self.state.active_gizmo_axis {
                                        GizmoAxis::PlaneXY => delta_local.z = 0.0,
                                        GizmoAxis::PlaneXZ => delta_local.y = 0.0,
                                        GizmoAxis::PlaneYZ => delta_local.x = 0.0,
                                        _ => {}
                                    }
                                    if snap_grid && gs_eff > 0.0 {
                                        let gs = gs_eff;
                                        delta_local.x = (delta_local.x / gs).round() * gs;
                                        delta_local.y = (delta_local.y / gs).round() * gs;
                                        delta_local.z = (delta_local.z / gs).round() * gs;
                                    }
                                    Some(start_loc + rot_mat * delta_local)
                                } else {
                                    None
                                }
                            }
                            GizmoAxis::ScreenPlane => {
                                let plane_normal = match self.state.active_viewport_type {
                                    crate::camera::ViewportType::Top => Vec3::Z,
                                    crate::camera::ViewportType::Front => Vec3::Y,
                                    crate::camera::ViewportType::Side => Vec3::X,
                                    crate::camera::ViewportType::Perspective => self.state.camera.forward(),
                                };
                                if let Some(hit) = ray.project_onto_plane(start_loc, plane_normal) {
                                    let mut delta_world = hit - self.drag_plane_offset - start_loc;
                                    if snap_grid && gs_eff > 0.0 {
                                        let gs = gs_eff;
                                        delta_world.x = (delta_world.x / gs).round() * gs;
                                        delta_world.y = (delta_world.y / gs).round() * gs;
                                        delta_world.z = (delta_world.z / gs).round() * gs;
                                    }
                                    Some(start_loc + delta_world)
                                } else {
                                    None
                                }
                            }
                            GizmoAxis::X | GizmoAxis::Y | GizmoAxis::Z => {
                                let local_axis_dir = match self.state.active_gizmo_axis {
                                    GizmoAxis::X => Vec3::X,
                                    GizmoAxis::Y => Vec3::Y,
                                    GizmoAxis::Z => Vec3::Z,
                                    _ => Vec3::ZERO,
                                };
                                let world_axis_dir = rot_mat * local_axis_dir;
                                if let Some(s) = ray.project_onto_axis(start_loc, world_axis_dir) {
                                    let mut delta_s = s - self.drag_axis_offset;
                                    if snap_grid && gs_eff > 0.0 {
                                        delta_s = (delta_s / gs_eff).round() * gs_eff;
                                    }
                                    Some(start_loc + world_axis_dir * delta_s)
                                } else {
                                    None
                                }
                            }
                            _ => None,
                        };

                        let surface_magnet = (ctrl && shift) || self.state.surface_magnet_mode;
                        let vertex_magnet = self.keys_down.contains(&KeyCode::KeyV) || self.state.vertex_magnet_mode;
                        let mut handled_by_magnet = false;

                        if surface_magnet {
                            if let Some((hit_pos, hit_norm)) = self.state.raycast_world(ray) {
                                let align_normal = alt;
                                if let Some(sel_idx) = self.state.selected_actor_index {
                                    self.state.snap_actor_to_surface(sel_idx, hit_pos, hit_norm, align_normal);
                                    self.state.cursor_delta_text = Some(format!("Surface Magnet: ({:.0}, {:.0}, {:.0})", hit_pos.x, hit_pos.y, hit_pos.z));
                                    handled_by_magnet = true;
                                    needs_redraw = true;
                                } else if let Some(b_idx) = self.state.selected_brush_index {
                                    self.state.snap_brush_to_surface(b_idx, hit_pos, hit_norm);
                                    self.state.cursor_delta_text = Some(format!("Surface Magnet: ({:.0}, {:.0}, {:.0})", hit_pos.x, hit_pos.y, hit_pos.z));
                                    handled_by_magnet = true;
                                    needs_redraw = true;
                                }
                            }
                        }

                        let final_loc = if !handled_by_magnet && vertex_magnet {
                            if let Some(v) = self.state.find_nearest_world_vertex(ray, 48.0) {
                                self.state.cursor_delta_text = Some(format!("Vertex Snap: ({:.0}, {:.0}, {:.0})", v.x, v.y, v.z));
                                Some(v)
                            } else {
                                new_loc
                            }
                        } else if !handled_by_magnet {
                            new_loc
                        } else {
                            None
                        };

                        if let Some(loc) = final_loc {
                            let prev_delta = self.state.active_gizmo_delta;
                            self.state.active_gizmo_delta = loc - start_loc;
                            let d = self.state.active_gizmo_delta;
                            if self.state.cursor_delta_text.is_none() {
                                self.state.cursor_delta_text = Some(format!("ΔX: {:+.0}  ΔY: {:+.0}  ΔZ: {:+.0}  [{:.1} UU]", d.x, d.y, d.z, d.length()));
                            }
                            if self.state.editor_mode == crate::editor_state::EditorMode::VertexEdit && !self.state.selected_vertex_indices.is_empty() {
                                let delta = loc - start_loc;
                                self.state.move_selected_vertices(delta);
                                self.drag_start_object_loc = Some(loc);
                            } else if let Some(snap) = self.transform_snapshot.as_ref() {
                                if self.shift_drag_tracking {
                                    self.state.camera.position += d - prev_delta;
                                }
                                snap.apply_translation(&mut self.state, d);
                            }
                        }
                    }
                    crate::raycast::GizmoMode::Rotate => {
                        let axis = self.state.active_gizmo_axis;
                        let local_axis = match axis {
                            GizmoAxis::X => Vec3::X,
                            GizmoAxis::Y => Vec3::Y,
                            GizmoAxis::Z => Vec3::Z,
                            _ => Vec3::ZERO,
                        };
                        if let (Some(snap), Some(cur_angle)) = (
                            self.transform_snapshot.as_ref(),
                            ray.get_rotation_angle_oriented(start_loc, axis, rot_mat),
                        ) {
                            let mut delta_deg = (cur_angle - self.drag_start_angle).to_degrees();
                            delta_deg = (delta_deg + 180.0).rem_euclid(360.0) - 180.0;
                            if snap_rot && rot_step_eff > 0.0 {
                                delta_deg = (delta_deg / rot_step_eff).round() * rot_step_eff;
                            }
                            self.state.active_gizmo_angle_deg = delta_deg;
                            snap.apply_rotation(&mut self.state, start_loc, rot_mat * local_axis, delta_deg.to_radians());
                        }
                    }
                    crate::raycast::GizmoMode::Scale => {
                        let axis = self.state.active_gizmo_axis;
                        let mask = match axis {
                            GizmoAxis::X => Vec3::X,
                            GizmoAxis::Y => Vec3::Y,
                            GizmoAxis::Z => Vec3::Z,
                            GizmoAxis::PlaneXY => Vec3::new(1.0, 1.0, 0.0),
                            GizmoAxis::PlaneXZ => Vec3::new(1.0, 0.0, 1.0),
                            GizmoAxis::PlaneYZ => Vec3::new(0.0, 1.0, 1.0),
                            GizmoAxis::ScreenPlane => Vec3::ONE,
                            _ => Vec3::ZERO,
                        };
                        // Absolute scale factor derived from the pointer, relative to where the drag started.
                        let factor_scalar = if mask == Vec3::ZERO {
                            None
                        } else if axis == GizmoAxis::ScreenPlane {
                            let plane_normal = match self.state.active_viewport_type {
                                crate::camera::ViewportType::Top => Vec3::Z,
                                crate::camera::ViewportType::Front => Vec3::Y,
                                crate::camera::ViewportType::Side => Vec3::X,
                                crate::camera::ViewportType::Perspective => self.state.camera.forward(),
                            };
                            let d0 = self.drag_plane_offset.length().max(1.0);
                            ray.project_onto_plane(start_loc, plane_normal).map(|hit| (hit - start_loc).length() / d0)
                        } else {
                            let dir = rot_mat * mask.normalize();
                            ray.project_onto_axis(start_loc, dir).map(|s| {
                                let s0 = *self.scale_drag_s0.get_or_insert(s);
                                1.0 + (s - s0) / 100.0
                            })
                        };
                        if let (Some(snap), Some(f)) = (self.transform_snapshot.as_ref(), factor_scalar) {
                            let f = f.clamp(0.01, 100.0);
                            let f = if snap_grid { let st = if precise { 0.01 } else { 0.05 }; (f / st).round() * st } else { f };
                            let f = f.max(0.01);
                            let factor = Vec3::ONE + (Vec3::splat(f) - Vec3::ONE) * mask;
                            snap.apply_scale(&mut self.state, start_loc, factor);
                            self.state.cursor_delta_text = Some(format!("Scale x{:.2} ({:?})", f, axis));
                        }
                    }
                }
            }
        }

        needs_redraw
    }

    fn get_ray_at_cursor(&mut self, (mx, my): (f64, f64)) -> Option<Ray> {
        let [vx, vy, vw, vh] = self.state.viewport_rect;
        let target_vtype = self.state.viewport_type_at(mx as f32, my as f32)?;

        let (local_x, local_y, target_w, target_h) = if self.state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
            let hw = vw * 0.5;
            let hh = vh * 0.5;
            let rw = vw - hw;
            let rh = vh - hh;

            match target_vtype {
                crate::camera::ViewportType::Top => ((mx as f32) - vx, (my as f32) - vy, hw, hh),
                crate::camera::ViewportType::Side => ((mx as f32) - (vx + hw), (my as f32) - vy, rw, hh),
                crate::camera::ViewportType::Front => ((mx as f32) - vx, (my as f32) - (vy + hh), hw, rh),
                crate::camera::ViewportType::Perspective => ((mx as f32) - (vx + hw), (my as f32) - (vy + hh), rw, rh),
            }
        } else {
            ((mx as f32) - vx, (my as f32) - vy, vw, vh)
        };

        self.state.active_viewport_type = target_vtype;
        let aspect = target_w / target_h.max(1.0);
        let inv_vp = self.state.camera.view_projection_matrix_for_type(target_vtype, aspect).inverse();
        Some(Ray::from_screen(local_x, local_y, target_w, target_h, inv_vp))
    }

    fn render_frame(&mut self) {
        let frame_started = Instant::now();
        if self.device_retry_at.is_some() || self.device_loss.lock().unwrap_or_else(|poison| poison.into_inner()).is_some() {
            return;
        }
        if let Some(deadline) = self.surface_recovery.pending_until(frame_started) {
            self.frame_schedule.request(deadline);
            return;
        }
        let (Some(window), Some(surface), Some(device), Some(queue), Some(config), Some(renderer), Some(egui_winit), Some(egui_render)) = (
            &self.window,
            &self.surface,
            &self.device,
            &self.queue,
            &mut self.surface_config,
            &mut self.renderer,
            &mut self.egui_winit_state,
            &mut self.egui_renderer,
        ) else {
            return;
        };

        let desired_pm = crate::graphics::resolve_present_mode(
            self.state.present_mode,
            &self.state.graphics_diagnostics.supported_present_modes,
        );
        let desired_lat = self.state.max_frame_latency.clamp(1, 2);
        if config.present_mode != desired_pm
            || config.desired_maximum_frame_latency != desired_lat
        {
            config.present_mode = desired_pm;
            config.desired_maximum_frame_latency = desired_lat;
            surface.configure(device, config);
            self.state.graphics_diagnostics.active_present_mode = Some(desired_pm);
            self.state.graphics_diagnostics.active_max_frame_latency = desired_lat;
        }

        let acquire_started = Instant::now();
        let output = match surface.get_current_texture() {
            Ok(tex) => tex,
            Err(error) => {
                let retry = self.surface_recovery.failed(error.clone(), Instant::now());
                if retry.report { tracing::warn!("Surface acquisition failed: {error}; scheduling retry"); }
                self.state.graphics_diagnostics.surface_failures = self.state.graphics_diagnostics.surface_failures.saturating_add(1);
                self.state.graphics_diagnostics.last_surface_error = Some(error.to_string());
                if retry.reconfigure { surface.configure(device, config); }
                self.frame_schedule.defer_until(retry.at);
                self.frame_schedule.request(retry.at);
                return;
            }
        };
        self.state.graphics_diagnostics.cpu_acquire_ms = acquire_started.elapsed().as_secs_f64() * 1000.0;

        if self.surface_recovery.succeeded() {
            tracing::info!("Graphics surface recovered");
            self.state.graphics_diagnostics.surface_recoveries = self.state.graphics_diagnostics.surface_recoveries.saturating_add(1);
        }
        self.frame_schedule.resume();
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        let ui_started = Instant::now();
        // 1. Run egui first to compute UI layout and determine exact central viewport rect
        let effective_scale = (window.scale_factor() as f32) * self.state.ui_scale.clamp(0.5, 3.0);
        self.egui_ctx.set_pixels_per_point(effective_scale);
        let raw_input = egui_winit.take_egui_input(window);
        let mut avail_rect = egui::Rect::NOTHING;
        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            render_ui(ctx, &mut self.state);
            avail_rect = ctx.available_rect();
        });

        for viewport in full_output.viewport_output.values() {
            if let Some(deadline) = Instant::now().checked_add(viewport.repaint_delay) {
                self.frame_schedule.request(deadline);
            }
        }

        // 2. Compute accurate 3D viewport canvas rectangle and aspect ratio (no distortion)
        let vx = (avail_rect.min.x * effective_scale).round().max(0.0);
        let vy = (avail_rect.min.y * effective_scale).round().max(0.0);
        let vw = (avail_rect.width() * effective_scale).round().max(1.0);
        let vh = (avail_rect.height() * effective_scale).round().max(1.0);
        self.state.viewport_rect = [vx, vy, vw, vh];
        self.state.camera.aspect = vw / vh;

        self.state.graphics_diagnostics.cpu_ui_ms = ui_started.elapsed().as_secs_f64() * 1000.0;

        // Encode both the 3D scene pass and the egui overlay pass into one CommandEncoder and submit once.
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Frame Render Encoder"),
        });

        let world_started = Instant::now();
        // 3. Encode 3D World (Brushes, Grid, Actors, Gizmos & Multi-Viewports)
        renderer.encode_render(
            device,
            queue,
            &mut encoder,
            &view,
            &self.state,
        );
        self.state.graphics_diagnostics.cpu_world_ms = world_started.elapsed().as_secs_f64() * 1000.0;
        let frame_stats = renderer.last_frame_stats();
        if frame_stats.csg_rebuilt {
            self.state.graphics_diagnostics.csg_rebuilds_total = self.state.graphics_diagnostics.csg_rebuilds_total.saturating_add(1);
            self.state.graphics_diagnostics.last_csg_rebuild_reason = frame_stats.csg_rebuild_reason;
        }
        self.state.graphics_diagnostics.last_render_stats = frame_stats;
        self.state.last_cull_stats = renderer.last_cull_stats;
        self.state.graphics_diagnostics.lightmap_layers_used = renderer.lightmap_atlas.len();
        self.state.graphics_diagnostics.lightmap_fallback_surfaces = renderer.last_lightmap_fallbacks;
        self.state.graphics_diagnostics.diffuse_texture_layer_cap = renderer.max_diffuse_texture_layers();
        self.state.graphics_diagnostics.diffuse_texture_array_bytes = renderer.diffuse_texture_array_bytes();
        self.state.graphics_diagnostics.lightmap_atlas_dim = renderer.lightmap_atlas.dim();
        self.state.graphics_diagnostics.lightmap_atlas_bytes = renderer.lightmap_atlas.vram_bytes();
        self.state.graphics_diagnostics.retained_vertex_budget_bytes = renderer.max_retained_vertex_bytes();
        self.state.graphics_diagnostics.cpu_vertex_storage_bytes = renderer.cpu_vertex_storage_bytes();
        self.state.graphics_diagnostics.cpu_grid_cache_bytes = renderer.grid_cache_bytes();
        self.state.graphics_diagnostics.gpu_vertex_buffer_bytes = renderer.gpu_vertex_buffer_bytes();
        self.state.graphics_diagnostics.texture_manager_used_bytes =
            self.state.texture_manager.total_memory_usage_bytes();
        self.state.graphics_diagnostics.texture_manager_budget_bytes =
            self.state.texture_manager.memory_budget_bytes();
        self.state.graphics_diagnostics.active_render_scale =
            self.state.viewport_render_scale.clamp(0.5, 1.0);
        self.state.graphics_diagnostics.active_render_resolution = renderer
            .scaled_target_dimensions()
            .unwrap_or((vw as u32, vh as u32));

        // Sorting the bounded timing history is needed only while the profiler is open.
        if self.state.show_map_budget_dialog {
            self.state.graphics_diagnostics.gpu_viewport_timing = renderer.gpu_timing_summary();
        }
        let overlay_started = Instant::now();
        // 4. Encode egui UI on top into the same encoder and submit once per frame
        egui_winit.handle_platform_output(window, full_output.platform_output);

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);
        for (id, delta) in &full_output.textures_delta.set {
            self.gui_texture_backup.update(*id, delta);
            egui_render.update_texture(device, queue, *id, delta);
        }

        let screen_desc = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [config.width, config.height],
            pixels_per_point: effective_scale,
        };

        let egui_cmd_bufs = egui_render.update_buffers(device, queue, &mut encoder, &tris, &screen_desc);

        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            }).forget_lifetime();

            egui_render.render(&mut rpass, &tris, &screen_desc);
        }

        let submit_started = Instant::now();
        queue.submit(egui_cmd_bufs.into_iter().chain(std::iter::once(encoder.finish())));
        renderer.mark_submitted();
        self.state.graphics_diagnostics.last_render_stats.encode_submit_ms +=
            submit_started.elapsed().as_secs_f64() * 1000.0;

        // Free textures AFTER submit so the command buffer referencing them is already submitted
        for id in &full_output.textures_delta.free {
            egui_render.free_texture(id);
            self.gui_texture_backup.free(id);
        }

        self.state.graphics_diagnostics.cpu_overlay_ms = overlay_started.elapsed().as_secs_f64() * 1000.0;
        self.state.graphics_diagnostics.gui_texture_backup_bytes = self.gui_texture_backup.bytes();
        let present_started = Instant::now();
        output.present();
        self.state.graphics_diagnostics.cpu_present_ms = present_started.elapsed().as_secs_f64() * 1000.0;
        self.state.graphics_diagnostics.record_frame(frame_started.elapsed().as_secs_f64() * 1000.0);
    }
}

fn pointer_hits_egui_ui(
    ctx: &egui::Context,
    viewport_rect: [f32; 4],
    pos_phys: (f64, f64),
) -> bool {
    let [vx, vy, vw, vh] = viewport_rect;
    let mx = pos_phys.0 as f32;
    let my = pos_phys.1 as f32;
    if vw > 0.0 && vh > 0.0 && (mx < vx || mx > vx + vw || my < vy || my > vy + vh) {
        return true;
    }
    let ppp = ctx.pixels_per_point().max(0.1);
    let pos = egui::pos2(mx / ppp, my / ppp);
    ctx.layer_id_at(pos)
        .is_some_and(|layer| layer.order != egui::Order::Background)
}

fn coalesce_trailing_pointer_moved(events: &mut Vec<egui::Event>) {
    if events.len() >= 2
        && matches!(events[events.len() - 1], egui::Event::PointerMoved(_))
        && matches!(events[events.len() - 2], egui::Event::PointerMoved(_))
    {
        let latest = events.pop().unwrap();
        *events.last_mut().unwrap() = latest;
    }
}

#[cfg(test)]
mod cursor_redraw_tests {
    use super::*;

    fn idle_app(layout: crate::camera::ViewportLayout) -> EditorApp {
        let mut app = EditorApp::default();
        app.dirty_redraw = false;
        app.state.viewport_layout = layout;
        app.state.viewport_rect = [0.0, 0.0, 1280.0, 720.0];
        app
    }

    #[test]
    fn inert_cursor_moved_inside_same_viewport_does_not_mark_dirty_redraw() {
        let mut single = idle_app(crate::camera::ViewportLayout::Single);
        assert!(!single.handle_cursor_moved((320.0, 180.0)));
        assert!(!single.handle_cursor_moved((325.0, 182.0)));
        assert!(!single.dirty_redraw);

        let mut four = idle_app(crate::camera::ViewportLayout::FourViewports);
        four.state.active_viewport_type = crate::camera::ViewportType::Top;
        assert!(!four.handle_cursor_moved((100.0, 100.0)));
        assert!(!four.handle_cursor_moved((150.0, 140.0)));
        assert!(!four.dirty_redraw);
    }

    #[test]
    fn crossing_quadrants_in_four_viewports_requests_single_redraw() {
        let mut app = idle_app(crate::camera::ViewportLayout::FourViewports);
        app.state.active_viewport_type = crate::camera::ViewportType::Top;
        assert!(!app.handle_cursor_moved((100.0, 100.0)));
        // Move into Bottom-Right Perspective quadrant -> redraw needed once
        assert!(app.handle_cursor_moved((960.0, 540.0)));
        assert_eq!(app.state.active_viewport_type, crate::camera::ViewportType::Perspective);
        // Subsequent inert movement inside the same quadrant does not request redraw
        assert!(!app.handle_cursor_moved((980.0, 550.0)));
    }

    #[test]
    fn hovering_and_leaving_gizmo_axis_requests_redraw_only_on_change() {
        let mut app = idle_app(crate::camera::ViewportLayout::Single);
        app.state.selected_brush_index = Some(0);
        let pivot = crate::transform_session::gizmo_pivot(&app.state).unwrap();
        app.state.camera.position = pivot - app.state.camera.forward() * 350.0;

        // Exact viewport center hits ScreenPlane gizmo handle
        assert!(app.handle_cursor_moved((640.0, 360.0)));
        assert_ne!(app.state.hovered_gizmo_axis, GizmoAxis::None);
        // Small move still on the same gizmo handle stays clean
        assert!(!app.handle_cursor_moved((641.0, 360.5)));
        // Moving outside gizmo transitions hover back to None -> redraw needed once
        assert!(app.handle_cursor_moved((40.0, 40.0)));
        assert_eq!(app.state.hovered_gizmo_axis, GizmoAxis::None);
        assert!(!app.handle_cursor_moved((45.0, 45.0)));
    }

    #[test]
    fn active_button_drag_drop_or_delta_hud_always_requests_redraw() {
        let mut app = idle_app(crate::camera::ViewportLayout::Single);
        app.lmb_down = true;
        assert!(app.handle_cursor_moved((300.0, 300.0)));
        app.lmb_down = false;

        app.rmb_down = true;
        assert!(app.handle_cursor_moved((310.0, 310.0)));
        app.rmb_down = false;

        app.mmb_down = true;
        assert!(app.handle_cursor_moved((320.0, 320.0)));
        app.mmb_down = false;

        app.state.active_drag_drop = Some(crate::editor_state::DragDropPayload::ActorClass("Light".into()));
        assert!(app.handle_cursor_moved((330.0, 330.0)));
        app.state.active_drag_drop = None;

        app.state.cursor_delta_text = Some("DX: +16".into());
        assert!(app.handle_cursor_moved((340.0, 340.0)));
    }

    #[test]
    fn hovering_or_leaving_egui_panels_and_windows_requests_redraw() {
        let mut app = idle_app(crate::camera::ViewportLayout::Single);
        app.egui_ctx.set_pixels_per_point(1.0);
        // Even before the very first Context::run(), cursor moves never panic on egui pass_state.
        assert!(!app.handle_cursor_moved((350.0, 150.0)));

        let run_ui_frame = |ctx: &egui::Context, pos: egui::Pos2| -> egui::Rect {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 720.0)));
            input.events = vec![egui::Event::PointerMoved(pos)];
            let mut avail = egui::Rect::NOTHING;
            let _ = ctx.run(input, |ctx| {
                egui::SidePanel::left("test_side").exact_width(200.0).show(ctx, |ui| {
                    ui.label("Panel");
                });
                egui::Window::new("test_win")
                    .fixed_pos(egui::pos2(600.0, 300.0))
                    .fixed_size(egui::vec2(160.0, 100.0))
                    .show(ctx, |ui| {
                        ui.label("Window");
                    });
                avail = ctx.available_rect();
            });
            avail
        };

        // Frame rendered while cursor is inside the floating egui Window
        let avail = run_ui_frame(&app.egui_ctx, egui::pos2(650.0, 340.0));
        app.state.viewport_rect = [avail.min.x, avail.min.y, avail.width(), avail.height()];

        // Hovering inside SidePanel or floating Window requires redraw
        assert!(app.handle_cursor_moved((50.0, 200.0)));
        assert!(app.handle_cursor_moved((660.0, 350.0)));

        // First move leaving the floating Window into empty viewport requests one redraw to clear hover
        assert!(app.handle_cursor_moved((350.0, 150.0)));

        // Subsequent moves inside the empty viewport stay idle even without another UI pass
        assert!(!app.handle_cursor_moved((360.0, 155.0)));
        run_ui_frame(&app.egui_ctx, egui::pos2(360.0, 155.0));
        assert!(!app.handle_cursor_moved((365.0, 158.0)));
    }

    #[test]
    fn coalesce_trailing_pointer_moved_preserves_other_events() {
        let mut events = vec![
            egui::Event::PointerButton {
                pos: egui::pos2(10.0, 10.0),
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ];
        for i in 0..500 {
            events.push(egui::Event::PointerMoved(egui::pos2(i as f32, 20.0)));
            coalesce_trailing_pointer_moved(&mut events);
        }
        events.push(egui::Event::Key {
            key: egui::Key::W,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        });
        events.push(egui::Event::PointerMoved(egui::pos2(1.0, 2.0)));
        coalesce_trailing_pointer_moved(&mut events);
        events.push(egui::Event::PointerMoved(egui::pos2(3.0, 4.0)));
        coalesce_trailing_pointer_moved(&mut events);

        assert_eq!(events.len(), 4);
        assert!(matches!(events[0], egui::Event::PointerButton { .. }));
        assert_eq!(events[1], egui::Event::PointerMoved(egui::pos2(499.0, 20.0)));
        assert!(matches!(events[2], egui::Event::Key { .. }));
        assert_eq!(events[3], egui::Event::PointerMoved(egui::pos2(3.0, 4.0)));
    }
}

#[cfg(all(test, target_os = "windows"))]
#[path = "device_recovery_tests.rs"]
mod device_recovery_tests;
