use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use ut99_bsp::{CollisionHullType, PolygonBudgetRating};

/// Interactive Static Mesh LOD & Convex Collision Studio
pub fn render_lod_collision_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_lod_collision_dialog {
        return;
    }

    let mut is_open = state.show_lod_collision_dialog;

    Window::new("Static Mesh LOD & Convex Collision Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(540.0)
        .min_width(620.0)
        .min_height(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.heading("📐 Mesh LOD & Collision");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚡ Bake Collision Hull").clicked() {
                        let hull_type = state.lod_collision_settings.hull_type;
                        let margin = state.lod_collision_settings.hull_margin;
                        state.bake_collision_hull_as_brush(hull_type, margin);
                    }
                    if ui.button("📉 Apply LOD to Brush").clicked() {
                        let level = state.lod_collision_settings.target_lod_level;
                        state.apply_lod_to_selected_brush(level);
                    }
                });
            });

            ui.separator();

            // Polygon Budget & Target Identification
            let (poly_count, budget_rating) = state.evaluate_selected_mesh_budget();
            let target_name = if let Some(idx) = state.selected_brush_index {
                format!("World Brush #{idx}")
            } else {
                "Builder Brush (Active Pivot)".to_string()
            };

            let badge_col = match budget_rating {
                PolygonBudgetRating::Optimal => Color32::from_rgb(80, 220, 100),
                PolygonBudgetRating::Acceptable => Color32::from_rgb(255, 210, 50),
                PolygonBudgetRating::HeavyBudget => Color32::from_rgb(255, 140, 40),
                PolygonBudgetRating::CriticalHazard => Color32::from_rgb(255, 60, 60),
            };

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Target: {}", target_name)).strong());
                    ui.separator();
                    ui.label(format!("Polygons: {}", poly_count));
                    ui.separator();
                    ui.colored_label(badge_col, budget_rating.title());
                    ui.separator();
                    ui.checkbox(&mut state.lod_collision_settings.show_preview_hull, "Preview Collision Hull");
                });
            });

            ui.add_space(4.0);

            // Main Editor Layout: Left = Collision Hull Synthesizer, Right = LOD Auto-Decimator
            ui.horizontal(|ui| {
                // Left Column: Collision Hull Synthesizer
                ui.vertical(|ui| {
                    ui.set_width(380.0);
                    render_collision_hull_controls(ui, state);
                });

                ui.separator();

                // Right Column: LOD Auto-Decimator
                ui.vertical(|ui| {
                    render_lod_decimator_controls(ui, state);
                });
            });
        });

    state.show_lod_collision_dialog = is_open;
}

fn render_collision_hull_controls(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Convex Collision Hull Synthesizer").strong().size(13.0));
    ui.label(RichText::new("Automated discrete oriented polytope (DOP) generation for fast BSP collision.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let cur_type = state.lod_collision_settings.hull_type;

    ui.group(|ui| {
        ui.vertical(|ui| {
            if ui.selectable_label(cur_type == CollisionHullType::AabbBox, "📦 AABB (6 Planes Axis-Aligned)").clicked() {
                state.lod_collision_settings.hull_type = CollisionHullType::AabbBox;
            }
            if ui.selectable_label(cur_type == CollisionHullType::Kdop10, "🛑 KDOP-10 (Octagonal Column / Pawn)").clicked() {
                state.lod_collision_settings.hull_type = CollisionHullType::Kdop10;
            }
            if ui.selectable_label(cur_type == CollisionHullType::Kdop18, "💎 KDOP-18 (Chamfered Bevel Edges)").clicked() {
                state.lod_collision_settings.hull_type = CollisionHullType::Kdop18;
            }
            if ui.selectable_label(cur_type == CollisionHullType::Kdop26, "🔮 KDOP-26 (Full Corner & Edge Bevels)").clicked() {
                state.lod_collision_settings.hull_type = CollisionHullType::Kdop26;
            }
        });
    });

    ui.add_space(6.0);

    ui.horizontal(|ui| {
        ui.label("Margin Inflation:");
        ui.add(egui::Slider::new(&mut state.lod_collision_settings.hull_margin, 0.0..=32.0).text("uu"));
    });

    ui.add_space(8.0);

    if ui.button("⚡ Bake Collision Hull to World").clicked() {
        let hull_type = state.lod_collision_settings.hull_type;
        let margin = state.lod_collision_settings.hull_margin;
        state.bake_collision_hull_as_brush(hull_type, margin);
    }
}

fn render_lod_decimator_controls(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("LOD Auto-Decimator & Optimizer").strong().size(13.0));
    ui.label(RichText::new("Hierarchical geometric simplification preserving silhouettes and tournament budget.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    if ui.button("🔄 Generate 4-Level LOD Chain").clicked() {
        state.generate_lod_chain_for_selected_brush();
    }

    ui.add_space(6.0);

    let cur_level = state.lod_collision_settings.target_lod_level;

    if let Some(ref chain) = state.lod_collision_settings.active_lod_chain {
        ui.group(|ui| {
            ui.vertical(|ui| {
                let l0_count = chain.lod0_polys.len();
                let l1_count = chain.lod1_polys.len();
                let l2_count = chain.lod2_polys.len();
                let l3_count = chain.lod3_polys.len();

                if ui.selectable_label(cur_level == 0, format!("LOD0 (100% High Detail): {} polys", l0_count)).clicked() {
                    state.lod_collision_settings.target_lod_level = 0;
                }
                if ui.selectable_label(cur_level == 1, format!("LOD1 (~60% Medium Dist): {} polys (-{:.0}%)", l1_count, (1.0 - l1_count as f32 / l0_count.max(1) as f32) * 100.0)).clicked() {
                    state.lod_collision_settings.target_lod_level = 1;
                }
                if ui.selectable_label(cur_level == 2, format!("LOD2 (~30% Far Dist): {} polys (-{:.0}%)", l2_count, (1.0 - l2_count as f32 / l0_count.max(1) as f32) * 100.0)).clicked() {
                    state.lod_collision_settings.target_lod_level = 2;
                }
                if ui.selectable_label(cur_level == 3, format!("LOD3 (~15% Silhouette): {} polys (-{:.0}%)", l3_count, (1.0 - l3_count as f32 / l0_count.max(1) as f32) * 100.0)).clicked() {
                    state.lod_collision_settings.target_lod_level = 3;
                }
            });
        });
    } else {
        ui.label(RichText::new("No LOD chain calculated yet. Click 'Generate 4-Level LOD Chain' above.").italics().color(Color32::GRAY));
    }

    ui.add_space(8.0);

    if ui.button("📉 Apply Selected LOD Level to Brush").clicked() {
        let level = state.lod_collision_settings.target_lod_level;
        state.apply_lod_to_selected_brush(level);
    }
}
