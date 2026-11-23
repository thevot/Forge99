// crates/ut99-editor/src/gui/destructible_studio_dialog.rs
//
// Interactive Destructible Architecture, Breakable Glass & Fracture Studio
// Provides analytical polygon fracture generation, real-time debris blast physics simulation,
// dynamic bot reachability pairing, and safety audit for Unreal Tournament 99 destructible elements.

use egui::{Color32, Pos2, Stroke, StrokeKind, Vec2};
use crate::editor_state::EditorState;
use ut99_bsp::{
    DestructibleArchetype, DestructibleHazardSeverity, FracturePatternType,
};

pub fn show(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_destructible_studio_dialog {
        return;
    }

    // Step physics simulation with delta time
    let dt = ctx.input(|i| i.stable_dt).min(0.05);
    state.step_destructible_simulation(dt);

    let mut open = state.show_destructible_studio_dialog;
    crate::gui::modal::studio_dialog_window(ctx, "Destructible Architecture & Fracture Studio", &mut open)
        .show(ctx, |ui| {
            crate::gui::modal::dialog_scroll_body(ui, |ui| {
                render_content(ui, state);
            });
        });
    state.show_destructible_studio_dialog = open;
}

fn render_content(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        ui.heading("💥 Destructible Architecture & Fracture Studio");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("🔄 Audit Hazards").clicked() {
                state.audit_destructible_hazards();
            }
        });
    });
    ui.separator();

    // Archetype Selector Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Archetype:").strong());
        for arch in [
            DestructibleArchetype::BreakableWindowGlass,
            DestructibleArchetype::ExplodingStoneWall,
            DestructibleArchetype::ReinforcedBlastDoor,
            DestructibleArchetype::FragileSkylightDome,
            DestructibleArchetype::VentGratingShatter,
        ] {
            let is_sel = state.destructible_studio_settings.selected_archetype == arch;
            if ui.selectable_label(is_sel, arch.display_name()).clicked() {
                state.apply_destructible_archetype(arch);
            }
        }
    });

    // Pattern Selector Ribbon
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new("Fracture Pattern:").strong());
        let current_pat = state.destructible_studio_settings.selected_pattern;
        for pat in [
            FracturePatternType::RadialSpiderweb,
            FracturePatternType::CellularVoronoi,
            FracturePatternType::RectangularPanes,
            FracturePatternType::DiagonalShatter,
        ] {
            let is_sel = current_pat == pat;
            if ui.selectable_label(is_sel, pat.display_name()).clicked() {
                state.destructible_studio_settings.selected_pattern = pat;
                state.destructible_studio_settings.active_config.fracture_pattern = pat;
                state.regenerate_destructible_shards();
            }
        }
    });

    ui.separator();

    // Main Center Section: 2D Canvas & Parameter Tuning
    ui.horizontal(|ui| {
        // Left Column: 2D Shard Fracture Canvas & Blast Simulator
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("🔬 2D Shard Geometry & Real-time Blast Simulator").strong());
            render_fracture_canvas(ui, state);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("💥 Shatter Blast Test").strong().color(Color32::from_rgb(255, 100, 100))).clicked() {
                    state.trigger_destructible_shatter();
                }
                if ui.button("🔄 Reset Fracture").clicked() {
                    state.destructible_studio_settings.simulation.reset();
                }
                ui.label(format!("Shards: {}", state.destructible_studio_settings.cached_shards.len()));
            });
        });

        ui.separator();

        // Right Column: Destructible Physical Parameters & Textures
        ui.vertical(|ui| {
            ui.set_min_width(340.0);
            render_parameters_panel(ui, state);
        });
    });

    ui.separator();

    // Bottom Section: Hazard Audit & Generation Button
    render_bottom_section(ui, state);
}

fn render_fracture_canvas(ui: &mut egui::Ui, state: &EditorState) {
    let (response, painter) = ui.allocate_painter(Vec2::new(460.0, 260.0), egui::Sense::hover());
    let rect = response.rect;

    // Dark canvas background
    painter.rect_filled(rect, 4.0, Color32::from_rgb(16, 20, 28));
    painter.rect_stroke(rect, 4.0, Stroke::new(1.0_f32, Color32::from_rgb(45, 60, 85)), StrokeKind::Inside);

    let center = rect.center();
    let size = state.destructible_studio_settings.active_config.size;
    let scale_x = (rect.width() * 0.75) / size.x.max(1.0);
    let scale_y = (rect.height() * 0.75) / size.y.max(1.0);
    let scale = scale_x.min(scale_y);

    let to_canvas = |pt: glam::Vec2| -> Pos2 {
        Pos2::new(
            center.x + pt.x * scale,
            center.y - pt.y * scale, // invert Y for screen
        )
    };

    if !state.destructible_studio_settings.simulation.is_shattered {
        // Render intact shards
        let shard_col = Color32::from_rgb(80, 200, 240);

        for shard in &state.destructible_studio_settings.cached_shards {
            let pts: Vec<Pos2> = shard.local_vertices.iter().map(|&v| to_canvas(v)).collect();
            if pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(pts, Color32::from_rgba_unmultiplied(40, 100, 140, 60), Stroke::new(1.0_f32, shard_col)));
            }
        }

        // Outer boundary rectangle
        let hw = size.x * 0.5;
        let hh = size.y * 0.5;
        let p0 = to_canvas(glam::Vec2::new(-hw, -hh));
        let p1 = to_canvas(glam::Vec2::new(hw, -hh));
        let p2 = to_canvas(glam::Vec2::new(hw, hh));
        let p3 = to_canvas(glam::Vec2::new(-hw, hh));

        painter.line_segment([p0, p1], Stroke::new(1.5_f32, Color32::from_rgb(255, 200, 80)));
        painter.line_segment([p1, p2], Stroke::new(1.5_f32, Color32::from_rgb(255, 200, 80)));
        painter.line_segment([p2, p3], Stroke::new(1.5_f32, Color32::from_rgb(255, 200, 80)));
        painter.line_segment([p3, p0], Stroke::new(1.5_f32, Color32::from_rgb(255, 200, 80)));
    } else {
        // Render animated flying particles projected to canvas XY
        let sim = &state.destructible_studio_settings.simulation;
        let p_col = Color32::from_rgb(255, 180, 60);
        let trail_col = Color32::from_rgb(180, 80, 40);

        for p in &sim.particles {
            let rel_x = p.position.x - state.destructible_studio_settings.active_config.center.x;
            let rel_z = p.position.z - state.destructible_studio_settings.active_config.center.z;
            let scr_pos = Pos2::new(center.x + rel_x * scale * 0.7, center.y - rel_z * scale * 0.7);

            let r = (p.scale * 3.5).clamp(2.0, 10.0);
            painter.circle_filled(scr_pos, r, p_col);

            // Velocity vector trail
            let trail_end = Pos2::new(scr_pos.x - p.velocity.x * 0.04, scr_pos.y + p.velocity.z * 0.04);
            painter.line_segment([scr_pos, trail_end], Stroke::new(1.0_f32, trail_col));
        }

        // Floor line
        let floor_rel_z = sim.floor_z - state.destructible_studio_settings.active_config.center.z;
        let floor_y = center.y - floor_rel_z * scale * 0.7;
        painter.line_segment(
            [Pos2::new(rect.min.x, floor_y), Pos2::new(rect.max.x, floor_y)],
            Stroke::new(1.5_f32, Color32::from_rgb(80, 120, 180)),
        );
        painter.text(
            Pos2::new(rect.min.x + 8.0, floor_y - 12.0),
            egui::Align2::LEFT_BOTTOM,
            "Floor Collision Plane",
            egui::FontId::proportional(10.0),
            Color32::from_rgb(100, 150, 220),
        );
    }
}

fn render_parameters_panel(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.label(egui::RichText::new("⚙ Physical & Gameplay Parameters").strong());

    let mut changed = false;
    let cfg = &mut state.destructible_studio_settings.active_config;

    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut cfg.name);
    });

    ui.horizontal(|ui| {
        ui.label("Health (HP):");
        if ui.add(egui::DragValue::new(&mut cfg.health).range(1.0..=2000.0).speed(5.0)).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Blast Force (uu/s):");
        if ui.add(egui::DragValue::new(&mut cfg.blast_force).range(50.0..=1500.0).speed(10.0)).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Shard Count:");
        if ui.add(egui::DragValue::new(&mut cfg.shard_count).range(4..=64).speed(1.0)).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Particle Scale:");
        ui.add(egui::DragValue::new(&mut cfg.particle_scale).range(0.2..=5.0).speed(0.1));
    });

    ui.horizontal(|ui| {
        ui.label("Width x Height (uu):");
        if ui.add(egui::DragValue::new(&mut cfg.size.x).range(16.0..=1024.0).speed(8.0)).changed() {
            changed = true;
        }
        ui.label("x");
        if ui.add(egui::DragValue::new(&mut cfg.size.y).range(16.0..=1024.0).speed(8.0)).changed() {
            changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Event On Break:");
        ui.text_edit_singleline(&mut cfg.event_on_break);
    });

    ui.horizontal(|ui| {
        ui.label("Shatter Audio:");
        ui.text_edit_singleline(&mut cfg.sound_shatter);
    });

    ui.horizontal(|ui| {
        ui.label("Intact Texture:");
        ui.text_edit_singleline(&mut cfg.texture_intact);
    });

    ui.checkbox(&mut cfg.auto_wire_bot_reachability, "Auto-wire Dynamic Bot ReachSpec (R_SPECIAL)");

    if changed {
        state.regenerate_destructible_shards();
        state.audit_destructible_hazards();
    }
}

fn render_bottom_section(ui: &mut egui::Ui, state: &mut EditorState) {
    ui.horizontal(|ui| {
        // Generation Action Button
        if ui.button(egui::RichText::new("🔨 Generate Destructible Barrier & Pathing in Map").strong()).clicked() {
            state.generate_destructible_barrier();
        }

        ui.separator();

        // Viewport visualizer toggles
        ui.checkbox(&mut state.destructible_studio_settings.show_fracture_pattern_viewport, "Fracture Lines (3D)");
        ui.checkbox(&mut state.destructible_studio_settings.show_debris_cone_viewport, "Debris Blast Cone (3D)");
        ui.checkbox(&mut state.destructible_studio_settings.show_bot_reachability_viewport, "Bot ReachSpec Gate (3D)");
    });

    ui.separator();

    // Hazard Audit List
    ui.label(egui::RichText::new("📋 Destructible Hazard Audit:").strong());
    egui::ScrollArea::vertical()
        .max_height(140.0)
        .show(ui, |ui| {
            for h in &state.destructible_studio_settings.cached_hazards {
                let (icon, color) = match h.severity {
                    DestructibleHazardSeverity::Critical => ("❌", Color32::from_rgb(255, 80, 80)),
                    DestructibleHazardSeverity::Warning => ("⚠", Color32::from_rgb(255, 180, 40)),
                    DestructibleHazardSeverity::Info => ("✅", Color32::from_rgb(80, 220, 100)),
                };

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).color(color));
                    ui.label(egui::RichText::new(&h.title).strong().color(color));
                    ui.label(egui::RichText::new(&h.description).small());
                });
            }
        });
}
