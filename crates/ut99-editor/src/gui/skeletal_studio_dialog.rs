use crate::editor_state::EditorState;
use egui::{Color32, RichText, Ui, Window};
use glam::{Quat, Vec3};
use ut99_bsp::{MeshHitBox, SkeletalSocket};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SkeletalTab {
    Animations,
    BoneHierarchy,
    SocketsAndAttachments,
    HitBoxesAndCollision,
    ActorXImportExport,
}

/// Interactive Skeletal Rigging, Bone Hierarchy & Character Animation Studio
pub fn show_skeletal_studio_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_skeletal_studio_dialog {
        return;
    }

    if state.skeletal_settings.is_playing {
        let dt = ctx.input(|i| i.stable_dt);
        state.update_skeletal_animation_timeline(dt);
        ctx.request_repaint();
    }

    let mut is_open = state.show_skeletal_studio_dialog;
    let tab_id = egui::Id::new("skeletal_studio_tab_id");
    let mut current_tab = ctx.data_mut(|d| {
        d.get_temp::<SkeletalTab>(tab_id)
            .unwrap_or(SkeletalTab::Animations)
    });

    Window::new("Skeletal Rigging & Character Animation Studio")
        .open(&mut is_open)
        .default_width(780.0)
        .default_height(540.0)
        .min_width(620.0)
        .min_height(420.0)
        .resizable(true)
        .show(ctx, |ui| {
            let mesh_idx = state.skeletal_settings.selected_mesh_index;
            let meshes_len = state.skeletal_meshes.len();

            if meshes_len == 0 {
                ui.colored_label(Color32::from_rgb(255, 100, 100), "No Skeletal Meshes loaded.");
                return;
            }

            let valid_mesh_idx = mesh_idx.min(meshes_len - 1);
            let mesh_name = state.skeletal_meshes[valid_mesh_idx].name.clone();
            let bones_count = state.skeletal_meshes[valid_mesh_idx].bones.len();
            let sockets_count = state.skeletal_meshes[valid_mesh_idx].sockets.len();
            let anims_count = state.skeletal_meshes[valid_mesh_idx].animations.len();
            let active_anim_name = state.skeletal_settings.active_animation.clone()
                .unwrap_or_else(|| "Run".to_string());
            let max_duration = state.skeletal_meshes[valid_mesh_idx]
                .animations
                .iter()
                .find(|a| a.name == active_anim_name)
                .map(|a| a.duration_secs)
                .unwrap_or(1.0)
                .max(0.1);

            // Header bar
            ui.horizontal(|ui| {
                ui.heading("🦴 Character Rig & Skeletal Animator");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📍 Place Animated Humanoid at Pivot").clicked() {
                        state.place_skeletal_mesh_at_pivot(valid_mesh_idx);
                    }
                    if ui.button("🔄 Reset to Bind Pose (T-Pose)").clicked() {
                        state.reset_skeletal_mesh_to_bind_pose(valid_mesh_idx);
                    }
                });
            });

            ui.separator();

            // Mesh details & metrics
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Mesh: '{mesh_name}'")).strong());
                ui.separator();
                ui.label(format!("Bones: {bones_count}"));
                ui.separator();
                ui.label(format!("Sockets: {sockets_count}"));
                ui.separator();
                ui.label(format!("Sequences: {anims_count}"));
                ui.separator();
                ui.checkbox(&mut state.skeletal_settings.show_bones_wireframe, "Show 3D Bones");
                ui.checkbox(&mut state.skeletal_settings.show_socket_helpers, "Show Sockets");
            });

            ui.add_space(4.0);

            // Transport Controls & Scrubbing Timeline
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    if state.skeletal_settings.is_playing {
                        if ui.button("⏸ Pause").clicked() {
                            state.skeletal_settings.is_playing = false;
                        }
                    } else {
                        if ui.button("▶ Play").clicked() {
                            if state.skeletal_settings.timeline_time >= max_duration {
                                state.skeletal_settings.timeline_time = 0.0;
                            }
                            state.skeletal_settings.is_playing = true;
                        }
                    }

                    if ui.button("⏹ Stop").clicked() {
                        state.skeletal_settings.is_playing = false;
                        state.skeletal_settings.timeline_time = 0.0;
                        state.update_skeletal_animation_timeline(0.0);
                    }

                    ui.separator();
                    ui.label("Speed:");
                    ui.selectable_value(&mut state.skeletal_settings.play_speed, 0.25, "0.25x");
                    ui.selectable_value(&mut state.skeletal_settings.play_speed, 0.5, "0.5x");
                    ui.selectable_value(&mut state.skeletal_settings.play_speed, 1.0, "1.0x");
                    ui.selectable_value(&mut state.skeletal_settings.play_speed, 2.0, "2.0x");

                    ui.separator();
                    ui.checkbox(&mut state.skeletal_settings.loop_preview, "Loop");

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let frame_idx = (state.skeletal_settings.timeline_time * 30.0) as usize;
                        let max_frame = (max_duration * 30.0) as usize;
                        ui.colored_label(
                            Color32::from_rgb(100, 220, 255),
                            format!("T = {:.2}s / {:.2}s (Frame {}/{})", state.skeletal_settings.timeline_time, max_duration, frame_idx, max_frame),
                        );
                    });
                });

                // Scrubbing Timeline Canvas
                let available_width = ui.available_width().max(200.0);
                let (canvas_rect, response) = ui.allocate_exact_size(egui::vec2(available_width, 32.0), egui::Sense::click_and_drag());

                if response.dragged() || response.clicked() {
                    if let Some(mouse_pos) = response.interact_pointer_pos() {
                        let frac = ((mouse_pos.x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0);
                        state.skeletal_settings.timeline_time = frac * max_duration;
                        state.update_skeletal_animation_timeline(0.0);
                    }
                }

                let painter = ui.painter_at(canvas_rect);
                painter.rect_filled(canvas_rect, 4.0, Color32::from_rgb(25, 28, 35));
                painter.rect_stroke(canvas_rect, 4.0, egui::Stroke::new(1.0_f32, Color32::from_rgb(60, 65, 80)), egui::StrokeKind::Inside);

                // Frame tick notches
                let total_ticks = 10;
                for i in 0..=total_ticks {
                    let frac = i as f32 / total_ticks as f32;
                    let x = canvas_rect.min.x + frac * canvas_rect.width();
                    painter.line_segment(
                        [egui::pos2(x, canvas_rect.max.y - 6.0), egui::pos2(x, canvas_rect.max.y - 2.0)],
                        egui::Stroke::new(1.0_f32, Color32::from_rgb(90, 100, 120)),
                    );
                }

                // Playhead cursor
                let playhead_frac = (state.skeletal_settings.timeline_time / max_duration).clamp(0.0, 1.0);
                let playhead_x = canvas_rect.min.x + playhead_frac * canvas_rect.width();
                painter.line_segment(
                    [egui::pos2(playhead_x, canvas_rect.min.y), egui::pos2(playhead_x, canvas_rect.max.y)],
                    egui::Stroke::new(2.5_f32, Color32::from_rgb(255, 210, 40)),
                );
                painter.circle_filled(egui::pos2(playhead_x, canvas_rect.min.y + 3.0), 4.0, Color32::from_rgb(255, 210, 40));
            });

            ui.add_space(4.0);

            // Tab Navigation
            ui.horizontal(|ui| {
                if ui.selectable_label(current_tab == SkeletalTab::Animations, "🎬 Animation Sequences").clicked() {
                    current_tab = SkeletalTab::Animations;
                }
                if ui.selectable_label(current_tab == SkeletalTab::BoneHierarchy, "🦴 Bone Hierarchy Tree").clicked() {
                    current_tab = SkeletalTab::BoneHierarchy;
                }
                if ui.selectable_label(current_tab == SkeletalTab::SocketsAndAttachments, "⚔ Weapon Sockets").clicked() {
                    current_tab = SkeletalTab::SocketsAndAttachments;
                }
                if ui.selectable_label(current_tab == SkeletalTab::HitBoxesAndCollision, "🎯 HitBoxes & Locational Damage").clicked() {
                    current_tab = SkeletalTab::HitBoxesAndCollision;
                }
                if ui.selectable_label(current_tab == SkeletalTab::ActorXImportExport, "📦 ActorX (.PSK / .PSA)").clicked() {
                    current_tab = SkeletalTab::ActorXImportExport;
                }
            });

            ctx.data_mut(|d| d.insert_temp(tab_id, current_tab));
            ui.separator();

            match current_tab {
                SkeletalTab::Animations => render_animations_tab(ui, state, valid_mesh_idx),
                SkeletalTab::BoneHierarchy => render_bone_hierarchy_tab(ui, state, valid_mesh_idx),
                SkeletalTab::SocketsAndAttachments => render_sockets_tab(ui, state, valid_mesh_idx),
                SkeletalTab::HitBoxesAndCollision => render_hitboxes_tab(ui, state, valid_mesh_idx),
                SkeletalTab::ActorXImportExport => render_actorx_tab(ui, state, valid_mesh_idx),
            }
        });

    state.show_skeletal_studio_dialog = is_open;
}

fn render_animations_tab(ui: &mut Ui, state: &mut EditorState, mesh_idx: usize) {
    ui.label(RichText::new("Canonical Tournament Character Animations").strong().size(13.0));
    ui.label(RichText::new("Select an animation track to evaluate forward kinematics, joint rotations, and weapon recoil in real-time.").size(11.0).color(Color32::GRAY));
    ui.add_space(4.0);

    let active_name = state.skeletal_settings.active_animation.clone().unwrap_or_default();
    let animations = state.skeletal_meshes[mesh_idx].animations.clone();

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        egui::Grid::new("anim_tracks_grid")
            .striped(true)
            .min_col_width(90.0)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("Sequence").strong());
                ui.label(RichText::new("Duration").strong());
                ui.label(RichText::new("FPS").strong());
                ui.label(RichText::new("Looping").strong());
                ui.label(RichText::new("Keyframes").strong());
                ui.label(RichText::new("Action").strong());
                ui.end_row();

                for anim in &animations {
                    let is_active = anim.name == active_name;
                    let name_text = if is_active {
                        RichText::new(format!("▶ {}", anim.name)).strong().color(Color32::from_rgb(255, 210, 40))
                    } else {
                        RichText::new(&anim.name)
                    };

                    ui.label(name_text);
                    ui.label(format!("{:.2}s", anim.duration_secs));
                    ui.label(format!("{:.0}", anim.frame_rate));
                    ui.label(if anim.loop_anim { "✔ Yes" } else { "❌ No" });
                    ui.label(format!("{}", anim.keyframes.len()));

                    if ui.button(if is_active { "Active" } else { "Select" }).clicked() {
                        state.select_skeletal_animation(mesh_idx, &anim.name);
                    }
                    ui.end_row();
                }
            });
    });
}

fn render_bone_hierarchy_tab(ui: &mut Ui, state: &EditorState, mesh_idx: usize) {
    ui.label(RichText::new("Hierarchical Bone Skeleton & Joint Transforms").strong().size(13.0));
    ui.label(RichText::new("Full inspection of parent-child linkages and local transformation vectors.").size(11.0).color(Color32::GRAY));
    ui.add_space(4.0);

    let mesh = &state.skeletal_meshes[mesh_idx];

    egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
        egui::Grid::new("bones_hierarchy_grid")
            .striped(true)
            .min_col_width(70.0)
            .spacing([10.0, 4.0])
            .show(ui, |ui| {
                ui.label(RichText::new("#").strong());
                ui.label(RichText::new("Bone Name").strong());
                ui.label(RichText::new("Parent Bone").strong());
                ui.label(RichText::new("Local Pos (X, Y, Z)").strong());
                ui.label(RichText::new("World Pos Z").strong());
                ui.end_row();

                for (idx, bone) in mesh.bones.iter().enumerate() {
                    let parent_name = bone.parent_index
                        .and_then(|p| mesh.bones.get(p))
                        .map(|b| b.name.as_str())
                        .unwrap_or("(Root)");

                    let world_z = bone.world_transform.transform_point3(glam::Vec3::ZERO).z;

                    ui.label(format!("#{idx}"));
                    ui.label(RichText::new(&bone.name).strong());
                    ui.label(parent_name);
                    ui.label(format!("({:.1}, {:.1}, {:.1})", bone.local_pos.x, bone.local_pos.y, bone.local_pos.z));
                    ui.colored_label(Color32::from_rgb(100, 200, 255), format!("{:.1} uu", world_z));
                    ui.end_row();
                }
            });
    });
}

fn render_sockets_tab(ui: &mut Ui, state: &mut EditorState, mesh_idx: usize) {
    ui.horizontal(|ui| {
        ui.heading("Bone Attachment Sockets & Weapon Mounting");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕ Add Weapon Socket").clicked() {
                let count = state.skeletal_meshes[mesh_idx].sockets.len();
                state.skeletal_meshes[mesh_idx].sockets.push(SkeletalSocket {
                    name: format!("WeaponPoint_Custom_{}", count + 1),
                    bone_name: "Hand_R".to_string(),
                    relative_pos: Vec3::new(0.0, 5.0, 0.0),
                    relative_rot: Quat::IDENTITY,
                    attached_mesh_name: Some("ShockRifle".to_string()),
                });
            }
        });
    });
    ui.label(RichText::new("Manage socket offsets on character limbs to attach Shock Rifles, Flak Cannons, and gear.").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    let sockets_len = state.skeletal_meshes[mesh_idx].sockets.len();

    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        for s_idx in 0..sockets_len {
            let socket_name = state.skeletal_meshes[mesh_idx].sockets[s_idx].name.clone();
            let bone_name = state.skeletal_meshes[mesh_idx].sockets[s_idx].bone_name.clone();

            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(&socket_name).strong().size(13.0));
                    ui.colored_label(Color32::from_rgb(120, 180, 240), format!("Attached to bone: '{}'", bone_name));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let current_attachment = state.skeletal_meshes[mesh_idx].sockets[s_idx].attached_mesh_name.clone().unwrap_or_else(|| "None".to_string());
                        egui::ComboBox::from_id_salt(format!("sock_combo_{}", socket_name))
                            .selected_text(&current_attachment)
                            .show_ui(ui, |ui| {
                                for &item in &["None", "ShockRifle", "FlakCannon", "Enforcer", "SniperRifle", "RocketLauncher"] {
                                    let is_sel = current_attachment == item;
                                    if ui.selectable_label(is_sel, item).clicked() {
                                        let val = if item == "None" { None } else { Some(item.to_string()) };
                                        state.set_skeletal_socket_attachment(mesh_idx, &socket_name, val);
                                    }
                                }
                            });
                        ui.label("Weapon Mount:");
                    });
                });

                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Relative Offset:").size(11.0).color(Color32::GRAY));
                    ui.add(egui::DragValue::new(&mut state.skeletal_meshes[mesh_idx].sockets[s_idx].relative_pos.x).speed(0.1).prefix("X:"));
                    ui.add(egui::DragValue::new(&mut state.skeletal_meshes[mesh_idx].sockets[s_idx].relative_pos.y).speed(0.1).prefix("Y:"));
                    ui.add(egui::DragValue::new(&mut state.skeletal_meshes[mesh_idx].sockets[s_idx].relative_pos.z).speed(0.1).prefix("Z:"));

                    if let Some(mat) = state.skeletal_meshes[mesh_idx].get_socket_world_transform(&socket_name) {
                        let w_pos = mat.transform_point3(glam::Vec3::ZERO);
                        ui.colored_label(Color32::from_rgb(100, 240, 150), format!("Live World Pos: ({:.1}, {:.1}, {:.1})", w_pos.x, w_pos.y, w_pos.z));
                    }
                });
            });
            ui.add_space(4.0);
        }
    });
}

fn render_hitboxes_tab(ui: &mut Ui, state: &mut EditorState, mesh_idx: usize) {
    ui.horizontal(|ui| {
        ui.heading("Locational Damage HitBoxes & Collision Volumes");
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("➕ Add Armor HitBox").clicked() {
                let count = state.skeletal_meshes[mesh_idx].hitboxes.len();
                state.skeletal_meshes[mesh_idx].hitboxes.push(MeshHitBox {
                    name: format!("TorsoArmor_{}", count + 1),
                    bone_name: "Chest".to_string(),
                    offset: Vec3::ZERO,
                    extents: Vec3::new(12.0, 12.0, 12.0),
                    damage_multiplier: 0.6,
                    is_critical_headshot: false,
                });
            }
            if ui.button("🔄 Reset Tournament Multipliers").clicked() {
                for hb in &mut state.skeletal_meshes[mesh_idx].hitboxes {
                    let lower = hb.name.to_lowercase();
                    if lower.contains("head") {
                        hb.damage_multiplier = 2.5;
                        hb.is_critical_headshot = true;
                    } else if lower.contains("chest") || lower.contains("torso") || lower.contains("spine") {
                        hb.damage_multiplier = 1.0;
                        hb.is_critical_headshot = false;
                    } else {
                        hb.damage_multiplier = 0.7;
                        hb.is_critical_headshot = false;
                    }
                }
            }
        });
    });

    ui.label(RichText::new("Oriented bounding box (OBB) volumes for headshots, locational multipliers, and Instagib raycasts (from Gildor's SkelEdit).").size(11.0).color(Color32::GRAY));
    ui.add_space(6.0);

    // Raycast simulation test button
    ui.horizontal(|ui| {
        let hitboxes_count = state.skeletal_meshes[mesh_idx].hitboxes.len();
        ui.label(RichText::new(format!("Active HitBoxes: {hitboxes_count}")).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let hit_id = egui::Id::new("sim_headshot_result");
            let mut last_result = ui.data(|d| d.get_temp::<String>(hit_id));

            if ui.button("🎯 Fire Simulated Sniper Shot (Headshot Test)").clicked() {
                let ray_orig = glam::Vec3::new(0.0, 100.0, 94.0);
                let ray_dir = glam::Vec3::new(0.0, -1.0, 0.0);
                if let Some(hit) = state.skeletal_meshes[mesh_idx].test_hitbox_ray(ray_orig, ray_dir) {
                    let res = format!(
                        "🎯 HIT: '{}' (Bone: '{}') | Damage Multiplier: {:.1}x | Critical Headshot: {}",
                        hit.hitbox_name, hit.bone_name, hit.damage_multiplier, if hit.is_critical_headshot { "YES (2.5x INSTAGIB)" } else { "NO" }
                    );
                    ui.data_mut(|d| d.insert_temp(hit_id, res.clone()));
                    last_result = Some(res);
                }
            }

            if let Some(res) = last_result {
                ui.colored_label(Color32::from_rgb(255, 215, 0), res);
            }
        });
    });

    ui.separator();

    egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
        egui::Grid::new("hitboxes_grid")
            .striped(true)
            .min_col_width(80.0)
            .spacing([10.0, 6.0])
            .show(ui, |ui| {
                ui.label(RichText::new("HitBox Name").strong());
                ui.label(RichText::new("Bone Attachment").strong());
                ui.label(RichText::new("Damage Mult.").strong());
                ui.label(RichText::new("Headshot").strong());
                ui.label(RichText::new("Extents (X, Y, Z)").strong());
                ui.end_row();

                for hb in &mut state.skeletal_meshes[mesh_idx].hitboxes {
                    ui.label(RichText::new(&hb.name).strong());
                    ui.colored_label(Color32::from_rgb(120, 180, 240), &hb.bone_name);

                    ui.add(
                        egui::DragValue::new(&mut hb.damage_multiplier)
                            .speed(0.05)
                            .range(0.1..=10.0)
                            .suffix("x"),
                    );
                    ui.checkbox(&mut hb.is_critical_headshot, "Critical");
                    ui.horizontal(|ui| {
                        ui.add(egui::DragValue::new(&mut hb.extents.x).speed(0.2).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut hb.extents.y).speed(0.2).prefix("Y:"));
                        ui.add(egui::DragValue::new(&mut hb.extents.z).speed(0.2).prefix("Z:"));
                    });
                    ui.end_row();
                }
            });
    });
}

fn render_actorx_tab(ui: &mut Ui, state: &mut EditorState, mesh_idx: usize) {
    ui.label(RichText::new("Epic Games ActorX (.PSK / .PSA) Interoperability").strong().size(13.0));
    ui.label(RichText::new("Import and export standard ActorX skeletal meshes and quaternion animation tracks for 3ds Max, Maya, and Blender.").size(11.0).color(Color32::GRAY));
    ui.add_space(8.0);

    let mesh = &state.skeletal_meshes[mesh_idx];

    ui.group(|ui| {
        ui.label(RichText::new("ActorX File Specifications & Telemetry").strong());
        ui.horizontal(|ui| {
            ui.label(format!("Mesh Name: '{}'", mesh.name));
            ui.separator();
            ui.label(format!("Bones (REFSKELT): {}", mesh.bones.len()));
            ui.separator();
            ui.label(format!("Vertices (PNTS0000): {}", mesh.base_vertices.len()));
            ui.separator();
            ui.label(format!("Triangles (FACE0000): {}", mesh.triangles.len()));
        });

        ui.add_space(4.0);
        ui.label(RichText::new("Binary Chunks: ACTRHEAD • PNTS0000 • VTXW0000 • FACE0000 • MATT0000 • REFSKELT • RAWWEIGHTS").size(11.0).color(Color32::from_rgb(130, 200, 255)));
    });

    ui.add_space(8.0);

    ui.horizontal(|ui| {
        let export_id = egui::Id::new("actorx_export_status");
        let mut status = ui.data(|d| d.get_temp::<String>(export_id));

        if ui.button(RichText::new("💾 Export Mesh to ActorX .PSK").strong()).clicked() {
            let bytes = mesh.to_actorx_psk();
            let msg = format!("✔ Successfully generated ActorX .PSK ({} bytes in memory) ready for Unreal Engine!", bytes.len());
            ui.data_mut(|d| d.insert_temp(export_id, msg.clone()));
            status = Some(msg);
        }

        if let Some(msg) = status {
            ui.colored_label(Color32::from_rgb(80, 230, 120), msg);
        }
    });
}
