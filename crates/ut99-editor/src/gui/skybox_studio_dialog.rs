//! SkyBox 3D Real-time Parallax & Multi-Zone Environmental Studio

use egui::{Color32, RichText, Ui};
use crate::editor_state::{EditorState, ZonePhysicsPreset};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkyboxStudioTab {
    SkyZoneParallax,
    ZoneProfiles,
    FluidPresets,
}

pub fn render_skybox_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_skybox_studio_dialog {
        return;
    }

    let mut open = state.show_skybox_studio_dialog;
    let mut current_tab = SkyboxStudioTab::SkyZoneParallax;

    // Use memory state for dialog tab
    let tab_id = egui::Id::new("skybox_studio_active_tab");
    ctx.data_mut(|d| {
        if let Some(t) = d.get_temp::<u8>(tab_id) {
            current_tab = match t {
                1 => SkyboxStudioTab::ZoneProfiles,
                2 => SkyboxStudioTab::FluidPresets,
                _ => SkyboxStudioTab::SkyZoneParallax,
            };
        }
    });

    egui::Window::new("SkyBox 3D Parallax & Multi-Zone Studio")
        .open(&mut open)
        .default_width(640.0)
        .default_height(480.0)
        .resizable(true)
        .show(ctx, |ui| {
            // Header Tabs
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == SkyboxStudioTab::SkyZoneParallax, "☁ SkyZone & Parallax").clicked() {
                    current_tab = SkyboxStudioTab::SkyZoneParallax;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 0u8));
                }
                if ui.selectable_label(current_tab == SkyboxStudioTab::ZoneProfiles, "🌐 Level Zones & Portals").clicked() {
                    current_tab = SkyboxStudioTab::ZoneProfiles;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 1u8));
                }
                if ui.selectable_label(current_tab == SkyboxStudioTab::FluidPresets, "≋ Environmental Presets").clicked() {
                    current_tab = SkyboxStudioTab::FluidPresets;
                    ctx.data_mut(|d| d.insert_temp(tab_id, 2u8));
                }
            });

            ui.separator();

            match current_tab {
                SkyboxStudioTab::SkyZoneParallax => render_skyzone_tab(ui, state),
                SkyboxStudioTab::ZoneProfiles => render_zone_profiles_tab(ui, state),
                SkyboxStudioTab::FluidPresets => render_fluid_presets_tab(ui, state),
            }
        });

    state.show_skybox_studio_dialog = open;
}

fn render_skyzone_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.vertical(|ui| {
        let sky_zone_opt = state.find_active_sky_zone();

        // SkyZone Status Banner
        ui.group(|ui| {
            ui.horizontal(|ui| {
                if let Some((_, pos, tag)) = &sky_zone_opt {
                    ui.label(RichText::new("✓").size(18.0).color(Color32::from_rgb(100, 220, 140)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Active SkyZone: '{}'", tag)).strong().color(Color32::from_rgb(120, 220, 255)));
                        ui.label(RichText::new(format!("Anchor Location: ({:.1}, {:.1}, {:.1})", pos.x, pos.y, pos.z)).weak().small());
                    });
                } else {
                    ui.label(RichText::new("⚠").size(18.0).color(Color32::from_rgb(255, 180, 80)));
                    ui.vertical(|ui| {
                        ui.label(RichText::new("No SkyZoneInfo Actor Found in Level").strong().color(Color32::from_rgb(255, 180, 80)));
                        ui.label(RichText::new("Surfaces with PF_FAKE_BACKDROP will display a black void until a SkyZoneInfo is placed.").weak().small());
                    });
                }
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("⬡ Place SkyZoneInfo at Pivot").clicked() {
                    match state.place_sky_zone_anchor_at_pivot() {
                        Ok(_) => state.toast_success("Placed SkyZoneInfo actor at pivot location."),
                        Err(e) => state.toast_warning(format!("Failed to place SkyZoneInfo: {}", e)),
                    }
                }
                if sky_zone_opt.is_some() {
                    if ui.button("◎ Focus on SkyZone").clicked() {
                        state.focus_camera_on_sky_zone();
                        state.toast_info("Camera focused on SkyZone.");
                    }
                }
                if ui.button("▣ Focus on Map Center").clicked() {
                    state.focus_camera_on_level_center();
                    state.toast_info("Camera focused on map center.");
                }
            });
        });

        ui.add_space(8.0);

        // Real-time Parallax Slider & Presets
        ui.group(|ui| {
            ui.label(RichText::new("3D Parallax Dynamics").strong());
            ui.label(RichText::new("Simulates perspective motion relative to the SkyZone room as the editor camera moves.").weak().small());

            ui.horizontal(|ui| {
                ui.label("Parallax Scale:");
                ui.add(egui::Slider::new(&mut state.sky_zone_settings.parallax_scale, 0.0..=1.0)
                    .step_by(0.01)
                    .text("ratio"));
            });

            ui.horizontal(|ui| {
                ui.label(RichText::new("Presets:").weak().small());
                if ui.small_button("0.0x (Fixed Infinite Sky)").clicked() {
                    state.sky_zone_settings.parallax_scale = 0.0;
                }
                if ui.small_button("0.05x (Distant Peaks)").clicked() {
                    state.sky_zone_settings.parallax_scale = 0.05;
                }
                if ui.small_button("0.20x (Floating Clouds)").clicked() {
                    state.sky_zone_settings.parallax_scale = 0.20;
                }
                if ui.small_button("1.0x (1:1 Near Room)").clicked() {
                    state.sky_zone_settings.parallax_scale = 1.0;
                }
            });
        });

        ui.add_space(8.0);

        // Viewport Overlays & Surface Operations
        ui.group(|ui| {
            ui.label(RichText::new("Viewport Visualizers & Surface Tagging").strong());

            ui.checkbox(&mut state.sky_zone_settings.show_frustum, "◫ Render Real-time Sky Frustum in 3D Viewport");
            ui.checkbox(&mut state.sky_zone_settings.highlight_sky_surfaces, "☼ Highlight Sky Backdrop Surfaces (PF_FAKE_BACKDROP)");

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                let sel_count = state.selected_surfaces.len();
                let sel_text = if sel_count > 0 {
                    format!("☁ Mark {} Selected Surface(s) as Sky Backdrop", sel_count)
                } else if state.selected_brush_index.is_some() {
                    "☁ Mark Selected Brush Surfaces as Sky Backdrop".to_string()
                } else {
                    "☁ Mark Surface as Sky Backdrop".to_string()
                };

                if ui.button(RichText::new(sel_text).color(Color32::from_rgb(100, 220, 255))).clicked() {
                    let count = state.mark_selected_surfaces_as_sky(true);
                    state.toast_success(format!("Marked {} surface(s) as Sky Backdrop.", count));
                }

                if ui.button("Clear Sky Backdrop Flag").clicked() {
                    let count = state.mark_selected_surfaces_as_sky(false);
                    state.toast_info(format!("Cleared Sky Backdrop flag on {} surface(s).", count));
                }
            });
        });
    });
}

fn render_zone_profiles_tab(ui: &mut Ui, state: &mut EditorState) {
    let zones = state.collect_level_zone_profiles();

    if zones.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);
            ui.label(RichText::new("No ZoneInfo or WaterZone actors found in level.").weak());
            ui.label("Add ZoneInfo or WaterZone actors from the Actor Class Browser to partition environments.");
        });
        return;
    }

    ui.vertical(|ui| {
        ui.label(RichText::new(format!("Detected Zones: {}", zones.len())).strong());

        egui::ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
            egui::Grid::new("zones_table_grid")
                .striped(true)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("Zone Tag").strong());
                    ui.label(RichText::new("Class").strong());
                    ui.label(RichText::new("Gravity (Z)").strong());
                    ui.label(RichText::new("Friction").strong());
                    ui.label(RichText::new("Type").strong());
                    ui.label(RichText::new("Action").strong());
                    ui.end_row();

                    for z in &zones {
                        let is_selected = state.selected_zone_tag.as_deref() == Some(&z.actor_tag);

                        let tag_color = if is_selected {
                            Color32::from_rgb(255, 220, 80)
                        } else if z.is_sky_zone {
                            Color32::from_rgb(180, 120, 255)
                        } else if z.is_water_zone {
                            Color32::from_rgb(80, 200, 255)
                        } else {
                            Color32::from_rgb(200, 200, 200)
                        };

                        ui.label(RichText::new(&z.actor_tag).strong().color(tag_color));
                        ui.label(&z.class_name);
                        ui.label(format!("{:.0}", z.gravity.z));
                        ui.label(format!("{:.2}", z.fluid_friction));

                        let type_str = if z.is_sky_zone {
                            "🌌 SkyZone"
                        } else if z.is_water_zone {
                            "🌊 Water"
                        } else {
                            "Standard"
                        };
                        ui.label(type_str);

                        ui.horizontal(|ui| {
                            if ui.small_button("🔍 Select").clicked() {
                                state.selected_zone_tag = Some(z.actor_tag.clone());
                            }
                        });
                        ui.end_row();
                    }
                });
        });

        ui.separator();

        // Selected Zone Detail Inspector
        if let Some(ref sel_tag) = state.selected_zone_tag.clone() {
            if let Some(cur_zone) = zones.iter().find(|z| z.actor_tag == *sel_tag) {
                ui.group(|ui| {
                    ui.label(RichText::new(format!("Zone Inspector: '{}' ({})", cur_zone.actor_tag, cur_zone.class_name)).strong().color(Color32::from_rgb(100, 220, 255)));

                    let mut grav = cur_zone.gravity;
                    let mut fric = cur_zone.fluid_friction;
                    let mut is_water = cur_zone.is_water_zone;
                    let mut dmg = cur_zone.damage_per_sec;
                    let mut dmg_type = cur_zone.damage_type.clone();

                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Gravity Vector:");
                        ui.label("X:");
                        if ui.add(egui::DragValue::new(&mut grav.x).speed(10.0)).changed() { changed = true; }
                        ui.label("Y:");
                        if ui.add(egui::DragValue::new(&mut grav.y).speed(10.0)).changed() { changed = true; }
                        ui.label("Z:");
                        if ui.add(egui::DragValue::new(&mut grav.z).speed(10.0)).changed() { changed = true; }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Fluid Friction:");
                        if ui.add(egui::DragValue::new(&mut fric).speed(0.05).range(0.0..=10.0)).changed() { changed = true; }

                        ui.add_space(10.0);
                        if ui.checkbox(&mut is_water, "bWaterZone (Swimming Physics)").changed() { changed = true; }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Damage/Sec:");
                        if ui.add(egui::DragValue::new(&mut dmg).speed(1.0).range(0..=100)).changed() { changed = true; }

                        ui.label("DamageType:");
                        if ui.add(egui::TextEdit::singleline(&mut dmg_type).desired_width(120.0)).changed() { changed = true; }
                    });

                    if changed {
                        let _ = state.update_zone_properties(&cur_zone.actor_tag, grav, fric, is_water, dmg, &dmg_type);
                    }
                });
            }
        }
    });
}

fn render_fluid_presets_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.vertical(|ui| {
        ui.label(RichText::new("Environmental Physical Presets").strong());
        ui.label(RichText::new("Quickly configure realistic fluid dynamics, hazard zones and low-gravity arenas.").weak().small());

        let target_zone = state.selected_zone_tag.clone();
        if let Some(ref tag) = target_zone {
            ui.label(RichText::new(format!("Target Zone: '{}'", tag)).color(Color32::from_rgb(100, 220, 255)).strong());
        } else {
            ui.label(RichText::new("Target: All WaterZone actors (or select a zone in the Zones tab)").weak());
        }

        ui.add_space(8.0);

        let presets = [
            ZonePhysicsPreset::StandardEarth,
            ZonePhysicsPreset::WaterSwimming,
            ZonePhysicsPreset::MuddySwamp,
            ZonePhysicsPreset::ToxicSlime,
            ZonePhysicsPreset::VolcanicLava,
            ZonePhysicsPreset::LowGravity,
            ZonePhysicsPreset::SpaceVacuum,
        ];

        egui::Grid::new("presets_grid")
            .spacing([12.0, 8.0])
            .show(ui, |ui| {
                for (idx, preset) in presets.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(preset.icon()).size(18.0));
                        if ui.button(preset.label()).clicked() {
                            if let Some(ref tag) = target_zone {
                                match state.apply_zone_physics_preset(tag, *preset) {
                                    Ok(_) => state.toast_success(format!("Applied {} physics to zone '{}'.", preset.label(), tag)),
                                    Err(e) => state.toast_warning(format!("Failed to apply zone physics: {}", e)),
                                }
                            } else {
                                // Apply to all zones or first available
                                let zones = state.collect_level_zone_profiles();
                                let mut count = 0;
                                for z in &zones {
                                    if z.is_water_zone || *preset == ZonePhysicsPreset::StandardEarth {
                                        if state.apply_zone_physics_preset(&z.actor_tag, *preset).is_ok() {
                                            count += 1;
                                        }
                                    }
                                }
                                if count > 0 {
                                    state.toast_success(format!("Applied {} physics to {} zone(s).", preset.label(), count));
                                } else {
                                    state.toast_info("No matching zones found to apply physics preset.");
                                }
                            }
                        }
                    });

                    if (idx + 1) % 2 == 0 {
                        ui.end_row();
                    }
                }
            });
    });
}
