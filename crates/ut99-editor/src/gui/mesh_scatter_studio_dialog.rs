// crates/ut99-editor/src/gui/mesh_scatter_studio_dialog.rs
//
// Interactive Surface Clutter, Procedural Foliage & Decoration Scatter Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{ScatterAlignmentMode, ScatterArchetype};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_scatter_studio_dialog {
        return;
    }

    let mut open = state.show_scatter_studio_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Surface Clutter, Foliage & Decoration Scatter Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_scatter_studio_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Scatter Archetypes:").strong());
        for &arch in ScatterArchetype::all() {
            let is_sel = state.scatter_studio_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_scatter_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.scatter_studio_settings.show_instances_viewport, "Instance Wireframes");
        ui.checkbox(&mut state.scatter_studio_settings.show_exclusion_discs_viewport, "Poisson Discs (Cyan)");
        ui.checkbox(&mut state.scatter_studio_settings.show_surface_normals_viewport, "Surface Normals");
        ui.checkbox(&mut state.scatter_studio_settings.show_bot_exclusion_zones_viewport, "Bot Node Buffers");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_scatter_preview();
        }
        if ui.button("🎲 Re-Roll Seed").clicked() {
            state.reroll_scatter_seed();
        }
        if ui.button("🧹 Clear").clicked() {
            state.clear_scatter_preview();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Distribution Canvas, Metrics & Commit Actions
        columns[0].vertical(|ui| {
            ui.heading("2D Scatter Distribution & Poisson-Disc Canvas");
            render_distribution_canvas(ui, state);

            ui.add_space(8.0);
            render_metrics_card(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Clutter to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Procedural Scattering Parameters");
            render_parameters_grid(ui, state);

            ui.add_space(8.0);
            ui.heading("Bot Navigation & Performance Safety Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_distribution_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width().max(200.0), 240.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(18, 22, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)), StrokeKind::Inside);

    let instances = &state.scatter_studio_settings.preview_instances;
    let cfg = &state.scatter_studio_settings.active_config;

    if instances.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No instances generated.\nClick 'Recalculate' or adjust slope angle filter.",
            egui::FontId::proportional(12.0),
            Color32::from_rgb(140, 150, 165),
        );
        return;
    }

    // Determine 2D bounding box (X and Y coordinates)
    let mut min_x = f32::MAX;
    let mut max_x = f32::MIN;
    let mut min_y = f32::MAX;
    let mut max_y = f32::MIN;

    for inst in instances {
        min_x = min_x.min(inst.position.x);
        max_x = max_x.max(inst.position.x);
        min_y = min_y.min(inst.position.y);
        max_y = max_y.max(inst.position.y);
    }

    let span_x = (max_x - min_x).max(64.0);
    let span_y = (max_y - min_y).max(64.0);

    let pad = 20.0;
    let draw_w = (rect.width() - pad * 2.0).max(10.0);
    let draw_h = (rect.height() - pad * 2.0).max(10.0);

    let scale_x = draw_w / span_x;
    let scale_y = draw_h / span_y;
    let scale = scale_x.min(scale_y);

    let to_canvas = |pos: Pos2| -> Pos2 {
        let cx = rect.min.x + pad + (pos.x - min_x) * scale;
        let cy = rect.max.y - pad - (pos.y - min_y) * scale;
        Pos2::new(cx, cy)
    };

    // Draw Poisson-disc exclusion radii (translucent cyan circles)
    let disc_radius_px = (cfg.min_spacing_radius * 0.5 * scale).max(2.0);
    for inst in instances {
        let center = to_canvas(Pos2::new(inst.position.x, inst.position.y));
        painter.circle_stroke(
            center,
            disc_radius_px,
            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(60, 200, 255, 60)),
        );
    }

    // Draw instance position markers (emerald for valid clearance, red for blocked)
    for inst in instances {
        let center = to_canvas(Pos2::new(inst.position.x, inst.position.y));
        let col = if inst.is_valid_clearance {
            Color32::from_rgb(60, 230, 110)
        } else {
            Color32::from_rgb(255, 70, 70)
        };
        painter.circle_filled(center, 3.5, col);
    }

    // Tooltip
    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("2D Scatter Distribution Canvas").strong());
        ui.label(format!("Instances: {}", instances.len()));
        ui.label(format!("Min Spacing: {:.0} uu", cfg.min_spacing_radius));
        ui.label(format!("Bounding Area: {:.0} x {:.0} uu", span_x, span_y));
    });
}

fn render_metrics_card(ui: &mut egui::Ui, state: &EditorState) {
    let metrics = &state.scatter_studio_settings.cached_metrics;

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Distribution Scorecard:").strong());
            if metrics.blocked_clearance_count == 0 && metrics.total_instances > 0 {
                ui.colored_label(Color32::from_rgb(60, 220, 100), "✓ 100% Bot Clear");
            } else if metrics.blocked_clearance_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 90, 90), format!("⛔ {} Blocked", metrics.blocked_clearance_count));
            } else {
                ui.colored_label(Color32::from_rgb(160, 170, 180), "Empty Preview");
            }
        });

        ui.horizontal(|ui| {
            ui.label(format!("Total: {}", metrics.total_instances));
            ui.separator();
            ui.label(format!("Valid: {}", metrics.valid_clearance_count));
            ui.separator();
            ui.label(format!("Avg Spacing: {:.0} uu", metrics.average_spacing));
            ui.separator();
            ui.label(format!("Surface Area: {:.0} uu²", metrics.sampled_surface_area));
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let cfg = &state.scatter_studio_settings.active_config;
    let instances = &state.scatter_studio_settings.preview_instances;
    let valid_count = instances.iter().filter(|i| i.is_valid_clearance).count();

    ui.horizontal(|ui| {
        ui.label(format!("Target Actor: {} ({})", cfg.class_name, cfg.mesh_name));
    });

    ui.horizontal_wrapped(|ui| {
        let commit_text = format!("✨ Commit {} Instances to World", valid_count);
        if ui.button(egui::RichText::new(commit_text).strong())
            .on_hover_text("Spawns all valid clutter instances as level actors in group 'ScatteredClutter' (100% Undoable)")
            .clicked()
        {
            state.commit_scatter_instances_to_world();
        }

        if ui.button("🎲 Re-Roll Seed").clicked() {
            state.reroll_scatter_seed();
        }

        if ui.button("🧹 Clear Preview").clicked() {
            state.clear_scatter_preview();
        }
    });
}

fn render_parameters_grid(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("scatter_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Actor Class Name:");
        if ui.text_edit_singleline(&mut state.scatter_studio_settings.active_config.class_name).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Static Mesh / Asset:");
        if ui.text_edit_singleline(&mut state.scatter_studio_settings.active_config.mesh_name).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Target Instance Count:");
        if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.target_count).speed(1).range(1..=200)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Min Spacing Radius (uu):");
        if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.min_spacing_radius).speed(2.0).range(16.0..=512.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Scale Range (Min - Max):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.scale_min).speed(0.05).range(0.1..=5.0)).changed() {
                changed = true;
            }
            ui.label("-");
            if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.scale_max).speed(0.05).range(0.1..=5.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        ui.label("Random Yaw Rotation:");
        if ui.checkbox(&mut state.scatter_studio_settings.active_config.random_yaw, "0°..360° Jitter").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Max Surface Tilt Angle (°):");
        if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.max_surface_tilt_deg).speed(1.0).range(0.0..=45.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Embed Normal Offset (uu):");
        if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.embed_offset_z).speed(0.5).range(-32.0..=32.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Slope Filter Range (°):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.slope_min_deg).speed(1.0).range(0.0..=180.0)).changed() {
                changed = true;
            }
            ui.label("to");
            if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.slope_max_deg).speed(1.0).range(0.0..=180.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        ui.label("Alignment Mode:");
        egui::ComboBox::from_id_salt("scatter_alignment_combo")
            .selected_text(state.scatter_studio_settings.active_config.alignment_mode.display_name())
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut state.scatter_studio_settings.active_config.alignment_mode, ScatterAlignmentMode::WorldZWithSurfaceTilt, "Upright (World Z + Tilt)").clicked() {
                    changed = true;
                }
                if ui.selectable_value(&mut state.scatter_studio_settings.active_config.alignment_mode, ScatterAlignmentMode::AlignToSurfaceNormal, "Align to Surface Normal").clicked() {
                    changed = true;
                }
            });
        ui.end_row();

        ui.label("Bot Clearance Buffer (uu):");
        if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.bot_clearance_radius).speed(2.0).range(16.0..=128.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Random Seed:");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.scatter_studio_settings.active_config.random_seed).speed(10)).changed() {
                changed = true;
            }
            if ui.button("🎲").clicked() {
                state.reroll_scatter_seed();
            }
        });
        ui.end_row();
    });

    if changed {
        state.recalculate_scatter_preview();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.scatter_studio_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} BOT HAZARD(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Clutter Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Navigation Safe").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("No bot path collisions or performance bottlenecks detected.");
        } else {
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                for hazard in hazards {
                    ui.horizontal(|ui| {
                        let badge_col = if hazard.is_critical {
                            Color32::from_rgb(255, 60, 60)
                        } else {
                            Color32::from_rgb(240, 180, 50)
                        };
                        let badge_txt = if hazard.is_critical { "BLOCKED" } else { "WARN" };
                        ui.colored_label(badge_col, format!("[{}]", badge_txt));
                        ui.label(egui::RichText::new(&hazard.title).strong());
                    });
                    ui.label(&hazard.description);
                    ui.colored_label(Color32::from_rgb(140, 200, 255), format!("💡 {}", hazard.recommendation));
                    ui.separator();
                }
            });
        }
    });
}
