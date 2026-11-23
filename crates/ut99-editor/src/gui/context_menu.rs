use crate::editor_state::EditorState;
use crate::gui::icons::{context_menu_item, context_menu_item_danger, paint_icon, Icon};
use egui::{Color32, RichText, Vec2};

pub fn render_viewport_context_menu(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_viewport_context_menu {
        return;
    }

    let mut open = true;
    let ppp = ctx.pixels_per_point().max(0.1);
    let screen = ctx.screen_rect();
    let logical_pos = egui::pos2(
        (state.context_menu_screen_pos.x / ppp).clamp(screen.min.x + 4.0, (screen.max.x - 260.0).max(screen.min.x + 4.0)),
        (state.context_menu_screen_pos.y / ppp).clamp(screen.min.y + 4.0, (screen.max.y - 320.0).max(screen.min.y + 4.0)),
    );

    let win_resp = egui::Window::new("viewport_context_menu_window")
        .fixed_pos(logical_pos)
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .frame(
            egui::Frame::window(&ctx.style())
                .fill(Color32::from_rgba_premultiplied(22, 26, 33, 252))
                .stroke(egui::Stroke::new(1.0_f32, Color32::from_rgb(60, 72, 95)))
                .corner_radius(6.0_f32)
                .inner_margin(egui::Margin::symmetric(6, 6))
        )
        .show(ctx, |ui| {
            ui.set_min_width(230.0);
            let mut close_menu = false;

            // =========================================================================
            // 1. Multi-Selection Active
            // =========================================================================
            if state.has_multi_selection() {
                let b_cnt = state.multi_selected_brushes.len();
                let a_cnt = state.multi_selected_actors.len();
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::SelectAll, Color32::from_rgb(80, 225, 165));
                    ui.label(
                        RichText::new(format!("Multi-Selection ({b_cnt} Brushes, {a_cnt} Actors)"))
                            .strong()
                            .color(Color32::from_rgb(80, 225, 165)),
                    );
                });
                ui.separator();

                if context_menu_item(ui, Some(Icon::TargetFocus), "Focus Selection", Some("F"), None).clicked() {
                    state.focus_camera_on_selection();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignGrid), "Snap All to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                    state.snap_selected_to_grid();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap All to Floor", Some("End"), None).clicked() {
                    state.snap_selected_to_floor();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap Floor Aligned", Some("Shift+End"), None).clicked() {
                    state.snap_selected_to_floor_aligned();
                    close_menu = true;
                }

                if a_cnt >= 2 {
                    ui.menu_button("📏 Align & Distribute Actors", |ui| {
                        if ui.button("Min X").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::X, crate::editor_state::AlignMode::Min);
                            ui.close_menu();
                            close_menu = true;
                        }
                        if ui.button("Center X").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::X, crate::editor_state::AlignMode::Center);
                            ui.close_menu();
                            close_menu = true;
                        }
                        if ui.button("Max X").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::X, crate::editor_state::AlignMode::Max);
                            ui.close_menu();
                            close_menu = true;
                        }
                        ui.separator();
                        if ui.button("Min Y").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::Y, crate::editor_state::AlignMode::Min);
                            ui.close_menu();
                            close_menu = true;
                        }
                        if ui.button("Center Y").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::Y, crate::editor_state::AlignMode::Center);
                            ui.close_menu();
                            close_menu = true;
                        }
                        if ui.button("Max Y").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::Y, crate::editor_state::AlignMode::Max);
                            ui.close_menu();
                            close_menu = true;
                        }
                        ui.separator();
                        if ui.button("Min Z (Floor)").clicked() {
                            state.align_selected_actors(crate::editor_state::AlignAxis::Z, crate::editor_state::AlignMode::Min);
                            ui.close_menu();
                            close_menu = true;
                        }
                        if a_cnt >= 3 {
                            ui.separator();
                            if ui.button("Distribute Along X").clicked() {
                                state.distribute_selected_actors_evenly(crate::editor_state::AlignAxis::X);
                                ui.close_menu();
                                close_menu = true;
                            }
                            if ui.button("Distribute Along Y").clicked() {
                                state.distribute_selected_actors_evenly(crate::editor_state::AlignAxis::Y);
                                ui.close_menu();
                                close_menu = true;
                            }
                        }
                        ui.separator();
                        if ui.button("Open Align & Distribute Studio...").clicked() {
                            state.show_actor_align_dialog = true;
                            ui.close_menu();
                            close_menu = true;
                        }
                    });
                }

                if b_cnt > 0 {
                    if context_menu_item(ui, Some(Icon::SurfaceProperties), "Apply Active Material to All", None, None).clicked() {
                        state.apply_active_texture_to_current_selection();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::InvertNormals), "Invert All Brush Normals", Some("Ctrl+Alt+I"), None).clicked() {
                        for &b_idx in &state.multi_selected_brushes.clone() {
                            state.selected_brush_index = Some(b_idx);
                            state.invert_selected_brush();
                        }
                        close_menu = true;
                    }
                }

                if context_menu_item(ui, Some(Icon::Duplicate), "Duplicate Batch", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::PresetCatalog), "Save as Prefab...", None, None).clicked() {
                    state.create_prefab_from_selection("", "Custom");
                    close_menu = true;
                }

                ui.separator();
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Selection Batch", Some("Del")).clicked() {
                    state.delete_selected_batch();
                    close_menu = true;
                }

            // =========================================================================
            // 2. Single Actor Selected
            // =========================================================================
            } else if let Some(act_idx) = state.selected_actor_index {
                let (act_name, class_name) = state.loaded_unr.as_ref()
                    .and_then(|u| u.actors.get(act_idx))
                    .map(|a| (a.name.clone(), a.class_name.clone()))
                    .unwrap_or_else(|| (format!("Actor #{act_idx}"), "Actor".to_string()));

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::ActorLight, Color32::from_rgb(120, 205, 255));
                    ui.label(
                        RichText::new(format!("{act_name} [{class_name}]"))
                            .strong()
                            .color(Color32::from_rgb(120, 205, 255))
                    );
                });
                ui.separator();

                if context_menu_item(ui, Some(Icon::ActorProperties), "Actor Properties...", Some("Enter"), None).clicked() {
                    state.show_actor_properties = true;
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::TargetFocus), "Focus Viewport Camera", Some("F"), None).clicked() {
                    state.focus_camera_on_selection();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Duplicate), "Duplicate Actor", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    close_menu = true;
                }

                ui.menu_button("📏 Transform & Snap", |ui| {
                    if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor", Some("End"), None).clicked() {
                        state.snap_selected_to_floor();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::AlignFloor), "Snap Floor Aligned to Normal", Some("Shift+End"), None).clicked() {
                        state.snap_selected_to_floor_aligned();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::AlignGrid), "Snap to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                        state.snap_selected_to_grid();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("🔄 Replace With...", |ui| {
                    if let Some(ref cur_cls) = state.selected_class_name.clone() {
                        let pkg = state.selected_class_package.as_deref().unwrap_or("Engine");
                        let full_cls = if cur_cls.contains('.') {
                            cur_cls.clone()
                        } else {
                            format!("{}.{}", pkg, cur_cls)
                        };
                        if ui.button(RichText::new(format!("⭐ Replace With Browser Class ({full_cls})")).strong().color(Color32::from_rgb(110, 240, 160))).clicked() {
                            state.replace_selected_actors_with_class(&full_cls);
                            close_menu = true;
                            ui.close_menu();
                        }
                        ui.separator();
                    }
                    for &(label, cls) in &[
                        ("💡 Light (Engine.Light)", "Engine.Light"),
                        ("🔦 Spotlight (Engine.Spotlight)", "Engine.Spotlight"),
                        ("🚩 PlayerStart (Engine.PlayerStart)", "Engine.PlayerStart"),
                        ("🔷 PathNode (Engine.PathNode)", "Engine.PathNode"),
                        ("⚡ Trigger (Engine.Trigger)", "Engine.Trigger"),
                        ("🌀 Teleporter (Engine.Teleporter)", "Engine.Teleporter"),
                        ("🚀 Kicker (Botpack.Kicker)", "Botpack.Kicker"),
                        ("💊 HealthVial (Botpack.HealthVial)", "Botpack.HealthVial"),
                        ("🏥 MedBox (Botpack.MedBox)", "Botpack.MedBox"),
                        ("🛡 Armor2 (Botpack.Armor2)", "Botpack.Armor2"),
                        ("⚡ ShieldBelt (Botpack.UT_ShieldBelt)", "Botpack.UT_ShieldBelt"),
                        ("🔫 ShockRifle (Botpack.ShockRifle)", "Botpack.ShockRifle"),
                        ("💣 FlakCannon (Botpack.UT_FlakCannon)", "Botpack.UT_FlakCannon"),
                        ("🚀 RocketLauncher (Botpack.UT_Eightball)", "Botpack.UT_Eightball"),
                    ] {
                        if ui.button(label).clicked() {
                            state.replace_selected_actors_with_class(cls);
                            close_menu = true;
                            ui.close_menu();
                        }
                    }
                });

                ui.menu_button("⚡ Tag & Event Wiring", |ui| {
                    if ui.button("👥 Select Matching Tag").clicked() {
                        state.select_actors_matching_tag();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("👥 Select Matching Event").clicked() {
                        state.select_actors_matching_event();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🎯 Select Targeted by Event (Event -> Tag)").clicked() {
                        state.select_actors_targeted_by_event();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🔍 Select Triggering Tag (Tag <- Event)").clicked() {
                        state.select_actors_triggering_tag();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("👥 Select Same Class", |ui| {
                    if ui.button(format!("Select All '{}' Actors in Level", class_name)).clicked() {
                        let matching: Vec<usize> = state.loaded_unr.as_ref().map(|level| {
                            level.actors.iter().enumerate()
                                .filter(|(_, a)| a.class_name.eq_ignore_ascii_case(&class_name))
                                .map(|(idx, _)| idx)
                                .collect()
                        }).unwrap_or_default();
                        if !matching.is_empty() {
                            state.clear_multi_selection();
                            for idx in matching {
                                state.multi_selected_actors.insert(idx);
                            }
                        }
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                let is_locked = state.is_actor_locked(act_idx);
                let lock_label = if is_locked { "🔓 Unlock Actor" } else { "🔒 Lock Actor" };
                if ui.button(lock_label).clicked() {
                    state.toggle_actor_lock(act_idx);
                    close_menu = true;
                }

                ui.separator();
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Actor", Some("Del")).clicked() {
                    state.delete_selected_batch();
                    close_menu = true;
                }

            // =========================================================================
            // 3. Single World Brush Selected (Surface & Geometry)
            // =========================================================================
            } else if let Some(b_idx) = state.selected_brush_index {
                let b = state.world_brushes.get(b_idx);
                let b_name = b.map(|b| b.name.clone()).unwrap_or_else(|| format!("Brush #{b_idx}"));
                let solidity_str = state.get_brush_solidity(b_idx).label();

                // Check if a specific surface on this brush is selected
                let active_surf = state.selected_surfaces.iter().find(|s| s.0 == b_idx).copied();

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::ShapeCube, Color32::from_rgb(255, 215, 110));
                    ui.label(
                        RichText::new(format!("{b_name} [{solidity_str}]"))
                            .strong()
                            .color(Color32::from_rgb(255, 215, 110))
                    );
                });
                ui.separator();

                // Surface-specific operations
                if context_menu_item(ui, Some(Icon::SurfaceProperties), "Surface Properties...", Some("F5"), None).clicked() {
                    state.show_surface_properties = true;
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::TextureFit), "Fit Texture 1x1", Some("Shift+F"), None).clicked() {
                    state.fit_selected_brush_texture(1.0, 1.0);
                    close_menu = true;
                }
                if let Some(ref tex) = state.active_texture {
                    let apply_lbl = format!("Apply Active Material ({tex})");
                    if context_menu_item(ui, Some(Icon::SurfaceProperties), &apply_lbl, None, Some(Color32::from_rgb(255, 230, 110))).clicked() {
                        state.apply_active_texture_to_current_selection();
                        close_menu = true;
                    }
                }
                if let Some((_, p_idx)) = active_surf {
                    if let Some(brush) = state.world_brushes.get(b_idx) {
                        if let Some(p) = brush.polys.get(p_idx) {
                            let tex_name = p.texture.clone();
                            if context_menu_item(ui, Some(Icon::TextureEyedropper), &format!("Sample Texture: '{tex_name}'"), Some("Alt+RMB"), None).clicked() {
                                state.active_texture = Some(tex_name);
                                state.toast_info("Sampled surface texture into active material");
                                close_menu = true;
                            }
                        }
                    }
                }

                ui.menu_button("🎨 Align Textures", |ui| {
                    if ui.button("Planar").clicked() {
                        state.align_selected_brush_texture("planar");
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Box (Automatic Cube Wrap)").clicked() {
                        state.align_selected_brush_texture("box");
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Cylinder (1x Wrap)").clicked() {
                        state.align_selected_brush_cylinder(1.0, 1.0);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Cylinder (2x Wrap)").clicked() {
                        state.align_selected_brush_cylinder(2.0, 1.0);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Cylinder (4x Wrap)").clicked() {
                        state.align_selected_brush_cylinder(4.0, 1.0);
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Wall Direction (Wall X)").clicked() {
                        state.align_selected_brush_texture("wall_x");
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Wall Direction (Wall Y)").clicked() {
                        state.align_selected_brush_texture("wall_y");
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Continuous Wall Strip (1x)").clicked() {
                        state.align_selected_surfaces_continuous(1.0, 1.0);
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("🎯 Select Surfaces", |ui| {
                    if context_menu_item(ui, None, "Matching Texture", Some("Shift+T"), None).clicked() {
                        state.select_surfaces_matching_texture();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, None, "Adjacent Coplanar", Some("Shift+C"), None).clicked() {
                        state.select_surfaces_coplanar_adjacent();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, None, "Adjacent Walls", Some("Shift+W"), None).clicked() {
                        state.select_surfaces_adjacent_walls();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, None, "Adjacent Slants / Floors", Some("Shift+Q"), None).clicked() {
                        state.select_surfaces_adjacent_slants();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, None, "All on Selected Brush", Some("Shift+B"), None).clicked() {
                        state.select_surfaces_on_selected_brushes();
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if context_menu_item(ui, None, "Invert Surface Selection", Some("Ctrl+Shift+I"), None).clicked() {
                        state.invert_surface_selection();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, None, "Deselect Surfaces", Some("Shift+N"), None).clicked() {
                        state.deselect_all_surfaces();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.separator();

                // Brush geometry operations
                if context_menu_item(ui, Some(Icon::TargetFocus), "Focus Viewport Camera", Some("F"), None).clicked() {
                    state.focus_camera_on_selection();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Duplicate), "Duplicate Brush", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    close_menu = true;
                }

                ui.menu_button("🧱 Brush Solidity", |ui| {
                    if ui.button("Solid").clicked() {
                        state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::Solid);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Semi-Solid").clicked() {
                        state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::SemiSolid);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Non-Solid").clicked() {
                        state.set_selected_brush_solidity(crate::editor_state::BrushSolidity::NonSolid);
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("⏮ Brush Order", |ui| {
                    if ui.button("Send to First (Earliest)").clicked() {
                        state.send_selected_brush_to_first();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Send to Last (Latest)").clicked() {
                        state.send_selected_brush_to_last();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Move Up").clicked() {
                        state.move_selected_brush_up();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Move Down").clicked() {
                        state.move_selected_brush_down();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("✂ Polygons (UnrealEd 2.0)", |ui| {
                    if ui.button("📥 To Brush (Copy into Red Builder)").clicked() {
                        state.polygons_to_brush();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("📤 From Brush (Replace from Red Builder)").clicked() {
                        state.polygons_from_brush();
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("🔗 Merge Coplanar Polygons").clicked() {
                        state.merge_selected_brush_polygons();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("✂ Separate Polygons (Triangulate)").clicked() {
                        state.separate_selected_brush_polygons();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("✂ Extract Selected Surface to Builder").clicked() {
                        state.extract_selected_surfaces_to_builder_brush();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("🔄 Convert Brush To...", |ui| {
                    if ui.button("🚪 Add Mover from this Brush").clicked() {
                        state.add_mover_from_builder();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🚫 Flag as Anti-Portal Occluder").clicked() {
                        state.make_selected_brush_anti_portal();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("📦 Save as Reusable Prefab...").clicked() {
                        state.create_prefab_from_selection("", "Custom");
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.menu_button("📐 Transform & Reset", |ui| {
                    if context_menu_item(ui, Some(Icon::InvertNormals), "Invert Normals & Winding", Some("Ctrl+Alt+I"), None).clicked() {
                        state.invert_selected_brush();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::AlignFloor), "Snap Pivot to Nearest Vertex", Some("Ctrl+End"), None).clicked() {
                        state.snap_selected_brush_pivot_to_nearest_vertex(None);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::TargetFocus), "Center Pivot to Bounding Box", None, None).clicked() {
                        state.snap_brush_pivot_to_center();
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Reset Rotation").clicked() {
                        state.reset_rotation_selected();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Reset Scaling").clicked() {
                        state.reset_scaling_selected();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("Transform Permanently (Bake)").clicked() {
                        state.transform_permanently_selected();
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                ui.separator();
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Brush", Some("Del")).clicked() {
                    state.delete_selected_batch();
                    close_menu = true;
                }

            // =========================================================================
            // 4. Single Mover Selected
            // =========================================================================
            } else if let Some(m_idx) = state.selected_mover_index {
                let m_name = state.movers.get(m_idx).map(|m| m.name.clone()).unwrap_or_else(|| format!("Mover #{m_idx}"));
                let cur_k = state.movers.get(m_idx).map(|m| m.current_key).unwrap_or(0);

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::ActorMover, Color32::from_rgb(255, 140, 255));
                    ui.label(
                        RichText::new(format!("{m_name} (Key {cur_k} / 7)"))
                            .strong()
                            .color(Color32::from_rgb(255, 140, 255))
                    );
                });
                ui.separator();

                if context_menu_item(ui, Some(Icon::ActorProperties), "Mover Properties...", Some("Enter"), None).clicked() {
                    state.show_actor_properties = true;
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::TargetFocus), "Focus Camera", Some("F"), None).clicked() {
                    state.focus_camera_on_selection();
                    close_menu = true;
                }

                ui.menu_button("🔑 Switch Keyframe (0..7)", |ui| {
                    for k in 0..8 {
                        let text = if k == cur_k {
                            format!("*Key {k} (Active)*")
                        } else {
                            format!("Key {k}")
                        };
                        if ui.button(text).clicked() {
                            state.set_mover_key(m_idx, k);
                            ui.close_menu();
                            close_menu = true;
                        }
                    }
                });
                if context_menu_item(ui, Some(Icon::Redo), "Next Keyframe", None, None).clicked() {
                    state.cycle_selected_mover_key(true);
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Undo), "Previous Keyframe", None, None).clicked() {
                    state.cycle_selected_mover_key(false);
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap Mover to Floor", Some("End"), None).clicked() {
                    state.snap_selected_to_floor();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Duplicate), "Duplicate Mover", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Wizard), "Mover Studio Dialog...", None, None).clicked() {
                    state.show_mover_studio_dialog = true;
                    close_menu = true;
                }

                ui.separator();
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Mover", Some("Del")).clicked() {
                    if m_idx < state.movers.len() {
                        let mv = state.movers.remove(m_idx);
                        state.history.push(crate::history::Action::DeleteMover { index: m_idx, mover: mv });
                        state.selected_mover_index = None;
                        state.map_is_dirty = true;
                    }
                    close_menu = true;
                }

            // =========================================================================
            // 5. Single Static Mesh Selected
            // =========================================================================
            } else if let Some(sm_idx) = state.selected_static_mesh_index {
                let sm_name = state.static_meshes.get(sm_idx).map(|s| s.name.clone()).unwrap_or_else(|| format!("Mesh #{sm_idx}"));
                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::StaticMesh, Color32::from_rgb(180, 220, 255));
                    ui.label(
                        RichText::new(format!("Static Mesh: {sm_name}"))
                            .strong()
                            .color(Color32::from_rgb(180, 220, 255))
                    );
                });
                ui.separator();

                if context_menu_item(ui, Some(Icon::TargetFocus), "Focus Camera", Some("F"), None).clicked() {
                    state.focus_camera_on_selection();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor", Some("End"), None).clicked() {
                    state.snap_selected_to_floor();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::AlignGrid), "Snap to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                    state.snap_selected_to_grid();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::ActorProperties), "Generate Invisible Collision Hull", None, None).clicked() {
                    state.generate_collision_hull_for_selected_static_mesh();
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Duplicate), "Duplicate Mesh", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    close_menu = true;
                }

                ui.separator();
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Static Mesh", Some("Del")).clicked() {
                    if sm_idx < state.static_meshes.len() {
                        let m = state.static_meshes.remove(sm_idx);
                        state.history.push(crate::history::Action::DeleteStaticMesh { index: sm_idx, mesh: m });
                        state.selected_static_mesh_index = None;
                        state.map_is_dirty = true;
                    }
                    close_menu = true;
                }

            // =========================================================================
            // 6. World Context / Empty Floor / Red Builder Brush
            // =========================================================================
            } else {
                let target_loc = state.context_menu_world_pos.unwrap_or(state.builder_brush.location);

                ui.horizontal(|ui| {
                    let (rect, _) = ui.allocate_exact_size(Vec2::splat(15.0), egui::Sense::hover());
                    paint_icon(ui.painter(), rect, Icon::ShapeCube, Color32::from_rgb(255, 130, 130));
                    ui.label(
                        RichText::new("World Context / Red Builder")
                            .strong()
                            .color(Color32::from_rgb(255, 130, 130))
                    );
                });
                ui.separator();

                // Canon UnrealEd 2.0 Feature: Add Browser Selected Class Here!
                if let Some(ref sel_cls) = state.selected_class_name.clone() {
                    let full_name = if let Some(ref pkg) = state.selected_class_package {
                        if !pkg.is_empty() && !sel_cls.contains('.') {
                            format!("{pkg}.{sel_cls}")
                        } else {
                            sel_cls.clone()
                        }
                    } else {
                        sel_cls.clone()
                    };
                    let add_lbl = format!("⭐ Add '{full_name}' Here");
                    if context_menu_item(ui, Some(Icon::ActorLight), &add_lbl, None, Some(Color32::from_rgb(110, 250, 160))).clicked() {
                        state.place_actor_of_class_at(&full_name, target_loc);
                        close_menu = true;
                    }
                    ui.separator();
                }

                if context_menu_item(ui, Some(Icon::TargetFocus), "Move Red Builder Brush Here", None, None).clicked() {
                    state.builder_brush.location = target_loc;
                    state.log(format!("Moved Builder Brush to ({:.0}, {:.0}, {:.0})", target_loc.x, target_loc.y, target_loc.z));
                    state.toast_info("Moved Builder Brush here");
                    close_menu = true;
                }

                ui.menu_button("💡 Add Quick Actor Here", |ui| {
                    if ui.button("💡 Light (Engine.Light)").clicked() {
                        state.place_actor_of_class_at("Light", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🔦 Spotlight (Engine.Spotlight)").clicked() {
                        state.place_actor_of_class_at("Spotlight", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🚩 PlayerStart (Engine.PlayerStart)").clicked() {
                        state.place_actor_of_class_at("PlayerStart", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🔷 PathNode (Engine.PathNode)").clicked() {
                        state.place_actor_of_class_at("PathNode", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("⚡ Trigger (Engine.Trigger)").clicked() {
                        state.place_actor_of_class_at("Trigger", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    if ui.button("🌀 Teleporter (Engine.Teleporter)").clicked() {
                        state.place_actor_of_class_at("Teleporter", target_loc);
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    ui.menu_button("🔫 Weapons", |ui| {
                        for &(lbl, cls) in &[
                            ("Shock Rifle", "ShockRifle"),
                            ("Flak Cannon", "UT_FlakCannon"),
                            ("Rocket Launcher", "UT_Eightball"),
                            ("Sniper Rifle", "SniperRifle"),
                            ("Minigun", "Minigun2"),
                            ("Bio Rifle", "UT_BioRifle"),
                            ("Pulse Gun", "PulseGun"),
                            ("Redeemer", "WarheadLauncher"),
                        ] {
                            if ui.button(lbl).clicked() {
                                state.place_actor_of_class_at(cls, target_loc);
                                ui.close_menu();
                                close_menu = true;
                            }
                        }
                    });
                    ui.menu_button("💊 Pickups & Health", |ui| {
                        for &(lbl, cls) in &[
                            ("Health Pack (+20)", "MedBox"),
                            ("Health Vial (+5)", "HealthVial"),
                            ("Body Armor (+100)", "Armor2"),
                            ("Thigh Pads (+50)", "ThighPads"),
                            ("Shield Belt (+150)", "UT_ShieldBelt"),
                            ("Damage Amplifier (UDamage)", "UDamage"),
                            ("Jump Boots", "JumpBoots"),
                        ] {
                            if ui.button(lbl).clicked() {
                                state.place_actor_of_class_at(cls, target_loc);
                                ui.close_menu();
                                close_menu = true;
                            }
                        }
                    });
                });

                ui.menu_button("➕ CSG Operations at Builder", |ui| {
                    if context_menu_item(ui, Some(Icon::CsgAdd), "CSG Add (Solid)", Some("Ctrl+A"), None).clicked() {
                        state.csg_add();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::CsgSubtract), "CSG Subtract (Carve)", Some("Ctrl+S"), None).clicked() {
                        state.csg_subtract();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::CsgIntersect), "CSG Intersect", None, None).clicked() {
                        state.csg_intersect();
                        ui.close_menu();
                        close_menu = true;
                    }
                    if context_menu_item(ui, Some(Icon::CsgDeintersect), "CSG Deintersect", None, None).clicked() {
                        state.csg_deintersect();
                        ui.close_menu();
                        close_menu = true;
                    }
                    ui.separator();
                    if ui.button("Add Special Brush...").clicked() {
                        state.show_add_special_dialog = true;
                        ui.close_menu();
                        close_menu = true;
                    }
                });

                if context_menu_item(ui, Some(Icon::Paste), "Paste from Clipboard", Some("Ctrl+V"), None).clicked() {
                    state.paste_from_clipboard();
                    close_menu = true;
                }

                ui.menu_button("📏 Grid Snapping Step", |ui| {
                    for &step in &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0] {
                        let is_active = (state.grid_step - step).abs() < 0.1;
                        if ui.selectable_label(is_active, format!("{step:.0} uu")).clicked() {
                            state.set_grid_step(step);
                            ui.close_menu();
                            close_menu = true;
                        }
                    }
                });

                ui.separator();
                if context_menu_item(ui, Some(Icon::CommandPalette), "Command Palette...", Some("Ctrl+P"), None).clicked() {
                    state.show_command_palette = true;
                    close_menu = true;
                }
                if context_menu_item(ui, Some(Icon::Wizard), "Action Wizards Hub...", Some("Ctrl+Shift+A"), None).clicked() {
                    state.show_action_wizards_dialog = true;
                    close_menu = true;
                }
            }

            if close_menu {
                open = false;
            }
        });

    if let Some(inner) = win_resp {
        if inner.response.clicked_elsewhere() || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            open = false;
        }
    }

    if !open {
        state.show_viewport_context_menu = false;
    }
}

