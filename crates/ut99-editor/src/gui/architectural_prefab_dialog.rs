use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use ut99_bsp::{
    ArchStyle, ArchitecturalPrefabPreset, ArchitecturalPrefabType, CsgOperation, PillarStyle,
};

/// Interactive Procedural Architectural Prefab & Brush Macro Studio
pub fn render_architectural_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_architectural_studio_dialog {
        return;
    }

    let mut is_open = state.show_architectural_studio_dialog;

    Window::new("Procedural Architectural Prefab & Brush Macro Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(580.0)
        .min_width(620.0)
        .min_height(440.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.heading("🏛 Architectural Prefabs");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("➖ Bake Subtractive").clicked() {
                        state.bake_prefab_to_world(CsgOperation::Subtract);
                    }
                    if ui.button("➕ Bake Additive").clicked() {
                        state.bake_prefab_to_world(CsgOperation::Add);
                    }
                    if ui.button("⚡ Synthesize to Builder").clicked() {
                        state.synthesize_prefab_to_builder();
                    }
                    if ui
                        .button("🪜 Spawn UT99 Climbable Ladder (WaterZone)")
                        .on_hover_text("Create a canonical UT99 climbable ladder at Builder Brush location using an invisible ZonePortal + WaterZone (ZoneUpVelocity) + bottom/top LiftCenter/LiftExit bot pathing")
                        .clicked()
                    {
                        let pos = state.builder_brush.location;
                        state.spawn_ut99_climbable_ladder_volume(pos, 64.0, 32.0, 256.0);
                    }
                });
            });

            ui.separator();

            // Preview & Live Diagnostics Bar
            let preview_polys = state.get_prefab_preview_polygons();
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        Color32::from_rgb(255, 200, 50),
                        format!("Generated Polygons: {}", preview_polys.len()),
                    );
                    ui.separator();
                    ui.checkbox(&mut state.architectural_settings.show_preview, "Live Viewport 3D Preview");
                    ui.checkbox(&mut state.architectural_settings.wireframe_mode, "Wireframe Overlay");
                    ui.label(RichText::new("(Centered at Builder Brush location)").size(11.0).color(Color32::GRAY));
                });
            });

            ui.add_space(4.0);

            // Prefab Category Selectors
            ui.horizontal(|ui| {
                ui.label(RichText::new("Architecture Type:").strong());
                let cur_type = state.architectural_settings.config.prefab_type;
                if ui.selectable_label(cur_type == ArchitecturalPrefabType::LinearStairs, "🪜 Linear Stairs").clicked() {
                    state.architectural_settings.config.prefab_type = ArchitecturalPrefabType::LinearStairs;
                }
                if ui.selectable_label(cur_type == ArchitecturalPrefabType::SpiralStairs, "🌀 Spiral Stairs").clicked() {
                    state.architectural_settings.config.prefab_type = ArchitecturalPrefabType::SpiralStairs;
                }
                if ui.selectable_label(cur_type == ArchitecturalPrefabType::VaultedArch, "⛩ Vaulted Arch").clicked() {
                    state.architectural_settings.config.prefab_type = ArchitecturalPrefabType::VaultedArch;
                }
                if ui.selectable_label(cur_type == ArchitecturalPrefabType::ClassicPillar, "🏛 Classical Pillar").clicked() {
                    state.architectural_settings.config.prefab_type = ArchitecturalPrefabType::ClassicPillar;
                }
                if ui.selectable_label(cur_type == ArchitecturalPrefabType::IndustrialConduit, "🚰 Industrial Conduit").clicked() {
                    state.architectural_settings.config.prefab_type = ArchitecturalPrefabType::IndustrialConduit;
                }
            });

            ui.separator();

            // Main Editor Layout: Parameters on Left, Presets on Right
            ui.horizontal(|ui| {
                // Left Column: Parameters
                ui.vertical(|ui| {
                    ui.set_width(450.0);
                    egui::ScrollArea::vertical().id_salt("arch_params_scroll").max_height(380.0).show(ui, |ui| {
                        match state.architectural_settings.config.prefab_type {
                            ArchitecturalPrefabType::LinearStairs => render_linear_stairs_params(ui, state),
                            ArchitecturalPrefabType::SpiralStairs => render_spiral_stairs_params(ui, state),
                            ArchitecturalPrefabType::VaultedArch => render_vaulted_arch_params(ui, state),
                            ArchitecturalPrefabType::ClassicPillar => render_classic_pillar_params(ui, state),
                            ArchitecturalPrefabType::IndustrialConduit => render_industrial_conduit_params(ui, state),
                        }
                    });
                });

                ui.separator();

                // Right Column: Presets Library
                ui.vertical(|ui| {
                    ui.label(RichText::new("⭐ Canonical UT99 Arena Presets").strong().size(13.0));
                    ui.label(RichText::new("Instant architectural templates calibrated for tournament play.").size(11.0).color(Color32::GRAY));
                    ui.add_space(6.0);

                    egui::ScrollArea::vertical().id_salt("arch_presets_scroll").max_height(380.0).show(ui, |ui| {
                        for preset in ArchitecturalPrefabPreset::all() {
                            ui.group(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(RichText::new(preset.title()).strong().color(Color32::from_rgb(100, 220, 255)));
                                    ui.label(RichText::new(preset.description()).size(11.0).color(Color32::LIGHT_GRAY));
                                    ui.add_space(4.0);
                                    if ui.button("Apply Preset").clicked() {
                                        state.apply_architectural_preset(*preset);
                                    }
                                });
                            });
                            ui.add_space(4.0);
                        }
                    });
                });
            });
        });

    state.show_architectural_studio_dialog = is_open;
}

fn render_linear_stairs_params(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Linear Staircase Flight Parameters").strong().size(13.0));
    ui.add_space(4.0);
    let cfg = &mut state.architectural_settings.config.linear_stairs;

    ui.horizontal(|ui| {
        ui.label("Step Count:");
        ui.add(egui::Slider::new(&mut cfg.step_count, 1..=64).text("steps"));
    });
    ui.horizontal(|ui| {
        ui.label("Step Width:");
        ui.add(egui::Slider::new(&mut cfg.step_width, 64.0..=1024.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Step Height (Rise):");
        ui.add(egui::Slider::new(&mut cfg.step_height, 4.0..=64.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Step Depth (Run):");
        ui.add(egui::Slider::new(&mut cfg.step_depth, 8.0..=128.0).text("uu"));
    });
    ui.checkbox(&mut cfg.solid_underside, "Solid Underside (Fill to Floor)");

    let total_h = (cfg.step_count as f32) * cfg.step_height;
    let total_d = (cfg.step_count as f32) * cfg.step_depth;
    ui.add_space(8.0);
    ui.label(format!("Total Rise: {:.0} uu | Total Run: {:.0} uu", total_h, total_d));
}

fn render_spiral_stairs_params(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Spiral Helical Staircase Parameters").strong().size(13.0));
    ui.add_space(4.0);
    let cfg = &mut state.architectural_settings.config.spiral_stairs;

    ui.horizontal(|ui| {
        ui.label("Step Count:");
        ui.add(egui::Slider::new(&mut cfg.step_count, 4..=64).text("steps"));
    });
    ui.horizontal(|ui| {
        ui.label("Inner Radius:");
        ui.add(egui::Slider::new(&mut cfg.inner_radius, 16.0..=256.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Outer Radius:");
        ui.add(egui::Slider::new(&mut cfg.outer_radius, 64.0..=512.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Step Height:");
        ui.add(egui::Slider::new(&mut cfg.step_height, 4.0..=32.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Total Angle:");
        ui.add(egui::Slider::new(&mut cfg.total_angle_degrees, 90.0..=1080.0).text("degrees"));
    });
    ui.checkbox(&mut cfg.has_central_pillar, "Generate Central Support Column");

    let total_h = (cfg.step_count as f32) * cfg.step_height;
    ui.add_space(8.0);
    ui.label(format!("Total Spiral Elevation: {:.0} uu", total_h));
}

fn render_vaulted_arch_params(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Vaulted Arch & Portal Parameters").strong().size(13.0));
    ui.add_space(4.0);
    let cfg = &mut state.architectural_settings.config.vaulted_arch;

    ui.horizontal(|ui| {
        ui.label("Arch Style:");
        if ui.selectable_label(cfg.arch_style == ArchStyle::RoundRoman, "Roman Semicircular").clicked() {
            cfg.arch_style = ArchStyle::RoundRoman;
        }
        if ui.selectable_label(cfg.arch_style == ArchStyle::GothicPointed, "Gothic Pointed").clicked() {
            cfg.arch_style = ArchStyle::GothicPointed;
        }
        if ui.selectable_label(cfg.arch_style == ArchStyle::SegmentalFlat, "Segmental Flat").clicked() {
            cfg.arch_style = ArchStyle::SegmentalFlat;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Outer Radius:");
        ui.add(egui::Slider::new(&mut cfg.outer_radius, 64.0..=1024.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Inner Radius:");
        ui.add(egui::Slider::new(&mut cfg.inner_radius, 32.0..=cfg.outer_radius - 16.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Thickness / Depth:");
        ui.add(egui::Slider::new(&mut cfg.thickness_depth, 16.0..=512.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Radial Segments:");
        ui.add(egui::Slider::new(&mut cfg.segments, 4..=32).text("segments"));
    });

    let rim_thickness = cfg.outer_radius - cfg.inner_radius;
    ui.add_space(8.0);
    ui.label(format!("Arch Keystone Rim Thickness: {:.0} uu", rim_thickness));
}

fn render_classic_pillar_params(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Classical Column & Pillar Parameters").strong().size(13.0));
    ui.add_space(4.0);
    let cfg = &mut state.architectural_settings.config.classic_pillar;

    ui.horizontal(|ui| {
        ui.label("Pillar Style:");
        if ui.selectable_label(cfg.pillar_style == PillarStyle::CylindricalFluted, "Cylindrical").clicked() {
            cfg.pillar_style = PillarStyle::CylindricalFluted;
            cfg.sides = 16;
        }
        if ui.selectable_label(cfg.pillar_style == PillarStyle::SquareChamfered, "Square").clicked() {
            cfg.pillar_style = PillarStyle::SquareChamfered;
            cfg.sides = 4;
        }
        if ui.selectable_label(cfg.pillar_style == PillarStyle::Hexagonal, "Hexagonal").clicked() {
            cfg.pillar_style = PillarStyle::Hexagonal;
            cfg.sides = 6;
        }
    });

    ui.horizontal(|ui| {
        ui.label("Total Height:");
        ui.add(egui::Slider::new(&mut cfg.height, 128.0..=1024.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Shaft Radius:");
        ui.add(egui::Slider::new(&mut cfg.radius, 16.0..=256.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Base Height:");
        ui.add(egui::Slider::new(&mut cfg.base_height, 8.0..=128.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Capital Height:");
        ui.add(egui::Slider::new(&mut cfg.capital_height, 8.0..=128.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Entasis Swell Factor:");
        ui.add(egui::Slider::new(&mut cfg.entasis_factor, 1.0..=1.5).text("x"));
    });
    ui.horizontal(|ui| {
        ui.label("Sides / Facets:");
        ui.add(egui::Slider::new(&mut cfg.sides, 3..=32).text("facets"));
    });
}

fn render_industrial_conduit_params(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Industrial Conduit & Pipeline Parameters").strong().size(13.0));
    ui.add_space(4.0);
    let cfg = &mut state.architectural_settings.config.industrial_conduit;

    ui.horizontal(|ui| {
        ui.label("Pipe Radius:");
        ui.add(egui::Slider::new(&mut cfg.pipe_radius, 8.0..=128.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Straight Length:");
        ui.add(egui::Slider::new(&mut cfg.length, 64.0..=1024.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Bend Angle:");
        if ui.selectable_label(cfg.bend_angle_degrees == 0.0, "Straight (0°)").clicked() {
            cfg.bend_angle_degrees = 0.0;
        }
        if ui.selectable_label(cfg.bend_angle_degrees == 45.0, "Elbow 45°").clicked() {
            cfg.bend_angle_degrees = 45.0;
        }
        if ui.selectable_label(cfg.bend_angle_degrees == 90.0, "Elbow 90°").clicked() {
            cfg.bend_angle_degrees = 90.0;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Elbow Bend Radius:");
        ui.add(egui::Slider::new(&mut cfg.bend_radius, 32.0..=512.0).text("uu"));
    });
    ui.horizontal(|ui| {
        ui.label("Cross-Section Facets:");
        ui.add(egui::Slider::new(&mut cfg.sides, 6..=24).text("sides"));
    });
    ui.checkbox(&mut cfg.has_flanges, "Include Joint Connection Flanges");
}
