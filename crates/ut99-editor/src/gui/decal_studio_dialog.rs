use crate::editor_state::EditorState;
use egui::{Color32, RichText, Stroke};
use ut99_bsp::ProjectorBlendMode;

pub fn render_decal_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_decal_studio_dialog {
        return;
    }

    let mut open = state.show_decal_studio_dialog;
    let mut close_requested = false;
    let mut stamp_at_builder_requested = false;
    let mut bake_requested = false;
    let mut remove_selected_requested = false;
    let mut clear_all_requested = false;
    let mut select_proj_idx: Option<usize> = None;
    let mut delete_proj_idx: Option<usize> = None;

    egui::Window::new("Decal Painting & Projector Studio")
        .open(&mut open)
        .default_size([580.0, 640.0])
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new("🎯 In-Viewport Decal & Projector Studio").strong().size(15.0).color(Color32::from_rgb(100, 200, 255)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✖ Close").clicked() {
                        close_requested = true;
                    }
                    if ui.button(RichText::new("⚡ Bake All").strong().color(Color32::from_rgb(255, 230, 80))).clicked() {
                        bake_requested = true;
                    }
                });
            });

            ui.add_space(4.0);

            // Status Banner
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(70, 110, 180)))
                .fill(Color32::from_rgba_premultiplied(20, 30, 50, 200))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("📊 {} Active Projectors", state.projectors.len())).strong().color(Color32::WHITE));
                        ui.separator();
                        ui.label(RichText::new(format!("🧩 {} Decal Mesh Fragments", state.active_projector_decals.len())).color(Color32::from_rgb(180, 220, 255)));
                        ui.separator();
                        let active_mat = state.active_texture.as_deref().unwrap_or("None");
                        ui.label(RichText::new(format!("🎨 Eyedropper: {}", active_mat)).color(Color32::from_rgb(255, 215, 0)));
                    });
                });

            ui.add_space(8.0);

            // Viewport Interaction Mode Toggle
            ui.horizontal(|ui| {
                let paint_mode = &mut state.decal_paint_mode;
                let text = if *paint_mode {
                    RichText::new("🖌 VIEWPORT DECAL STAMP MODE: ACTIVE (LMB Stamps Decals)").strong().color(Color32::from_rgb(80, 240, 120))
                } else {
                    RichText::new("🖌 Viewport Decal Stamp Mode: Standby (Alt+RMB or Click to Stamp)").color(Color32::LIGHT_GRAY)
                };
                ui.checkbox(paint_mode, text);
            });
            ui.label(RichText::new("Tip: In 3D viewport, use Alt+LMB to Eyedrop any texture, and click 'Stamp Decal' or enable Stamp Mode to paint decals directly on walls, floors and movers.").weak().size(11.0));

            ui.add_space(8.0);

            // Preset Library Section
            ui.collapsing("🎨 Quick Decal Presets (Blood, Impacts, Runes)", |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("🩸 Blood Splatter (64x64)").on_hover_text("Classic UT99 Botpack blood mark (Modulated)").clicked() {
                        state.active_texture = Some("Botpack.BloodSplat".to_string());
                        state.decal_width = 64.0;
                        state.decal_height = 64.0;
                        state.decal_blend_mode = ProjectorBlendMode::Modulated;
                    }
                    if ui.button("💥 Bullet Hole (32x32)").on_hover_text("Kinetic bullet impact crater (Modulated)").clicked() {
                        state.active_texture = Some("Botpack.BulletHole".to_string());
                        state.decal_width = 32.0;
                        state.decal_height = 32.0;
                        state.decal_blend_mode = ProjectorBlendMode::Modulated;
                    }
                    if ui.button("🔥 Blast Mark (96x96)").on_hover_text("Rocket / Shock scorch blast mark (Modulated)").clicked() {
                        state.active_texture = Some("Botpack.BlastMark".to_string());
                        state.decal_width = 96.0;
                        state.decal_height = 96.0;
                        state.decal_blend_mode = ProjectorBlendMode::Modulated;
                    }
                    if ui.button("🟢 Acid Slime (64x64)").on_hover_text("BioRifle / Toxic residue splatter (Translucent)").clicked() {
                        state.active_texture = Some("Botpack.GreenBlood".to_string());
                        state.decal_width = 64.0;
                        state.decal_height = 64.0;
                        state.decal_blend_mode = ProjectorBlendMode::Translucent;
                    }
                    if ui.button("⚡ Energy Rune (128x128)").on_hover_text("Glowing mystical rune / portal emblem (Translucent)").clicked() {
                        state.active_texture = Some("Botpack.EnergyMark".to_string());
                        state.decal_width = 128.0;
                        state.decal_height = 128.0;
                        state.decal_blend_mode = ProjectorBlendMode::Translucent;
                    }
                });
            });

            ui.add_space(6.0);

            // Decal Parameters & Geometry Configuration
            ui.group(|ui| {
                ui.label(RichText::new("⚙ Decal Projector Parameters").strong());

                // Texture specification
                ui.horizontal(|ui| {
                    ui.label("Decal Texture:");
                    let mut current_tex = state.active_texture.clone().unwrap_or_else(|| "Botpack.BloodSplat".to_string());
                    if ui.text_edit_singleline(&mut current_tex).changed() {
                        state.active_texture = Some(current_tex);
                    }
                    if ui.button("🎨 From Eyedropper").clicked() {
                        // Keep current active texture
                    }
                });

                // Blend Mode Selection
                ui.horizontal(|ui| {
                    ui.label("Blend Mode:");
                    ui.radio_value(&mut state.decal_blend_mode, ProjectorBlendMode::Modulated, "Modulated (Multiplicative)");
                    ui.radio_value(&mut state.decal_blend_mode, ProjectorBlendMode::Translucent, "Translucent (Additive)");
                    ui.radio_value(&mut state.decal_blend_mode, ProjectorBlendMode::Masked, "Masked (Cutout)");
                });

                // Dimensions
                ui.horizontal(|ui| {
                    ui.label("Width:");
                    ui.add(egui::Slider::new(&mut state.decal_width, 8.0..=512.0).suffix(" uu"));
                    ui.label("Height:");
                    ui.add(egui::Slider::new(&mut state.decal_height, 8.0..=512.0).suffix(" uu"));
                });

                // Quick Size Buttons
                ui.horizontal(|ui| {
                    ui.label("Presets:");
                    if ui.small_button("32x32").clicked() { state.decal_width = 32.0; state.decal_height = 32.0; }
                    if ui.small_button("64x64").clicked() { state.decal_width = 64.0; state.decal_height = 64.0; }
                    if ui.small_button("96x96").clicked() { state.decal_width = 96.0; state.decal_height = 96.0; }
                    if ui.small_button("128x128").clicked() { state.decal_width = 128.0; state.decal_height = 128.0; }
                    if ui.small_button("256x256").clicked() { state.decal_width = 256.0; state.decal_height = 256.0; }
                });

                // Trace Distance
                ui.horizontal(|ui| {
                    ui.label("Max Trace Distance:");
                    ui.add(egui::Slider::new(&mut state.decal_max_distance, 16.0..=512.0).suffix(" uu"));
                });
            });

            ui.add_space(8.0);

            // Action Buttons
            ui.horizontal(|ui| {
                if ui.button(RichText::new("🎯 Stamp Decal at Builder Brush").strong().color(Color32::from_rgb(100, 220, 255))).on_hover_text("Places a projector decal at current builder brush position").clicked() {
                    stamp_at_builder_requested = true;
                }

                if ui.button("🗑 Remove Selected").clicked() {
                    remove_selected_requested = true;
                }

                if ui.button(RichText::new("🧹 Clear All Decals").color(Color32::from_rgb(255, 120, 120))).clicked() {
                    clear_all_requested = true;
                }
            });

            ui.add_space(10.0);

            // Projectors Table / Inspector
            ui.label(RichText::new("📋 Active Map Projector Decals").strong().color(Color32::LIGHT_GRAY));
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    if state.projectors.is_empty() {
                        ui.label(RichText::new("No decal projectors placed yet. Click surfaces in 3D viewport or use 'Stamp Decal at Builder Brush'.").weak().italics());
                    } else {
                        egui::Grid::new("projectors_grid")
                            .striped(true)
                            .num_columns(6)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("#").strong());
                                ui.label(RichText::new("Name").strong());
                                ui.label(RichText::new("Texture").strong());
                                ui.label(RichText::new("Size").strong());
                                ui.label(RichText::new("Location").strong());
                                ui.label(RichText::new("Actions").strong());
                                ui.end_row();

                                for (idx, p) in state.projectors.iter().enumerate() {
                                    let is_selected = state.selected_projector_index == Some(idx);
                                    let idx_label = if is_selected {
                                        RichText::new(format!("▶ {}", idx)).strong().color(Color32::from_rgb(255, 230, 80))
                                    } else {
                                        RichText::new(format!("{}", idx))
                                    };
                                    ui.label(idx_label);
                                    ui.label(&p.name);
                                    ui.label(&p.texture);
                                    ui.label(format!("{:.0}x{:.0}", p.width, p.height));
                                    ui.label(format!("({:.0}, {:.0}, {:.0})", p.location.x, p.location.y, p.location.z));

                                    ui.horizontal(|ui| {
                                        if ui.small_button("Select").clicked() {
                                            select_proj_idx = Some(idx);
                                        }
                                        if ui.small_button("🗑").clicked() {
                                            delete_proj_idx = Some(idx);
                                        }
                                    });
                                    ui.end_row();
                                }
                            });
                    }
                });
        });

    if close_requested {
        state.show_decal_studio_dialog = false;
    }
    if stamp_at_builder_requested {
        state.add_projector_at_builder();
    }
    if bake_requested {
        state.bake_projector_decals();
    }
    if remove_selected_requested {
        state.remove_selected_projector();
    }
    if clear_all_requested {
        state.clear_all_decals();
    }
    if let Some(idx) = select_proj_idx {
        state.selected_projector_index = Some(idx);
    }
    if let Some(idx) = delete_proj_idx {
        state.selected_projector_index = Some(idx);
        state.remove_selected_projector();
    }
}
