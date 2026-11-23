// crates/ut99-editor/src/gui/shape_loft_studio_dialog.rs
//
// Interactive 2D Shape Loft, Curved Pipe & Architectural Molding Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{CsgOperation, LoftProfileArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_loft_studio_dialog {
        return;
    }

    let mut open = state.show_loft_studio_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "2D Shape Loft, Curved Pipe & Architectural Molding Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_loft_studio_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Architectural Profiles:").strong());
        for &arch in LoftProfileArchetype::all() {
            let is_sel = state.loft_studio_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_loft_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlay Toggles
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.loft_studio_settings.show_tangent_frames_viewport, "RMF Tangent Frames");
        ui.checkbox(&mut state.loft_studio_settings.show_profile_cross_sections_viewport, "Cross-Section Rings");
        ui.checkbox(&mut state.loft_studio_settings.show_wireframe_ghost_viewport, "3D Mesh Ghost Wireframe");
        ui.separator();
        if ui.button("🔄 Recalculate Loft").clicked() {
            state.recalculate_loft_mesh();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Cross Section & Path Spline Canvases
        columns[0].vertical(|ui| {
            ui.heading("2D Cross-Section Shape");
            render_profile_canvas(ui, state);

            ui.add_space(8.0);
            ui.heading("Longitudinal Spline Path & Tapering");
            render_path_elevation_canvas(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Geometry to Level");
            ui.horizontal(|ui| {
                if ui.button("🔴 Set Builder Brush").on_hover_text("Assigns this lofted mesh to the Red Active Builder Brush").clicked() {
                    state.commit_loft_to_builder_brush();
                }
                if ui.button("➕ Add Solid Brush").on_hover_text("Adds a permanent additive world brush").clicked() {
                    state.commit_loft_to_world_brush(CsgOperation::Add);
                }
                if ui.button("🛡 Make SemiSolid").on_hover_text("Adds an intricate non-cutting SemiSolid brush for architectural molding").clicked() {
                    let idx = state.commit_loft_to_world_brush(CsgOperation::Add);
                    if let Some(b) = state.world_brushes.get_mut(idx) {
                        for p in &mut b.polys {
                            p.flags |= ut99_bsp::PF_SEMISOLID;
                        }
                    }
                }
                if ui.button("➖ Subtract Tunnel").on_hover_text("Subtracts this pipe volume from the world to carve out conduits or halls").clicked() {
                    state.commit_loft_to_world_brush(CsgOperation::Subtract);
                }
            });
        });

        // Right Column: Parameters, Station Keyframes & Safety Audit
        columns[1].vertical(|ui| {
            ui.heading("Lofting Engine Parameters");
            let mut cfg_changed = false;

            egui::Grid::new("loft_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label("Base Profile Radius (uu):");
                if ui.add(egui::DragValue::new(&mut state.loft_studio_settings.active_config.profile_radius).speed(1.0).range(4.0..=512.0)).changed() {
                    cfg_changed = true;
                    let arch = state.loft_studio_settings.active_config.archetype;
                    state.loft_studio_settings.custom_profile = arch.to_profile(state.loft_studio_settings.active_config.profile_radius);
                }
                ui.end_row();

                ui.label("Tessellation Segments:");
                if ui.add(egui::Slider::new(&mut state.loft_studio_settings.active_config.tessellation_steps, 3..=48).text("rings")).changed() {
                    cfg_changed = true;
                }
                ui.end_row();

                ui.label("Hollow Conduit:");
                if ui.checkbox(&mut state.loft_studio_settings.active_config.is_hollow, "Generate Inner Wall").changed() {
                    cfg_changed = true;
                }
                ui.end_row();

                if state.loft_studio_settings.active_config.is_hollow {
                    ui.label("Wall Thickness (uu):");
                    if ui.add(egui::DragValue::new(&mut state.loft_studio_settings.active_config.wall_thickness).speed(0.5).range(1.0..=64.0)).changed() {
                        cfg_changed = true;
                    }
                    ui.end_row();
                }

                ui.label("End Capping:");
                ui.horizontal(|ui| {
                    if ui.checkbox(&mut state.loft_studio_settings.active_config.cap_start, "Start Cap").changed() { cfg_changed = true; }
                    if ui.checkbox(&mut state.loft_studio_settings.active_config.cap_end, "End Cap").changed() { cfg_changed = true; }
                });
                ui.end_row();

                ui.label("Texture:");
                if ui.text_edit_singleline(&mut state.loft_studio_settings.active_config.texture_name).changed() {
                    cfg_changed = true;
                }
                ui.end_row();
            });

            if cfg_changed {
                state.recalculate_loft_mesh();
            }

            ui.add_space(8.0);
            ui.heading("Spline Path Control Stations");

            let mut kf_changed = false;
            let mut add_pos = None;
            let mut remove_idx = None;

            egui::ScrollArea::vertical().max_height(190.0).show(ui, |ui| {
                egui::Grid::new("loft_keyframes_table")
                    .striped(true)
                    .num_columns(5)
                    .spacing([10.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("#").strong());
                        ui.label(egui::RichText::new("Position (X, Y, Z)").strong());
                        ui.label(egui::RichText::new("Scale").strong());
                        ui.label(egui::RichText::new("Twist").strong());
                        ui.label(egui::RichText::new("Del").strong());
                        ui.end_row();

                        let count = state.loft_studio_settings.path_keyframes.len();
                        for i in 0..count {
                            let kf = &mut state.loft_studio_settings.path_keyframes[i];
                            ui.label(format!("{}", i));

                            ui.horizontal(|ui| {
                                if ui.add(egui::DragValue::new(&mut kf.position.x).speed(4.0)).changed() { kf_changed = true; }
                                if ui.add(egui::DragValue::new(&mut kf.position.y).speed(4.0)).changed() { kf_changed = true; }
                                if ui.add(egui::DragValue::new(&mut kf.position.z).speed(4.0)).changed() { kf_changed = true; }
                            });

                            ui.horizontal(|ui| {
                                if ui.add(egui::DragValue::new(&mut kf.scale.x).speed(0.05).range(0.1..=10.0)).changed() { kf_changed = true; }
                                if ui.add(egui::DragValue::new(&mut kf.scale.y).speed(0.05).range(0.1..=10.0)).changed() { kf_changed = true; }
                            });

                            if ui.add(egui::DragValue::new(&mut kf.twist_deg).speed(2.0).suffix("°")).changed() {
                                kf_changed = true;
                            }

                            if ui.button("🗑").clicked() {
                                remove_idx = Some(i);
                            }
                            ui.end_row();
                        }
                    });
            });

            if let Some(idx) = remove_idx {
                state.remove_loft_keyframe(idx);
            }

            ui.horizontal(|ui| {
                if ui.button("➕ Append Station (+128 Z)").clicked() {
                    let last_pos = state.loft_studio_settings.path_keyframes.last().map(|k| k.position).unwrap_or(glam::Vec3::ZERO);
                    add_pos = Some(last_pos + glam::Vec3::new(64.0, 0.0, 128.0));
                }
            });

            if let Some(pos) = add_pos {
                state.add_loft_keyframe(pos);
            }

            if kf_changed {
                state.recalculate_loft_mesh();
            }

            ui.add_space(8.0);
            ui.heading("Safety & Curvature Diagnostic Audit");
            render_hazards_box(ui, state);
        });
    });
}

fn render_profile_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width(), 160.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();

    // Dark grid background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 20, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 65)), StrokeKind::Inside);

    let center = rect.center();
    let pts = &state.loft_studio_settings.custom_profile.points;
    if pts.len() < 3 {
        return;
    }

    // Determine scale to fit canvas
    let max_extent = pts.iter().fold(1.0f32, |acc, p| acc.max(p.x.abs()).max(p.y.abs())).max(10.0);
    let zoom = (rect.height() * 0.4) / max_extent;

    // Crosshairs
    painter.line_segment(
        [Pos2::new(center.x - 40.0, center.y), Pos2::new(center.x + 40.0, center.y)],
        Stroke::new(1.0_f32, Color32::from_rgb(55, 65, 80)),
    );
    painter.line_segment(
        [Pos2::new(center.x, center.y - 40.0), Pos2::new(center.x, center.y + 40.0)],
        Stroke::new(1.0_f32, Color32::from_rgb(55, 65, 80)),
    );

    // Draw profile polygon loop
    let col = Color32::from_rgb(52, 152, 219); // Azure
    let to_canvas = |p: glam::Vec2| Pos2::new(center.x + p.x * zoom, center.y - p.y * zoom);

    for i in 0..pts.len() {
        let next_i = (i + 1) % pts.len();
        let p0 = to_canvas(pts[i]);
        let p1 = to_canvas(pts[next_i]);
        painter.line_segment([p0, p1], Stroke::new(2.0_f32, col));
        painter.circle_filled(p0, 3.0, Color32::from_rgb(241, 196, 15));
    }

    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.max.y - 6.0),
        egui::Align2::LEFT_BOTTOM,
        format!("{} vertices | Base Radius: {:.0} uu", pts.len(), state.loft_studio_settings.active_config.profile_radius),
        egui::FontId::monospace(10.0),
        Color32::from_rgb(160, 175, 195),
    );
}

fn render_path_elevation_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width(), 160.0);
    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();

    // Dark grid background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 20, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 65)), StrokeKind::Inside);

    let frames = &state.loft_studio_settings.cached_frames;
    if frames.len() < 2 {
        return;
    }

    // Compute bounding dimensions of path
    let min_x = frames.iter().map(|f| f.position.x).fold(f32::INFINITY, f32::min);
    let max_x = frames.iter().map(|f| f.position.x).fold(f32::NEG_INFINITY, f32::max);
    let min_z = frames.iter().map(|f| f.position.z).fold(f32::INFINITY, f32::min);
    let max_z = frames.iter().map(|f| f.position.z).fold(f32::NEG_INFINITY, f32::max);

    let range_x = (max_x - min_x).max(10.0);
    let range_z = (max_z - min_z).max(10.0);

    let to_canvas = |pos: glam::Vec3| {
        let u = (pos.x - min_x) / range_x;
        let v = (pos.z - min_z) / range_z;
        Pos2::new(
            rect.min.x + 20.0 + u * (rect.width() - 40.0),
            rect.max.y - 20.0 - v * (rect.height() - 40.0),
        )
    };

    // Draw spline curve
    let spline_col = Color32::from_rgb(46, 204, 113); // Emerald
    for i in 0..(frames.len() - 1) {
        let p0 = to_canvas(frames[i].position);
        let p1 = to_canvas(frames[i + 1].position);
        painter.line_segment([p0, p1], Stroke::new(2.5_f32, spline_col));
    }

    // Draw control station markers
    for (idx, kf) in state.loft_studio_settings.path_keyframes.iter().enumerate() {
        let pt = to_canvas(kf.position);
        painter.circle_filled(pt, 5.0, Color32::from_rgb(231, 76, 60));
        painter.text(
            Pos2::new(pt.x + 8.0, pt.y - 6.0),
            egui::Align2::LEFT_BOTTOM,
            format!("#{}", idx),
            egui::FontId::monospace(10.0),
            Color32::from_rgb(255, 255, 255),
        );
    }

    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.max.y - 6.0),
        egui::Align2::LEFT_BOTTOM,
        format!("Elevation Profile (X vs Z) | {} RMF Frames", frames.len()),
        egui::FontId::monospace(10.0),
        Color32::from_rgb(160, 175, 195),
    );
}

fn render_hazards_box(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.loft_studio_settings.cached_hazards;
    let brush_polys = state.loft_studio_settings.cached_preview_brush.as_ref().map(|b| b.polys.len()).unwrap_or(0);

    egui::Frame::canvas(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Generated Polygons:").strong());
            let poly_col = if brush_polys <= 128 {
                Color32::from_rgb(46, 204, 113)
            } else if brush_polys <= 384 {
                Color32::from_rgb(241, 196, 15)
            } else {
                Color32::from_rgb(231, 76, 60)
            };
            ui.colored_label(poly_col, format!("{} polys", brush_polys));
        });

        if hazards.is_empty() {
            ui.colored_label(Color32::from_rgb(46, 204, 113), "✓ Zero self-intersections or pinch hazards. Geometry is watertight!");
        } else {
            for h in hazards {
                let col = if h.is_critical {
                    Color32::from_rgb(231, 76, 60)
                } else {
                    Color32::from_rgb(241, 196, 15)
                };
                ui.colored_label(col, format!("⚠ Station #{}: {}", h.keyframe_index, h.message));
            }
        }
    });
}
