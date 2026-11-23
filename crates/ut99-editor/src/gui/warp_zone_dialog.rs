use egui::{Color32, RichText, Ui};
use crate::editor_state::{EditorState, WarpConnectionStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WarpDialogTab {
    Topology,
    Generator,
    Settings,
}

pub fn render_warp_zone_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_warp_zone_studio_dialog {
        return;
    }

    let mut open = state.show_warp_zone_studio_dialog;
    let topology = state.collect_warpzone_topology();

    let tab_id = egui::Id::new("warp_zone_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<WarpDialogTab>(tab_id)
            .unwrap_or(WarpDialogTab::Topology)
    });

    egui::Window::new("WarpZone & Non-Euclidean Portal Studio")
        .open(&mut open)
        .resizable(true)
        .default_size([740.0, 520.0])
        .min_width(580.0)
        .min_height(380.0)
        .show(ctx, |ui| {
            // Header stats
            let total = topology.len();
            let matched = topology.iter().filter(|e| e.status == WarpConnectionStatus::MatchedBiDirectional).count();
            let loops = topology.iter().filter(|e| e.status == WarpConnectionStatus::LoopChain).count();
            let orphans = topology.iter().filter(|e| e.status == WarpConnectionStatus::OrphanBroken).count();

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Portals: {}", total)).strong());
                ui.separator();
                ui.colored_label(Color32::from_rgb(40, 220, 110), format!("✅ Matched: {}", matched));
                ui.separator();
                ui.colored_label(Color32::from_rgb(190, 90, 255), format!("🔄 Loops: {}", loops));
                ui.separator();
                if orphans > 0 {
                    ui.colored_label(Color32::from_rgb(255, 70, 70), format!("⚠ Orphans: {}", orphans));
                } else {
                    ui.colored_label(Color32::from_rgb(140, 160, 180), "Zero Orphans");
                }
            });

            ui.add_space(4.0);

            // Tabs
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == WarpDialogTab::Topology, "🌀 Topology & Portals").clicked() {
                    current_tab = WarpDialogTab::Topology;
                }
                if ui.selectable_label(current_tab == WarpDialogTab::Generator, "🛠 Pair Generator").clicked() {
                    current_tab = WarpDialogTab::Generator;
                }
                if ui.selectable_label(current_tab == WarpDialogTab::Settings, "📐 Verification & Options").clicked() {
                    current_tab = WarpDialogTab::Settings;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            match current_tab {
                WarpDialogTab::Topology => render_topology_tab(ui, state, &topology),
                WarpDialogTab::Generator => render_generator_tab(ui, state),
                WarpDialogTab::Settings => render_settings_tab(ui, state, &topology),
            }
        });

    state.show_warp_zone_studio_dialog = open;
}

fn render_topology_tab(
    ui: &mut Ui,
    state: &mut EditorState,
    topology: &[crate::editor_state::WarpZoneTopologyEntry],
) {
    if topology.is_empty() {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);
            ui.label(RichText::new("No WarpZoneInfo actors found in the level.").size(15.0).color(Color32::LIGHT_GRAY));
            ui.label(RichText::new("Use the '🛠 Pair Generator' tab to create a synchronized non-euclidean portal pair in 1 click.").size(12.0).color(Color32::GRAY));
            ui.add_space(10.0);
            if ui.button(RichText::new("✨ Switch to Pair Generator").strong()).clicked() {
                let tab_id = egui::Id::new("warp_zone_studio_tab_id");
                ui.ctx().data_mut(|d| d.insert_temp(tab_id, WarpDialogTab::Generator));
            }
        });
        return;
    }

    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
        for entry in topology {
            let is_selected = state.warp_zone_settings.selected_warp_index == Some(entry.actor_index);
            let frame = if is_selected {
                egui::Frame::group(ui.style()).fill(Color32::from_rgba_unmultiplied(70, 45, 110, 80))
            } else {
                egui::Frame::group(ui.style())
            };

            frame.show(ui, |ui| {
                ui.horizontal(|ui| {
                    let badge_c = entry.status.badge_color();
                    let color = Color32::from_rgb(
                        (badge_c[0] * 255.0) as u8,
                        (badge_c[1] * 255.0) as u8,
                        (badge_c[2] * 255.0) as u8,
                    );
                    ui.colored_label(color, RichText::new(format!("● {}", entry.status.label())).strong());

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("👁 Warp Camera").on_hover_text("Seamlessly project viewport camera through this portal").clicked() {
                            let _ = state.teleport_camera_through_warp_zone(entry.actor_index);
                        }
                        if ui.button("🎯 Focus").on_hover_text("Center camera on this WarpZone anchor").clicked() {
                            state.focus_camera_on_warp_zone(entry.actor_index);
                            state.warp_zone_settings.selected_warp_index = Some(entry.actor_index);
                        }
                        if ui.button("🔄 180° Yaw").on_hover_text("Invert portal exit orientation (+180° Yaw)").clicked() {
                            let _ = state.invert_warp_orientation(entry.actor_index);
                        }
                    });
                });

                ui.separator();

                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("Source: {} (Tag: '{}')", entry.name, entry.this_tag)).strong());
                        ui.label(format!("Location: ({:.0}, {:.0}, {:.0})", entry.location.x, entry.location.y, entry.location.z));
                        ui.label(format!("Orientation: Yaw={:.1}° Pitch={:.1}°", entry.yaw_deg, entry.pitch_deg));
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        if let Some(target_name) = &entry.target_name {
                            let t_tag = entry.target_tag.as_deref().unwrap_or("None");
                            ui.label(RichText::new(format!("Target: {} (Tag: '{}')", target_name, t_tag)).color(Color32::from_rgb(170, 220, 255)));
                            if let Some(dist) = entry.distance_uu {
                                ui.label(format!("Spatial Distance: {:.0} uu", dist));
                            }
                            if let Some(d_yaw) = entry.delta_yaw_deg {
                                let mut text = format!("ΔYaw: {:.1}°", d_yaw);
                                if entry.is_seamless_mirror {
                                    text.push_str(" [180° Seamless Mirror]");
                                    ui.colored_label(Color32::from_rgb(50, 220, 130), text);
                                } else {
                                    ui.label(text);
                                }
                            }
                        } else {
                            ui.colored_label(Color32::from_rgb(255, 90, 90), RichText::new(format!("Destination: '{}' (Unresolved)", entry.other_side_url)).strong());
                            ui.label("Actor does not exist or Tag is mismatched.");
                        }
                    });

                    ui.separator();

                    ui.vertical(|ui| {
                        if entry.has_adjacent_portal_surface {
                            ui.colored_label(Color32::from_rgb(80, 220, 160), format!("✅ PF_PORTAL ({:.0} uu)", entry.nearest_portal_surface_distance));
                        } else {
                            ui.colored_label(Color32::from_rgb(255, 170, 60), "⚠ No PF_PORTAL Sheet");
                        }
                        if ui.button("📐 Align to Sheet").on_hover_text("Align orientation to nearest PF_PORTAL normal").clicked() {
                            let _ = state.align_warp_to_nearest_portal_surface(entry.actor_index);
                        }
                    });
                });
            });
            ui.add_space(3.0);
        }
    });
}

fn render_generator_tab(ui: &mut Ui, state: &mut EditorState) {
    ui.label(RichText::new("Coordinated Non-Euclidean WarpZone Pair Generator").strong().size(14.0));
    ui.label("Generates two mutually linked, opposing WarpZoneInfo actors with 180° inverse orientation and atomic Undo/Redo.");

    ui.add_space(10.0);

    let pivot = state.builder_brush.location;

    ui.group(|ui| {
        ui.label(RichText::new("1. Base Anchor Location").strong());
        ui.horizontal(|ui| {
            ui.label(format!("Builder Pivot / Origin: ({:.0}, {:.0}, {:.0})", pivot.x, pivot.y, pivot.z));
            if ui.button("Use Camera Pos").clicked() {
                state.builder_brush.location = state.camera.position + state.camera.forward() * 150.0;
            }
        });
    });

    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("2. Separation & Axis Presets").strong());
        ui.horizontal(|ui| {
            ui.selectable_value(&mut state.warp_zone_settings.pair_generation_axis, 0, "Axis X (East/West)");
            ui.selectable_value(&mut state.warp_zone_settings.pair_generation_axis, 1, "Axis Y (North/South)");
            ui.selectable_value(&mut state.warp_zone_settings.pair_generation_axis, 2, "Axis Z (Vertical)");
        });

        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label("Separation Distance:");
            ui.add(egui::DragValue::new(&mut state.warp_zone_settings.pair_generation_distance).speed(32.0).range(128.0..=16384.0).suffix(" uu"));
        });

        ui.horizontal(|ui| {
            if ui.button("512 uu (Room)").clicked() { state.warp_zone_settings.pair_generation_distance = 512.0; }
            if ui.button("1024 uu (Hallway)").clicked() { state.warp_zone_settings.pair_generation_distance = 1024.0; }
            if ui.button("2048 uu (Opposite Area)").clicked() { state.warp_zone_settings.pair_generation_distance = 2048.0; }
            if ui.button("4096 uu (Long Distance)").clicked() { state.warp_zone_settings.pair_generation_distance = 4096.0; }
        });
    });

    ui.add_space(12.0);

    let sep_vec = match state.warp_zone_settings.pair_generation_axis {
        0 => glam::Vec3::new(state.warp_zone_settings.pair_generation_distance, 0.0, 0.0),
        1 => glam::Vec3::new(0.0, state.warp_zone_settings.pair_generation_distance, 0.0),
        _ => glam::Vec3::new(0.0, 0.0, state.warp_zone_settings.pair_generation_distance),
    };

    if ui.button(RichText::new("✨ Create Coordinated WarpZone Pair").strong().size(14.0)).clicked() {
        let (a, _b) = state.create_coordinated_warp_pair(pivot, sep_vec);
        state.warp_zone_settings.selected_warp_index = Some(a);
        state.focus_camera_on_warp_zone(a);
    }
}

fn render_settings_tab(
    ui: &mut Ui,
    state: &mut EditorState,
    topology: &[crate::editor_state::WarpZoneTopologyEntry],
) {
    ui.label(RichText::new("Viewport Visualization & Topology Linker").strong().size(14.0));
    ui.add_space(6.0);

    ui.group(|ui| {
        ui.label(RichText::new("Viewport Rendering Options").strong());
        ui.checkbox(&mut state.show_warpzones, "Show WarpZone Portals & Ribbons (Alt+W)");
        ui.checkbox(&mut state.warp_zone_settings.show_portal_normal_vectors, "Show 3D Normal & Orientation Vectors");
        ui.checkbox(&mut state.warp_zone_settings.show_connecting_helices, "Show Inter-dimensional Helical Beams");
        ui.checkbox(&mut state.warp_zone_settings.show_portal_camera_frustum, "Show Portal Exit Frustum Preview");
    });

    ui.add_space(10.0);

    ui.group(|ui| {
        ui.label(RichText::new("Manual Bi-directional Linker").strong());
        ui.label("Link two arbitrary WarpZones together with mutual reciprocity:");

        let warp_names: Vec<(usize, String)> = topology.iter().map(|e| (e.actor_index, format!("{} [{}]", e.name, e.this_tag))).collect();

        if warp_names.len() >= 2 {
            let mut src_idx = state.warp_zone_settings.selected_warp_index.unwrap_or(warp_names[0].0);
            let target_default = warp_names.iter().find(|(idx, _)| *idx != src_idx).map(|(idx, _)| *idx).unwrap_or(warp_names[1].0);
            let mut tgt_idx = target_default;

            ui.horizontal(|ui| {
                ui.label("Source:");
                egui::ComboBox::from_id_salt("wz_link_src")
                    .selected_text(warp_names.iter().find(|(i, _)| *i == src_idx).map(|(_, n)| n.as_str()).unwrap_or("Select"))
                    .show_ui(ui, |ui| {
                        for (idx, name) in &warp_names {
                            ui.selectable_value(&mut src_idx, *idx, name);
                        }
                    });

                ui.label("Target:");
                egui::ComboBox::from_id_salt("wz_link_tgt")
                    .selected_text(warp_names.iter().find(|(i, _)| *i == tgt_idx).map(|(_, n)| n.as_str()).unwrap_or("Select"))
                    .show_ui(ui, |ui| {
                        for (idx, name) in &warp_names {
                            ui.selectable_value(&mut tgt_idx, *idx, name);
                        }
                    });
            });

            ui.add_space(4.0);
            if ui.button("🔗 Link WarpZones Bi-directionally").clicked() {
                if src_idx != tgt_idx {
                    let _ = state.auto_pair_warp_zones(src_idx, tgt_idx);
                }
            }
        } else {
            ui.label("Need at least 2 WarpZone actors to perform manual linking.");
        }
    });
}
