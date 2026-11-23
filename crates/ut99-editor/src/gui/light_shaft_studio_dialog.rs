// crates/ut99-editor/src/gui/light_shaft_studio_dialog.rs
//
// Interactive Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{
    calculate_optical_transmission, LightShaftArchetype, LightShaftCrossSection,
};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_light_shaft_dialog {
        return;
    }

    let mut open = state.show_light_shaft_dialog;

    crate::gui::modal::studio_dialog_window(ctx, "Volumetric Light Shaft & Window God Ray Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_studio_content(ui, state);
            });
        });

    state.show_light_shaft_dialog = open;
}

fn render_studio_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // 1. Archetype Presets Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Light Shaft Archetypes:").strong());
        for &arch in LightShaftArchetype::all() {
            let is_sel = state.light_shaft_settings.active_config.archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_light_shaft_archetype(arch);
            }
        }
    });
    ui.separator();

    // 2. Viewport Overlays Ribbon
    ui.horizontal(|ui| {
        ui.checkbox(&mut state.light_shaft_settings.show_beam_ghost_viewport, "Volumetric Frustum");
        ui.checkbox(&mut state.light_shaft_settings.show_core_hotspot_viewport, "Core Hotspot");
        ui.checkbox(&mut state.light_shaft_settings.show_dust_motes_viewport, "Dust Motes (Sparkles)");
        ui.checkbox(&mut state.light_shaft_settings.show_aperture_frame_viewport, "Aperture Frame");
        ui.checkbox(&mut state.light_shaft_settings.show_floor_disc_viewport, "Ground Pool Disc");
        ui.separator();
        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_light_shaft_geometry();
        }
    });
    ui.separator();

    // 3. Two-Column Main Layout
    ui.columns(2, |columns| {
        // Left Column: 2D Attenuation Canvas, Color Swatches & Commit Actions
        columns[0].vertical(|ui| {
            ui.heading("2D Photometric Attenuation & Beam Divergence Profile");
            render_photometric_canvas(ui, state);

            ui.add_space(8.0);
            render_chromatic_swatches(ui, state);

            ui.add_space(8.0);
            ui.heading("Commit Beam to Level");
            render_commit_panel(ui, state);
        });

        // Right Column: Parameters & Hazard Audit
        columns[1].vertical(|ui| {
            ui.heading("Optics & Frustum Parameters");
            render_parameters_grid(ui, state);

            ui.add_space(8.0);
            ui.heading("Photometric Safety & Map Boundary Audit");
            render_safety_audit_card(ui, state);
        });
    });
}

fn render_photometric_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let desired_size = Vec2::new(ui.available_width().max(200.0), 220.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::from_rgb(14, 18, 24));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 55, 70)), StrokeKind::Inside);

    let cfg = &state.light_shaft_settings.active_config;
    let pad = 24.0;
    let plot_w = rect.width() - pad * 2.0;
    let plot_h = rect.height() - pad * 2.0;

    let col_rgb = cfg.color_rgb;
    let beam_col = Color32::from_rgb(
        (col_rgb[0] * 255.0) as u8,
        (col_rgb[1] * 255.0) as u8,
        (col_rgb[2] * 255.0) as u8,
    );

    // Draw beam boundary trapezoid (apex to base)
    let spread_tan = (cfg.divergence_angle_deg.to_radians() * 0.5).tan();
    let spread_offset = cfg.beam_length * spread_tan;
    let apex_half = (cfg.aperture_width * 0.5).max(4.0);
    let base_half = apex_half + spread_offset;

    let max_span = base_half.max(128.0) * 2.2;
    let scale_y = (plot_h * 0.45) / (max_span * 0.5);

    let cy = rect.center().y;
    let x_start = rect.min.x + pad;
    let x_end = rect.max.x - pad;

    let y_apex_top = cy - apex_half * scale_y;
    let y_apex_bot = cy + apex_half * scale_y;
    let y_base_top = cy - base_half * scale_y;
    let y_base_bot = cy + base_half * scale_y;

    // Draw shaded volumetric trapezoid
    let mesh_color = Color32::from_rgba_unmultiplied(
        (col_rgb[0] * 255.0) as u8,
        (col_rgb[1] * 255.0) as u8,
        (col_rgb[2] * 255.0) as u8,
        45,
    );
    let trapezoid = vec![
        Pos2::new(x_start, y_apex_top),
        Pos2::new(x_end, y_base_top),
        Pos2::new(x_end, y_base_bot),
        Pos2::new(x_start, y_apex_bot),
    ];
    painter.add(egui::Shape::convex_polygon(
        trapezoid,
        mesh_color,
        Stroke::new(1.5_f32, beam_col),
    ));

    // Draw optical transmission attenuation curve (Beer-Lambert decay)
    let steps = 40;
    let mut curve_pts = Vec::with_capacity(steps);
    for i in 0..steps {
        let frac = i as f32 / (steps - 1) as f32;
        let x = x_start + frac * plot_w;
        let t = calculate_optical_transmission(cfg, frac);
        // Map transmission to height
        let y = (cy + apex_half * scale_y) - t * (apex_half * 2.0 * scale_y);
        curve_pts.push(Pos2::new(x, y));
    }

    let curve_stroke = Stroke::new(2.5_f32, Color32::from_rgb(255, 230, 100));
    for window in curve_pts.windows(2) {
        painter.line_segment([window[0], window[1]], curve_stroke);
    }

    // Apex and Base Labels
    painter.text(
        Pos2::new(x_start, y_apex_top - 12.0),
        egui::Align2::LEFT_BOTTOM,
        format!("Apex: {:.0} uu", cfg.aperture_width),
        egui::FontId::proportional(11.0),
        Color32::from_rgb(200, 210, 220),
    );

    painter.text(
        Pos2::new(x_end, y_base_top - 12.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("Base: {:.0} uu", base_half * 2.0),
        egui::FontId::proportional(11.0),
        Color32::from_rgb(200, 210, 220),
    );

    response.on_hover_ui(|ui| {
        ui.label(egui::RichText::new("Photometric Profile").strong());
        ui.label(format!("Beam Length: {:.0} uu", cfg.beam_length));
        ui.label(format!("Spread Angle: {:.1}°", cfg.divergence_angle_deg));
        ui.label(format!("Attenuation Factor: {:.2}", cfg.attenuation_factor));
        ui.label(format!("Floor Impact Pool Width: {:.0} uu", base_half * 2.0));
    });
}

fn render_chromatic_swatches(ui: &mut egui::Ui, state: &mut EditorState) {
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Atmospheric Color Presets:").strong());
            let swatches = [
                ("☀ Sun Gold", [1.0, 0.85, 0.45]),
                ("🌙 Moon Azure", [0.65, 0.85, 1.0]),
                ("🧪 Slime Emerald", [0.35, 1.0, 0.45]),
                ("🔥 Lava Amber", [1.0, 0.55, 0.15]),
                ("⚪ Daylight", [0.98, 0.98, 1.0]),
            ];

            for (name, rgb) in swatches {
                if ui.button(name).clicked() {
                    state.light_shaft_settings.active_config.color_rgb = rgb;
                    state.recalculate_light_shaft_geometry();
                }
            }
        });

        ui.horizontal(|ui| {
            ui.label("Custom Tint RGB:");
            let mut rgb = state.light_shaft_settings.active_config.color_rgb;
            if ui.color_edit_button_rgb(&mut rgb).changed() {
                state.light_shaft_settings.active_config.color_rgb = rgb;
                state.recalculate_light_shaft_geometry();
            }
            ui.label(format!("R: {:.2} G: {:.2} B: {:.2}", rgb[0], rgb[1], rgb[2]));
        });
    });
}

fn render_commit_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    let _cfg = &state.light_shaft_settings.active_config;
    let poly_count = state.light_shaft_settings.cached_preview_brush
        .as_ref()
        .map(|b| b.polys.len())
        .unwrap_or(0);

    ui.horizontal(|ui| {
        ui.label(format!("Brush: {} Polygons (PF_TRANSLUCENT | PF_UNLIT | PF_NOT_SOLID)", poly_count));
    });

    ui.horizontal_wrapped(|ui| {
        if ui.button(egui::RichText::new("🔴 Commit to Active Builder Brush").strong())
            .on_hover_text("Assigns the volumetric light shaft mesh to the Red Active Builder Brush for custom editing or CSG operations")
            .clicked()
        {
            state.commit_light_shaft_to_builder_brush();
        }

        if ui.button(egui::RichText::new("✨ Add Non-Solid Translucent World Brush").strong())
            .on_hover_text("Spawns the volumetric light shaft directly into the world as a non-solid atmospheric detail brush (Zero BSP cuts, 100% Undoable)")
            .clicked()
        {
            state.commit_light_shaft_to_world();
        }

        if ui.button("🔄 Recalculate").clicked() {
            state.recalculate_light_shaft_geometry();
        }
    });
}

fn render_parameters_grid(ui: &mut egui::Ui, state: &mut EditorState) {
    let mut changed = false;

    egui::Grid::new("light_shaft_params_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label("Beam Length (uu):");
        if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.beam_length).speed(8.0).range(64.0..=4096.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Aperture Dimensions (W x H):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.aperture_width).speed(4.0).range(8.0..=512.0)).changed() {
                changed = true;
            }
            ui.label("x");
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.aperture_height).speed(4.0).range(8.0..=512.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        ui.label("Divergence Spread Angle (°):");
        if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.divergence_angle_deg).speed(0.5).range(2.0..=60.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Cross-Section Frustum:");
        egui::ComboBox::from_id_salt("light_shaft_cross_section_combo")
            .selected_text(state.light_shaft_settings.active_config.cross_section.display_name())
            .show_ui(ui, |ui| {
                for &cs in LightShaftCrossSection::all() {
                    if ui.selectable_value(&mut state.light_shaft_settings.active_config.cross_section, cs, cs.display_name()).clicked() {
                        changed = true;
                    }
                }
            });
        ui.end_row();

        if state.light_shaft_settings.active_config.cross_section == LightShaftCrossSection::ParallelSlats {
            ui.label("Louver Slat Count:");
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.slat_count).speed(1).range(2..=16)).changed() {
                changed = true;
            }
            ui.end_row();
        }

        ui.label("Dual-Layer Core Hotspot:");
        if ui.checkbox(&mut state.light_shaft_settings.active_config.has_core_hotspot, "Concentrated Inner Core Beam").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Ground Impact Pool Disc:");
        if ui.checkbox(&mut state.light_shaft_settings.active_config.has_floor_light_disc, "Generate Floor Light Pool").changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Orientation Angles (P / Y / R):");
        ui.horizontal(|ui| {
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.pitch_deg).speed(1.0).range(-90.0..=90.0)).changed() {
                changed = true;
            }
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.yaw_deg).speed(1.0).range(-180.0..=180.0)).changed() {
                changed = true;
            }
            if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.roll_deg).speed(1.0).range(-180.0..=180.0)).changed() {
                changed = true;
            }
        });
        ui.end_row();

        ui.label("Optical Attenuation (μ):");
        if ui.add(egui::DragValue::new(&mut state.light_shaft_settings.active_config.attenuation_factor).speed(0.05).range(0.1..=3.0)).changed() {
            changed = true;
        }
        ui.end_row();

        ui.label("Texture Name:");
        if ui.text_edit_singleline(&mut state.light_shaft_settings.active_config.texture_name).changed() {
            changed = true;
        }
        ui.end_row();
    });

    if changed {
        state.recalculate_light_shaft_geometry();
    }
}

fn render_safety_audit_card(ui: &mut egui::Ui, state: &EditorState) {
    let hazards = &state.light_shaft_settings.cached_hazards;
    let critical_count = hazards.iter().filter(|h| h.is_critical).count();

    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            if critical_count > 0 {
                ui.colored_label(Color32::from_rgb(255, 60, 60), egui::RichText::new(format!("⛔ {} OPTICAL ISSUE(S)", critical_count)).strong());
            } else if !hazards.is_empty() {
                ui.colored_label(Color32::from_rgb(240, 180, 50), egui::RichText::new("⚠ Beam Warnings").strong());
            } else {
                ui.colored_label(Color32::from_rgb(60, 220, 100), egui::RichText::new("✓ 100% Boundary & Overdraw Safe").strong());
            }
        });

        if hazards.is_empty() {
            ui.label("Beam length and divergence are within authentic architectural limits.");
        } else {
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                for hazard in hazards {
                    ui.horizontal(|ui| {
                        let badge_col = if hazard.is_critical {
                            Color32::from_rgb(255, 60, 60)
                        } else {
                            Color32::from_rgb(240, 180, 50)
                        };
                        let badge_txt = if hazard.is_critical { "CRITICAL" } else { "WARN" };
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
