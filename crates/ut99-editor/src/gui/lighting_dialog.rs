use crate::editor_state::EditorState;
use egui::{Color32, RichText};

pub fn render_lighting_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_lighting_dialog {
        return;
    }

    let mut open = state.show_lighting_dialog;

    crate::gui::modal::standard_dialog_window(
        ctx,
        "Build Lighting (Raytracing)",
        &mut open,
        [440.0, 520.0],
        [360.0, 320.0],
    )
    .show(ctx, |ui| {
            ui.heading("Raytraced Light Compiler (Rayon)");
            ui.label(
                RichText::new("CPU parallel raytracer with Lambertian diffuse, shadow occlusion, and UT99 HSV lighting.")
                    .size(11.0)
                    .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            ui.label(RichText::new("Raytracing & Falloff Settings").strong());
            ui.checkbox(&mut state.lighting_config.enable_shadows, "Enable Raytraced Shadow Occlusion");
            ui.checkbox(&mut state.lighting_config.enable_overbright, "Enable Overbright Lighting (>1.0 HDR)");

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label("Lightmap Grid Resolution:");
                let mut res = state.lighting_config.lightmap_resolution;
                ui.selectable_value(&mut res, 16.0, "16 (Fine)");
                ui.selectable_value(&mut res, 32.0, "32 (Normal)");
                ui.selectable_value(&mut res, 64.0, "64 (Fast)");
                state.lighting_config.lightmap_resolution = res;
            });

            ui.add_space(6.0);
            ui.label(RichText::new("Ambient Light Color (RGB)").strong());
            ui.horizontal(|ui| {
                ui.label("R:");
                ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.x).speed(0.01).range(0.0..=1.0));
                ui.label("G:");
                ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.y).speed(0.01).range(0.0..=1.0));
                ui.label("B:");
                ui.add(egui::DragValue::new(&mut state.lighting_config.ambient_color.z).speed(0.01).range(0.0..=1.0));
            });

            ui.add_space(10.0);
            ui.separator();

            // UnrealEd 2.0 Scale Lights... Tool
            ui.label(RichText::new("⚖ Scale Lights (LightBrightness 0..255)").strong());
            ui.horizontal(|ui| {
                ui.label("All Lights:");
                if ui.small_button("75%").on_hover_text("Scale all lights LightBrightness by 75%").clicked() {
                    state.scale_lights_brightness(75.0, true, false);
                }
                if ui.small_button("125%").on_hover_text("Scale all lights LightBrightness by 125%").clicked() {
                    state.scale_lights_brightness(125.0, true, false);
                }
                if ui.small_button("-16").on_hover_text("Subtract 16 from all lights LightBrightness").clicked() {
                    state.scale_lights_brightness(-16.0, false, false);
                }
                if ui.small_button("+16").on_hover_text("Add 16 to all lights LightBrightness").clicked() {
                    state.scale_lights_brightness(16.0, false, false);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Selected Only:");
                if ui.small_button("75%").on_hover_text("Scale selected lights LightBrightness by 75%").clicked() {
                    state.scale_lights_brightness(75.0, true, true);
                }
                if ui.small_button("125%").on_hover_text("Scale selected lights LightBrightness by 125%").clicked() {
                    state.scale_lights_brightness(125.0, true, true);
                }
                if ui.small_button("-16").on_hover_text("Subtract 16 from selected lights LightBrightness").clicked() {
                    state.scale_lights_brightness(-16.0, false, true);
                }
                if ui.small_button("+16").on_hover_text("Add 16 to selected lights LightBrightness").clicked() {
                    state.scale_lights_brightness(16.0, false, true);
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // Run Build Buttons
            ui.horizontal(|ui| {
                let build_btn = egui::Button::new(
                    RichText::new("⚡ Build All Lights")
                        .color(Color32::from_rgb(255, 230, 80))
                        .strong(),
                );
                if ui.add_sized([180.0, 32.0], build_btn).clicked() {
                    state.build_lighting();
                }
                let sel_btn = egui::Button::new(
                    RichText::new("🔦 Selected Lights Only")
                        .color(Color32::from_rgb(120, 220, 255))
                        .strong(),
                );
                if ui.add_sized([ui.available_width(), 32.0], sel_btn).on_hover_text("UnrealEd 2.0 Selected Lights Only: bake lightmaps only from selected Light actor(s)").clicked() {
                    let _ = state.bake_selected_lights_only();
                }
            });

            // Statistics from last build
            if let Some(ref stats) = state.last_lighting_stats {
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new("Last Build Statistics").strong());
                ui.label(format!("• Polygons Processed: {}", stats.total_polygons));
                ui.label(format!("• Total Luxels Baked: {}", stats.total_luxels));
                ui.label(format!("• Shadow Rays Cast: {}", stats.shadow_rays_cast));
                ui.label(format!("• Execution Time: {} ms", stats.elapsed_ms));

                if stats.elapsed_ms > 0 {
                    let rps = (stats.shadow_rays_cast as f64) / (stats.elapsed_ms as f64 / 1000.0);
                    ui.label(
                        RichText::new(format!("• Throughput: {:.0} rays/sec", rps))
                            .color(Color32::from_rgb(100, 240, 150))
                            .size(11.0),
                    );
                }
            }
        });

    state.show_lighting_dialog = open;
}
