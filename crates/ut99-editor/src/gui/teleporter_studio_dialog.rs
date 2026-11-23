use egui::{pos2, vec2, Color32, RichText, Stroke, StrokeKind, Ui, Window};
use ut99_bsp::{TeleportDiagnosticCategory, TeleportDiagnosticSeverity};
use crate::editor_state::EditorState;

pub fn render_teleporter_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_teleporter_studio_dialog {
        return;
    }
    let mut is_open = state.show_teleporter_studio_dialog;
    Window::new("Teleporter Destination & Network Link Studio")
        .open(&mut is_open)
        .default_width(900.0)
        .default_height(660.0)
        .min_width(740.0)
        .min_height(520.0)
        .resizable(true)
        .show(ctx, |ui| {
            show(ui, state);
        });
    state.show_teleporter_studio_dialog = is_open;
}

pub fn show(ui: &mut Ui, state: &mut EditorState) {
    ui.heading("🌀 Teleporter Destination & Network Link Studio");
    ui.label(
        RichText::new(
            "Analyze teleporter topology, audit broken links and telefrag hazards, and generate paired exit trajectories.",
        )
        .size(11.0)
        .color(Color32::from_rgb(160, 160, 175)),
    );
    ui.separator();

    let graph = state.collect_teleporter_network();

    // 1. Diagnostics Alert Bar
    let broken_count = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::BrokenUrl || d.category == TeleportDiagnosticCategory::InfiniteLoopHazard)
        .count();
    let warning_count = graph
        .diagnostics
        .iter()
        .filter(|d| d.category == TeleportDiagnosticCategory::TelefragHazard || d.category == TeleportDiagnosticCategory::OrphanDestination)
        .count();

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Network Status: {} Nodes, {} Active Links", graph.nodes.len(), graph.links.len())).strong());

        if broken_count > 0 {
            ui.label(
                RichText::new(format!("⛔ {} Critical Errors", broken_count))
                    .color(Color32::from_rgb(255, 80, 80))
                    .strong(),
            );
        } else if warning_count > 0 {
            ui.label(
                RichText::new(format!("⚠ {} Warnings", warning_count))
                    .color(Color32::from_rgb(255, 180, 50))
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("✅ 100% Healthy Network")
                    .color(Color32::from_rgb(80, 240, 120))
                    .strong(),
            );
        }
    });

    ui.add_space(4.0);

    // 2. Interactive 2D Network Topology Canvas
    let canvas_size = vec2(ui.available_width(), 220.0);
    let (response, painter) = ui.allocate_painter(canvas_size, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Dark grid background
    painter.rect_filled(rect, 6.0, Color32::from_rgb(18, 20, 28));
    painter.rect_stroke(
        rect,
        6.0,
        Stroke::new(1.0_f32, Color32::from_rgb(45, 50, 70)),
        StrokeKind::Inside,
    );

    // Coordinate mapping helper
    let map_to_canvas = |loc: glam::Vec3| -> egui::Pos2 {
        if graph.nodes.is_empty() {
            return pos2(rect.center().x, rect.center().y);
        }

        let mut min_x = f32::MAX;
        let mut max_x = f32::MIN;
        let mut min_y = f32::MAX;
        let mut max_y = f32::MIN;

        for n in &graph.nodes {
            min_x = min_x.min(n.location.x);
            max_x = max_x.max(n.location.x);
            min_y = min_y.min(n.location.y);
            max_y = max_y.max(n.location.y);
        }

        let span_x = (max_x - min_x).max(800.0);
        let span_y = (max_y - min_y).max(800.0);

        let margin = 35.0;
        let norm_x = (loc.x - min_x) / span_x;
        let norm_y = (loc.y - min_y) / span_y;

        let px = rect.min.x + margin + norm_x * (rect.width() - margin * 2.0);
        let py = rect.min.y + margin + norm_y * (rect.height() - margin * 2.0);
        pos2(px, py)
    };

    // Draw Links
    let time = ui.input(|i| i.time) as f32;
    for link in &graph.links {
        let src_node = graph.nodes.iter().find(|n| n.id == link.source_id);
        let dst_node = graph.nodes.iter().find(|n| n.id == link.dest_id);

        if let (Some(src), Some(dst)) = (src_node, dst_node) {
            let p1 = map_to_canvas(src.location);
            let p2 = map_to_canvas(dst.location);

            let link_color = if link.is_bidirectional {
                Color32::from_rgb(0, 220, 255)
            } else {
                Color32::from_rgb(180, 100, 255)
            };

            painter.line_segment([p1, p2], Stroke::new(1.5_f32, link_color));

            // Animated directional pulse dot
            let pulse_t = (time * 1.5) % 1.0;
            let dot_pos = pos2(
                p1.x + (p2.x - p1.x) * pulse_t,
                p1.y + (p2.y - p1.y) * pulse_t,
            );
            painter.circle_filled(dot_pos, 3.5, Color32::from_rgb(255, 255, 120));
        }
    }

    // Draw Nodes
    let mut clicked_node = None;
    for node in &graph.nodes {
        let p = map_to_canvas(node.location);

        let has_error = graph.diagnostics.iter().any(|d| d.node_id == node.id && d.severity == TeleportDiagnosticSeverity::Error);
        let has_warning = graph.diagnostics.iter().any(|d| d.node_id == node.id && d.severity == TeleportDiagnosticSeverity::Warning);

        let (node_color, ring_color) = if has_error {
            (Color32::from_rgb(255, 50, 50), Color32::from_rgb(255, 150, 150))
        } else if has_warning {
            (Color32::from_rgb(255, 170, 30), Color32::from_rgb(255, 220, 100))
        } else {
            (Color32::from_rgb(40, 200, 100), Color32::from_rgb(100, 255, 180))
        };

        let is_selected = state.teleporter_studio_settings.selected_node_id == Some(node.id);
        let radius = if is_selected { 9.0 } else { 6.5 };

        if is_selected {
            painter.circle_filled(p, radius + 4.0, Color32::from_rgba_premultiplied(255, 215, 0, 80));
        }

        painter.circle_filled(p, radius, node_color);
        painter.circle_stroke(p, radius, Stroke::new(1.5_f32, ring_color));

        // Node label
        let display_label = if node.tag.is_empty() { &node.name } else { &node.tag };
        painter.text(
            pos2(p.x, p.y + radius + 3.0),
            egui::Align2::CENTER_TOP,
            display_label,
            egui::FontId::proportional(10.0),
            Color32::from_rgb(220, 220, 230),
        );

        // Click detection
        if response.clicked() {
            if let Some(hover_pos) = response.hover_pos() {
                if (hover_pos - p).length() < 14.0 {
                    clicked_node = Some(node.id);
                }
            }
        }
    }

    if let Some(id) = clicked_node {
        state.teleporter_studio_settings.selected_node_id = Some(id);
    }

    ui.add_space(6.0);

    // 3. Middle Control Panels: Inspector & Pair Generator
    ui.columns(2, |cols| {
        // Left Column: Selected Node Inspector
        cols[0].group(|ui| {
            ui.label(RichText::new("⚙ Selected Teleporter Node").strong().color(Color32::from_rgb(120, 210, 255)));

            let sel_id = state.teleporter_studio_settings.selected_node_id;
            let current_node = sel_id.and_then(|id| graph.nodes.iter().find(|n| n.id == id).cloned());

            if let Some(node) = current_node {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Actor: {} (ID #{})", node.name, node.id)).size(11.0));
                    if ui.button("📍 Focus Viewport 3D").clicked() {
                        state.builder_brush.location = node.location;
                        state.selected_actor_index = Some(node.id);
                        state.log(format!("Focused 3D camera on Teleporter '{}'.", node.name));
                    }
                });

                let mut tag = node.tag.clone();
                let mut url = node.url.clone();
                let mut b_changes_vel = node.b_changes_velocity;
                let mut target_vel = node.target_velocity;
                let mut b_changes_yaw = node.b_changes_yaw;

                ui.horizontal(|ui| {
                    ui.label("Tag (Receive):");
                    ui.text_edit_singleline(&mut tag);
                });

                ui.horizontal(|ui| {
                    ui.label("URL (Destination):");
                    ui.text_edit_singleline(&mut url);
                });

                ui.horizontal(|ui| {
                    ui.checkbox(&mut b_changes_vel, "bChangesVelocity");
                    ui.checkbox(&mut b_changes_yaw, "bChangesYaw");
                });

                if b_changes_vel {
                    ui.horizontal(|ui| {
                        ui.label("Target Vel (X,Y,Z):");
                        ui.add(egui::DragValue::new(&mut target_vel.x).speed(10.0).prefix("X: "));
                        ui.add(egui::DragValue::new(&mut target_vel.y).speed(10.0).prefix("Y: "));
                        ui.add(egui::DragValue::new(&mut target_vel.z).speed(10.0).prefix("Z: "));
                    });
                }

                if ui.button("💾 Apply & Save Node Changes").clicked() {
                    state.update_teleporter_properties(
                        node.id,
                        &tag,
                        &url,
                        b_changes_vel,
                        target_vel,
                        b_changes_yaw,
                    );
                }
            } else {
                ui.label(RichText::new("Select a node on the canvas or from the table below.").italics().color(Color32::GRAY));
            }
        });

        // Right Column: Automated Link Pair Generator
        cols[1].group(|ui| {
            ui.label(RichText::new("🔗 1-Click Coordinated Link Generator").strong().color(Color32::from_rgb(255, 200, 80)));
            ui.label(
                RichText::new("Generates a perfectly paired bidirectional entrance & exit with safe momentum vectors.")
                    .size(11.0)
                    .color(Color32::from_rgb(160, 160, 175)),
            );

            let mut prefix = state.teleporter_studio_settings.auto_pair_prefix.clone();
            let mut offset = state.teleporter_studio_settings.auto_pair_offset;

            ui.horizontal(|ui| {
                ui.label("Tag Prefix:");
                ui.text_edit_singleline(&mut prefix);
            });

            ui.horizontal(|ui| {
                ui.label("Distance Offset (uu):");
                ui.add(egui::Slider::new(&mut offset, 300.0..=5000.0).step_by(50.0));
            });

            state.teleporter_studio_settings.auto_pair_prefix = prefix;
            state.teleporter_studio_settings.auto_pair_offset = offset;

            if ui.button(RichText::new("✨ Spawn Paired Teleporter Link at Pivot").strong()).clicked() {
                let (id_a, id_b) = state.create_paired_teleporters_at_pivot(offset);
                state.teleporter_studio_settings.selected_node_id = Some(id_a);
                state.log(format!("Spawned teleporters #{} and #{}.", id_a, id_b));
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut state.teleporter_studio_settings.show_portal_discs_viewport, "Portal Discs (WGPU)");
                ui.checkbox(&mut state.teleporter_studio_settings.show_network_lines_viewport, "Trajectory Lines");
                ui.checkbox(&mut state.teleporter_studio_settings.show_exit_vectors_viewport, "Exit Vectors");
            });
        });
    });

    ui.add_space(4.0);

    // 4. Diagnostics & Auditing Table
    ui.label(RichText::new("📋 Network Audit & Diagnostics").strong());
    egui::ScrollArea::vertical()
        .max_height(140.0)
        .show(ui, |ui| {
            if graph.diagnostics.is_empty() {
                ui.label(RichText::new("No issues detected. All teleporter links are valid and free of hazards.").color(Color32::from_rgb(100, 220, 120)));
            } else {
                for diag in &graph.diagnostics {
                    let (badge, color) = match diag.severity {
                        TeleportDiagnosticSeverity::Error => ("⛔ ERROR", Color32::from_rgb(255, 80, 80)),
                        TeleportDiagnosticSeverity::Warning => ("⚠ WARN", Color32::from_rgb(255, 180, 40)),
                        TeleportDiagnosticSeverity::Info => ("ℹ INFO", Color32::from_rgb(100, 180, 255)),
                    };

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(badge).color(color).strong());
                        ui.label(RichText::new(&diag.message).size(11.0));
                        if ui.button("Select Node").clicked() {
                            state.teleporter_studio_settings.selected_node_id = Some(diag.node_id);
                            if let Some(n) = graph.nodes.iter().find(|n| n.id == diag.node_id) {
                                state.builder_brush.location = n.location;
                                state.selected_actor_index = Some(n.id);
                            }
                        }
                    });
                }
            }
        });
}
