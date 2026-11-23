// crates/ut99-editor/src/gui/bsp_quality_studio_dialog.rs
//
// Interactive BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio

use egui::{Color32, Pos2, Rect, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_bsp_quality_studio_dialog {
        return;
    }

    let mut open = state.show_bsp_quality_studio_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "BSP Tree Quality, Cut Minimizer & Sliver Purger Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_bsp_quality_studio_dialog = open;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StudioTab {
    CutOffenders,
    SliverPolygons,
    CoplanarOverlaps,
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Top Action Toolbar
    ui.horizontal(|ui| {
        if ui.button("🔍 Run BSP Audit").on_hover_text("Re-analyzes current BSP tree, cut ratio, slivers and coplanar overlaps").clicked() {
            state.run_bsp_quality_audit();
        }

        if ui.button("🧹 Purge All Slivers").on_hover_text("Deletes all micro-sliver triangles below the area threshold across all brushes with Undo support").clicked() {
            state.purge_all_sliver_polygons();
        }

        if ui.button("🛡 Convert Top 5 Cuts to Semi-Solid").on_hover_text("Converts the 5 highest-cut additive brushes to Semi-Solid to eliminate world BSP fragmentation").clicked() {
            state.convert_top_cut_offenders_to_semisolid(5);
        }

        ui.separator();

        ui.checkbox(&mut state.bsp_quality_studio_settings.show_cut_heatmap_viewport, "Cut Outlines");
        ui.checkbox(&mut state.bsp_quality_studio_settings.show_sliver_markers_viewport, "Sliver Glyphs");
        ui.checkbox(&mut state.bsp_quality_studio_settings.show_coplanar_markers_viewport, "Coplanar Overlaps");
    });
    ui.separator();

    // 2. Main Two-Column Layout
    ui.columns(2, |columns| {
        // Left Column: Depth Histogram & Quality Metrics Summary
        columns[0].vertical(|ui| {
            ui.heading("BSP Node Depth Histogram (0..63)");
            render_depth_histogram(ui, state);

            ui.add_space(10.0);
            ui.heading("BSP Tree Quality Scorecard");
            render_metrics_summary(ui, state);
        });

        // Right Column: Tabbed Inspector (Cut Offenders, Slivers, Coplanar)
        columns[1].vertical(|ui| {
            static CURRENT_TAB: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
            let tab_id = CURRENT_TAB.load(std::sync::atomic::Ordering::Relaxed);
            let mut current_tab = match tab_id {
                1 => StudioTab::SliverPolygons,
                2 => StudioTab::CoplanarOverlaps,
                _ => StudioTab::CutOffenders,
            };

            ui.horizontal(|ui| {
                let off_count = state.bsp_quality_studio_settings.cut_offenders.len();
                let sliv_count = state.bsp_quality_studio_settings.detected_slivers.len();
                let cop_count = state.bsp_quality_studio_settings.detected_coplanar.len();

                if ui.selectable_label(current_tab == StudioTab::CutOffenders, format!("🔪 Cut Offenders ({})", off_count)).clicked() {
                    current_tab = StudioTab::CutOffenders;
                    CURRENT_TAB.store(0, std::sync::atomic::Ordering::Relaxed);
                }
                if ui.selectable_label(current_tab == StudioTab::SliverPolygons, format!("📐 Slivers ({})", sliv_count)).clicked() {
                    current_tab = StudioTab::SliverPolygons;
                    CURRENT_TAB.store(1, std::sync::atomic::Ordering::Relaxed);
                }
                if ui.selectable_label(current_tab == StudioTab::CoplanarOverlaps, format!("⚠ Coplanar Z-Fight ({})", cop_count)).clicked() {
                    current_tab = StudioTab::CoplanarOverlaps;
                    CURRENT_TAB.store(2, std::sync::atomic::Ordering::Relaxed);
                }
            });
            ui.separator();

            match current_tab {
                StudioTab::CutOffenders => render_cut_offenders_tab(ui, state),
                StudioTab::SliverPolygons => render_slivers_tab(ui, state),
                StudioTab::CoplanarOverlaps => render_coplanar_tab(ui, state),
            }
        });
    });
}

fn render_depth_histogram(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width(), 190.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());
    let painter = ui.painter();

    // Background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 20, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 65)), StrokeKind::Inside);

    let hist = &state.bsp_quality_studio_settings.quality_metrics.depth_histogram;
    let max_count = hist.iter().copied().max().unwrap_or(1).max(1) as f32;

    let bar_count = 64;
    let bar_width = (rect.width() - 10.0) / bar_count as f32;

    for i in 0..bar_count {
        let count = hist[i];
        let h = (count as f32 / max_count) * (rect.height() - 30.0);

        let x0 = rect.min.x + 5.0 + i as f32 * bar_width;
        let x1 = x0 + (bar_width - 1.0).max(1.0);
        let y1 = rect.max.y - 18.0;
        let y0 = y1 - h;

        let color = if i < 25 {
            Color32::from_rgb(46, 204, 113) // Green (optimal)
        } else if i < 40 {
            Color32::from_rgb(241, 196, 15) // Yellow (warning)
        } else {
            Color32::from_rgb(231, 76, 60)  // Red (danger - hom hazard)
        };

        if h > 0.0 {
            painter.rect_filled(Rect::from_min_max(Pos2::new(x0, y0), Pos2::new(x1, y1)), 1.0, color);
        }
    }

    // Baseline axis
    painter.line_segment(
        [Pos2::new(rect.min.x + 5.0, rect.max.y - 18.0), Pos2::new(rect.max.x - 5.0, rect.max.y - 18.0)],
        Stroke::new(1.0_f32, Color32::from_rgb(70, 80, 95)),
    );

    // Labels
    painter.text(
        Pos2::new(rect.min.x + 8.0, rect.max.y - 6.0),
        egui::Align2::LEFT_BOTTOM,
        "Depth 0",
        egui::FontId::monospace(10.0),
        Color32::from_rgb(140, 150, 165),
    );
    painter.text(
        Pos2::new(rect.center().x, rect.max.y - 6.0),
        egui::Align2::CENTER_BOTTOM,
        "Depth 32 (Safe limit < 40)",
        egui::FontId::monospace(10.0),
        Color32::from_rgb(180, 190, 205),
    );
    painter.text(
        Pos2::new(rect.max.x - 8.0, rect.max.y - 6.0),
        egui::Align2::RIGHT_BOTTOM,
        "Depth 63",
        egui::FontId::monospace(10.0),
        Color32::from_rgb(140, 150, 165),
    );

    // Tooltip on hover
    if let Some(mouse_pos) = response.hover_pos() {
        if mouse_pos.x >= rect.min.x + 5.0 && mouse_pos.x <= rect.max.x - 5.0 && mouse_pos.y <= rect.max.y - 18.0 {
            let idx = (((mouse_pos.x - (rect.min.x + 5.0)) / bar_width).floor() as usize).min(63);
            response.on_hover_text(format!("Depth {}: {} nodes", idx, hist[idx]));
        }
    }
}

fn render_metrics_summary(ui: &mut egui::Ui, state: &EditorState) {
    let m = &state.bsp_quality_studio_settings.quality_metrics;

    egui::Frame::canvas(ui.style()).show(ui, |ui| {
        egui::Grid::new("bsp_metrics_summary_grid").num_columns(2).spacing([16.0, 6.0]).show(ui, |ui| {
            ui.label(egui::RichText::new("Input Brush Polys:").strong());
            ui.label(format!("{}", m.total_input_brush_polys));
            ui.end_row();

            ui.label(egui::RichText::new("World BSP Polys:").strong());
            ui.label(format!("{}", m.total_world_bsp_polys));
            ui.end_row();

            ui.label(egui::RichText::new("Cut Ratio:").strong());
            let ratio_color = if m.cut_ratio <= 1.35 {
                Color32::from_rgb(46, 204, 113)
            } else if m.cut_ratio <= 2.0 {
                Color32::from_rgb(241, 196, 15)
            } else {
                Color32::from_rgb(231, 76, 60)
            };
            ui.label(egui::RichText::new(format!("{:.2}x", m.cut_ratio)).color(ratio_color));
            ui.end_row();

            ui.label(egui::RichText::new("Max Tree Depth:").strong());
            let depth_color = if m.max_tree_depth < 25 {
                Color32::from_rgb(46, 204, 113)
            } else if m.max_tree_depth < 40 {
                Color32::from_rgb(241, 196, 15)
            } else {
                Color32::from_rgb(231, 76, 60)
            };
            ui.label(egui::RichText::new(format!("{}", m.max_tree_depth)).color(depth_color));
            ui.end_row();

            ui.label(egui::RichText::new("Average Depth:").strong());
            ui.label(format!("{:.1}", m.average_tree_depth));
            ui.end_row();

            ui.label(egui::RichText::new("Tree Balance %:").strong());
            ui.label(format!("{:.1}%", m.tree_balance_pct));
            ui.end_row();

            ui.label(egui::RichText::new("Total Nodes / Leaves:").strong());
            ui.label(format!("{} nodes / {} leaves", m.total_nodes, m.total_leaves));
            ui.end_row();
        });
    });
}

fn render_cut_offenders_tab(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.label("Brushes identified as inducing the most planar BSP cuts onto other architecture:");
    ui.add_space(4.0);

    let offenders = state.bsp_quality_studio_settings.cut_offenders.clone();
    if offenders.is_empty() {
        ui.colored_label(Color32::from_rgb(46, 204, 113), "✓ No high-cut offenders detected! BSP tree is healthy.");
        return;
    }

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("cut_offenders_table")
            .striped(true)
            .num_columns(5)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Brush").strong());
                ui.label(egui::RichText::new("CSG").strong());
                ui.label(egui::RichText::new("Polys").strong());
                ui.label(egui::RichText::new("Est. Cuts").strong());
                ui.label(egui::RichText::new("Optimization").strong());
                ui.end_row();

                for offender in &offenders {
                    let is_sel = state.bsp_quality_studio_settings.selected_offender_index == Some(offender.brush_index);
                    let label = format!("#{} {}", offender.brush_index, offender.brush_name);
                    if ui.selectable_label(is_sel, label).clicked() {
                        state.bsp_quality_studio_settings.selected_offender_index = Some(offender.brush_index);
                        state.selected_brush_index = Some(offender.brush_index);
                    }

                    ui.label(format!("{:?}", offender.csg_oper));
                    ui.label(format!("{}", offender.poly_count));
                    ui.colored_label(Color32::from_rgb(231, 76, 60), format!("~{}", offender.estimated_cuts_induced));

                    if offender.can_be_semisolid && offender.csg_oper == ut99_bsp::CsgOperation::Add {
                        if ui.button("🛡 Make SemiSolid").on_hover_text("Sets PF_SEMISOLID flag so this brush will not cut other geometry").clicked() {
                            if let Some(b) = state.world_brushes.get_mut(offender.brush_index) {
                                let old_polys = b.polys.clone();
                                for p in &mut b.polys {
                                    p.flags |= ut99_bsp::PF_SEMISOLID;
                                }
                                let new_polys = b.polys.clone();
                                state.history.push(crate::history::Action::BatchConvertSemiSolid {
                                    conversions: vec![(offender.brush_index, old_polys, new_polys)],
                                    description: format!("Converted brush {} to Semi-Solid", offender.brush_name),
                                });
                                state.map_is_dirty = true;
                                state.run_bsp_quality_audit();
                            }
                        }
                    } else {
                        ui.label("-");
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_slivers_tab(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.label("Min Area (uu²):");
        if ui.add(egui::Slider::new(&mut state.bsp_quality_studio_settings.min_sliver_area, 0.5..=10.0)).changed() {
            state.run_bsp_quality_audit();
        }

        ui.label("Max Aspect Ratio:");
        if ui.add(egui::Slider::new(&mut state.bsp_quality_studio_settings.max_aspect_ratio, 10.0..=100.0)).changed() {
            state.run_bsp_quality_audit();
        }
    });
    ui.separator();

    let slivers = state.bsp_quality_studio_settings.detected_slivers.clone();
    if slivers.is_empty() {
        ui.colored_label(Color32::from_rgb(46, 204, 113), "✓ No micro-sliver polygons detected! Geometry is clean.");
        return;
    }

    ui.horizontal(|ui| {
        ui.label(format!("Detected {} sliver polygons.", slivers.len()));
        if ui.button("Purge All Slivers").clicked() {
            state.purge_all_sliver_polygons();
        }
    });

    egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
        egui::Grid::new("slivers_table")
            .striped(true)
            .num_columns(5)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Brush").strong());
                ui.label(egui::RichText::new("Poly Idx").strong());
                ui.label(egui::RichText::new("Area (uu²)").strong());
                ui.label(egui::RichText::new("Aspect").strong());
                ui.label(egui::RichText::new("Action").strong());
                ui.end_row();

                for (idx, sliv) in slivers.iter().enumerate() {
                    let is_sel = state.bsp_quality_studio_settings.selected_sliver_index == Some(idx);
                    let brush_name = state.world_brushes.get(sliv.brush_index).map(|b| b.name.clone()).unwrap_or_else(|| "Brush".to_string());
                    let label = format!("#{} {}", sliv.brush_index, brush_name);
                    if ui.selectable_label(is_sel, label).clicked() {
                        state.bsp_quality_studio_settings.selected_sliver_index = Some(idx);
                        state.selected_brush_index = Some(sliv.brush_index);
                    }

                    ui.label(format!("{}", sliv.poly_index));
                    ui.colored_label(Color32::from_rgb(231, 76, 60), format!("{:.2}", sliv.area));
                    ui.label(format!("{:.1}:1", sliv.aspect_ratio));

                    if ui.button("Purge").clicked() {
                        if let Some(b) = state.world_brushes.get_mut(sliv.brush_index) {
                            if sliv.poly_index < b.polys.len() {
                                let old_polys = b.polys.clone();
                                b.polys.remove(sliv.poly_index);
                                let new_polys = b.polys.clone();
                                state.history.push(crate::history::Action::PurgeSliverPolygons {
                                    modifications: vec![(sliv.brush_index, old_polys, new_polys)],
                                    description: format!("Purged sliver poly {} from brush {}", sliv.poly_index, brush_name),
                                });
                                state.map_is_dirty = true;
                                state.run_bsp_quality_audit();
                            }
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_coplanar_tab(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.label("Coplanar polygons located within 0.1 uu of each other that induce Z-fighting visual flicker:");
    ui.add_space(4.0);

    let coplanars = state.bsp_quality_studio_settings.detected_coplanar.clone();
    if coplanars.is_empty() {
        ui.colored_label(Color32::from_rgb(46, 204, 113), "✓ No coplanar overlaps detected! No Z-fighting flicker.");
        return;
    }

    egui::ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
        egui::Grid::new("coplanar_table")
            .striped(true)
            .num_columns(5)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new("Brush A").strong());
                ui.label(egui::RichText::new("Brush B").strong());
                ui.label(egui::RichText::new("Plane Dist").strong());
                ui.label(egui::RichText::new("Center (X, Y, Z)").strong());
                ui.label(egui::RichText::new("Remedy").strong());
                ui.end_row();

                for (idx, cop) in coplanars.iter().enumerate() {
                    let is_sel = state.bsp_quality_studio_settings.selected_coplanar_index == Some(idx);
                    let name_a = state.world_brushes.get(cop.brush_a_index).map(|b| b.name.clone()).unwrap_or_else(|| "Brush A".to_string());
                    let name_b = state.world_brushes.get(cop.brush_b_index).map(|b| b.name.clone()).unwrap_or_else(|| "Brush B".to_string());
                    let label_a = format!("#{} {}", cop.brush_a_index, name_a);
                    let label_b = format!("#{} {}", cop.brush_b_index, name_b);

                    if ui.selectable_label(is_sel, label_a).clicked() {
                        state.bsp_quality_studio_settings.selected_coplanar_index = Some(idx);
                        state.selected_brush_index = Some(cop.brush_a_index);
                    }

                    ui.label(label_b);
                    ui.colored_label(Color32::from_rgb(241, 196, 15), format!("{:.2} uu", cop.plane.dist));
                    ui.label(format!("({:.0}, {:.0}, {:.0})", cop.approximate_center.x, cop.approximate_center.y, cop.approximate_center.z));
                    ui.label("Nudge 1 uu or SemiSolid");
                    ui.end_row();
                }
            });
    });
}
