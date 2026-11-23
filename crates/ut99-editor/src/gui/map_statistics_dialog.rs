use egui::{self, Color32, RichText};
use crate::editor_state::EditorState;

pub fn render_map_statistics_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_map_statistics_dialog {
        return;
    }

    let mut open = state.show_map_statistics_dialog;
    let mut close_requested = false;

    crate::gui::modal::standard_dialog_window(ctx, RichText::new("Map Statistics & Geometry Metrics").strong(), &mut open, [480.0, 500.0], [360.0, 320.0])
        .show(ctx, |ui| {
            ui.add_space(4.0);

            let stats = state.compute_map_statistics();

            // Map filename header
            let map_name = state.map_path.as_ref()
                .and_then(|p| p.file_name())
                .and_then(|f| f.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Untitled.unr".to_string());
            ui.horizontal(|ui| {
                ui.label(RichText::new("Level:").strong());
                ui.label(RichText::new(&map_name).monospace().color(Color32::from_rgb(245, 182, 45)));
            });

            ui.separator();

            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                // SECTION 1: CSG GEOMETRY
                ui.label(RichText::new("📐 CSG World Geometry:").strong().color(Color32::from_rgb(100, 200, 255)));
                egui::Grid::new("stats_geom_grid")
                    .striped(true)
                    .min_col_width(180.0)
                    .show(ui, |ui| {
                        ui.label("Total World Brushes:");
                        ui.label(RichText::new(format!("{}", stats.total_brushes)).strong());
                        ui.end_row();

                        ui.label("  • Solid Add Brushes:");
                        ui.colored_label(Color32::from_rgb(100, 220, 100), format!("{}", stats.solid_brushes));
                        ui.end_row();

                        ui.label("  • Subtractive Brushes:");
                        ui.colored_label(Color32::from_rgb(240, 100, 100), format!("{}", stats.subtract_brushes));
                        ui.end_row();

                        ui.label("Total Polygons:");
                        ui.label(RichText::new(format!("{}", stats.total_polys)).strong());
                        ui.end_row();

                        ui.label("Total Vertices:");
                        ui.label(format!("{}", stats.total_vertices));
                        ui.end_row();

                        ui.label("Unique Textures Used:");
                        ui.label(RichText::new(format!("{}", stats.unique_textures_count)).color(Color32::from_rgb(220, 180, 80)));
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.separator();

                // SECTION 2: ACTORS & GAMEPLAY
                ui.label(RichText::new("◈ Actors & Gameplay Entities:").strong().color(Color32::from_rgb(245, 182, 45)));
                egui::Grid::new("stats_actors_grid")
                    .striped(true)
                    .min_col_width(180.0)
                    .show(ui, |ui| {
                        ui.label("Total Actors in Level:");
                        ui.label(RichText::new(format!("{}", stats.total_actors)).strong());
                        ui.end_row();

                        ui.label("  ☼ Lights (Point/Spot/Sun):");
                        ui.colored_label(Color32::from_rgb(255, 230, 120), format!("{}", stats.lights_count));
                        ui.end_row();

                        ui.label("  ◈ Bot Navigation Nodes:");
                        ui.colored_label(Color32::from_rgb(80, 180, 240), format!("{}", stats.nav_points_count));
                        ui.end_row();

                        ui.label("  ⚔ Weapons & Pickups:");
                        ui.colored_label(Color32::from_rgb(120, 230, 140), format!("{}", stats.weapons_pickups_count));
                        ui.end_row();

                        ui.label("  ⚡ Triggers & Dispatchers:");
                        ui.colored_label(Color32::from_rgb(240, 140, 80), format!("{}", stats.triggers_count));
                        ui.end_row();

                        ui.label("  ▯ Movers (Doors / Lifts):");
                        ui.colored_label(Color32::from_rgb(200, 140, 240), format!("{}", stats.movers_count));
                        ui.end_row();

                        ui.label("  ⬡ Static Mesh Instances:");
                        ui.colored_label(Color32::from_rgb(180, 190, 205), format!("{}", stats.static_meshes_count));
                        ui.end_row();
                    });
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("⎘ Copy Summary").on_hover_text("Copy statistics report to system clipboard").clicked() {
                    let summary = format!(
                        "Level: {}\nTotal Brushes: {}\n  - Solid: {}\n  - Subtractive: {}\nTotal Polygons: {}\nTotal Actors: {}\n  - Lights: {}\n  - Navigation: {}\n  - Pickups: {}\n  - Movers: {}\nUnique Textures: {}",
                        map_name, stats.total_brushes, stats.solid_brushes, stats.subtract_brushes, stats.total_polys, stats.total_actors, stats.lights_count, stats.nav_points_count, stats.weapons_pickups_count, stats.movers_count, stats.unique_textures_count
                    );
                    ctx.copy_text(summary);
                    state.log("Copied Map Statistics summary to clipboard.");
                    state.toast_info("Map statistics copied to clipboard");
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                });
            });
        });

    if !open || close_requested {
        state.show_map_statistics_dialog = false;
    }
}
