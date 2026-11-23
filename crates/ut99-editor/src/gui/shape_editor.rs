use crate::editor_state::EditorState;
use egui::{pos2, vec2, Color32, Pos2, Rect, Sense, Stroke};
use glam::Vec2;
use ut99_bsp::{Segment2DType, Shape2D};

pub fn render_shape_editor(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_shape_editor {
        return;
    }

    let mut open = state.show_shape_editor;
    crate::gui::modal::studio_dialog_window(ctx, "2D Shape Editor - UnrealEd 2 Paradigm", &mut open)
        .show(ctx, |ui| {
            // Top Toolbar: Presets & Shape generation actions
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(!state.shape2d_undo_stack.is_empty(), egui::Button::new("↩ Undo"))
                    .on_hover_text("Undo 2D Shape edit")
                    .clicked()
                {
                    state.undo_shape2d();
                }
                if ui
                    .add_enabled(!state.shape2d_redo_stack.is_empty(), egui::Button::new("↪ Redo"))
                    .on_hover_text("Redo 2D Shape edit")
                    .clicked()
                {
                    state.redo_shape2d();
                }

                ui.separator();

                ui.menu_button("Presets", |ui| {
                    if ui.button("Rectangle (256 x 256)").clicked() {
                        state.push_shape2d_undo();
                        state.shape2d = Shape2D::new_box(256.0, 256.0);
                        state.shape_editor_selected_segment = 0;
                        ui.close_menu();
                    }
                    if ui.button("Circle (16 sides, R:128)").clicked() {
                        state.push_shape2d_undo();
                        state.shape2d = Shape2D::new_circle(128.0, 16);
                        state.shape_editor_selected_segment = 0;
                        ui.close_menu();
                    }
                    if ui.button("Gothic Arch (W:256, H:384)").clicked() {
                        state.push_shape2d_undo();
                        state.shape2d = Shape2D::new_arch(256.0, 384.0, 12);
                        state.shape_editor_selected_segment = 0;
                        ui.close_menu();
                    }
                    if ui.button("Cross Column (256 x 64)").clicked() {
                        state.push_shape2d_undo();
                        state.shape2d = Shape2D::new_cross(256.0, 64.0);
                        state.shape_editor_selected_segment = 0;
                        ui.close_menu();
                    }
                });

                ui.separator();

                if ui.button("➕ Add Point").clicked() {
                    state.push_shape2d_undo();
                    let c = state.shape2d.center();
                    state.shape2d.points.push(c + Vec2::new(32.0, 32.0));
                    state.shape2d.segments.push(Segment2DType::Linear);
                }

                if ui.button("✂ Split Segment").on_hover_text("Split the selected edge at its midpoint (UnrealEd 2 Split Side)").clicked() {
                    state.push_shape2d_undo();
                    let seg = state.shape_editor_selected_segment;
                    state.shape2d.split_segment(seg);
                }

                if ui.button("🗑 Clear").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.points.clear();
                    state.shape2d.segments.clear();
                }

                ui.separator();

                if ui.button("💾 Save .2ds").on_hover_text("Save native UnrealEd 2 .2ds shape or JSON").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("UnrealEd 2D Shape (*.2ds)", &["2ds"])
                        .add_filter("JSON Shape (*.json)", &["json"])
                        .set_file_name("Shape.2ds")
                        .save_file()
                    {
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("2ds").to_lowercase();
                        let content = if ext == "json" {
                            state.shape2d.to_json().unwrap_or_default()
                        } else {
                            state.shape2d.to_2ds()
                        };
                        let _ = std::fs::write(&path, content);
                        state.log(format!("2D Shape Editor: Saved shape to '{}'.", path.display()));
                    }
                }

                if ui.button("📂 Open .2ds").on_hover_text("Load native UnrealEd 2 .2ds shape or JSON").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("2D Shape (*.2ds, *.json)", &["2ds", "json"])
                        .pick_file()
                    {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            let loaded = Shape2D::from_json(&content).or_else(|_| Shape2D::from_2ds(&content));
                            if let Ok(s) = loaded {
                                state.shape2d = s;
                                state.shape_editor_selected_segment = 0;
                                state.log(format!("2D Shape Editor: Loaded shape from '{}'.", path.display()));
                            }
                        }
                    }
                }
            });

            // Second Toolbar: Segment Type (Linear / Bezier) & Shape Transforms (Scale / Rotate / Flip)
            ui.horizontal_wrapped(|ui| {
                let n_pts = state.shape2d.points.len();
                if n_pts >= 2 {
                    if state.shape_editor_selected_segment >= n_pts {
                        state.shape_editor_selected_segment = 0;
                    }
                    ui.label(egui::RichText::new("Segment:").strong());
                    ui.add(egui::DragValue::new(&mut state.shape_editor_selected_segment).range(0..=n_pts.saturating_sub(1)));

                    let seg_idx = state.shape_editor_selected_segment;
                    let is_bezier = matches!(
                        state.shape2d.segments.get(seg_idx),
                        Some(Segment2DType::Bezier { .. })
                    );

                    if ui.selectable_label(!is_bezier, "📏 Linear").clicked() && is_bezier {
                        state.push_shape2d_undo();
                        state.shape2d.set_segment_linear(seg_idx);
                    }
                    if ui.selectable_label(is_bezier, "〰 Bezier Curve").clicked() && !is_bezier {
                        state.push_shape2d_undo();
                        state.shape2d.set_segment_bezier(seg_idx);
                    }
                }

                ui.separator();
                ui.label(egui::RichText::new("Transform:").strong());
                if ui.button("🔍 Scale 2x").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.scale_shape(2.0);
                }
                if ui.button("🔉 Scale 0.5x").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.scale_shape(0.5);
                }
                if ui.button("↻ Rotate 45°").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.rotate_shape(45.0);
                }
                if ui.button("↻ Rotate 90°").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.rotate_shape(90.0);
                }
                if ui.button("↔ Flip X").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.flip(true);
                }
                if ui.button("↕ Flip Y").clicked() {
                    state.push_shape2d_undo();
                    state.shape2d.flip(false);
                }
            });

            ui.separator();

            // Extrude & Revolve controls bar
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Extrude Depth:").strong());
                ui.add(egui::DragValue::new(&mut state.shape_editor_depth).speed(16.0).range(8.0..=4096.0));

                if ui.button(egui::RichText::new("📦 Extrude to Builder").color(Color32::from_rgb(100, 220, 120)).strong()).clicked() {
                    state.apply_shape2d_extrude();
                }

                if ui.button("▲ Apex (Pyramid)").clicked() {
                    state.apply_shape2d_pyramid(state.shape_editor_depth);
                }

                if ui.button("💎 Bevel Extrude").clicked() {
                    state.apply_shape2d_bevel();
                }

                if ui.button("📄 Flat Sheet").clicked() {
                    state.apply_shape2d_sheet();
                }
            });

            let zoom_id = egui::Id::new("shape2d_canvas_zoom");
            let pan_id = egui::Id::new("shape2d_canvas_pan");
            let mut zoom: f32 = ui.ctx().data_mut(|d| *d.get_temp_mut_or(zoom_id, 0.5f32));
            let mut pan: egui::Vec2 = ui.ctx().data_mut(|d| *d.get_temp_mut_or(pan_id, egui::Vec2::ZERO));

            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Revolve / Lathe:").strong());
                ui.label("Angle:");
                ui.add(egui::DragValue::new(&mut state.shape_editor_revolve_angle).speed(15.0).range(15.0..=360.0));
                ui.label("Segments:");
                ui.add(egui::DragValue::new(&mut state.shape_editor_revolve_segs).range(3..=64));

                if ui.button(egui::RichText::new("🔄 Revolve to Builder").color(Color32::from_rgb(255, 200, 80)).strong()).clicked() {
                    state.apply_shape2d_revolve();
                }

                ui.separator();
                ui.label(format!("Zoom: {:.0}%", zoom * 200.0));
                if ui.small_button("🔍-").clicked() {
                    zoom = (zoom * 0.8).clamp(0.1, 4.0);
                }
                if ui.small_button("🔍+").clicked() {
                    zoom = (zoom * 1.25).clamp(0.1, 4.0);
                }
                if ui.small_button("Reset View").clicked() {
                    zoom = 0.5;
                    pan = egui::Vec2::ZERO;
                }
            });

            ui.separator();

            // 2D Canvas Area
            let canvas_size = ui.available_size();
            let (response, painter) = ui.allocate_painter(canvas_size, Sense::click_and_drag());

            if response.hovered() {
                let scroll_y = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll_y.abs() > 0.01 {
                    let factor = if scroll_y > 0.0 { 1.12 } else { 0.89 };
                    zoom = (zoom * factor).clamp(0.1, 4.0);
                }
            }
            if response.dragged_by(egui::PointerButton::Middle) {
                pan += response.drag_delta();
            }
            ui.ctx().data_mut(|d| {
                d.insert_temp(zoom_id, zoom);
                d.insert_temp(pan_id, pan);
            });

            let rect = response.rect;
            // Dark Blueprint Canvas background
            painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 24, 38));
            painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(50, 70, 100)), egui::StrokeKind::Inside);

            let center = rect.center() + pan;

            // Draw Grid Lines (every 64 units)
            let grid_step_pixels = (64.0 * zoom).max(4.0);
            let start_x = rect.left() + ((center.x - rect.left()).rem_euclid(grid_step_pixels));
            let mut x = start_x;
            while x < rect.right() {
                if x >= rect.left() {
                    painter.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, Color32::from_rgb(28, 42, 65)));
                }
                x += grid_step_pixels;
            }
            let start_y = rect.top() + ((center.y - rect.top()).rem_euclid(grid_step_pixels));
            let mut y = start_y;
            while y < rect.bottom() {
                if y >= rect.top() {
                    painter.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, Color32::from_rgb(28, 42, 65)));
                }
                y += grid_step_pixels;
            }

            // Draw Origin Axes
            if center.x >= rect.left() && center.x <= rect.right() {
                painter.line_segment([pos2(center.x, rect.top()), pos2(center.x, rect.bottom())], Stroke::new(1.5_f32, Color32::from_rgb(80, 110, 160)));
            }
            if center.y >= rect.top() && center.y <= rect.bottom() {
                painter.line_segment([pos2(rect.left(), center.y), pos2(rect.right(), center.y)], Stroke::new(1.5_f32, Color32::from_rgb(80, 110, 160)));
            }

            // Convert world 2D point to canvas Pos2
            let to_screen = |p: Vec2| -> Pos2 {
                pos2(center.x + p.x * zoom, center.y - p.y * zoom)
            };
            // Convert canvas Pos2 to world 2D point
            let to_world = |p: Pos2| -> Vec2 {
                Vec2::new((p.x - center.x) / zoom, -(p.y - center.y) / zoom)
            };

            let n_pts = state.shape2d.points.len();

            // Draw evaluated shape edges (supports both Linear and Bezier curves)
            let eval_pts = state.shape2d.tessellated_points();
            let n_eval = eval_pts.len();
            if n_eval >= 2 {
                for i in 0..n_eval {
                    let next = if state.shape2d.closed {
                        (i + 1) % n_eval
                    } else if i + 1 < n_eval {
                        i + 1
                    } else {
                        continue;
                    };
                    let p0 = to_screen(eval_pts[i]);
                    let p1 = to_screen(eval_pts[next]);
                    painter.line_segment([p0, p1], Stroke::new(2.5_f32, Color32::from_rgb(255, 60, 60)));
                }
            }

            // Highlight selected segment and draw Bezier control handles
            if n_pts >= 2 && state.shape_editor_selected_segment < n_pts {
                let s_idx = state.shape_editor_selected_segment;
                let next_idx = (s_idx + 1) % n_pts;
                let sp0 = to_screen(state.shape2d.points[s_idx]);
                let sp1 = to_screen(state.shape2d.points[next_idx]);
                painter.line_segment([sp0, sp1], Stroke::new(1.5_f32, Color32::from_rgb(100, 220, 255)));

                if let Some(Segment2DType::Bezier { ctrl1, ctrl2 }) = state.shape2d.segments.get(s_idx).copied() {
                    let sc1 = to_screen(ctrl1);
                    let sc2 = to_screen(ctrl2);
                    painter.line_segment([sp0, sc1], Stroke::new(1.0_f32, Color32::from_rgb(80, 255, 160)));
                    painter.line_segment([sp1, sc2], Stroke::new(1.0_f32, Color32::from_rgb(80, 255, 160)));
                    painter.circle_filled(sc1, 5.0, Color32::from_rgb(80, 255, 160));
                    painter.circle_filled(sc2, 5.0, Color32::from_rgb(80, 255, 160));
                }
            }

            // Click on segment to select it
            if response.clicked() {
                if let Some(mouse_pos) = response.interact_pointer_pos() {
                    for i in 0..n_pts {
                        let next = (i + 1) % n_pts;
                        let p0 = to_screen(state.shape2d.points[i]);
                        let p1 = to_screen(state.shape2d.points[next]);
                        let seg_mid = pos2((p0.x + p1.x) * 0.5, (p0.y + p1.y) * 0.5);
                        if seg_mid.distance(mouse_pos) < 24.0 {
                            state.shape_editor_selected_segment = i;
                            break;
                        }
                    }
                }
            }

            // Handle point or Bezier control handle dragging
            if response.drag_started_by(egui::PointerButton::Primary) {
                state.push_shape2d_undo();
            }
            if response.dragged() {
                if let Some(mouse_pos) = response.interact_pointer_pos() {
                    let mut snap_world = to_world(mouse_pos);
                    if state.snap_to_grid && state.grid_step > 0.0 {
                        snap_world.x = (snap_world.x / state.grid_step).round() * state.grid_step;
                        snap_world.y = (snap_world.y / state.grid_step).round() * state.grid_step;
                    }

                    // Check Bezier control points of selected segment first
                    let mut moved_ctrl = false;
                    let s_idx = state.shape_editor_selected_segment;
                    if let Some(Segment2DType::Bezier { mut ctrl1, mut ctrl2 }) = state.shape2d.segments.get(s_idx).copied() {
                        if to_screen(ctrl1).distance(mouse_pos) < 14.0 {
                            ctrl1 = snap_world;
                            state.shape2d.segments[s_idx] = Segment2DType::Bezier { ctrl1, ctrl2 };
                            moved_ctrl = true;
                        } else if to_screen(ctrl2).distance(mouse_pos) < 14.0 {
                            ctrl2 = snap_world;
                            state.shape2d.segments[s_idx] = Segment2DType::Bezier { ctrl1, ctrl2 };
                            moved_ctrl = true;
                        }
                    }

                    if !moved_ctrl {
                        // Find vertex point within 14 pixels
                        let mut dragged_point_idx = None;
                        for (i, p) in state.shape2d.points.iter().enumerate() {
                            if to_screen(*p).distance(mouse_pos) < 14.0 {
                                dragged_point_idx = Some(i);
                                break;
                            }
                        }
                        if let Some(idx) = dragged_point_idx {
                            state.shape2d.points[idx] = snap_world;
                        }
                    }
                }
            }

            // Draw point handles
            for (i, p) in state.shape2d.points.iter().enumerate() {
                let sp = to_screen(*p);
                let handle_rect = Rect::from_center_size(sp, vec2(10.0, 10.0));
                painter.rect_filled(handle_rect, 2.0, Color32::from_rgb(255, 230, 80));
                painter.rect_stroke(handle_rect, 2.0, Stroke::new(1.0_f32, Color32::BLACK), egui::StrokeKind::Inside);

                painter.text(
                    sp + vec2(10.0, -10.0),
                    egui::Align2::LEFT_BOTTOM,
                    format!("{}: ({:.0}, {:.0})", i, p.x, p.y),
                    egui::FontId::monospace(10.0),
                    Color32::from_rgb(200, 220, 255),
                );
            }
        });

    state.show_shape_editor = open;
}
