use crate::editor_state::EditorState;
use egui::{Color32, Pos2, Rect, Sense, Stroke, Vec2};
use ut99_bsp::CsgOperation;

pub fn render_minimap_overlay(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_minimap {
        return;
    }

    let mut is_open = state.show_minimap;
    let window_size = Vec2::new(240.0, 260.0);
    let bottom_offset = if state.show_bottom_console { -140.0 } else { -32.0 };

    egui::Window::new("Map Radar (M)")
        .open(&mut is_open)
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-15.0, bottom_offset))
        .fixed_size(window_size)
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let cam_pos = state.camera.position;
                ui.label(
                    egui::RichText::new(format!("Cam: ({:.0}, {:.0})", cam_pos.x, cam_pos.y))
                        .monospace()
                        .size(10.5)
                        .color(Color32::from_rgb(245, 182, 45)),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("🎯 Center").on_hover_text("Center camera at map origin").clicked() {
                        state.camera.position.x = 0.0;
                        state.camera.position.y = 0.0;
                    }
                });
            });

            ui.separator();

            // Reuse cached world brush AABBs when world_brushes and map_instance_id are unchanged.
            state.ensure_world_brush_bounds();
            let (mut min_x, mut max_x, mut min_y, mut max_y) =
                state.scene_bounds_cache.borrow().world_brushes_total_xy;

            // Also bound camera position
            let cam_p = state.camera.position;
            min_x = min_x.min(cam_p.x - 256.0);
            max_x = max_x.max(cam_p.x + 256.0);
            min_y = min_y.min(cam_p.y - 256.0);
            max_y = max_y.max(cam_p.y + 256.0);

            // Add margin
            let pad_x = ((max_x - min_x) * 0.10).max(256.0);
            let pad_y = ((max_y - min_y) * 0.10).max(256.0);
            min_x -= pad_x;
            max_x += pad_x;
            min_y -= pad_y;
            max_y += pad_y;

            let span_x = (max_x - min_x).max(100.0);
            let span_y = (max_y - min_y).max(100.0);

            let (response, painter) = ui.allocate_painter(Vec2::new(220.0, 200.0), Sense::click_and_drag());
            let canvas = response.rect;

            // Background grid
            painter.rect_filled(canvas, 4.0_f32, Color32::from_rgb(16, 18, 22));
            painter.rect_stroke(canvas, 4.0_f32, Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 60)), egui::StrokeKind::Inside);

            // Subtle radar crosshair at origin (0, 0)
            let to_canvas = |wx: f32, wy: f32| -> Pos2 {
                let u = ((wx - min_x) / span_x).clamp(0.0, 1.0);
                let v = ((max_y - wy) / span_y).clamp(0.0, 1.0); // Inverted Y for top-down
                Pos2::new(canvas.min.x + u * canvas.width(), canvas.min.y + v * canvas.height())
            };

            let origin_pos = to_canvas(0.0, 0.0);
            painter.line_segment(
                [Pos2::new(canvas.min.x, origin_pos.y), Pos2::new(canvas.max.x, origin_pos.y)],
                Stroke::new(0.5_f32, Color32::from_rgb(32, 38, 48)),
            );
            painter.line_segment(
                [Pos2::new(origin_pos.x, canvas.min.y), Pos2::new(origin_pos.x, canvas.max.y)],
                Stroke::new(0.5_f32, Color32::from_rgb(32, 38, 48)),
            );

            // Paint Brushes from cached bounds
            {
                let cache = state.scene_bounds_cache.borrow();
                for (idx, (brush, &(b_min, b_max))) in state
                    .world_brushes
                    .iter()
                    .zip(cache.world_brushes.iter())
                    .enumerate()
                {
                    let is_sel = state.is_brush_selected(idx);
                    let p0 = to_canvas(b_min.x, b_min.y);
                    let p1 = to_canvas(b_max.x, b_max.y);
                    let b_rect = Rect::from_two_pos(p0, p1);

                    let fill = match brush.csg_oper {
                        CsgOperation::Subtract => Color32::from_rgba_unmultiplied(210, 175, 40, 50),
                        _ => Color32::from_rgba_unmultiplied(40, 100, 200, 50),
                    };
                    let stroke = if is_sel {
                        Stroke::new(1.5_f32, Color32::WHITE)
                    } else {
                        match brush.csg_oper {
                            CsgOperation::Subtract => Stroke::new(1.0_f32, Color32::from_rgb(220, 180, 45)),
                            _ => Stroke::new(1.0_f32, Color32::from_rgb(60, 130, 230)),
                        }
                    };
                    painter.rect_filled(b_rect, 1.0_f32, fill);
                    painter.rect_stroke(b_rect, 1.0_f32, stroke, egui::StrokeKind::Inside);
                }
            }

            // Paint Key Actors if UNR loaded
            if let Some(ref level) = state.loaded_unr {
                for start in &level.player_starts {
                    let pt = to_canvas(start.location.x, start.location.y);
                    painter.circle_filled(pt, 3.0, Color32::from_rgb(50, 230, 90));
                }
                for light in &level.lights {
                    let pt = to_canvas(light.location.x, light.location.y);
                    painter.circle_filled(pt, 2.0, Color32::from_rgb(255, 230, 80));
                }
                for node in &level.path_nodes {
                    let pt = to_canvas(node.location.x, node.location.y);
                    painter.circle_filled(pt, 2.0, Color32::from_rgb(30, 200, 230));
                }
            }

            // Paint Camera (Blip & View Cone)
            let cam_pt = to_canvas(cam_p.x, cam_p.y);
            let yaw = state.camera.yaw;
            let fov_half = state.camera.fov_y * 0.5;
            let cone_len = 22.0;

            let ray_l = Pos2::new(
                cam_pt.x + (yaw - fov_half).cos() * cone_len,
                cam_pt.y - (yaw - fov_half).sin() * cone_len,
            );
            let ray_r = Pos2::new(
                cam_pt.x + (yaw + fov_half).cos() * cone_len,
                cam_pt.y - (yaw + fov_half).sin() * cone_len,
            );

            // View cone
            let cone_color = Color32::from_rgba_unmultiplied(245, 182, 45, 80);
            painter.line_segment([cam_pt, ray_l], Stroke::new(1.0_f32, cone_color));
            painter.line_segment([cam_pt, ray_r], Stroke::new(1.0_f32, cone_color));
            painter.line_segment([ray_l, ray_r], Stroke::new(1.0_f32, cone_color));

            // Camera center dot
            painter.circle_filled(cam_pt, 4.0, Color32::from_rgb(245, 182, 45));
            painter.circle_stroke(cam_pt, 4.0, Stroke::new(1.0_f32, Color32::WHITE));

            // Click-to-teleport interaction
            if response.clicked() || response.dragged() {
                if let Some(mouse_pos) = response.interact_pointer_pos() {
                    let u = ((mouse_pos.x - canvas.min.x) / canvas.width()).clamp(0.0, 1.0);
                    let v = ((mouse_pos.y - canvas.min.y) / canvas.height()).clamp(0.0, 1.0);
                    let target_x = min_x + u * span_x;
                    let target_y = max_y - v * span_y;
                    state.camera.position.x = target_x;
                    state.camera.position.y = target_y;
                }
            }
        });

    state.show_minimap = is_open;
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Context, Event, Pos2, PointerButton, Modifiers};
    use glam::Vec3;

    fn run_minimap_frame(ctx: &Context, state: &mut EditorState, events: Vec<Event>) -> egui::FullOutput {
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)));
        input.time = Some(ctx.input(|i| i.time) + 0.1);
        input.events = events;
        ctx.run(input, |ctx| {
            render_minimap_overlay(ctx, state);
        })
    }

    #[test]
    fn minimap_reuses_bounds_cache_and_updates_on_mutations() {
        let ctx = Context::default();
        let mut state = EditorState::default();
        state.loaded_unr = None;
        state.show_minimap = true;
        state.world_brushes.clear();
        let mut b = ut99_bsp::build_cube("MBrush0", CsgOperation::Add, Vec3::splat(128.0));
        b.location = Vec3::new(300.0, -400.0, 0.0);
        state.world_brushes.push(b);

        run_minimap_frame(&ctx, &mut state, vec![]);
        let ptr_before = state.scene_bounds_cache.borrow().world_brushes.as_ptr();
        let rev_before = state.world_brushes.rev();
        let xy_before = state.scene_bounds_cache.borrow().world_brushes_total_xy;

        run_minimap_frame(&ctx, &mut state, vec![]);
        assert_eq!(state.world_brushes.rev(), rev_before);
        assert_eq!(state.scene_bounds_cache.borrow().world_brushes.as_ptr(), ptr_before);
        assert_eq!(state.scene_bounds_cache.borrow().world_brushes_total_xy, xy_before);

        // Move brush far outward and add another
        state.world_brushes[0].location = Vec3::new(2500.0, 1800.0, 0.0);
        run_minimap_frame(&ctx, &mut state, vec![]);
        let xy_moved = state.scene_bounds_cache.borrow().world_brushes_total_xy;
        assert!(xy_moved.1 >= 2564.0 && xy_moved.3 >= 1864.0);

        let mut b2 = ut99_bsp::build_cube("MBrush1", CsgOperation::Add, Vec3::splat(64.0));
        b2.location = Vec3::new(-3200.0, -2400.0, 0.0);
        state.world_brushes.push(b2);
        run_minimap_frame(&ctx, &mut state, vec![]);
        let xy_added = state.scene_bounds_cache.borrow().world_brushes_total_xy;
        assert!(xy_added.0 <= -3232.0 && xy_added.2 <= -2432.0);

        // Click inside the radar canvas teleports camera X/Y
        let radar_center = Pos2::new(800.0 - 15.0 - 120.0, 600.0 - 32.0 - 110.0);
        state.camera.position.x = 12345.0;
        for pressed in [true, false] {
            run_minimap_frame(&ctx, &mut state, vec![
                Event::PointerMoved(radar_center),
                Event::PointerButton {
                    pos: radar_center,
                    button: PointerButton::Primary,
                    pressed,
                    modifiers: Modifiers::NONE,
                },
            ]);
        }
        assert!((state.camera.position.x - 12345.0).abs() > 100.0);

        // new_map resets cache and updates bounds on next frame
        state.new_map();
        assert!(state.scene_bounds_cache.borrow().world_brushes.is_empty());
        run_minimap_frame(&ctx, &mut state, vec![]);
        assert_eq!(state.scene_bounds_cache.borrow().world_brushes.len(), 1);
    }
}
