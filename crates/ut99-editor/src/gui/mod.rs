pub mod acoustics_dialog;
pub mod actor_align_dialog;
pub mod actor_browser;
pub mod actor_properties;
pub mod autosave_dialog;
pub mod bookmarks_dialog;
pub mod brush_scale_dialog;
pub mod brush_shear_dialog;
pub mod build_options_dialog;
pub mod find_actors_dialog;
pub mod map_statistics_dialog;
pub mod theme;
pub mod icons;
pub mod bot_simulator;
pub mod budget_inspector;
pub mod dispatcher_dialog;
pub mod gameplay_balance_dialog;
pub mod group_browser;
pub mod level_doctor;
pub mod lighting_dialog;
pub mod map_packaging_dialog;
pub mod save_feedback_dialog;
pub mod music_browser;
pub mod play_settings_dialog;
pub mod settings_dialog;
pub mod prefab_browser;
pub mod shape_editor;
pub mod sound_browser;
pub mod special_brush_dialog;
pub mod surface_properties;
pub mod texture_browser;
pub mod zone_seal_dialog;
pub mod ctf_symmetry_dialog;
pub mod command_palette;
pub mod minimap;
pub mod replace_textures_dialog;
pub mod level_properties_dialog;
pub mod scale_map_dialog;
pub mod mesh_browser;
pub mod procedural_texture_studio;
pub mod package_dependency_dialog;
pub mod decal_studio_dialog;
pub mod layers_outliner;
pub mod uc_viewer_dialog;
pub mod skybox_studio_dialog;
pub mod acoustic_studio_dialog;
pub mod warp_zone_dialog;
pub mod lighting_studio_dialog;
pub mod zone_fog_dialog;
pub mod skeletal_studio_dialog;
pub mod skin_studio_dialog;
pub mod package_extractor_dialog;
pub mod uc_class_wizard_dialog;
pub mod particle_studio_dialog;
pub mod bot_nav_studio_dialog;
pub mod architectural_prefab_dialog;
pub mod terrain_studio_dialog;
pub mod mesh_lod_collision_dialog;
pub mod cinematic_director_dialog;
pub mod light_effects_studio_dialog;
pub mod zone_occlusion_studio_dialog;
pub mod music_studio_dialog;
pub mod corona_studio_dialog;
pub mod fluid_studio_dialog;
pub mod ballistics_studio_dialog;
pub mod teleporter_studio_dialog;
pub mod translocator_studio_dialog;
pub mod lift_pathing_dialog;
pub mod water_studio_dialog;
pub mod mover_studio_dialog;
pub mod team_objective_dialog;
pub mod destructible_studio_dialog;
pub mod kicker_studio_dialog;
pub mod bsp_quality_studio_dialog;
pub mod shape_loft_studio_dialog;
pub mod stairway_studio_dialog;
pub mod mesh_scatter_studio_dialog;
pub mod light_shaft_studio_dialog;
pub mod track_mover_studio_dialog;
pub mod surveillance_studio_dialog;
pub mod laser_grid_studio_dialog;
pub mod ventilator_studio_dialog;
pub mod conveyor_studio_dialog;
pub mod turret_studio_dialog;
pub mod soundscape_studio_dialog;
pub mod hazard_chamber_studio_dialog;
pub mod pipe_conduit_studio_dialog;
pub mod hologram_studio_dialog;
pub mod weather_studio_dialog;
pub mod arena_rules_dialog;
pub mod armor_stacking_dialog;
pub mod bot_jump_routes_dialog;
pub mod weapon_circuit_dialog;
pub mod powerup_control_dialog;
pub mod spawn_distribution_dialog;
pub mod domination_dialog;
pub mod assault_dialog;
pub mod radar_dialog;
pub mod ctf_studio_dialog;
pub mod team_cannon_dialog;
pub mod bio_trap_dialog;
pub mod trigger_network_dialog;
pub mod elevator_dialog;
pub mod zone_velocity_dialog;
pub mod hazard_barrel_dialog;
pub mod special_lit_dialog;
pub mod sniper_vantage_dialog;
pub mod secret_door_dialog;
pub mod teleporter_arch_dialog;
pub mod fluid_basin_dialog;
pub mod alarm_beacon_dialog;
pub mod supply_locker_dialog;
pub mod skylight_dialog;
pub mod vent_duct_dialog;
pub mod piston_dialog;
pub mod jump_boots_dialog;
pub mod force_field_dialog;
pub mod reactor_core_dialog;
pub mod help_dialog;
pub mod shortcut_help_dialog;
pub mod context_menu;
pub mod action_wizards;
pub mod visual_presets_dialog;
pub mod map_wizard_dialog;
pub mod welcome_wizard;
pub mod modal;
pub mod toolbar;
pub mod widgets;


use crate::editor_state::{BuilderShape, EditorMode, EditorState, InspectorPanelMode};
use crate::gui::icons::{context_menu_item, context_menu_item_danger, Icon};
use crate::gui::theme::{ProTooltipExt, SemanticPalette};
use crate::i18n::tr;
use egui::Color32;
use glam::Vec3;


pub fn render_ui(ctx: &egui::Context, state: &mut EditorState) {
    // Process staged save / export progress pipeline
    state.process_pending_save_operation(ctx);

    // Render Rubber-Band Marquee Selection overlay
    if let Some(marquee) = state.active_marquee {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("marquee_overlay")));
        let scale = ctx.pixels_per_point().max(0.1);
        let p1 = egui::pos2(marquee.start_screen.0 / scale, marquee.start_screen.1 / scale);
        let p2 = egui::pos2(marquee.current_screen.0 / scale, marquee.current_screen.1 / scale);
        let rect = egui::Rect::from_two_pos(p1, p2);

        painter.rect_filled(
            rect,
            0.0,
            egui::Color32::from_rgba_unmultiplied(47, 128, 237, 35),
        );
        painter.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(47, 128, 237)),
            egui::StrokeKind::Middle,
        );
    }

    // Real-Time Transformation HUD Badge during active Gizmo drag or surface magnet
    if state.active_gizmo_axis != crate::raycast::GizmoAxis::None || state.cursor_delta_text.is_some() {
        let badge_pos = ctx
            .input(|i| i.pointer.hover_pos())
            .map(|p| p + egui::vec2(18.0, -28.0))
            .unwrap_or_else(|| egui::pos2(420.0, 96.0));
        egui::Area::new(egui::Id::new("pro_302_gizmo_hud_badge"))
            .order(egui::Order::Tooltip)
            .fixed_pos(badge_pos)
            .interactable(false)
            .show(ctx, |ui| {
                let out_of_bounds = crate::transform_session::exceeds_world_limit(state);
                let stroke_color = if out_of_bounds {
                    egui::Color32::from_rgb(255, 75, 75)
                } else {
                    egui::Color32::from_rgb(80, 175, 255)
                };
                egui::Frame::NONE
                    .fill(egui::Color32::from_rgba_premultiplied(15, 18, 24, 235))
                    .stroke(egui::Stroke::new(1.0_f32, stroke_color))
                    .corner_radius(4.0_f32)
                    .inner_margin(egui::Margin::symmetric(8, 4))
                    .show(ui, |ui| {
                        let text = if let Some(ref custom_text) = state.cursor_delta_text {
                            custom_text.clone()
                        } else {
                            match state.gizmo_mode {
                                crate::raycast::GizmoMode::Translate => {
                                    let d = state.active_gizmo_delta;
                                    format!("ΔX: {:+.0}  ΔY: {:+.0}  ΔZ: {:+.0} UU", d.x, d.y, d.z)
                                }
                                crate::raycast::GizmoMode::Rotate => {
                                    format!("Δθ: {:+.1}° ({:?})", state.active_gizmo_angle_deg, state.active_gizmo_axis)
                                }
                                crate::raycast::GizmoMode::Scale => {
                                    format!("Scale ({:?})", state.active_gizmo_axis)
                                }
                            }
                        };
                        ui.label(
                            egui::RichText::new(text)
                                .monospace()
                                .strong()
                                .size(11.5)
                                .color(if out_of_bounds { egui::Color32::from_rgb(255, 120, 120) } else { egui::Color32::from_rgb(130, 225, 255) }),
                        );
                        if out_of_bounds {
                            ui.label(
                                egui::RichText::new("⚠️ Exceeds World Limit (65,536)")
                                    .size(10.0)
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 80, 80)),
                            );
                        }
                        ui.label(
                            egui::RichText::new("Esc: Cancel  Ctrl: Invert Snap  Shift: Fine  X/Y/Z: Axis")
                                .size(9.5)
                                .color(egui::Color32::from_rgb(160, 175, 195)),
                        );
                    });
            });
    }

    // 1. Top Menu Bar
    egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
        egui::menu::bar(ui, |ui| {
            let lang = state.language;
            // -----------------------------------------------------------------
            // 1. FILE MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.file"), |ui| {
                if context_menu_item(ui, Some(Icon::FileNew), "New Map", Some("Ctrl+N"), None).clicked() {
                    if state.map_is_dirty {
                        state.pending_dirty_action = Some(crate::editor_state::PendingDirtyAction::NewMap);
                        state.show_unsaved_exit_modal = true;
                    } else {
                        state.new_map();
                    }
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Wizard), "New Map Wizard...", Some("Ctrl+Shift+N"), Some(Color32::from_rgb(110, 240, 160))).clicked() {
                    state.show_map_wizard_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::FileOpen), "Open .unr Map...", Some("Ctrl+O"), None).clicked() {
                    state.prompt_or_open_unr_dialog();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::FileOpen), "Open T3D Map...", None, None).clicked() {
                    state.prompt_or_open_t3d_dialog();
                    ui.close_menu();
                }
                if !state.recent_maps.is_empty() {
                    ui.menu_button("📂 Recent Maps", |ui| {
                        let mut to_load = None;
                        for map_path in &state.recent_maps {
                            let label = map_path.file_name()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_else(|| map_path.display().to_string());
                            if ui.button(&label).on_hover_text(map_path.display().to_string()).clicked() {
                                to_load = Some(map_path.clone());
                                ui.close_menu();
                            }
                        }
                        if let Some(path) = to_load {
                            state.prompt_or_open_map_path(path);
                        }
                    });
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::FileSave), "Save .unr Map", Some("Ctrl+S"), None).clicked() {
                    if let Some(ref path) = state.map_path.clone() {
                        state.trigger_save_unr(path);
                    } else {
                        let default_dir = state.default_maps_directory();
                        let initial_name = state.map_path.as_ref()
                            .and_then(|p| p.file_name())
                            .and_then(|n| n.to_str())
                            .unwrap_or("Untitled.unr");
                        let dialog = rfd::FileDialog::new()
                            .add_filter("Unreal Map (*.unr)", &["unr"])
                            .set_directory(&default_dir)
                            .set_file_name(initial_name);
                        if let Some(path) = dialog.save_file() {
                            state.trigger_save_unr(&path);
                        }
                    }
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::FileSave), "Save .unr Map As...", None, None).clicked() {
                    let default_dir = state.default_maps_directory();
                    let initial_name = state.map_path.as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled.unr");
                    let dialog = rfd::FileDialog::new()
                        .add_filter("Unreal Map (*.unr)", &["unr"])
                        .set_directory(&default_dir)
                        .set_file_name(initial_name);
                    if let Some(path) = dialog.save_file() {
                        state.trigger_save_unr(&path);
                    }
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::FileSave), "Save Native .unr (No UCC)...", None, None).clicked() {
                    let default_dir = state.default_maps_directory();
                    let initial_name = state.map_path.as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("Untitled.unr");
                    let dialog = rfd::FileDialog::new()
                        .add_filter("Unreal Map (*.unr)", &["unr"])
                        .set_directory(&default_dir)
                        .set_file_name(initial_name);
                    if let Some(path) = dialog.save_file() {
                        state.trigger_save_unr_native(&path);
                    }
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("📥 Import", |ui| {
                    if context_menu_item(ui, Some(Icon::FileOpen), "Import T3D Into Current Map (Merge)...", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        let default_dir = state.default_maps_directory();
                        let dialog = rfd::FileDialog::new()
                            .add_filter("Unreal Text 3D (*.t3d)", &["t3d"])
                            .set_directory(&default_dir);
                        if let Some(path) = dialog.pick_file() {
                            if let Err(e) = state.import_t3d_into_existing_map(&path) {
                                state.log(format!("Error merging T3D map: {e}"));
                            }
                        }
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::FileOpen), "Import 3D Model (.obj) into Builder...", None, None).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                            .pick_file()
                        {
                            if let Ok(content) = std::fs::read_to_string(&path) {
                                if let Err(e) = state.import_obj_to_builder(&content) {
                                    state.log(format!("Error importing OBJ: {e}"));
                                }
                            }
                        }
                        ui.close_menu();
                    }
                });
                ui.menu_button("📤 Export", |ui| {
                    if context_menu_item(ui, Some(Icon::FileSave), "Export T3D Map...", None, None).clicked() {
                        let default_dir = state.default_maps_directory();
                        let initial_name = state.map_path.as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|s| format!("{}.t3d", s.to_string_lossy()))
                            .unwrap_or_else(|| "Untitled.t3d".to_string());
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Unreal Text 3D (*.t3d)", &["t3d"])
                            .set_directory(&default_dir)
                            .set_file_name(&initial_name)
                            .save_file()
                        {
                            state.trigger_export_t3d(&path);
                        }
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::FileSave), "Export Map to Wavefront OBJ...", None, None).clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                            .save_file()
                        {
                            let obj_data = state.export_map_to_obj("LevelExport");
                            let _ = std::fs::write(&path, obj_data);
                            state.log(format!("Exported map geometry to OBJ: {:?}", path));
                        }
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::FileSave), "Export Selected Brush to OBJ...", None, None).clicked() {
                        if let Some(obj_data) = state.export_selected_brush_to_obj() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                                .save_file()
                            {
                                let _ = std::fs::write(&path, obj_data);
                                state.log(format!("Exported brush to OBJ: {:?}", path));
                            }
                        }
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::FileSave), "Export Builder Brush to OBJ...", None, None).clicked() {
                        let default_dir = state.default_maps_directory();
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Wavefront OBJ (*.obj)", &["obj"])
                            .set_directory(&default_dir)
                            .set_file_name("BuilderBrush.obj")
                            .save_file()
                        {
                            state.trigger_export_obj(&path);
                        }
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::FileSave), "Export Map to GLTF 2.0 (.gltf)...", None, Some(Color32::from_rgb(110, 230, 240))).clicked() {
                        let default_dir = state.default_maps_directory();
                        let initial_name = state.map_path.as_ref()
                            .and_then(|p| p.file_stem())
                            .map(|s| format!("{}.gltf", s.to_string_lossy()))
                            .unwrap_or_else(|| "Untitled.gltf".to_string());
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("GL Transmission Format (*.gltf)", &["gltf"])
                            .set_directory(&default_dir)
                            .set_file_name(&initial_name)
                            .save_file()
                        {
                            state.trigger_export_gltf(&path);
                        }
                        ui.close_menu();
                    }
                });
                ui.separator();
                if context_menu_item(ui, Some(Icon::Settings), "Auto-Save & Recovery...", None, None).clicked() {
                    state.show_autosave_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Settings), "Map Packaging & Release...", None, None).clicked() {
                    state.show_map_packaging_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item_danger(ui, None, "Exit", None).clicked() {
                    if state.map_is_dirty {
                        state.pending_dirty_action = Some(crate::editor_state::PendingDirtyAction::ExitApp);
                        state.show_unsaved_exit_modal = true;
                    } else {
                        state.confirm_exit_without_saving = true;
                    }
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 2. EDIT MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.edit"), |ui| {
                let undo_lbl = if let Some(desc) = state.next_undo_description() {
                    format!("Undo: {desc}")
                } else {
                    "Undo".to_string()
                };
                if context_menu_item(ui, Some(Icon::Undo), &undo_lbl, Some("Ctrl+Z"), None).clicked() {
                    state.undo();
                    ui.close_menu();
                }

                let redo_lbl = if let Some(desc) = state.next_redo_description() {
                    format!("Redo: {desc}")
                } else {
                    "Redo".to_string()
                };
                if context_menu_item(ui, Some(Icon::Redo), &redo_lbl, Some("Ctrl+Y"), None).clicked() {
                    state.redo();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::Cut), "Cut", Some("Ctrl+X"), None).clicked() {
                    state.cut_selected_to_clipboard();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Copy), "Copy", Some("Ctrl+C"), None).clicked() {
                    state.copy_selected_to_clipboard();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Paste), "Paste", Some("Ctrl+V"), None).clicked() {
                    state.paste_from_clipboard();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Copy), "Duplicate Selected", Some("Ctrl+D"), None).clicked() {
                    state.duplicate_selected();
                    ui.close_menu();
                }
                if context_menu_item_danger(ui, Some(Icon::Delete), "Delete Selected", Some("Del")).clicked() {
                    state.delete_selected_batch();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor", Some("End"), None).clicked() {
                    state.snap_selected_to_floor();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::AlignFloor), "Snap to Floor Aligned", Some("Shift+End"), None).clicked() {
                    state.snap_selected_to_floor_aligned();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::AlignGrid), "Snap to Grid", Some("Ctrl+Alt+G"), None).clicked() {
                    state.snap_selected_to_grid();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::SelectAll), "Select All", Some("Ctrl+A"), None).clicked() {
                    state.select_all();
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "Invert Selection", Some("Ctrl+I"), None).clicked() {
                    state.invert_selection();
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "Deselect All", Some("Esc"), None).clicked() {
                    state.deselect_all();
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Select Brushes", |ui| {
                    if context_menu_item(ui, Some(Icon::SelectAll), "Select All Brushes", None, None).clicked() {
                        state.select_all_brushes();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::CsgAdd), "➕ Additive Brushes (Solid)", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.select_additive_brushes();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::CsgSubtract), "➖ Subtractive Brushes (Hollow)", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                        state.select_subtractive_brushes();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::Wizard), "🌿 Semi-Solid Brushes", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                        state.select_semisolid_brushes();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "⚡ Non-Solid Brushes", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.select_nonsolid_brushes();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Select Actors", |ui| {
                    if context_menu_item(ui, Some(Icon::SelectAll), "Select All Actors", None, None).clicked() {
                        state.select_all_actors();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeCube), "📦 Inside Red Builder Brush", None, Some(Color32::from_rgb(255, 120, 120))).clicked() {
                        state.select_inside_builder_brush();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ActorLight), "💡 All Lights", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.select_all_lights();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ActorPathNode), "🚩 All Navigation Points (PathNodes)", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                        state.select_all_pathnodes();
                        ui.close_menu();
                    }
                    ui.separator();
                    if context_menu_item(ui, None, "Matching Tag", None, None).clicked() {
                        state.select_actors_matching_tag();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Matching Event", None, None).clicked() {
                        state.select_actors_matching_event();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Targeted by Event (Event -> Tag)", None, None).clicked() {
                        state.select_actors_targeted_by_event();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Triggering Tag (Tag <- Event)", None, None).clicked() {
                        state.select_actors_triggering_tag();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Select Surfaces", |ui| {
                    if context_menu_item(ui, Some(Icon::SelectAll), "Select All Surfaces", Some("Shift+S"), None).clicked() {
                        state.select_all_surfaces();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::TextureFit), "Matching Texture", Some("Shift+T"), Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.select_surfaces_matching_texture();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Adjacent Coplanar", Some("Shift+C"), None).clicked() {
                        state.select_surfaces_coplanar_adjacent();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "All Coplanar", None, None).clicked() {
                        state.select_surfaces_coplanar_all();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Adjacent Walls", Some("Shift+W"), None).clicked() {
                        state.select_surfaces_adjacent_walls();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Adjacent Slants / Floors", Some("Shift+Q"), None).clicked() {
                        state.select_surfaces_adjacent_slants();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "All on Selected Brush(es)", Some("Shift+B"), None).clicked() {
                        state.select_surfaces_on_selected_brushes();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Continuous Wall Strip (1x)", None, None).clicked() {
                        state.align_selected_surfaces_continuous(1.0, 1.0);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::InvertNormals), "Invert Surface Normals", None, None).clicked() {
                        state.invert_selected_surfaces();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Invert Surface Selection", Some("Ctrl+Shift+I"), None).clicked() {
                        state.invert_surface_selection();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Deselect Surfaces", Some("Shift+N"), None).clicked() {
                        state.deselect_all_surfaces();
                        ui.close_menu();
                    }
                });
                ui.separator();
                if context_menu_item(ui, None, "Find Actors...", Some("Ctrl+F"), None).clicked() {
                    state.show_find_actors_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::TextureFit), "Replace Textures...", Some("Ctrl+R"), None).clicked() {
                    state.show_replace_textures_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::AlignDistribution), "Align & Distribute Actors...", None, None).clicked() {
                    state.show_actor_align_dialog = true;
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::Settings), "Preferences & Settings...", Some("Ctrl+,"), None).clicked() {
                    state.show_settings_dialog = true;
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 3. VIEW MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.view"), |ui| {
                ui.menu_button("Viewport Layout", |ui| {
                    if ui.selectable_label(state.viewport_layout == crate::camera::ViewportLayout::Single, "Single Viewport (Space)").clicked() {
                        state.viewport_layout = crate::camera::ViewportLayout::Single;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.viewport_layout == crate::camera::ViewportLayout::FourViewports, "2x2 Multi-Viewport (Space)").clicked() {
                        state.viewport_layout = crate::camera::ViewportLayout::FourViewports;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Active Viewport", |ui| {
                    if ui.selectable_label(state.active_viewport_type == crate::camera::ViewportType::Perspective, "3D Perspective (F1)").clicked() {
                        state.active_viewport_type = crate::camera::ViewportType::Perspective;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.active_viewport_type == crate::camera::ViewportType::Top, "Top [X-Y] (F2)").clicked() {
                        state.active_viewport_type = crate::camera::ViewportType::Top;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.active_viewport_type == crate::camera::ViewportType::Front, "Front [X-Z] (F3)").clicked() {
                        state.active_viewport_type = crate::camera::ViewportType::Front;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.active_viewport_type == crate::camera::ViewportType::Side, "Side [Y-Z] (Shift+F4)").clicked() {
                        state.active_viewport_type = crate::camera::ViewportType::Side;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Field of View (FOV)", |ui| {
                    for (label, deg) in [
                        ("65° (Default)", 65.0),
                        ("80°", 80.0),
                        ("90° (UT Standard)", 90.0),
                        ("100° (Wide)", 100.0),
                        ("110° (Ultra-Wide)", 110.0),
                    ] {
                        let is_active = (state.get_camera_fov_degrees() - deg).abs() < 1.0;
                        if ui.selectable_label(is_active, label).clicked() {
                            state.set_camera_fov_degrees(deg);
                            ui.close_menu();
                        }
                    }
                });
                if context_menu_item(ui, None, "Camera Bookmarks...", Some("0..9"), None).clicked() {
                    state.show_camera_bookmarks_dialog = true;
                    ui.close_menu();
                }
                let fs_label = if state.is_fullscreen { "Exit Fullscreen" } else { "Toggle Fullscreen" };
                if context_menu_item(ui, None, fs_label, Some("F11 / Alt+Enter"), None).clicked() {
                    state.pending_toggle_fullscreen = true;
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Background Image (2D Viewports)", |ui| {
                    if ui.button("📂 Open Background Image...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Reference Images (*.png, *.bmp, *.pcx, *.jpg)", &["png", "bmp", "pcx", "jpg"])
                            .pick_file()
                        {
                            let mode = state.background_display_mode;
                            state.set_background_image(Some(path.display().to_string()), mode);
                        }
                        ui.close_menu();
                    }
                    if ui.button("🗑 Clear Background Image").clicked() {
                        let mode = state.background_display_mode;
                        state.set_background_image(None, mode);
                        ui.close_menu();
                    }
                    ui.separator();
                    let cur_path = state.background_image_path.clone();
                    if ui.selectable_label(state.background_display_mode == crate::editor_state::BackgroundDisplayMode::Center, "Center").clicked() {
                        state.set_background_image(cur_path.clone(), crate::editor_state::BackgroundDisplayMode::Center);
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.background_display_mode == crate::editor_state::BackgroundDisplayMode::Tile, "Tile").clicked() {
                        state.set_background_image(cur_path.clone(), crate::editor_state::BackgroundDisplayMode::Tile);
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.background_display_mode == crate::editor_state::BackgroundDisplayMode::Stretch, "Stretch").clicked() {
                        state.set_background_image(cur_path, crate::editor_state::BackgroundDisplayMode::Stretch);
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.menu_button("Viewport Overlays & Visualizers", |ui| {
                    ui.checkbox(&mut state.show_ai_paths, "Show Bot AI Navigation Paths");
                    ui.checkbox(&mut state.show_actor_radii, "Radii View: Collision & Attenuation");
                    ui.checkbox(&mut state.show_event_links, "Show Tag/Event Links");
                    ui.checkbox(&mut state.show_trajectories, "Show Flight Trajectories & Teleport Arcs");
                    ui.checkbox(&mut state.show_skybox_links, "Show SkyZone & Fake Backdrop Links");
                    ui.checkbox(&mut state.show_audio_zones, "Show Zone Audio Resonance Cages");
                    ui.checkbox(&mut state.show_warpzones, "Show WarpZone Portals & Ribbons");
                    ui.checkbox(&mut state.show_anti_portals, "Show Anti-Portal Occluders");
                    ui.checkbox(&mut state.show_skeletal_bones, "Show Skeletal Rig & Bones");
                    ui.checkbox(&mut state.show_particles, "Show Dynamic Particle Emitters");
                    ui.checkbox(&mut state.show_diagnostic_markers, "Level Doctor 3D Beacons");
                    ui.checkbox(&mut state.show_mover_polys, "Mover: Show Solid Polys (UnrealEd 2)");
                    ui.checkbox(&mut state.render_lux_heatmap, "Luxmeter Lighting Heatmap (Shift+F6)");
                    ui.checkbox(&mut state.show_nav_coverage, "Bot Navigation Coverage Heatmap (F9)");
                    ui.checkbox(&mut state.distance_fog_enabled, "Real-Time Distance Fog (F10)");
                    ui.checkbox(&mut state.zone_fog_settings.show_distance_shells, "Show Fog Distance Attenuation Shells");
                });
                let cull_label = if state.frustum_culling_enabled {
                    let (_rb, cb, _ra, ca) = state.last_cull_stats;
                    format!("Dynamic Frustum Culling [Active: -{}b, -{}a]", cb, ca)
                } else {
                    "Dynamic Frustum Culling".to_string()
                };
                if ui.checkbox(&mut state.frustum_culling_enabled, cull_label).changed() {
                    let s = if state.frustum_culling_enabled { "ENABLED" } else { "DISABLED" };
                    state.log(format!("Dynamic View Frustum Culling Engine {}", s));
                }
                ui.separator();
                ui.menu_button("Keymap Profile", |ui| {
                    if ui.selectable_label(state.keymap_profile == crate::editor_state::KeymapProfile::Modern, "Modern (UE5 / Blender)").clicked() {
                        state.keymap_profile = crate::editor_state::KeymapProfile::Modern;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.keymap_profile == crate::editor_state::KeymapProfile::ClassicUnrealEd2, "Classic UnrealEd 2.0 (B, F4..F8, Shift+B/C/F/I/N/S/T/W)").clicked() {
                        state.keymap_profile = crate::editor_state::KeymapProfile::ClassicUnrealEd2;
                        ui.close_menu();
                    }
                });
                ui.menu_button("Target Engine Compatibility", |ui| {
                    if ui.selectable_label(state.target_engine == crate::editor_state::TargetEngine::Ut99Standard, "UT99 Standard (v436 / v451 / v469)").clicked() {
                        state.target_engine = crate::editor_state::TargetEngine::Ut99Standard;
                        ui.close_menu();
                    }
                    if ui.selectable_label(state.target_engine == crate::editor_state::TargetEngine::Unreal227, "Unreal 227 Extended (Weather / StaticMesh)").clicked() {
                        state.target_engine = crate::editor_state::TargetEngine::Unreal227;
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("UI Scale:");
                    let mut pct = (state.ui_scale * 100.0).round() as i32;
                    if ui.add(egui::DragValue::new(&mut pct).range(75..=250).suffix("%")).changed() {
                        state.ui_scale = pct as f32 / 100.0;
                    }
                });
            });

            // -----------------------------------------------------------------
            // 4. BRUSH MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.brush"), |ui| {
                ui.menu_button("Builder Presets", |ui| {
                    if context_menu_item(ui, Some(Icon::ShapeCube), "Cube 256 x 256 x 256", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.set_builder_cube(Vec3::splat(256.0));
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeCube), "Room 1024 x 1024 x 512", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.set_builder_cube(Vec3::new(1024.0, 1024.0, 512.0));
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeCylinder), "Pillar (Cylinder R:64, H:256)", None, None).clicked() {
                        state.set_builder_cylinder(256.0, 64.0, 16);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeSpiralStairs), "Spiral Staircase (16 steps)", None, None).clicked() {
                        state.set_builder_spiral_stairs(16, 16.0, 64.0, 128.0, 360.0, true);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeTorus), "Torus (Tarquin Ring R:256, Tube:64)", None, None).clicked() {
                        state.set_builder_torus(256.0, 64.0, 16, 8);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ShapeTerrain), "Terrain (8x8 Grid)", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                        state.set_builder_terrain(8, 8, 128.0, 64.0, 42);
                        ui.close_menu();
                    }
                });
                ui.menu_button("Hollow Room", |ui| {
                    if context_menu_item(ui, None, "Make Hollow (16 uu walls)", None, None).clicked() {
                        state.make_builder_brush_hollow(16.0);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Make Hollow (32 uu walls)", None, None).clicked() {
                        state.make_builder_brush_hollow(32.0);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Make Hollow (64 uu walls)", None, None).clicked() {
                        state.make_builder_brush_hollow(64.0);
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.menu_button("Reset", |ui| {
                    if context_menu_item(ui, Some(Icon::Undo), "Reset Rotation", None, None).clicked() {
                        state.reset_rotation_selected();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::Undo), "Reset Scaling", None, None).clicked() {
                        state.reset_scaling_selected();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::Undo), "Reset All (Rot + Scale)", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                        state.reset_all_selected();
                        ui.close_menu();
                    }
                });
                if context_menu_item(ui, Some(Icon::Settings), "Transform Permanently", None, None).clicked() {
                    state.transform_permanently_selected();
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "Scale Brush Dialog...", None, None).clicked() {
                    state.show_scale_brush_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "Shear Brush Dialog...", None, None).clicked() {
                    state.show_shear_brush_dialog = true;
                    ui.close_menu();
                }
                ui.menu_button("Mirror Brush", |ui| {
                    if context_menu_item(ui, None, "Mirror across X", None, None).clicked() {
                        state.mirror_selected(crate::editor_state::MirrorAxis::X);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Mirror across Y", None, None).clicked() {
                        state.mirror_selected(crate::editor_state::MirrorAxis::Y);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Mirror across Z", None, None).clicked() {
                        state.mirror_selected(crate::editor_state::MirrorAxis::Z);
                        ui.close_menu();
                    }
                });
                if context_menu_item(ui, Some(Icon::InvertNormals), "Invert Normals & Winding", Some("Ctrl+Alt+I"), None).clicked() {
                    state.invert_selected_brush();
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Pivot", |ui| {
                    if context_menu_item(ui, Some(Icon::TargetFocus), "Snap Pivot to Nearest Vertex", Some("Ctrl+End"), None).clicked() {
                        state.snap_selected_brush_pivot_to_nearest_vertex(None);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::TargetFocus), "Center Pivot to Bounding Box", None, None).clicked() {
                        state.snap_brush_pivot_to_center();
                        ui.close_menu();
                    }
                });
                ui.menu_button("Brush Order", |ui| {
                    if context_menu_item(ui, None, "Send to First", Some("Ctrl+Home"), None).clicked() {
                        state.send_selected_brush_to_first();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Send to Last", None, None).clicked() {
                        state.send_selected_brush_to_last();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Move Up", None, None).clicked() {
                        state.move_selected_brush_up();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Move Down", None, None).clicked() {
                        state.move_selected_brush_down();
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.menu_button("Polygons (UnrealEd 2.0)", |ui| {
                    if context_menu_item(ui, Some(Icon::ExtractSurface), "📥 To Brush (Selected -> Red Builder)", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.polygons_to_brush();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ExtractSurface), "📤 From Brush (Red Builder -> Selected)", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                        state.polygons_from_brush();
                        ui.close_menu();
                    }
                    ui.separator();
                    if context_menu_item(ui, Some(Icon::PolyMerge), "🔗 Merge Coplanar Polygons", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.merge_selected_brush_polygons();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::PolySeparate), "✂ Separate Polygons (Triangulate)", None, None).clicked() {
                        state.separate_selected_brush_polygons();
                        ui.close_menu();
                    }
                });
                if context_menu_item(ui, Some(Icon::ExtractSurface), "Extract Surface to Builder", None, None).clicked() {
                    state.extract_selected_surfaces_to_builder_brush();
                    ui.close_menu();
                }
            });

            // 5. CSG MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.csg"), |ui| {
                if context_menu_item(ui, Some(Icon::CsgAdd), "CSG Add (Solid)", None, Some(Color32::from_rgb(100, 160, 255)))
                    .pro_tooltip("CSG Add", Some("Ctrl+A"), "Add the Red Builder Brush into world geometry as solid blocking matter.")
                    .clicked()
                {
                    state.csg_add();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::CsgSubtract), "CSG Subtract (Hollow)", None, Some(Color32::from_rgb(255, 180, 80)))
                    .pro_tooltip("CSG Subtract", Some("Ctrl+S"), "Carve the Red Builder Brush volume out of solid world geometry.")
                    .clicked()
                {
                    state.csg_subtract();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::CsgIntersect), "CSG Intersect", None, Some(Color32::from_rgb(100, 220, 255)))
                    .pro_tooltip("CSG Intersect", None, "Crop the Builder Brush to only keep the volume overlapping existing solid geometry.")
                    .clicked()
                {
                    state.csg_intersect();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::CsgDeintersect), "CSG Deintersect", None, Some(Color32::from_rgb(255, 140, 220)))
                    .pro_tooltip("CSG Deintersect", None, "Remove overlapping solid geometry from Builder Brush, isolating exterior void volume.")
                    .clicked()
                {
                    state.csg_deintersect();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::ActorMover), "Add Mover (Door / Lift)", None, Some(Color32::from_rgb(255, 215, 80)))
                    .pro_tooltip("Add Mover", None, "Convert the Red Builder Brush shape into a dynamic mover for doors, elevators, or traps.")
                    .clicked()
                {
                    state.add_mover_from_builder();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Wizard), "Add Special Brush...", None, Some(Color32::from_rgb(120, 230, 180)))
                    .pro_tooltip("Add Special Brush", None, "Configure and add Semi-Solids, Non-Solids, Zone Portals, or Water sheets.")
                    .clicked()
                {
                    state.show_add_special_dialog = true;
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 6. ACTORS MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.actors"), |ui| {
                if let Some(ref cls) = state.selected_class_name.clone() {
                    let pkg = state.selected_class_package.as_deref().unwrap_or("Engine");
                    let full_name = format!("{pkg}.{cls}");
                    if context_menu_item(ui, Some(Icon::ActorBrowser), &format!("⭐ Add '{cls}' Here"), None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.place_actor_of_class(&full_name);
                        ui.close_menu();
                    }
                    ui.separator();
                }
                ui.menu_button("Quick Place Actor", |ui| {
                    if context_menu_item(ui, Some(Icon::ActorLight), "Add Light Here", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.add_light_at_builder();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ActorPlayerStart), "Add PlayerStart Here", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.add_player_start_at_builder();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::ActorPathNode), "Add PathNode (Bot AI) Here", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                        state.add_path_node_at_builder();
                        ui.close_menu();
                    }
                    ui.separator();
                    if context_menu_item(ui, None, "Add SkyZoneInfo Here", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.add_sky_zone_info_at_builder();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Add ZoneInfo Here", None, Some(Color32::from_rgb(110, 230, 240))).clicked() {
                        state.add_zone_info_at_builder("GeneralZone");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Add WaterZone Here", None, Some(Color32::from_rgb(90, 190, 255))).clicked() {
                        state.add_water_zone_at_builder();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Add Projector Decal Here", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.add_projector_at_builder();
                        ui.close_menu();
                    }
                });
                ui.menu_button("🔫 Add Weapon", |ui| {
                    let wep_color = Some(Color32::from_rgb(255, 130, 130));
                    if context_menu_item(ui, None, "Shock Rifle (ShockRifle)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("ShockRifle");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Flak Cannon (UT_FlakCannon)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("UT_FlakCannon");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Rocket Launcher (UT_Eightball)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("UT_Eightball");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Sniper Rifle (SniperRifle)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("SniperRifle");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Minigun (Minigun2)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("Minigun2");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Bio Rifle (UT_BioRifle)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("UT_BioRifle");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Pulse Gun (PulseGun)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("PulseGun");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Chainsaw (Chainsaw)", None, wep_color).clicked() {
                        state.add_weapon_or_pickup_at_builder("Chainsaw");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Redeemer (WarheadLauncher)", None, Some(Color32::from_rgb(255, 90, 90))).clicked() {
                        state.add_weapon_or_pickup_at_builder("WarheadLauncher");
                        ui.close_menu();
                    }
                });
                ui.menu_button("💊 Add Pickup", |ui| {
                    if context_menu_item(ui, None, "Health Pack +20 (MedBox)", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.add_weapon_or_pickup_at_builder("MedBox");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Health Vial +5 (HealthVial)", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.add_weapon_or_pickup_at_builder("HealthVial");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Body Armor +100 (Armor2)", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.add_weapon_or_pickup_at_builder("Armor2");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Thigh Pads +50 (ThighPads)", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.add_weapon_or_pickup_at_builder("ThighPads");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Shield Belt +150 (UT_ShieldBelt)", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.add_weapon_or_pickup_at_builder("UT_ShieldBelt");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Damage Amplifier (UDamage)", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                        state.add_weapon_or_pickup_at_builder("UDamage");
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Jump Boots (JumpBoots)", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.add_weapon_or_pickup_at_builder("JumpBoots");
                        ui.close_menu();
                    }
                });
                ui.separator();
                if context_menu_item(ui, Some(Icon::AlignDistribution), "Align & Distribute Actors...", None, None).clicked() {
                    state.show_actor_align_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorProperties), "Actor Properties...", Some("Enter"), None).clicked() {
                    state.show_actor_properties = true;
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 7. BUILD MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.build"), |ui| {
                if context_menu_item(ui, Some(Icon::BuildGeometry), "Rebuild Geometry & BSP Tree", Some("Ctrl+Shift+G"), Some(Color32::from_rgb(100, 210, 255)))
                    .pro_tooltip("Rebuild Geometry", Some("Ctrl+Shift+G"), "Recompute CSG operations, compile BSP tree node cuts, and generate PVS visibility.")
                    .clicked()
                {
                    state.build_geometry();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorLight), "Build Lighting (Raytracing)...", None, Some(Color32::from_rgb(255, 215, 80)))
                    .pro_tooltip("Lighting Studio", None, "Open advanced raytracing dialog with multi-bounce radiosity and shadow parameters.")
                    .clicked()
                {
                    state.show_lighting_dialog = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorLight), "Fast Bake Lighting", Some("Ctrl+Shift+L"), None)
                    .pro_tooltip("Fast Bake Lighting", Some("Ctrl+Shift+L"), "Rebake static surface lightmaps for all point lights.")
                    .clicked()
                {
                    state.build_lighting();
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorLight), "Bake Selected Lights Only", None, None)
                    .pro_tooltip("Bake Selected Lights", None, "Fast iterative re-bake of only currently selected Light actors.")
                    .clicked()
                {
                    let _ = state.bake_selected_lights_only();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::ActorPathNode), "Build AI Navigation Paths", Some("Ctrl+Shift+P"), Some(Color32::from_rgb(120, 230, 160)))
                    .pro_tooltip("Build AI Paths", Some("Ctrl+Shift+P"), "Test collision cylinders and line of sight to compile ReachSpecs bot network.")
                    .clicked()
                {
                    state.build_ai_paths();
                    ui.close_menu();
                }
                if context_menu_item_danger(ui, None, "Clear AI Paths (PATHS UNDEFINE)", None)
                    .pro_tooltip("Clear AI Paths", None, "Remove all compiled ReachSpecs links between navigation nodes.")
                    .clicked()
                {
                    state.clear_ai_paths();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, None, "Bake Projector Decals", None, None)
                    .pro_tooltip("Bake Decals", None, "Project and clip static decal textures onto surrounding BSP surfaces.")
                    .clicked()
                {
                    state.bake_projector_decals();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::BuildGeometry), "Build All (Geom + Light + Paths)", Some("Ctrl+F8"), Some(Color32::from_rgb(110, 240, 160)))
                    .pro_tooltip("Build All Complete", Some("Ctrl+F8"), "Full compilation sequence: Geometry + Lighting + Paths + Decals in one click.")
                    .clicked()
                {
                    state.build_all_complete();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::Settings), "Build Options...", Some("Ctrl+Shift+B"), None)
                    .pro_tooltip("Build Options", Some("Ctrl+Shift+B"), "Configure compiler threading, BSP optimization balance, and lightmap quality.")
                    .clicked()
                {
                    state.show_build_options_dialog = true;
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 8. STUDIOS & TOOLS MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.studios"), |ui| {
                ui.menu_button("✨ Creation & Action Wizards", |ui| {
                    if context_menu_item(ui, Some(Icon::Wizard), "New Valid Map Generator Wizard...", Some("Ctrl+Shift+N"), Some(Color32::from_rgb(110, 250, 160))).clicked() {
                        state.show_map_wizard_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::Wizard), "Action Wizards (Lifts, Teleporters, Pads)...", Some("Ctrl+Shift+A"), Some(Color32::from_rgb(100, 220, 255))).clicked() {
                        state.show_action_wizards_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, Some(Icon::MaterialPreset), "Visual & Lighting Presets Catalog...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.show_visual_presets_dialog = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if context_menu_item(ui, Some(Icon::LevelDoctor), "Level Doctor (Check Map Integrity)...", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                        state.show_level_doctor_window = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "📊 Map Statistics...", None, None).clicked() {
                        state.show_map_statistics_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "📊 Map Performance & Budget Profiler", Some("F7"), Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.show_map_budget_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "🏆 Tournament Item Balance", Some("F12"), Some(Color32::from_rgb(255, 130, 130))).clicked() {
                        state.show_gameplay_balance_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "🌐 Scale Entire Map...", None, None).clicked() {
                        state.show_scale_map_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("📐 Geometry & Architecture Studios", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, Some(Icon::ShapeCube), "2D Shape Editor...", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                            state.show_shape_editor = !state.show_shape_editor;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::BuildGeometry), "BSP Quality & Sliver Purger...", Some("Ctrl+Shift+Q"), Some(Color32::from_rgb(100, 210, 255))).clicked() {
                            state.show_bsp_quality_studio_dialog = true;
                            state.run_bsp_quality_audit();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "2D Shape Loft & Extrusion...", None, None).clicked() {
                            state.show_loft_studio_dialog = true;
                            state.recalculate_loft_mesh();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Curved Stairway & Spiral Ramp Builder...", None, None).clicked() {
                            state.show_stairway_studio_dialog = true;
                            state.recalculate_stair_geometry();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Architectural Prefab Builder...", None, None).clicked() {
                            state.show_architectural_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Terrain & Heightmap Sculptor...", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                            state.show_terrain_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Mesh LOD & Collision Inspector...", None, None).clicked() {
                            state.show_lod_collision_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Surface Foliage & Clutter Scatter...", None, None).clicked() {
                            state.show_scatter_studio_dialog = true;
                            state.recalculate_scatter_preview();
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, Some(Icon::ActorBrowser), "📦 Prefab & Template Library...", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                            state.show_prefab_browser = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorBrowser), "🗿 Mesh & Static Mesh Browser...", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                            state.show_mesh_browser = true;
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("💡 Lighting, Sky, Fog & Atmosphere", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, Some(Icon::ActorLight), "💡 Radiosity & Direct Light Baking...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_lighting_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorLight), "Dynamic Light FX & Spotlights...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_light_effects_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorLight), "SpecialLit & Lighting Masking...", None, None).clicked() {
                            state.show_special_lit_dialog = true;
                            state.recalculate_special_lit_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Sun, Corona & Lens Flares...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_corona_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Soft Penumbra Shadow Profiler", None, None).clicked() {
                            state.show_shadow_penumbra = true;
                            state.evaluate_selected_shadow_penumbra();
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, None, "SkyBox 3D Parallax & Multi-Zone...", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                            state.show_skybox_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "🌫 Zone Fog & Distance Attenuation...", Some("F10"), Some(Color32::from_rgb(180, 215, 255))).clicked() {
                            state.show_zone_fog_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Weather & Precipitation Setup...", None, None).clicked() {
                            state.show_weather_dialog = true;
                            state.recalculate_weather_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Water Zones & Fluid Surfaces...", None, Some(Color32::from_rgb(90, 190, 255))).clicked() {
                            state.show_water_surface_studio_dialog = true;
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, None, "🌊 Zone Boundary Leak Detector...", None, Some(Color32::from_rgb(255, 120, 120))).clicked() {
                            state.show_zone_seal_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Zone Portals & PVS Occlusion...", None, None).clicked() {
                            state.show_zone_occlusion_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Zone PVS Graph & Cell Occlusion", None, None).clicked() {
                            state.show_zone_pvs = true;
                            state.compute_zone_pvs();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "WarpZone Portals...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                            state.show_warp_zone_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "🚫 Flag Brush as Anti-Portal", None, None).clicked() {
                            state.make_selected_brush_anti_portal();
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("🔊 Audio, Music & Acoustics", |ui| {
                    if context_menu_item(ui, None, "Zone Acoustics & EAX Reverb...", Some("F11"), Some(Color32::from_rgb(205, 165, 255))).clicked() {
                        state.show_acoustics_dialog = true;
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Ambient Soundscape Placement...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                        state.show_soundscape_dialog = true;
                        state.recalculate_soundscape_preview();
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Music Track (.umx) Sequencer...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                        state.show_music_studio_dialog = true;
                        ui.close_menu();
                    }
                });

                ui.menu_button("🤖 AI, Navigation & Bot Pathing", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, Some(Icon::ActorPathNode), "Bot Navigation & Reachability...", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                            state.show_bot_nav_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorPathNode), "🤖 Bot AI Navigation Simulator...", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                            state.show_bot_simulator_window = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorPathNode), "🔷 Rebuild Bot AI Paths", Some("Ctrl+Shift+P"), Some(Color32::from_rgb(120, 235, 150))).clicked() {
                            state.build_ai_paths();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Auto-Connect ReachSpecs", None, None).clicked() {
                            state.build_auto_paths(state.auto_path_max_distance, state.auto_path_max_step, state.auto_path_max_jump);
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, None, "Bot JumpSpot Routes...", None, None).clicked() {
                            state.show_bot_jump_routes_dialog = true;
                            state.recalculate_jump_route_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Translocator Trajectory & Beacons...", None, None).clicked() {
                            state.show_translocator_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "LiftCenter & Elevator Pathing...", None, None).clicked() {
                            state.show_lift_pathing_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Teleporter Network & Routing...", None, None).clicked() {
                            state.show_teleporter_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Bot Combat Sightlines & Vantage", None, None).clicked() {
                            state.show_combat_sightlines = true;
                            state.compute_combat_sightlines();
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("🚪 Movers, Elevators & Triggers", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, Some(Icon::ActorMover), "Mover Encroachment & Door Setup...", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                            state.show_mover_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, Some(Icon::ActorMover), "Elevator & Lift Builder...", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                            state.show_elevator_dialog = true;
                            state.recalculate_elevator_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Spline Rail & Keyframe Mover...", None, None).clicked() {
                            state.show_track_studio_dialog = true;
                            state.recalculate_track_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Trigger & Event Network Builder...", None, None).clicked() {
                            state.show_trigger_network_dialog = true;
                            state.recalculate_trigger_network_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Jump Pad & Kicker Setup...", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                            state.show_kicker_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Destructible Geometry & Glass...", None, Some(Color32::from_rgb(255, 120, 120))).clicked() {
                            state.show_destructible_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Zone Gravity & Velocity Setup...", None, None).clicked() {
                            state.show_zone_velocity_dialog = true;
                            state.recalculate_zone_velocity_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Zone Fluid Physics...", None, None).clicked() {
                            state.show_fluid_studio_dialog = true;
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("🏆 Tournament Modes (CTF / DOM / Assault)", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, None, "CTF Flag Base & Transit Routes...", None, Some(Color32::from_rgb(255, 130, 130))).clicked() {
                            state.show_ctf_base_dialog = true;
                            state.recalculate_ctf_base_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Symmetrical CTF Base Mirroring...", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                            state.show_ctf_symmetry_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Domination Control Point Setup...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_domination_dialog = true;
                            state.recalculate_domination_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Assault Objective Chain Setup...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                            state.show_assault_dialog = true;
                            state.recalculate_assault_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Team DefensePoint AI Priorities...", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                            state.show_team_objective_dialog = true;
                            state.scan_and_refresh_team_objectives();
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, None, "PlayerStart Spawn Distribution...", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                            state.show_spawn_distribution_dialog = true;
                            state.recalculate_spawn_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Weapon & Ammo Placement Audit...", None, Some(Color32::from_rgb(255, 130, 130))).clicked() {
                            state.show_weapon_circuit_dialog = true;
                            state.recalculate_weapon_circuit_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Powerup & Relic Placement...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                            state.show_powerup_control_dialog = true;
                            state.recalculate_powerup_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Arena Match Rules & Mutators...", None, None).clicked() {
                            state.show_arena_rules_dialog = true;
                            state.recalculate_arena_rules_preview();
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("📜 Scripting, Skins, FX & Packaging", |ui| {
                    egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
                        if context_menu_item(ui, None, "Dispatcher Sequence & Timeline...", None, Some(Color32::from_rgb(110, 230, 240))).clicked() {
                            state.show_dispatcher_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Spectator & Intro Camera Paths...", None, Some(Color32::from_rgb(205, 165, 255))).clicked() {
                            state.show_cinematic_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Skeletal Rig & Mesh Inspector...", None, Some(Color32::from_rgb(110, 230, 240))).clicked() {
                            state.show_skeletal_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "🎨 UT99 Skin & Character Studio...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_skin_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "📦 Package Asset Extractor (.uz)...", None, None).clicked() {
                            state.show_package_extractor_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "📜 UnrealScript Class Wizard...", None, Some(Color32::from_rgb(110, 240, 160))).clicked() {
                            state.show_uc_class_wizard_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Particle FX & Smoke Generators...", None, Some(Color32::from_rgb(255, 170, 70))).clicked() {
                            state.show_particle_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "🔥 FireEngine Procedural Textures...", None, Some(Color32::from_rgb(255, 120, 80))).clicked() {
                            state.show_procedural_texture_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Decal & Projector Placement...", None, None).clicked() {
                            state.show_decal_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "🎯 Bake Projector Decals", None, None).clicked() {
                            state.bake_projector_decals();
                            ui.close_menu();
                        }
                        ui.separator();
                        if context_menu_item(ui, None, "UnrealScript Class Source Viewer...", None, None).clicked() {
                            let cls = state.selected_class_name.clone().unwrap_or_else(|| "Actor".to_string());
                            state.open_uc_viewer_for_class(&cls);
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Package Dependencies Manifest...", None, None).clicked() {
                            state.show_package_manifest_dialog = true;
                            state.refresh_map_manifest();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Map Packaging & Release Zip...", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                            state.show_map_packaging_dialog = true;
                            ui.close_menu();
                        }
                    });
                });

                ui.separator();
                ui.menu_button("🧩 Setpiece & Hazard Prefab Generators", |ui| {
                    egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                        ui.label(egui::RichText::new("Architectural Setpieces").size(11.0).strong().color(Color32::from_rgb(130, 190, 255)));
                        if context_menu_item(ui, None, "Glass Skylight & Observation Dome...", None, None).clicked() {
                            state.show_skylight_dialog = true;
                            state.recalculate_skylight_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Ventilation Duct & Crawlspace...", None, None).clicked() {
                            state.show_vent_duct_dialog = true;
                            state.recalculate_vent_duct_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Industrial Pipe Conduit & Valve...", None, None).clicked() {
                            state.show_pipe_conduit_dialog = true;
                            state.recalculate_pipe_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Volumetric Light Shaft Brush...", None, None).clicked() {
                            state.show_light_shaft_dialog = true;
                            state.recalculate_light_shaft_geometry();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Teleporter Arch & Gate Prefab...", None, None).clicked() {
                            state.show_teleporter_arch_dialog = true;
                            state.recalculate_teleporter_arch_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Secret Door & Concealed Passage...", None, None).clicked() {
                            state.show_secret_door_dialog = true;
                            state.recalculate_secret_door_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Holographic Signage & Billboard...", None, None).clicked() {
                            state.show_hologram_dialog = true;
                            state.recalculate_hologram_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Security CCTV & Monitor Setpiece...", None, None).clicked() {
                            state.show_surveillance_dialog = true;
                            state.recalculate_surveillance_preview();
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Mechanical & Environmental Hazards").size(11.0).strong().color(Color32::from_rgb(255, 180, 90)));
                        if context_menu_item(ui, None, "Industrial Conveyor Belt...", None, None).clicked() {
                            state.show_conveyor_dialog = true;
                            state.recalculate_conveyor_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Steam Piston & Crusher...", None, None).clicked() {
                            state.show_piston_dialog = true;
                            state.recalculate_piston_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Reactor Core & Kinetic Rings...", None, None).clicked() {
                            state.show_reactor_core_dialog = true;
                            state.recalculate_reactor_core_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Environmental Ventilator & Wind...", None, None).clicked() {
                            state.show_ventilator_dialog = true;
                            state.recalculate_ventilator_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Pressure Airlock & Hazard Chamber...", None, None).clicked() {
                            state.show_hazard_chamber_dialog = true;
                            state.recalculate_hazard_chamber_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Slime / Acid Basin Prefab...", None, None).clicked() {
                            state.show_fluid_basin_dialog = true;
                            state.recalculate_fluid_basin_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Bio-Slime Hazard Trap...", None, None).clicked() {
                            state.show_bio_trap_dialog = true;
                            state.recalculate_bio_trap_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Explosive Tarydium Barrels...", None, None).clicked() {
                            state.show_hazard_barrel_dialog = true;
                            state.recalculate_hazard_barrel_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Laser Tripwire Grid...", None, None).clicked() {
                            state.show_laser_grid_dialog = true;
                            state.recalculate_laser_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Force Field Energy Barrier...", None, None).clicked() {
                            state.show_force_field_dialog = true;
                            state.recalculate_force_field_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Alarm Siren & Rotating Strobe...", None, None).clicked() {
                            state.show_alarm_beacon_dialog = true;
                            state.recalculate_alarm_beacon_preview();
                            ui.close_menu();
                        }

                        ui.separator();
                        ui.label(egui::RichText::new("Tactical Stations & Alcoves").size(11.0).strong().color(Color32::from_rgb(120, 235, 150)));
                        if context_menu_item(ui, None, "Armor & Shield Pedestal Station...", None, None).clicked() {
                            state.show_armor_stacking_dialog = true;
                            state.recalculate_armor_stacking_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Jump Boots Alcove...", None, None).clicked() {
                            state.show_jump_boots_dialog = true;
                            state.recalculate_jump_boots_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Weapon & Ammo Supply Locker...", None, None).clicked() {
                            state.show_supply_locker_dialog = true;
                            state.recalculate_supply_locker_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Sniper Vantage Balcony...", None, None).clicked() {
                            state.show_sniper_vantage_dialog = true;
                            state.recalculate_sniper_vantage_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Weapon Ballistics Sightline Probe...", None, None).clicked() {
                            state.show_ballistics_studio_dialog = true;
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Automated Defense Sentry...", None, None).clicked() {
                            state.show_turret_dialog = true;
                            state.recalculate_turret_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Assault Stationary Cannon...", None, None).clicked() {
                            state.show_team_cannon_dialog = true;
                            state.recalculate_team_cannon_preview();
                            ui.close_menu();
                        }
                        if context_menu_item(ui, None, "Base Radar & Proximity Sensor...", None, None).clicked() {
                            state.show_radar_dialog = true;
                            state.recalculate_radar_preview();
                            ui.close_menu();
                        }
                    });
                });
            });

            // 9. PLAY MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.play"), |ui| {
                if context_menu_item(ui, Some(Icon::PlayGame), "Play Map in UT99", Some("F8"), Some(Color32::from_rgb(110, 240, 160))).clicked() {
                    let _ = state.play_map_in_ut99();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::Settings), "UT99 Launch Settings...", None, None).clicked() {
                    state.show_play_settings_dialog = true;
                    ui.close_menu();
                }
            });

            // -----------------------------------------------------------------
            // 10. WINDOW MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.window"), |ui| {
                if context_menu_item(ui, None, "Close All Floating Windows", Some("Ctrl+Shift+W"), None).clicked() {
                    state.close_all_floating_dialogs();
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Workspace Presets", |ui| {
                    if context_menu_item(ui, None, "Default Workspace", None, None).clicked() {
                        state.apply_workspace_preset(crate::editor_state::WorkspacePreset::Default);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Level Design (4-View + Textures)", None, Some(Color32::from_rgb(100, 210, 255))).clicked() {
                        state.apply_workspace_preset(crate::editor_state::WorkspacePreset::LevelDesign);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Lighting & Atmosphere (Baking + Skybox)", None, Some(Color32::from_rgb(255, 215, 80))).clicked() {
                        state.apply_workspace_preset(crate::editor_state::WorkspacePreset::LightingAndAtmosphere);
                        ui.close_menu();
                    }
                    if context_menu_item(ui, None, "Bot & Navigation (Paths + Simulator)", None, Some(Color32::from_rgb(120, 235, 150))).clicked() {
                        state.apply_workspace_preset(crate::editor_state::WorkspacePreset::BotAndGameplay);
                        ui.close_menu();
                    }
                });
                ui.separator();
                ui.checkbox(&mut state.show_master_browser, "Master Browser (Unified Tabs)");
                ui.separator();
                ui.checkbox(&mut state.show_texture_browser, "Texture Browser (F4)");
                ui.checkbox(&mut state.show_actor_browser, "Actor Classes Browser");
                ui.checkbox(&mut state.show_mesh_browser, "Mesh Browser");
                ui.checkbox(&mut state.show_sound_browser, "Sound Browser");
                ui.checkbox(&mut state.show_music_browser, "Music Browser");
                ui.checkbox(&mut state.show_group_browser, "Groups Browser");
                ui.separator();
                ui.checkbox(&mut state.show_surface_properties, "Surface Properties (F5)");
                ui.checkbox(&mut state.show_actor_properties, "Actor Properties (Enter)");
                ui.checkbox(&mut state.show_level_properties_dialog, "Level Properties (F6)");
                ui.checkbox(&mut state.show_bottom_console, "Output Console (Ctrl+` / Ctrl+Shift+C)");
            });

            // -----------------------------------------------------------------
            // 11. HELP MENU
            // -----------------------------------------------------------------
            ui.menu_button(tr(lang, "menu.help"), |ui| {
                if context_menu_item(ui, Some(Icon::FileOpen), "📖 Documentation & Mapping Manual", Some("F1"), Some(Color32::from_rgb(120, 210, 255)))
                    .pro_tooltip("Documentation Manual", Some("F1"), "Open comprehensive editor documentation, CSG rules, lighting and bot guides")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::QuickStart);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "⌨ Keyboard Shortcuts Cheatsheet", Some("?"), None)
                    .pro_tooltip("Keyboard Shortcuts", Some("? / Shift+F1"), "Search and browse all editor keyboard shortcuts and viewport navigation controls")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::Shortcuts);
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, None, "🧱 CSG & Brush Geometry Reference", None, None)
                    .pro_tooltip("CSG Reference", None, "Learn about Solid, Semi-Solid (PF_SEMISOLID), Non-Solid brushes, and boolean rules")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::CsgGeometry);
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorLight), "💡 Lighting & Radiosity Guide", None, None)
                    .pro_tooltip("Lighting Guide", None, "Guide to LightBrightness, LightEffects (TorchWaver, FireWaver) and radiosity baking")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::Lighting);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "🚪 Movers, Doors & Lifts Guide", None, None)
                    .pro_tooltip("Movers & Lifts", None, "Keyframes, BumpOpenTimed, StandOpenTimed elevators, sound FX, and crush handling")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::MoversAndTriggers);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "🌊 Zones, Water & Atmosphere Guide", None, None)
                    .pro_tooltip("Zones & Atmosphere", None, "ZonePortals, ZoneInfo, water swimming, low gravity, distance fog, and skyboxes")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::ZonesAndAtmosphere);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "🎨 Surfaces, Materials & SkyBoxes", None, None)
                    .pro_tooltip("Surfaces & Materials", None, "Surface Properties (F5), Unlit, Translucent, Masked, FakeBackdrop & UV align")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::SurfacesAndMaterials);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "⚡ Event & Tag Logic (Triggers & Dispatchers)", None, None)
                    .pro_tooltip("Event & Tag Logic", None, "Event/Tag messaging, Trigger types, Dispatcher sequences, Counters & Teleporters")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::EventSystem);
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::ActorPathNode), "🤖 Bot AI & Navigation Pathing", None, None)
                    .pro_tooltip("Bot Pathing Guide", None, "How to build valid ReachSpecs networks with PathNodes, LiftCenters and LevelDoctor")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::BotNavigation);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "🎮 Tournament Game Modes (DM/CTF/DOM/AS)", None, None)
                    .pro_tooltip("Game Modes Guide", None, "Design rules for DeathMatch, Capture The Flag, Domination and Assault")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::GameModes);
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Wizard), "✨ Studios & Procedural Generators", None, None)
                    .pro_tooltip("Studios & Tools", None, "Creation Hub, Architectural Prefab Studio, Bio Traps, Armory, and LevelDoctor")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::StudiosAndTools);
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "⚠️ UnrealEd 2.0 Gotchas & FAQ", None, Some(Color32::from_rgb(255, 205, 90)))
                    .pro_tooltip("UnrealEd 2.0 Gotchas", None, "Historical gotchas: solving BSP holes, HOM errors, package hygiene and 65k uu limits")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::UnrealEdGotchas);
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, Some(Icon::CommandPalette), "Command Palette", Some("Ctrl+P"), Some(Color32::from_rgb(245, 182, 45)))
                    .pro_tooltip("Command Palette", Some("Ctrl+P"), "Fuzzy search and execute any editor action, tool, or studio")
                    .clicked()
                {
                    state.show_command_palette = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, Some(Icon::Wizard), "LevelDoctor Map Audit...", Some("Ctrl+Shift+D"), Some(Color32::from_rgb(100, 255, 160)))
                    .pro_tooltip("LevelDoctor Diagnostics", Some("Ctrl+Shift+D"), "Automated audit of BSP holes, reachability, orphan nodes, and lighting")
                    .clicked()
                {
                    state.run_level_doctor_audit();
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, None, "🌟 Welcome & Quickstart Setup...", None, Some(Color32::from_rgb(255, 215, 100)))
                    .pro_tooltip("Welcome Wizard", None, "First-run game detection, controls profile, and starter map options")
                    .clicked()
                {
                    state.welcome_wizard_step = 0;
                    state.show_welcome_wizard = true;
                    ui.close_menu();
                }
                if context_menu_item(ui, None, "❤ Support Forge99 (Donate / Sponsor)...", None, Some(Color32::from_rgb(255, 130, 160)))
                    .pro_tooltip("Support the Project", None, "Support Forge99 ongoing open-source development and multi-OS builds")
                    .clicked()
                {
                    ui.ctx().open_url(egui::OpenUrl::new_tab("https://github.com/sponsors/thevot"));
                    ui.close_menu();
                }
                ui.separator();
                if context_menu_item(ui, None, "ℹ About Forge99 (Native 64-bit Rust)", None, None)
                    .pro_tooltip("About Forge99", None, "Architecture overview, system specs, WGPU backend, and live telemetry")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::About);
                    ui.close_menu();
                }
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let palette = SemanticPalette::current(ui.ctx());
                let rem_w = ui.available_width();

                // 1. Quick Help Manual Button (Vector Icon)
                let help_lbl = if rem_w > 380.0 { Some("Help") } else { None };
                if icons::icon_label_button(ui, Icon::InfoCircle, help_lbl, state.show_help_dialog)
                    .pro_tooltip("Documentation & Manual", Some("? / Shift+F1"), "Open comprehensive editor documentation, tutorials, shortcuts and gotchas")
                    .clicked()
                {
                    state.open_help_tab(crate::editor_state::HelpTab::QuickStart);
                }
                ui.separator();

                // 2. Command Palette Quick Button (Vector Icon)
                let pal_lbl = if rem_w > 660.0 {
                    Some("Command Palette (Ctrl+P)")
                } else if rem_w > 340.0 {
                    Some("Ctrl+P")
                } else {
                    None
                };
                if icons::icon_label_button(ui, Icon::CommandPalette, pal_lbl, state.show_command_palette)
                    .pro_tooltip("Command Palette", Some("Ctrl+P"), "Fuzzy search and launch all editor tools, dialogs and actions")
                    .clicked()
                {
                    state.show_command_palette = !state.show_command_palette;
                }
                ui.separator();

                // 3. New Valid Map Wizard Launcher (always accessible: full label -> short label -> icon)
                let wiz_lbl = if rem_w > 520.0 {
                    Some("Map Wizard")
                } else if rem_w > 300.0 {
                    Some("Wizard")
                } else {
                    None
                };
                if icons::icon_label_button(ui, Icon::Wizard, wiz_lbl, state.show_map_wizard_dialog)
                    .pro_tooltip("New Valid Map Generator Wizard", Some("Ctrl+Shift+N"), "Generate complete playtest-ready UT99 arenas or clone Official Map DNA")
                    .clicked()
                {
                    state.show_map_wizard_dialog = !state.show_map_wizard_dialog;
                }
                ui.separator();

                // 4. Unified Browsers Access (Master Browser + Quick Popover - always accessible)
                let browsers_btn_lbl = if rem_w > 260.0 { "Browsers ▾" } else { "Browsers" };
                ui.menu_button(browsers_btn_lbl, |ui| {
                    if context_menu_item(ui, Some(Icon::Folder), "Master Browser Hub", Some("Ctrl+B"), Some(palette.accent_gold)).clicked() {
                        state.show_master_browser = !state.show_master_browser;
                        ui.close_menu();
                    }
                    ui.separator();
                    let tex_label = if state.show_texture_browser { "Textures (Open)" } else { "Textures" };
                    if context_menu_item(ui, Some(Icon::TextureBrowser), tex_label, Some("F4"), Some(palette.accent_azure)).clicked() {
                        state.show_texture_browser = !state.show_texture_browser;
                        ui.close_menu();
                    }
                    let act_label = if state.show_actor_browser { "Actor Classes (Open)" } else { "Actor Classes" };
                    if context_menu_item(ui, Some(Icon::ActorBrowser), act_label, None, Some(palette.accent_gold)).clicked() {
                        state.show_actor_browser = !state.show_actor_browser;
                        ui.close_menu();
                    }
                    let mesh_label = if state.show_mesh_browser { "Static Meshes (Open)" } else { "Static Meshes" };
                    if context_menu_item(ui, Some(Icon::MeshBrowser), mesh_label, None, Some(palette.accent_azure)).clicked() {
                        state.show_mesh_browser = !state.show_mesh_browser;
                        ui.close_menu();
                    }
                    let snd_label = if state.show_sound_browser { "Sounds (Open)" } else { "Sounds" };
                    if context_menu_item(ui, Some(Icon::SoundBrowser), snd_label, None, None).clicked() {
                        state.show_sound_browser = !state.show_sound_browser;
                        ui.close_menu();
                    }
                    let mus_label = if state.show_music_browser { "Music (Open)" } else { "Music" };
                    if context_menu_item(ui, Some(Icon::MusicBrowser), mus_label, None, None).clicked() {
                        state.show_music_browser = !state.show_music_browser;
                        ui.close_menu();
                    }
                    let grp_label = if state.show_group_browser { "Groups (Open)" } else { "Groups" };
                    if context_menu_item(ui, Some(Icon::GroupBrowser), grp_label, None, None).clicked() {
                        state.show_group_browser = !state.show_group_browser;
                        ui.close_menu();
                    }
                    ui.separator();
                    let shape_label = if state.show_shape_editor { "2D Shape Editor (Open)" } else { "2D Shape Editor" };
                    if context_menu_item(ui, Some(Icon::ShapeCube), shape_label, None, None).clicked() {
                        state.show_shape_editor = !state.show_shape_editor;
                        ui.close_menu();
                    }
                    let fire_label = if state.show_procedural_texture_dialog { "FireEngine FX (Open)" } else { "FireEngine FX" };
                    if context_menu_item(ui, Some(Icon::Sparkles), fire_label, None, None).clicked() {
                        state.show_procedural_texture_dialog = !state.show_procedural_texture_dialog;
                        ui.close_menu();
                    }
                });
                ui.separator();

                // 5. Active Material Badge (if set and space permits)
                if ui.available_width() > 220.0 {
                    if let Some(ref active_tex) = state.active_texture {
                        let mat_label = format!("Mat: {}", active_tex);
                        let btn = icons::icon_label_button(ui, Icon::TextureBrowser, Some(&mat_label), state.show_texture_browser);
                        if btn.clicked() {
                            state.show_texture_browser = true;
                        }
                        btn.on_hover_ui(|ui| {
                            ui.heading(tr(lang, "sidebar.active_material"));
                            ui.label(egui::RichText::new(active_tex).strong().color(palette.accent_gold));
                            ui.separator();
                            ui.label("• Left-Click: Open Texture Browser (F4)");
                            ui.label("• Shift+F: Fit to selected brush surface");
                            ui.label("• Shift+Click: Apply to brush surface in 3D");
                        });
                        ui.separator();
                    }
                }

                // 6. Active Map Name Badge
                if ui.available_width() > 140.0 {
                    if let Some(ref path) = state.map_path {
                        let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("Map");
                        ui.label(egui::RichText::new(format!("Map: {}", fname)).color(palette.accent_azure).strong())
                            .pro_tooltip("Loaded Map", None, "Currently loaded Unreal Tournament level file");
                        ui.separator();
                    }
                }
            });
        });
    });

    // 1b. Main Toolbar (Responsive Multi-Tier & Atomic Cluster Row Engine - UI-01 & UI-04)
    toolbar::render_sub_toolbar(ctx, state);

    // 2. Left Toolbar (Builder Brush and CSG Operations)
    egui::SidePanel::left("left_toolbar")
        .default_width(220.0)
        .show(ctx, |ui| {
            let lang = state.language;
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading(tr(lang, "sidebar.editor_mode"));

                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(state.editor_mode == EditorMode::CameraSelection, tr(lang, "sidebar.mode.camera"))
                            .pro_tooltip("Camera Mode", Some("1"), "Default viewport navigation and actor/brush selection mode with mouse picking and marquee box.")
                            .clicked()
                        {
                            state.editor_mode = EditorMode::CameraSelection;
                        }
                        if ui.selectable_label(state.editor_mode == EditorMode::VertexEdit, tr(lang, "sidebar.mode.vertex"))
                            .pro_tooltip("Vertex Editing Mode", None, "Interactive vertex editing: select, drag, snap, weld, and split brush vertices directly in 2D/3D viewports.")
                            .clicked()
                        {
                            state.editor_mode = EditorMode::VertexEdit;
                            state.selected_vertex_indices.clear();
                        }
                        if ui.selectable_label(state.editor_mode == EditorMode::BrushClip, tr(lang, "sidebar.mode.clip"))
                            .pro_tooltip("Brush Clipping Mode", None, "Slice world brushes or builder brush using a 2-point or 3-point interactive clipping plane.")
                            .clicked()
                        {
                            state.editor_mode = EditorMode::BrushClip;
                        }
                        if ui.selectable_label(state.editor_mode == EditorMode::FreehandPolygon, format!("✏ {}", tr(lang, "sidebar.mode.freehand")))
                            .pro_tooltip("Freehand Polygon Mode", None, "UnrealEd 2.0 Freehand Drawing: click in 2D/3D viewports to place vertices and extrude into 3D Red Builder Brush.")
                            .clicked()
                        {
                            state.editor_mode = EditorMode::FreehandPolygon;
                        }
                    });
                    ui.separator();

            match state.editor_mode {
                EditorMode::FreehandPolygon => {
                    ui.heading("✏ Freehand Polygon");
                    ui.separator();
                    ui.label(
                        egui::RichText::new(format!("Placed Vertices: {}", state.freehand_points.len()))
                            .strong()
                            .color(egui::Color32::from_rgb(80, 230, 255)),
                    );
                    for (i, pt) in state.freehand_points.iter().enumerate() {
                        ui.label(format!("  #{}: ({:.0}, {:.0}, {:.0})", i + 1, pt.x, pt.y, pt.z));
                    }
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label("Extrude Depth:");
                        ui.add(egui::DragValue::new(&mut state.freehand_extrude_depth).speed(8.0).suffix(" uu"));
                    });
                    ui.horizontal(|ui| {
                        if ui.button("➕ Vertex at Builder").clicked() {
                            state.add_freehand_vertex(state.builder_brush.location);
                        }
                        if ui.button("🗑 Clear").clicked() {
                            state.clear_freehand_polygon();
                        }
                    });
                    ui.add_space(6.0);
                    let can_extrude = state.freehand_points.len() >= 3;
                    if ui
                        .add_enabled(
                            can_extrude,
                            egui::Button::new(
                                egui::RichText::new("🚀 Extrude to Builder (Enter)")
                                    .strong()
                                    .color(egui::Color32::from_rgb(100, 255, 160)),
                            ),
                        )
                        .clicked()
                    {
                        let d = state.freehand_extrude_depth;
                        let _ = state.extrude_freehand_polygon_to_builder(d);
                    }
                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("Tip: Left-click in any 2D (Top/Front/Side) or 3D viewport to place vertices on the active grid plane, then press Enter to extrude.")
                            .italics()
                            .size(11.0),
                    );
                }
                EditorMode::VertexEdit => {
                    ui.heading("Vertex Editing");
                    ui.separator();
                    let target_name = if let Some(idx) = state.selected_brush_index {
                        format!("Brush #{} ({})", idx, state.world_brushes.get(idx).map(|b| b.name.as_str()).unwrap_or("Unknown"))
                    } else {
                        "Active Red Builder Brush".to_string()
                    };
                    ui.label(egui::RichText::new(format!("Target: {}", target_name)).strong().color(egui::Color32::from_rgb(100, 200, 255)));

                    let unique_verts = state.get_active_brush_unique_vertices();
                    ui.label(format!("Total Vertices: {}", unique_verts.len()));
                    ui.label(format!("Selected Vertices: {}", state.selected_vertex_indices.len()));

                    ui.horizontal(|ui| {
                        if ui.button("Select All").clicked() {
                            state.selected_vertex_indices = (0..unique_verts.len()).collect();
                        }
                        if ui.button("Clear Selection").clicked() {
                            state.selected_vertex_indices.clear();
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Nudge Selected Vertices").strong());
                    let step = state.grid_step;
                    ui.horizontal(|ui| {
                        if ui.button("-X").clicked() { state.move_selected_vertices(Vec3::new(-step, 0.0, 0.0)); }
                        if ui.button("+X").clicked() { state.move_selected_vertices(Vec3::new(step, 0.0, 0.0)); }
                        if ui.button("-Y").clicked() { state.move_selected_vertices(Vec3::new(0.0, -step, 0.0)); }
                        if ui.button("+Y").clicked() { state.move_selected_vertices(Vec3::new(0.0, step, 0.0)); }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("-Z").clicked() { state.move_selected_vertices(Vec3::new(0.0, 0.0, -step)); }
                        if ui.button("+Z").clicked() { state.move_selected_vertices(Vec3::new(0.0, 0.0, step)); }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Vertex Snapping & Welding").strong());
                    if ui.button("Snap to Grid (Ctrl+Alt+S)").on_hover_text("Snaps selected vertices directly to the active grid step").clicked() {
                        state.snap_selected_vertices_to_grid();
                    }
                    if ui.button("Magnetic Weld (16 uu) (Ctrl+Alt+W)").on_hover_text("Welds selected vertices magnetically to nearest adjacent vertices").clicked() {
                        state.weld_selected_vertices_to_nearest(16.0);
                    }
                    let can_split = state.selected_vertex_indices.len() == 2;
                    if ui.add_enabled(can_split, egui::Button::new("Split Selected Edge (Ctrl+Alt+E)")).on_hover_text("Inserts a new vertex along the edge connecting the 2 selected vertices").clicked() {
                        state.split_selected_edge();
                    }
                    let can_collapse = state.selected_vertex_indices.len() == 2;
                    if ui.add_enabled(can_collapse, egui::Button::new("Collapse Edge / Merge (Ctrl+Alt+M)")).on_hover_text("Merges the 2 selected vertices into their midpoint, simplifying edges").clicked() {
                        state.collapse_selected_edge();
                    }
                    if ui.button("Snap to Nearest Vertex (64 uu)").on_hover_text("Snaps selected vertices seamlessly to nearest vertices of adjacent world brushes").clicked() {
                        state.snap_selected_vertices_to_nearest(64.0);
                    }

                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Tip: Click vertex handles in 3D viewport or use Shift+Click to multi-select.").italics().size(11.0));
                }
                EditorMode::BrushClip => {
                    ui.heading("Brush Clipping Tool");
                    ui.separator();

                    let target_name = if let Some(idx) = state.selected_brush_index {
                        format!("Brush #{} ({})", idx, state.world_brushes.get(idx).map(|b| b.name.as_str()).unwrap_or("Unknown"))
                    } else {
                        "Active Red Builder Brush".to_string()
                    };
                    ui.label(egui::RichText::new(format!("Target: {}", target_name)).strong().color(egui::Color32::from_rgb(255, 200, 100)));

                    ui.label(format!("Clip Markers: {}/3", state.clip_points.len()));
                    for (i, pt) in state.clip_points.iter().enumerate() {
                        ui.label(format!("  #{}: ({:.0}, {:.0}, {:.0})", i + 1, pt.x, pt.y, pt.z));
                    }

                    ui.horizontal(|ui| {
                        if ui.button("Marker at Builder").clicked() {
                            state.add_clip_point(state.builder_brush.location);
                        }
                        if ui.button("Marker at Camera").clicked() {
                            let cam_focus = state.camera.position + state.camera.forward() * 250.0;
                            state.add_clip_point(cam_focus);
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.button("Clear Markers").clicked() {
                            state.clear_clip_points();
                        }
                        let flip_text = if state.clip_normal_flipped { "Invert Normal" } else { "Normal: Default" };
                        if ui.button(flip_text).clicked() {
                            state.flip_clip_normal();
                        }
                    });

                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Perform Cut").strong());
                    if ui.button(egui::RichText::new("Clip Brush (Keep Half)").color(egui::Color32::from_rgb(255, 120, 120)).strong()).clicked() {
                        state.execute_clip_brush();
                    }
                    if ui.button(egui::RichText::new("Split Brush (Keep Both)").color(egui::Color32::from_rgb(120, 220, 255)).strong()).clicked() {
                        state.execute_split_brush();
                    }
                }
                EditorMode::CameraSelection => {
                    if state.ui_experience_mode == crate::editor_state::UiExperienceMode::Standard {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(format!("✨ {}", tr(lang, "sidebar.creation_hub"))).strong().color(egui::Color32::from_rgb(100, 220, 255)));
                                if ui.small_button(tr(lang, "action.open_ext")).clicked() {
                                    state.show_action_wizards_dialog = true;
                                }
                            });
                            ui.label(egui::RichText::new(tr(lang, "sidebar.creation_hub_desc")).size(10.5).color(egui::Color32::from_rgb(160, 175, 195)));
                            ui.horizontal(|ui| {
                                if ui.button(format!("🚪 {}", tr(lang, "sidebar.hub.lift"))).clicked() {
                                    state.active_wizard_tab = crate::editor_state::WizardTab::MoverLift;
                                    state.show_action_wizards_dialog = true;
                                }
                                if ui.button(format!("🌀 {}", tr(lang, "sidebar.hub.teleport"))).clicked() {
                                    state.active_wizard_tab = crate::editor_state::WizardTab::TeleporterPair;
                                    state.show_action_wizards_dialog = true;
                                }
                                if ui.button(format!("💧 {}", tr(lang, "sidebar.hub.water"))).clicked() {
                                    state.active_wizard_tab = crate::editor_state::WizardTab::WaterBasin;
                                    state.show_action_wizards_dialog = true;
                                }
                            });
                            ui.horizontal(|ui| {
                                if ui.button(format!("🎯 {}", tr(lang, "sidebar.hub.jumppad"))).clicked() {
                                    state.active_wizard_tab = crate::editor_state::WizardTab::JumpPad;
                                    state.show_action_wizards_dialog = true;
                                }
                                if ui.button(format!("🏗 {}", tr(lang, "sidebar.hub.room"))).clicked() {
                                    state.active_wizard_tab = crate::editor_state::WizardTab::RoomCorridor;
                                    state.show_action_wizards_dialog = true;
                                }
                                if ui.button(format!("🎨 {}", tr(lang, "sidebar.hub.presets"))).clicked() {
                                    state.show_visual_presets_dialog = true;
                                }
                            });
                        });
                        ui.add_space(4.0);
                    }

                    ui.heading(tr(lang, "sidebar.builder_brush"));
                    ui.separator();

                    // Core 4 Primitives always visible (Cube, Cylinder, Sheet, Stairs)
                    egui::Grid::new("builder_shapes_core_grid").num_columns(2).spacing([4.0, 4.0]).show(ui, |ui| {
                        if ui.selectable_label(state.builder_shape == BuilderShape::Cube, tr(lang, "builder.cube"))
                            .pro_tooltip("Cube Builder", None, "Generates a rectangular room or solid box brush with configurable X, Y, Z dimensions.")
                            .clicked()
                        {
                            let s = state.builder_cube_size;
                            state.set_builder_cube(s);
                        }
                        if ui.selectable_label(state.builder_shape == BuilderShape::Cylinder, tr(lang, "builder.cylinder"))
                            .pro_tooltip("Cylinder Builder", None, "Generates a circular polygonal cylinder with configurable height, radius, and side subdivisions.")
                            .clicked()
                        {
                            let (h, r, s) = state.builder_cylinder_params;
                            state.set_builder_cylinder(h, r, s);
                        }
                        ui.end_row();

                        if ui.selectable_label(state.builder_shape == BuilderShape::Sheet, tr(lang, "builder.sheet"))
                            .pro_tooltip("Sheet Builder", None, "Generates a flat 2D planar polygon sheet for water surfaces, zone portals, or banners.")
                            .clicked()
                        {
                            let (w, h) = state.builder_sheet_params;
                            state.set_builder_sheet(w, h);
                        }
                        if ui.selectable_label(state.builder_shape == BuilderShape::Stairs, tr(lang, "builder.stairs"))
                            .pro_tooltip("Linear Stairs Builder", None, "Generates straight stairs with customizable step count, step height, width, and step length.")
                            .clicked()
                        {
                            let (steps, h, w, l) = state.builder_stairs_params;
                            state.set_builder_stairs(steps, h, w, l);
                        }
                        ui.end_row();
                    });

                    // Progressive Disclosure: In Standard mode, specialist primitives & Math live in a collapsible section
                    let is_specialist_shape = !matches!(
                        state.builder_shape,
                        BuilderShape::Cube | BuilderShape::Cylinder | BuilderShape::Sheet | BuilderShape::Stairs
                    );
                    let render_specialist_shapes_and_math = |ui: &mut egui::Ui, state: &mut EditorState| {
                        egui::Grid::new("builder_shapes_specialist_grid").num_columns(3).spacing([4.0, 4.0]).show(ui, |ui| {
                            if ui.selectable_label(state.builder_shape == BuilderShape::Cone, tr(lang, "builder.cone"))
                                .pro_tooltip("Cone Builder", None, "Generates a conical brush with configurable radius, height, and circular sides.")
                                .clicked()
                            {
                                let (h, r, s) = state.builder_cone_params;
                                state.set_builder_cone(h, r, s);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::CurvedStairs, tr(lang, "builder.curved"))
                                .pro_tooltip("Curved Stairs Builder", None, "Generates curved circular steps with inner radius, step height, width, and angular sweep.")
                                .clicked()
                            {
                                let (steps, h, ir, w, a, ccw) = state.builder_curved_stairs_params;
                                state.set_builder_curved_stairs(steps, h, ir, w, a, ccw);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::SpiralStairs, tr(lang, "builder.spiral"))
                                .pro_tooltip("Spiral Stairs Builder", None, "Generates multi-revolution spiral staircases wrapping around a central vertical column.")
                                .clicked()
                            {
                                let (steps, h, ir, w, a, ccw) = state.builder_spiral_stairs_params;
                                state.set_builder_spiral_stairs(steps, h, ir, w, a, ccw);
                            }
                            ui.end_row();

                            if ui.selectable_label(state.builder_shape == BuilderShape::Sphere, tr(lang, "builder.sphere"))
                                .pro_tooltip("Sphere Builder", None, "Generates a geodesic sphere with configurable radius, rings, and segment divisions.")
                                .clicked()
                            {
                                let (r, rings, segs) = state.builder_sphere_params;
                                state.set_builder_sphere(r, rings, segs);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::Tetrahedron, tr(lang, "builder.tetra"))
                                .pro_tooltip("Tetrahedron Builder", None, "Generates a regular 4-sided polyhedral pyramid.")
                                .clicked()
                            {
                                let r = state.builder_tetrahedron_params;
                                state.set_builder_tetrahedron(r);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::Torus, tr(lang, "builder.torus"))
                                .pro_tooltip("Torus / Ring Builder", None, "Generates a circular donut/ring brush (Tarquin Torus) with ring radius and tube thickness.")
                                .clicked()
                            {
                                let (rr, tr, rs, ts) = state.builder_torus_params;
                                state.set_builder_torus(rr, tr, rs, ts);
                            }
                            ui.end_row();

                            if ui.selectable_label(state.builder_shape == BuilderShape::Volumetric, tr(lang, "builder.volume"))
                                .pro_tooltip("Volumetric Sheet Builder", None, "Generates crossed multi-plane sheets for atmospheric volumetric fog, smoke, or light rays.")
                                .clicked()
                            {
                                let (w, h, s) = state.builder_volumetric_params;
                                state.set_builder_volumetric(w, h, s);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::Terrain, tr(lang, "builder.terrain"))
                                .pro_tooltip("Procedural Terrain Builder", None, "Generates a heightmapped undulating terrain grid with procedural noise seed and amplitude.")
                                .clicked()
                            {
                                let (ws, ls, sz, amp, seed) = state.builder_terrain_params;
                                state.set_builder_terrain(ws, ls, sz, amp, seed);
                            }
                            if ui.selectable_label(state.builder_shape == BuilderShape::Arch, tr(lang, "builder.arch"))
                                .pro_tooltip("Architectural Arch Builder", None, "Generates Roman, Gothic, Norman, or Tudor structural arches and hollow walkable doorways.")
                                .clicked()
                            {
                                state.set_builder_arch(state.builder_arch_config.clone());
                            }
                            ui.end_row();
                        });

                        ui.add_space(6.0);

                        // UnrealEd Math Expression Evaluator
                        ui.horizontal(|ui| {
                            ui.label("Math:");
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut state.math_expression_input)
                                    .desired_width(120.0)
                                    .hint_text("=512-128 or =60*3.5"),
                            );
                            let eval_clicked = ui.small_button("=").on_hover_text("Evaluate mathematical formula").clicked();
                            let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                            if (eval_clicked || enter_pressed || resp.lost_focus()) && !state.math_expression_input.trim().is_empty() {
                                let _ = state.evaluate_math_expression_input();
                            }
                        });
                        if let Some(res) = state.last_math_result {
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    egui::RichText::new(format!("= {res:.2}"))
                                        .monospace()
                                        .strong()
                                        .color(egui::Color32::from_rgb(100, 230, 150)),
                                );
                                if state.builder_shape == BuilderShape::Cube && (16.0..=4096.0).contains(&res) {
                                    if ui.small_button("→X").on_hover_text("Apply result to Cube X").clicked() {
                                        let mut s = state.builder_cube_size;
                                        s.x = res;
                                        state.set_builder_cube(s);
                                    }
                                    if ui.small_button("→Y").on_hover_text("Apply result to Cube Y").clicked() {
                                        let mut s = state.builder_cube_size;
                                        s.y = res;
                                        state.set_builder_cube(s);
                                    }
                                    if ui.small_button("→Z").on_hover_text("Apply result to Cube Z").clicked() {
                                        let mut s = state.builder_cube_size;
                                        s.z = res;
                                        state.set_builder_cube(s);
                                    }
                                    if ui.small_button("→XYZ").on_hover_text("Apply result to Cube X, Y, Z").clicked() {
                                        state.set_builder_cube(Vec3::splat(res));
                                    }
                                }
                            });
                        }
                    };

                    if state.ui_experience_mode == crate::editor_state::UiExperienceMode::Standard {
                        egui::CollapsingHeader::new("More Primitives & Math...")
                            .default_open(is_specialist_shape)
                            .show(ui, |ui| {
                                render_specialist_shapes_and_math(ui, state);
                            });
                    } else {
                        ui.add_space(4.0);
                        render_specialist_shapes_and_math(ui, state);
                    }

            ui.add_space(6.0);

            match state.builder_shape {
                BuilderShape::Cube => {
                    let mut size = state.builder_cube_size;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        changed |= ui.add(egui::DragValue::new(&mut size.x).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        changed |= ui.add(egui::DragValue::new(&mut size.y).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        changed |= ui.add(egui::DragValue::new(&mut size.z).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    if changed {
                        state.set_builder_cube(size);
                    }
                }
                BuilderShape::Cylinder => {
                    let (mut h, mut r, mut s) = state.builder_cylinder_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut r).speed(8.0).range(8.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sides:");
                        changed |= ui.add(egui::DragValue::new(&mut s).range(3..=64)).changed();
                    });
                    if changed {
                        state.set_builder_cylinder(h, r, s);
                    }
                }
                BuilderShape::Cone => {
                    let (mut h, mut r, mut s) = state.builder_cone_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut r).speed(8.0).range(8.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sides:");
                        changed |= ui.add(egui::DragValue::new(&mut s).range(3..=64)).changed();
                    });
                    if changed {
                        state.set_builder_cone(h, r, s);
                    }
                }
                BuilderShape::Sheet => {
                    let (mut w, mut h) = state.builder_sheet_params;
                    let mut axis = state.builder_sheet_axis;
                    let mut preset = state.builder_sheet_preset;
                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(&mut w).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Axis:");
                        changed |= ui.selectable_value(&mut axis, ut99_bsp::SheetAxis::XY, "XY (Floor)").changed();
                        changed |= ui.selectable_value(&mut axis, ut99_bsp::SheetAxis::XZ, "XZ (Front)").changed();
                        changed |= ui.selectable_value(&mut axis, ut99_bsp::SheetAxis::YZ, "YZ (Side)").changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Preset:");
                        egui::ComboBox::from_id_salt("sheet_preset_combo")
                            .selected_text(match preset {
                                ut99_bsp::SheetPreset::Normal => "Normal (Solid)",
                                ut99_bsp::SheetPreset::Water => "Water Surface",
                                ut99_bsp::SheetPreset::AcidSlime => "Acid / Slime",
                                ut99_bsp::SheetPreset::Banner => "Banner / Flag",
                                ut99_bsp::SheetPreset::ZonePortal => "Zone Portal",
                            })
                            .show_ui(ui, |ui| {
                                changed |= ui.selectable_value(&mut preset, ut99_bsp::SheetPreset::Normal, "Normal (Solid)").changed();
                                changed |= ui.selectable_value(&mut preset, ut99_bsp::SheetPreset::Water, "Water Surface").changed();
                                changed |= ui.selectable_value(&mut preset, ut99_bsp::SheetPreset::AcidSlime, "Acid / Slime").changed();
                                changed |= ui.selectable_value(&mut preset, ut99_bsp::SheetPreset::Banner, "Banner / Flag").changed();
                                changed |= ui.selectable_value(&mut preset, ut99_bsp::SheetPreset::ZonePortal, "Zone Portal").changed();
                            });
                    });

                    if changed {
                        state.set_builder_sheet_advanced(w, h, axis, preset);
                    }
                }
                BuilderShape::Stairs => {
                    let (mut steps, mut h, mut w, mut l) = state.builder_stairs_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Steps:");
                        changed |= ui.add(egui::DragValue::new(&mut steps).range(1..=64)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Step H:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(2.0).range(1.0..=256.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(&mut w).speed(16.0).range(16.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Step L:");
                        changed |= ui.add(egui::DragValue::new(&mut l).speed(4.0).range(4.0..=512.0)).changed();
                    });
                    if changed {
                        state.set_builder_stairs(steps, h, w, l);
                    }
                }
                BuilderShape::CurvedStairs => {
                    let (mut steps, mut h, mut ir, mut w, mut a, mut ccw) = state.builder_curved_stairs_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Steps:");
                        changed |= ui.add(egui::DragValue::new(&mut steps).range(1..=64)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Step H:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(2.0).range(1.0..=256.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Inner R:");
                        changed |= ui.add(egui::DragValue::new(&mut ir).speed(8.0).range(16.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(&mut w).speed(8.0).range(16.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Angle:");
                        changed |= ui.add(egui::DragValue::new(&mut a).speed(5.0).range(15.0..=360.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut ccw, "Counter-Clockwise").changed();
                    });
                    if changed {
                        state.set_builder_curved_stairs(steps, h, ir, w, a, ccw);
                    }
                }
                BuilderShape::SpiralStairs => {
                    let (mut steps, mut h, mut ir, mut w, mut a, mut ccw) = state.builder_spiral_stairs_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Steps:");
                        changed |= ui.add(egui::DragValue::new(&mut steps).range(2..=128)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Step H:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(2.0).range(1.0..=256.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Inner R:");
                        changed |= ui.add(egui::DragValue::new(&mut ir).speed(8.0).range(16.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(&mut w).speed(8.0).range(16.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total Angle:");
                        changed |= ui.add(egui::DragValue::new(&mut a).speed(15.0).range(90.0..=1440.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut ccw, "Counter-Clockwise").changed();
                    });
                    if changed {
                        state.set_builder_spiral_stairs(steps, h, ir, w, a, ccw);
                    }
                }
                BuilderShape::Sphere => {
                    let (mut r, mut rings, mut segs) = state.builder_sphere_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut r).speed(8.0).range(8.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rings:");
                        changed |= ui.add(egui::DragValue::new(&mut rings).range(2..=32)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Segments:");
                        changed |= ui.add(egui::DragValue::new(&mut segs).range(3..=64)).changed();
                    });
                    if changed {
                        state.set_builder_sphere(r, rings, segs);
                    }
                }
                BuilderShape::Tetrahedron => {
                    let mut r = state.builder_tetrahedron_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut r).speed(8.0).range(16.0..=2048.0)).changed();
                    });
                    if changed {
                        state.set_builder_tetrahedron(r);
                    }
                }
                BuilderShape::Torus => {
                    let (mut rr, mut tr, mut rs, mut ts) = state.builder_torus_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Ring Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut rr).speed(8.0).range(32.0..=2048.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tube Radius:");
                        changed |= ui.add(egui::DragValue::new(&mut tr).speed(4.0).range(8.0..=1024.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Ring Sides:");
                        changed |= ui.add(egui::DragValue::new(&mut rs).range(3..=64)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Tube Sides:");
                        changed |= ui.add(egui::DragValue::new(&mut ts).range(3..=32)).changed();
                    });
                    if changed {
                        state.set_builder_torus(rr, tr, rs, ts);
                    }
                }
                BuilderShape::Volumetric => {
                    let (mut w, mut h, mut s) = state.builder_volumetric_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui.add(egui::DragValue::new(&mut w).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Height:");
                        changed |= ui.add(egui::DragValue::new(&mut h).speed(16.0).range(16.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Sheets:");
                        changed |= ui.add(egui::DragValue::new(&mut s).range(1..=8)).changed();
                    });
                    if changed {
                        state.set_builder_volumetric(w, h, s);
                    }
                }
                BuilderShape::Terrain => {
                    let (mut ws, mut ls, mut sz, mut amp, mut seed) = state.builder_terrain_params;
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Width Steps:");
                        changed |= ui.add(egui::DragValue::new(&mut ws).range(1..=32)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Length Steps:");
                        changed |= ui.add(egui::DragValue::new(&mut ls).range(1..=32)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Step Size:");
                        changed |= ui.add(egui::DragValue::new(&mut sz).speed(8.0).range(16.0..=512.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Amplitude:");
                        changed |= ui.add(egui::DragValue::new(&mut amp).speed(8.0).range(0.0..=1024.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Seed:");
                        changed |= ui.add(egui::DragValue::new(&mut seed).range(0..=9999)).changed();
                    });
                    if changed {
                        state.set_builder_terrain(ws, ls, sz, amp, seed);
                    }
                }
                BuilderShape::Arch => {
                    let mut cfg = state.builder_arch_config.clone();
                    let mut changed = false;

                    ui.horizontal(|ui| {
                        ui.label("Style:");
                        egui::ComboBox::from_id_salt("arch_type_combo")
                            .selected_text(cfg.arch_type.name())
                            .show_ui(ui, |ui| {
                                for t in ut99_bsp::ArchType::ALL {
                                    changed |= ui.selectable_value(&mut cfg.arch_type, *t, t.name()).changed();
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Opening Span:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.span_width).speed(16.0).range(32.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Jamb Height:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.jamb_height).speed(16.0).range(0.0..=4096.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Total Height:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.total_height).speed(16.0).range(48.0..=8192.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wall Depth:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.depth).speed(8.0).range(16.0..=1024.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Wall Thickness:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.wall_thickness).speed(4.0).range(8.0..=512.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Segments:");
                        changed |= ui.add(egui::DragValue::new(&mut cfg.segments).range(4..=64)).changed();
                    });
                    ui.horizontal(|ui| {
                        changed |= ui.checkbox(&mut cfg.is_doorway, "Hollow Doorway (Walkable)").changed();
                    });

                    if changed {
                        state.set_builder_arch(cfg);
                    }
                }
            }

            ui.add_space(8.0);
            ui.heading(tr(lang, "sidebar.builder_transform"));
            ui.separator();
            let old_builder_brush = state.builder_brush.clone();
            let mut builder_xform_changed = false;
            ui.horizontal(|ui| {
                ui.label("Pos:");
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.x).speed(16.0).prefix("X:")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.y).speed(16.0).prefix("Y:")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.z).speed(16.0).prefix("Z:")).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Rot:");
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.rotation.x).speed(1.0).prefix("P:").suffix("°")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.rotation.y).speed(1.0).prefix("Y:").suffix("°")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.rotation.z).speed(1.0).prefix("R:").suffix("°")).changed();
            });
            ui.horizontal(|ui| {
                ui.label("Scale:");
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.scale.x).speed(0.05).range(0.01..=100.0).prefix("X:")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.scale.y).speed(0.05).range(0.01..=100.0).prefix("Y:")).changed();
                builder_xform_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.scale.z).speed(0.05).range(0.01..=100.0).prefix("Z:")).changed();
            });
            if builder_xform_changed {
                let interacting = ui.ctx().input(|i| i.pointer.any_down())
                    || ui.ctx().memory(|m| m.focused().is_some());
                state.history.push_interactive(
                    crate::history::Action::ModifyBuilderBrush {
                        old_brush: old_builder_brush,
                        new_brush: state.builder_brush.clone(),
                        description: "Transform Builder Brush".to_string(),
                    },
                    interacting,
                );
            }
            ui.add_space(4.0);
            if ui.button(format!("↺ {}", tr(lang, "sidebar.reset_builder")))
                .pro_tooltip("Reset Builder to Origin", None, "Reset Red Builder Brush position to origin (0, 0, 0) and rotations to zero.")
                .clicked()
            {
                state.reset_builder_brush();
            }

            ui.add_space(12.0);
            ui.heading(tr(lang, "sidebar.csg_actions"));
            ui.separator();

            if ui.button(egui::RichText::new(tr(lang, "sidebar.csg_add")).color(egui::Color32::from_rgb(100, 160, 255)).strong())
                .pro_tooltip_4level(
                    "CSG Add (Solid)",
                    Some("Ctrl+A"),
                    "Add the Red Builder Brush into world geometry as solid matter",
                    Some("Shift+Click: Keep builder in place"),
                    Some("Operation: Additive Solid"),
                    Some("UnrealEd 2.0 Gotcha: Always subtract the room first, then add objects inside. Use Semi-Solids for pillars and beams to prevent BSP cuts."),
                )
                .clicked()
            {
                state.csg_add();
            }
            if ui.button(egui::RichText::new(tr(lang, "sidebar.csg_subtract")).color(egui::Color32::from_rgb(255, 220, 80)).strong())
                .pro_tooltip_4level(
                    "CSG Subtract (Carve)",
                    Some("Ctrl+S"),
                    "Carve the Red Builder Brush volume out of solid world geometry",
                    Some("Shift+Click: Keep builder in place"),
                    Some("Operation: Subtractive Hollow"),
                    Some("UnrealEd 2.0 Gotcha: In Unreal Engine 1, all playable volumes are carved out of solid rock. Keep vertices snapped to grid."),
                )
                .clicked()
            {
                state.csg_subtract();
            }
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "sidebar.csg_intersect"))
                    .pro_tooltip("CSG Intersect", None, "Crop the Red Builder Brush to keep only the volume overlapping existing solid world geometry.")
                    .clicked()
                {
                    state.csg_intersect();
                }
                if ui.button(tr(lang, "sidebar.csg_deintersect"))
                    .pro_tooltip("CSG Deintersect", None, "Remove all existing solid world geometry from the Red Builder Brush, isolating exterior void volume.")
                    .clicked()
                {
                    state.csg_deintersect();
                }
            });
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "sidebar.add_special")).on_hover_text("Open Add Special Brush dialog (Semi-solid, Non-solid, Portals, Water)").clicked() {
                    state.show_add_special_dialog = true;
                }
                let mover_btn = ui
                    .button("Add Mover")
                    .on_hover_text("Left-click: Add Mover | Right-click: Choose Mover Subclass (AssertMover, AttachMover, ElevatorMover, GradualMover, LoopMover, MixMover, RotatingMover)");
                if mover_btn.clicked() {
                    state.add_mover_from_builder();
                }
                mover_btn.context_menu(|ui| {
                    ui.label(egui::RichText::new("Select Mover Class:").strong());
                    ui.separator();
                    for &cls in &[
                        "Mover",
                        "AssertMover",
                        "AttachMover",
                        "ElevatorMover",
                        "GradualMover",
                        "LoopMover",
                        "MixMover",
                        "RotatingMover",
                    ] {
                        if ui.button(cls).clicked() {
                            state.add_mover_from_builder_with_class(cls);
                            ui.close_menu();
                        }
                    }
                });
            });
            ui.horizontal(|ui| {
                if ui.button(format!("{} X", tr(lang, "action.mirror"))).on_hover_text("Mirror active/selected brush across X axis").clicked() {
                    state.mirror_selected(crate::editor_state::MirrorAxis::X);
                }
                if ui.button(format!("{} Y", tr(lang, "action.mirror"))).on_hover_text("Mirror active/selected brush across Y axis").clicked() {
                    state.mirror_selected(crate::editor_state::MirrorAxis::Y);
                }
                if ui.button(format!("{} Z", tr(lang, "action.mirror"))).on_hover_text("Mirror active/selected brush across Z axis").clicked() {
                    state.mirror_selected(crate::editor_state::MirrorAxis::Z);
                }
            });
                } // end EditorMode::CameraSelection
            } // end match state.editor_mode



            if state.ui_experience_mode == crate::editor_state::UiExperienceMode::Architect {
                ui.add_space(12.0);
                ui.heading(tr(lang, "sidebar.grid_snap"));
                ui.separator();
                ui.horizontal(|ui| {
                    ui.checkbox(&mut state.snap_to_grid, tr(lang, "sidebar.snap"));
                    if ui.button("➖ ([)").on_hover_text("Halve grid step (UnrealEd Standard [ key)").clicked() {
                        state.step_grid_down();
                    }
                    if ui.button("➕ (])").on_hover_text("Double grid step (UnrealEd Standard ] key)").clicked() {
                        state.step_grid_up();
                    }
                    ui.label(egui::RichText::new(format!("{:.0} uu", state.grid_step)).strong().color(egui::Color32::from_rgb(100, 220, 255)));
                });
                ui.horizontal_wrapped(|ui| {
                    for &step in &[1.0, 2.0, 4.0, 8.0, 16.0, 32.0, 64.0, 128.0, 256.0, 512.0] {
                        let desc = match step as i32 {
                            1 | 2 | 4 => "Fine architectural detailing, trims, and vertex alignment",
                            8 | 16 => "Stair risers, door jambs, and thin structural walls",
                            32 | 64 => "Standard doorways, corridors, and window frames",
                            128 => "Main player corridors, room heights, and standard portals",
                            256 => "Large halls, multi-floor atriums, and building footprints",
                            512 => "Arena scale volumes, large outdoor terrain, and skyboxes",
                            _ => "Grid step size in Unreal Units",
                        };
                        if ui.selectable_label((state.grid_step - step).abs() < 0.1, format!("{step:.0}"))
                            .pro_tooltip(&format!("Grid Step: {step:.0} UU"), None, desc)
                            .clicked()
                        {
                            state.set_grid_step(step);
                        }
                    }
                });
            }

            ui.add_space(16.0);
            ui.heading(tr(lang, "sidebar.move_builder"));
            ui.separator();
            let step = state.grid_step;
            ui.horizontal(|ui| {
                for (lbl, delta) in [
                    ("-X", Vec3::new(-step, 0.0, 0.0)),
                    ("+X", Vec3::new(step, 0.0, 0.0)),
                    ("-Y", Vec3::new(0.0, -step, 0.0)),
                    ("+Y", Vec3::new(0.0, step, 0.0)),
                    ("-Z", Vec3::new(0.0, 0.0, -step)),
                    ("+Z", Vec3::new(0.0, 0.0, step)),
                ] {
                    if ui.button(lbl).clicked() {
                        let old_brush = state.builder_brush.clone();
                        state.builder_brush.location += delta;
                        state.history.push(crate::history::Action::ModifyBuilderBrush {
                            old_brush,
                            new_brush: state.builder_brush.clone(),
                            description: format!("Nudge Builder Brush {lbl}"),
                        });
                    }
                }
            });

            ui.add_space(16.0);
            ui.heading(tr(lang, "sidebar.active_material"));
            ui.separator();
            if let Some(ref tex) = state.active_texture {
                ui.label(egui::RichText::new(tex).strong().color(egui::Color32::from_rgb(255, 215, 0)));
            } else {
                ui.label(tr(lang, "sidebar.none_selected"));
            }
            ui.horizontal(|ui| {
                if ui.button(tr(lang, "sidebar.browse_textures")).clicked() {
                    state.show_texture_browser = true;
                }
                if ui.button(tr(lang, "sidebar.apply_to_builder")).clicked() {
                    state.apply_active_texture_to_builder();
                }
            });
            });
        });

    // 3. Right Panel (Inspector & World Outliner)
    egui::SidePanel::right("right_inspector")
        .default_width(280.0)
        .show(ctx, |ui| {
            let lang = state.language;
            // Panel View Mode Selector (Split / Outliner / Inspector with Vector Icons)
            ui.horizontal(|ui| {
                if icons::icon_label_button(ui, Icon::Split, Some(tr(lang, "inspector.split")), state.inspector_panel_mode == InspectorPanelMode::Split)
                    .pro_tooltip("Split View", None, "Show World Outliner (top) and Selection Properties (bottom)")
                    .clicked()
                {
                    state.inspector_panel_mode = InspectorPanelMode::Split;
                }
                if icons::icon_label_button(ui, Icon::Outliner, Some(tr(lang, "inspector.outliner")), state.inspector_panel_mode == InspectorPanelMode::OutlinerOnly)
                    .pro_tooltip("Outliner Only", None, "Maximize World Outliner to 100% height for complex scene navigation")
                    .clicked()
                {
                    state.inspector_panel_mode = InspectorPanelMode::OutlinerOnly;
                }
                if icons::icon_label_button(ui, Icon::Inspector, Some(tr(lang, "inspector.inspector")), state.inspector_panel_mode == InspectorPanelMode::InspectorOnly)
                    .pro_tooltip("Inspector Only", None, "Maximize Selection Properties to 100% height for object editing")
                    .clicked()
                {
                    state.inspector_panel_mode = InspectorPanelMode::InspectorOnly;
                }
            });
            ui.separator();

            let show_outliner = state.inspector_panel_mode != InspectorPanelMode::InspectorOnly;
            let show_inspector = state.inspector_panel_mode != InspectorPanelMode::OutlinerOnly;

            if show_outliner {
                layers_outliner::render_layers_outliner(ui, state);
            }

            if show_inspector {

            // Multi-Object Selection Batch Operations Panel
            if state.has_multi_selection() {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(tr(lang, "inspector.multi_selection")).strong().color(egui::Color32::from_rgb(100, 220, 255)));
                        ui.label(format!("{} Brushes, {} Actors", state.multi_selected_brushes.len(), state.multi_selected_actors.len()));
                    });
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr(lang, "inspector.snap_floor")).on_hover_text("Snap all selected objects down to the nearest floor").clicked() {
                            state.snap_multi_selected_to_floor();
                        }
                        if ui.button(tr(lang, "inspector.snap_grid")).on_hover_text("Snap all selected objects to the active grid step").clicked() {
                            state.snap_multi_selected_to_grid();
                        }
                        if ui.button(tr(lang, "inspector.apply_texture")).on_hover_text("Apply currently active browser texture to all selected brushes").clicked() {
                            state.apply_active_texture_to_multi_selected();
                        }
                        if ui.button(tr(lang, "inspector.duplicate")).on_hover_text("Duplicate all selected objects with offset").clicked() {
                            state.duplicate_selected_batch(glam::Vec3::new(state.grid_step, state.grid_step, 0.0));
                        }
                        if ui.button(tr(lang, "inspector.save_prefab")).on_hover_text("Save all selected brushes as a composite prefab").clicked() {
                            state.create_prefab_from_selection("", "Custom");
                        }
                        if ui.button(tr(lang, "inspector.delete")).on_hover_text("Delete all selected objects").clicked() {
                            state.delete_selected_batch();
                        }
                        if ui.button(tr(lang, "action.clear")).on_hover_text("Clear multi-selection").clicked() {
                            state.clear_multi_selection();
                        }
                    });
                    if state.multi_selected_actors.len() >= 2 {
                        ui.separator();
                        ui.horizontal_wrapped(|ui| {
                            ui.label(egui::RichText::new(tr(lang, "inspector.align_actors")).strong().color(egui::Color32::from_rgb(255, 200, 100)));
                            if ui.button("Min X").on_hover_text("Align actors to minimum X").clicked() {
                                state.align_selected_actors(crate::editor_state::AlignAxis::X, crate::editor_state::AlignMode::Min);
                            }
                            if ui.button("Center X").on_hover_text("Align actors to average X").clicked() {
                                state.align_selected_actors(crate::editor_state::AlignAxis::X, crate::editor_state::AlignMode::Center);
                            }
                            if ui.button("Min Y").on_hover_text("Align actors to minimum Y").clicked() {
                                state.align_selected_actors(crate::editor_state::AlignAxis::Y, crate::editor_state::AlignMode::Min);
                            }
                            if ui.button("Center Y").on_hover_text("Align actors to average Y").clicked() {
                                state.align_selected_actors(crate::editor_state::AlignAxis::Y, crate::editor_state::AlignMode::Center);
                            }
                            if ui.button("Min Z (Floor)").on_hover_text("Align actors to common lowest floor height").clicked() {
                                state.align_selected_actors(crate::editor_state::AlignAxis::Z, crate::editor_state::AlignMode::Min);
                            }
                            if state.multi_selected_actors.len() >= 3 {
                                if ui.button("Distribute X").on_hover_text("Distribute actors evenly along X").clicked() {
                                    state.distribute_selected_actors_evenly(crate::editor_state::AlignAxis::X);
                                }
                                if ui.button("Distribute Y").on_hover_text("Distribute actors evenly along Y").clicked() {
                                    state.distribute_selected_actors_evenly(crate::editor_state::AlignAxis::Y);
                                }
                            }
                            if ui.button("Align Dialog...").on_hover_text("Open full Align & Distribute Dialog").clicked() {
                                state.show_actor_align_dialog = true;
                            }
                        });
                    }
                });
            }

            ui.add_space(12.0);
            ui.heading(tr(lang, "inspector.selection_properties"));
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("inspector_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut duplicate_req = false;
                    let mut snap_floor_req = false;

            // Inspector for selected Mover
            if let Some(m_idx) = state.selected_mover_index {
                let mut focus_loc = None;
                let mut delete_mover = false;
                let mut switch_key = None;
                let mut save_cur_key = false;

                if let Some(mover) = state.movers.get_mut(m_idx) {
                    ui.label(format!("Mover: {}", mover.name));
                    ui.label(format!("Polygons: {}", mover.brush.polys.len()));
                    ui.separator();

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Keyframe Selection (0..7):").strong());
                        if ui.button("◀ Prev").on_hover_text("Switch to previous keyframe (cycles 0..7)").clicked() {
                            let cur = mover.current_key;
                            let prev = if cur == 0 { 7 } else { cur - 1 };
                            switch_key = Some(prev);
                        }
                        if ui.button("Next ▶").on_hover_text("Switch to next keyframe (cycles 0..7)").clicked() {
                            let cur = mover.current_key;
                            let next = (cur + 1) % 8;
                            switch_key = Some(next);
                        }
                    });
                    ui.horizontal_wrapped(|ui| {
                        for k in 0..8 {
                            let is_cur = mover.current_key == k;
                            let has_kf = mover.keyframes[k].is_some();
                            let text = if is_cur {
                                format!("*Key {}*", k)
                            } else if has_kf {
                                format!("Key {}", k)
                            } else {
                                format!("Key {} (+)", k)
                            };
                            let btn = if is_cur {
                                egui::Button::new(egui::RichText::new(text).color(egui::Color32::from_rgb(255, 220, 50)))
                            } else {
                                egui::Button::new(text)
                            };
                            if ui.add(btn).clicked() {
                                switch_key = Some(k);
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Scrub Pose:");
                        ui.add(egui::Slider::new(&mut state.mover_preview_time, 0.0..=1.0).text("t"));
                    });
                    ui.checkbox(&mut state.show_mover_trajectories, "Show 3D Trajectory Spline");

                    ui.label(format!("Editing Key {}:", mover.current_key));
                    let old_mover_snapshot = mover.clone();
                    ui.horizontal(|ui| {
                        ui.label("Pos X:");
                        ui.add(egui::DragValue::new(&mut mover.brush.location.x).speed(16.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Y:");
                        ui.add(egui::DragValue::new(&mut mover.brush.location.y).speed(16.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Z:");
                        ui.add(egui::DragValue::new(&mut mover.brush.location.z).speed(16.0));
                    });

                    ui.horizontal(|ui| {
                        if ui.button("💾 Save Key").clicked() {
                            save_cur_key = true;
                        }
                        if ui.button("🎯 Focus").clicked() {
                            focus_loc = Some(mover.brush.location);
                        }
                        if ui.button("📋 Duplicate").on_hover_text("Duplicate (Ctrl+D)").clicked() {
                            duplicate_req = true;
                        }
                        if ui.button("⬇ Snap Floor").on_hover_text("Snap to Floor (End)").clicked() {
                            snap_floor_req = true;
                        }
                        if ui.button("🗑 Delete").clicked() {
                            delete_mover = true;
                        }
                    });

                    ui.separator();
                    ui.label(egui::RichText::new("Class, State & Timing").strong());
                    ui.horizontal(|ui| {
                        ui.label("Mover Class:");
                        egui::ComboBox::from_id_salt("mover_class_combo")
                            .selected_text(&mover.mover_class)
                            .show_ui(ui, |ui| {
                                for &cls in &[
                                    "Mover",
                                    "AssertMover",
                                    "AttachMover",
                                    "ElevatorMover",
                                    "GradualMover",
                                    "LoopMover",
                                    "MixMover",
                                    "RotatingMover",
                                ] {
                                    ui.selectable_value(&mut mover.mover_class, cls.to_string(), cls);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("InitialState:");
                        egui::ComboBox::from_id_salt("mover_initial_state_combo")
                            .selected_text(&mover.initial_state)
                            .show_ui(ui, |ui| {
                                for &st in &[
                                    "StandOpenTimed",
                                    "BumpOpenTimed",
                                    "BumpButton",
                                    "TriggerOpenTimed",
                                    "TriggerToggle",
                                    "TriggerControl",
                                    "TriggerPound",
                                    "ConstantLoop",
                                ] {
                                    ui.selectable_value(&mut mover.initial_state, st.to_string(), st);
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Move Time (s):");
                        ui.add(egui::DragValue::new(&mut mover.move_time).speed(0.1).range(0.05..=60.0));
                        ui.label("Stay Open (s):");
                        ui.add(egui::DragValue::new(&mut mover.stay_open_time).speed(0.2).range(0.0..=120.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Delay Time (s):");
                        ui.add(egui::DragValue::new(&mut mover.delay_time).speed(0.1).range(0.0..=60.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Glide:");
                        egui::ComboBox::from_id_salt("mover_glide_combo")
                            .selected_text(&mover.glide_type)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut mover.glide_type, "MV_MoveByTime".to_string(), "MV_MoveByTime");
                                ui.selectable_value(&mut mover.glide_type, "MV_GlideByTime".to_string(), "MV_GlideByTime");
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("Encroach:");
                        egui::ComboBox::from_id_salt("mover_encroach_combo")
                            .selected_text(&mover.encroach_type)
                            .show_ui(ui, |ui| {
                                for &et in &[
                                    "ME_StopWhenEncroach",
                                    "ME_ReturnWhenEncroach",
                                    "ME_CrushWhenEncroach",
                                    "ME_IgnoreWhenEncroach",
                                ] {
                                    ui.selectable_value(&mut mover.encroach_type, et.to_string(), et);
                                }
                            });
                    });

                    ui.horizontal(|ui| {
                        ui.label("Tag:");
                        ui.text_edit_singleline(&mut mover.tag);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Event:");
                        ui.text_edit_singleline(&mut mover.event);
                    });

                    egui::CollapsingHeader::new("🔊 MoverSounds (5 Slots)")
                        .default_open(false)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("OpeningSound:");
                                ui.text_edit_singleline(&mut mover.opening_sound);
                            });
                            ui.horizontal(|ui| {
                                ui.label("OpenedSound:");
                                ui.text_edit_singleline(&mut mover.opened_sound);
                            });
                            ui.horizontal(|ui| {
                                ui.label("ClosingSound:");
                                ui.text_edit_singleline(&mut mover.closing_sound);
                            });
                            ui.horizontal(|ui| {
                                ui.label("ClosedSound:");
                                ui.text_edit_singleline(&mut mover.closed_sound);
                            });
                            ui.horizontal(|ui| {
                                ui.label("MoveAmbientSound:");
                                ui.text_edit_singleline(&mut mover.move_ambient_sound);
                            });
                        });

                    let mover_changed = old_mover_snapshot != *mover;
                    if mover_changed {
                        let new_mover_snapshot = mover.clone();
                        let mover_name = mover.name.clone();
                        let interacting = ui.ctx().input(|i| i.pointer.any_down()) || ui.ctx().memory(|m| m.focused().is_some());
                        state.history.push_interactive(
                            crate::history::Action::UpdateElevatorMoverConfig {
                                mover_index: m_idx,
                                old_mover: old_mover_snapshot,
                                new_mover: new_mover_snapshot,
                                description: format!("Edit Mover ({})", mover_name),
                            },
                            interacting,
                        );
                        state.map_is_dirty = true;
                    }
                }

                if let Some(k) = switch_key {
                    state.set_selected_mover_key(k);
                }
                if save_cur_key {
                    if let Some(mover) = state.movers.get(m_idx) {
                        let loc = mover.brush.location;
                        let rot = mover.brush.rotation;
                        state.save_selected_mover_current_key_pos(loc, rot);
                    }
                }
                if let Some(loc) = focus_loc {
                    state.camera.position = loc + Vec3::new(0.0, -350.0, 150.0);
                }
                if delete_mover {
                    if m_idx < state.movers.len() {
                        let mv = state.movers.remove(m_idx);
                        state.history.push(crate::history::Action::DeleteMover {
                            index: m_idx,
                            mover: mv,
                        });
                        state.selected_mover_index = None;
                        state.map_is_dirty = true;
                    }
                }
            } else if let Some(sel_idx) = state.selected_brush_index {
                let mut focus_brush = false;
                let mut delete_brush = false;
                let mut apply_active_tex = false;
                let mut pan_u = 0.0f32;
                let mut pan_v = 0.0f32;
                let mut scale_factor = None;
                let mut align_mode = None;
                let mut cylinder_repeats = None;
                let mut rotate_angle = None;
                let mut order_first = false;
                let mut order_last = false;
                let mut order_up = false;
                let mut order_down = false;
                let mut commit_brush_move: Option<(Vec3, Vec3)> = None;
                let mut commit_brush_rot: Option<(Vec3, Vec3)> = None;
                let mut commit_brush_scale: Option<(Vec3, Vec3, Vec<ut99_bsp::Polygon>)> = None;
                let mut start_brush_move: Option<Vec3> = None;
                let mut start_brush_rot: Option<Vec3> = None;
                let mut start_brush_scale: Option<Vec3> = None;
                let mut invert_brush_req = false;
                let mut bake_brush_req = false;
                let mut reset_rot_req = false;
                let mut reset_scale_req = false;
                let mut set_solidity_req = None;

                let cur_solidity = state.get_brush_solidity(sel_idx);

                if let Some(brush) = state.world_brushes.get_mut(sel_idx) {
                    ui.label(format!("Name: {}", brush.name));
                    ui.label(format!("Operation: {:?}", brush.csg_oper));
                    ui.label(format!("Polygons: {}", brush.polys.len()));

                    let (min_b, max_b) = brush.world_bounds();
                    let width = (max_b.x - min_b.x).abs();
                    let depth = (max_b.y - min_b.y).abs();
                    let height = (max_b.z - min_b.z).abs();

                    // UI-03: Accordion Section 1 - Transform & Bounds
                    egui::CollapsingHeader::new("📐 Transform & Bounds").default_open(true).show(ui, |ui| {
                        ui.label(format!("Size: {:.1} × {:.1} × {:.1} uu", width, depth, height));
                        ui.label(egui::RichText::new(format!("Metric: {:.2}m × {:.2}m × {:.2}m", width * 0.0213, depth * 0.0213, height * 0.0213)).color(egui::Color32::from_rgb(180, 200, 240)));

                        if height >= 8.0 && height <= 54.0 {
                            ui.label(egui::RichText::new(format!("ℹ Step Height: {:.0} uu (Walkable without jump)", height)).color(egui::Color32::from_rgb(100, 220, 120)));
                        } else if height > 54.0 && height <= 64.0 {
                            ui.label(egui::RichText::new(format!("⚠ Step: {:.0} uu (Requires jump; standard jump = 64 uu)", height)).color(egui::Color32::from_rgb(255, 200, 80)));
                        } else if height >= 65.0 && height <= 80.0 {
                            ui.label(egui::RichText::new(format!("ℹ Humanoid Height: {:.0} uu (UT99 player standing = 78 uu)", height)).color(egui::Color32::from_rgb(120, 200, 255)));
                        } else if height >= 120.0 && height <= 220.0 && (width >= 80.0 || depth >= 80.0) {
                            ui.label(egui::RichText::new("🚪 Doorway / Portal Sized").color(egui::Color32::from_rgb(160, 230, 160)));
                        }
                        if (width > 0.0 && width < 36.0) || (depth > 0.0 && depth < 36.0) {
                            ui.label(egui::RichText::new("⚠ Narrow Passage (UT99 player collision diameter = 34 uu)").color(egui::Color32::from_rgb(255, 140, 80)));
                        }

                        ui.separator();
                        let loc_before = brush.location;
                        let rot_before = brush.rotation;
                        let scale_before = brush.scale;

                        ui.horizontal(|ui| {
                            ui.label("Pos X:");
                            let rx = ui.add(egui::DragValue::new(&mut brush.location.x).speed(16.0));
                            ui.label("Pos Y:");
                            let ry = ui.add(egui::DragValue::new(&mut brush.location.y).speed(16.0));
                            ui.label("Pos Z:");
                            let rz = ui.add(egui::DragValue::new(&mut brush.location.z).speed(16.0));

                            // WRK-01: Atomic Undo registration on drag_stopped or lost_focus
                            if rx.drag_started() || ry.drag_started() || rz.drag_started() || rx.gained_focus() || ry.gained_focus() || rz.gained_focus() {
                                start_brush_move = Some(loc_before);
                            }
                            if rx.drag_stopped() || ry.drag_stopped() || rz.drag_stopped() || rx.lost_focus() || ry.lost_focus() || rz.lost_focus() {
                                commit_brush_move = Some((loc_before, brush.location));
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Pitch:");
                            let rp = ui.add(egui::DragValue::new(&mut brush.rotation.x).speed(1.0).suffix("°"));
                            ui.label("Yaw:");
                            let ry = ui.add(egui::DragValue::new(&mut brush.rotation.y).speed(1.0).suffix("°"));
                            ui.label("Roll:");
                            let rr = ui.add(egui::DragValue::new(&mut brush.rotation.z).speed(1.0).suffix("°"));

                            // WRK-01: Atomic Undo registration for rotation
                            if rp.drag_started() || ry.drag_started() || rr.drag_started() || rp.gained_focus() || ry.gained_focus() || rr.gained_focus() {
                                start_brush_rot = Some(rot_before);
                            }
                            if rp.drag_stopped() || ry.drag_stopped() || rr.drag_stopped() || rp.lost_focus() || ry.lost_focus() || rr.lost_focus() {
                                commit_brush_rot = Some((rot_before, brush.rotation));
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Scale:");
                            let sx = ui.add(egui::DragValue::new(&mut brush.scale.x).speed(0.05).range(0.01..=100.0).prefix("X:"));
                            let sy = ui.add(egui::DragValue::new(&mut brush.scale.y).speed(0.05).range(0.01..=100.0).prefix("Y:"));
                            let sz = ui.add(egui::DragValue::new(&mut brush.scale.z).speed(0.05).range(0.01..=100.0).prefix("Z:"));

                            // WRK-01: Atomic Undo registration for scale
                            if sx.drag_started() || sy.drag_started() || sz.drag_started() || sx.gained_focus() || sy.gained_focus() || sz.gained_focus() {
                                start_brush_scale = Some(scale_before);
                            }
                            if sx.drag_stopped() || sy.drag_stopped() || sz.drag_stopped() || sx.lost_focus() || sy.lost_focus() || sz.lost_focus() {
                                commit_brush_scale = Some((scale_before, brush.scale, brush.polys.clone()));
                            }
                        });

                        ui.horizontal_wrapped(|ui| {
                            if ui.button("🎯 Focus").clicked() {
                                focus_brush = true;
                            }
                            if ui.button("📋 Duplicate").on_hover_text("Duplicate (Ctrl+D)").clicked() {
                                duplicate_req = true;
                            }
                            if ui.button("⬇ Snap Floor").on_hover_text("Snap to Floor (End)").clicked() {
                                snap_floor_req = true;
                            }
                            if ui.button("🗑 Delete").clicked() {
                                delete_brush = true;
                            }
                            if ui.button("🔄 Invert").on_hover_text("Invert Normals & Winding (Ctrl+Alt+I)").clicked() {
                                invert_brush_req = true;
                            }
                            if ui.button("⚙ Bake").on_hover_text("Bake Rotation & Scale Permanently into Geometry").clicked() {
                                bake_brush_req = true;
                            }
                            if ui.button("↺ Reset Rot").on_hover_text("Reset Rotation to 0°").clicked() {
                                reset_rot_req = true;
                            }
                            if ui.button("↔ Reset Scale").on_hover_text("Reset Scale to 1.0").clicked() {
                                reset_scale_req = true;
                            }
                        });
                    });

                    // UI-03: Accordion Section 2 - Solidity & CSG Order
                    egui::CollapsingHeader::new("🧊 Solidity & CSG Order").default_open(true).show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Type:");
                            if ui.selectable_label(cur_solidity == crate::editor_state::BrushSolidity::Solid, "Solid").clicked() {
                                set_solidity_req = Some(crate::editor_state::BrushSolidity::Solid);
                            }
                            if ui.selectable_label(cur_solidity == crate::editor_state::BrushSolidity::SemiSolid, "🌿 Semi-Solid").on_hover_text("Semi-solid brushes do not split BSP tree nodes").clicked() {
                                set_solidity_req = Some(crate::editor_state::BrushSolidity::SemiSolid);
                            }
                            if ui.selectable_label(cur_solidity == crate::editor_state::BrushSolidity::NonSolid, "⚡ Non-Solid").on_hover_text("Non-solid brushes have zero collision and do not affect BSP").clicked() {
                                set_solidity_req = Some(crate::editor_state::BrushSolidity::NonSolid);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Order:");
                            if ui.button("⏮ First").on_hover_text("Send to First (evaluated earliest)").clicked() {
                                order_first = true;
                            }
                            if ui.button("▲ Up").clicked() {
                                order_up = true;
                            }
                            if ui.button("▼ Down").clicked() {
                                order_down = true;
                            }
                            if ui.button("⏭ Last").on_hover_text("Send to Last (evaluated latest)").clicked() {
                                order_last = true;
                            }
                        });
                    });

                    // UI-03: Accordion Section 3 - Surface Material & UVs
                    egui::CollapsingHeader::new("🎨 Material & UV Alignment").default_open(true).show(ui, |ui| {
                        if ui.button("🎨 Apply Active Texture").clicked() {
                            apply_active_tex = true;
                        }

                        ui.horizontal(|ui| {
                            ui.label("Pan U:");
                            if ui.button("-64").clicked() { pan_u -= 64.0; }
                            if ui.button("-16").clicked() { pan_u -= 16.0; }
                            if ui.button("+16").clicked() { pan_u += 16.0; }
                            if ui.button("+64").clicked() { pan_u += 64.0; }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Pan V:");
                            if ui.button("-64").clicked() { pan_v -= 64.0; }
                            if ui.button("-16").clicked() { pan_v -= 16.0; }
                            if ui.button("+16").clicked() { pan_v += 16.0; }
                            if ui.button("+64").clicked() { pan_v += 64.0; }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Scale:");
                            if ui.button("0.5x").clicked() { scale_factor = Some(0.5); }
                            if ui.button("2.0x").clicked() { scale_factor = Some(2.0); }
                        });

                        ui.horizontal(|ui| {
                            ui.label("Align:");
                            if ui.button("Floor").clicked() { align_mode = Some("floor"); }
                            if ui.button("Wall X").clicked() { align_mode = Some("wall_x"); }
                            if ui.button("Wall Y").clicked() { align_mode = Some("wall_y"); }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Align 3D:");
                            if ui.button("Planar").clicked() { align_mode = Some("planar"); }
                            if ui.button("Box").clicked() { align_mode = Some("box"); }
                            if ui.button("Fit 1x1").clicked() { align_mode = Some("fit"); }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Cylinder:");
                            if ui.button("1x").on_hover_text("Cylindrical wrap 1 repeat").clicked() {
                                cylinder_repeats = Some((1.0, 1.0));
                            }
                            if ui.button("2x").on_hover_text("Cylindrical wrap 2 repeats").clicked() {
                                cylinder_repeats = Some((2.0, 1.0));
                            }
                            if ui.button("4x").on_hover_text("Cylindrical wrap 4 repeats").clicked() {
                                cylinder_repeats = Some((4.0, 1.0));
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Rotate:");
                            if ui.button("-45°").clicked() { rotate_angle = Some(-45.0); }
                            if ui.button("+45°").clicked() { rotate_angle = Some(45.0); }
                            if ui.button("90°").clicked() { rotate_angle = Some(90.0); }
                        });
                    });
                }

                // WRK-01: Apply deferred brush transform and state mutations outside borrow scope
                if let Some(loc) = start_brush_move {
                    if state.numeric_edit_brush_loc_initial.is_none() || state.numeric_edit_brush_loc_initial.map(|(idx, _)| idx != sel_idx).unwrap_or(false) {
                        state.numeric_edit_brush_loc_initial = Some((sel_idx, loc));
                    }
                }
                if let Some((_, new_loc)) = commit_brush_move {
                    if let Some((b_idx, init_loc)) = state.numeric_edit_brush_loc_initial.take() {
                        if b_idx == sel_idx && init_loc != new_loc {
                            state.history.push(crate::history::Action::MoveBrush {
                                brush_index: sel_idx,
                                old_location: init_loc,
                                new_location: new_loc,
                            });
                            state.log(format!("Inspector: Moved brush #{} to ({:.1}, {:.1}, {:.1})", sel_idx, new_loc.x, new_loc.y, new_loc.z));
                        }
                    }
                }
                if let Some(rot) = start_brush_rot {
                    if state.numeric_edit_brush_rot_initial.is_none() || state.numeric_edit_brush_rot_initial.map(|(idx, _)| idx != sel_idx).unwrap_or(false) {
                        state.numeric_edit_brush_rot_initial = Some((sel_idx, rot));
                    }
                }
                if let Some((_, new_rot)) = commit_brush_rot {
                    if let Some((b_idx, init_rot)) = state.numeric_edit_brush_rot_initial.take() {
                        if b_idx == sel_idx && init_rot != new_rot {
                            state.history.push(crate::history::Action::RotateBrush {
                                brush_index: sel_idx,
                                old_rotation: init_rot,
                                new_rotation: new_rot,
                            });
                            state.log(format!("Inspector: Rotated brush #{} to ({:.1}°, {:.1}°, {:.1}°)", sel_idx, new_rot.x, new_rot.y, new_rot.z));
                        }
                    }
                }
                if let Some(scale) = start_brush_scale {
                    if state.numeric_edit_brush_scale_initial.is_none() || state.numeric_edit_brush_scale_initial.map(|(idx, _)| idx != sel_idx).unwrap_or(false) {
                        state.numeric_edit_brush_scale_initial = Some((sel_idx, scale));
                    }
                }
                if let Some((_, new_scale, polys)) = commit_brush_scale {
                    if let Some((b_idx, init_scale)) = state.numeric_edit_brush_scale_initial.take() {
                        if b_idx == sel_idx && init_scale != new_scale {
                            state.history.push(crate::history::Action::TransformBrush {
                                brush_index: sel_idx,
                                old_rotation: if let Some(b) = state.world_brushes.get(sel_idx) { b.rotation } else { Vec3::ZERO },
                                new_rotation: if let Some(b) = state.world_brushes.get(sel_idx) { b.rotation } else { Vec3::ZERO },
                                old_scale: init_scale,
                                new_scale,
                                old_polys: polys.clone(),
                                new_polys: polys,
                                description: format!("Scale Brush to ({:.2}, {:.2}, {:.2})", new_scale.x, new_scale.y, new_scale.z),
                            });
                            state.log(format!("Inspector: Scaled brush #{} to ({:.2}, {:.2}, {:.2})", sel_idx, new_scale.x, new_scale.y, new_scale.z));
                        }
                    }
                }
                if invert_brush_req {
                    state.invert_selected_brush();
                }
                if bake_brush_req {
                    state.transform_permanently_selected();
                }
                if reset_rot_req {
                    state.reset_rotation_selected();
                }
                if reset_scale_req {
                    state.reset_scaling_selected();
                }
                if let Some(sol) = set_solidity_req {
                    state.set_selected_brush_solidity(sol);
                }

                if order_first {
                    state.send_selected_brush_to_first();
                } else if order_last {
                    state.send_selected_brush_to_last();
                } else if order_up {
                    state.move_selected_brush_up();
                } else if order_down {
                    state.move_selected_brush_down();
                }

                if apply_active_tex {
                    state.apply_active_texture_to_selected_brush();
                }
                if pan_u != 0.0 || pan_v != 0.0 {
                    state.pan_selected_brush_texture(pan_u, pan_v);
                }
                if let Some(scale) = scale_factor {
                    state.scale_selected_brush_texture(scale, scale);
                }
                if let Some(mode) = align_mode {
                    state.align_selected_brush_texture(mode);
                }
                if let Some((ru, rv)) = cylinder_repeats {
                    state.align_selected_brush_cylinder(ru, rv);
                }
                if let Some(angle) = rotate_angle {
                    state.rotate_selected_brush_texture(angle);
                }

                if focus_brush {
                    state.focus_camera_on_selection();
                }
                if delete_brush {
                    if sel_idx < state.world_brushes.len() {
                        let b = state.world_brushes.remove(sel_idx);
                        state.history.push(crate::history::Action::DeleteBrush {
                            index: sel_idx,
                            brush: b,
                        });
                        state.selected_brush_index = None;
                        state.map_is_dirty = true;
                    }
                }
            } else if let Some(sm_idx) = state.selected_static_mesh_index {
                let mut focus_mesh = false;
                let mut delete_mesh = false;
                let mut make_hull = false;

                if let Some(mesh) = state.static_meshes.get_mut(sm_idx) {
                    let old_loc = mesh.location;
                    let old_scale = mesh.scale;
                    ui.label(format!("Static Mesh: {}", mesh.name));
                    ui.label(format!("Polygons: {}", mesh.polys.len()));

                    let (min_b, max_b) = mesh.world_bounds();
                    let width = (max_b.x - min_b.x).abs();
                    let depth = (max_b.y - min_b.y).abs();
                    let height = (max_b.z - min_b.z).abs();

                    ui.separator();
                    ui.label(egui::RichText::new("📐 Extents & Spatial Metrics").strong());
                    ui.label(format!("Size: {:.1} × {:.1} × {:.1} uu ({:.2}m × {:.2}m × {:.2}m)", width, depth, height, width * 0.0213, depth * 0.0213, height * 0.0213));

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Pos X:");
                        ui.add(egui::DragValue::new(&mut mesh.location.x).speed(16.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Y:");
                        ui.add(egui::DragValue::new(&mut mesh.location.y).speed(16.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Z:");
                        ui.add(egui::DragValue::new(&mut mesh.location.z).speed(16.0));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.add(egui::DragValue::new(&mut mesh.scale.x).speed(0.05).range(0.01..=100.0).prefix("X:"));
                        ui.add(egui::DragValue::new(&mut mesh.scale.y).speed(0.05).range(0.01..=100.0).prefix("Y:"));
                        ui.add(egui::DragValue::new(&mut mesh.scale.z).speed(0.05).range(0.01..=100.0).prefix("Z:"));
                    });

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("🎯 Focus").clicked() {
                            focus_mesh = true;
                        }
                        if ui.button("📋 Duplicate").on_hover_text("Duplicate (Ctrl+D)").clicked() {
                            duplicate_req = true;
                        }
                        if ui.button("⬇ Snap Floor").on_hover_text("Snap to Floor (End)").clicked() {
                            snap_floor_req = true;
                        }
                        if ui.button("🛡 Gen Hull").on_hover_text("Generates an invisible blocking bounding hull (PF_INVISIBLE | PF_SEMISOLID)").clicked() {
                            make_hull = true;
                        }
                        if ui.button("🗑 Delete").clicked() {
                            delete_mesh = true;
                        }
                    });

                    let new_loc = mesh.location;
                    let new_scale = mesh.scale;
                    if old_loc != new_loc {
                        let interacting = ui.ctx().input(|i| i.pointer.any_down()) || ui.ctx().memory(|m| m.focused().is_some());
                        state.history.push_interactive(
                            crate::history::Action::MoveStaticMesh {
                                mesh_index: sm_idx,
                                old_location: old_loc,
                                new_location: new_loc,
                            },
                            interacting,
                        );
                        state.map_is_dirty = true;
                    } else if old_scale != new_scale {
                        state.map_is_dirty = true;
                    }
                }

                if make_hull {
                    state.generate_collision_hull_for_selected_static_mesh();
                }
                if focus_mesh {
                    state.focus_camera_on_selection();
                }
                if delete_mesh {
                    if sm_idx < state.static_meshes.len() {
                        let m = state.static_meshes.remove(sm_idx);
                        state.history.push(crate::history::Action::DeleteStaticMesh {
                            index: sm_idx,
                            mesh: m,
                        });
                        state.selected_static_mesh_index = None;
                        state.map_is_dirty = true;
                    }
                }
            } else if let Some(sel_idx) = state.selected_actor_index {
                // Inspector for selected Actor (UI-03 & WRK-01)
                let mut focus_actor = false;
                let mut delete_actor = false;
                let mut show_actor_props = false;
                let mut snap_aligned = false;
                let mut start_actor_move = None;
                let mut commit_actor_move = None;
                let mut pending_actor_prop_edit = None;
                if let Some(ref mut level) = state.loaded_unr {
                    let mut need_sync_views = false;
                    if let Some(actor) = level.actors.get_mut(sel_idx) {
                        ui.label(format!("Name: {}", actor.name));
                        ui.label(format!("Class: {}", actor.class_name));

                        // UI-03: Accordion Section 1 - Transform & Placement
                        egui::CollapsingHeader::new("📍 Transform & Placement").default_open(true).show(ui, |ui| {
                            let actor_loc_before = actor.location;

                            ui.horizontal(|ui| {
                                ui.label("Pos X:");
                                let rx = ui.add(egui::DragValue::new(&mut actor.location.x).speed(8.0));
                                ui.label("Pos Y:");
                                let ry = ui.add(egui::DragValue::new(&mut actor.location.y).speed(8.0));
                                ui.label("Pos Z:");
                                let rz = ui.add(egui::DragValue::new(&mut actor.location.z).speed(8.0));

                                if rx.changed() || ry.changed() || rz.changed() {
                                    need_sync_views = true;
                                }

                                // WRK-01: Atomic Undo registration on drag_stopped or lost_focus
                                if rx.drag_started() || ry.drag_started() || rz.drag_started() || rx.gained_focus() || ry.gained_focus() || rz.gained_focus() {
                                    start_actor_move = Some(actor_loc_before);
                                }
                                if rx.drag_stopped() || ry.drag_stopped() || rz.drag_stopped() || rx.lost_focus() || ry.lost_focus() || rz.lost_focus() {
                                    commit_actor_move = Some((actor_loc_before, actor.location, actor.name.clone()));
                                }
                            });

                            ui.horizontal_wrapped(|ui| {
                                if ui.button("⚙ Props").on_hover_text("Open Actor Properties window (Enter)").clicked() {
                                    show_actor_props = true;
                                }
                                if ui.button("🎯 Focus").clicked() {
                                    focus_actor = true;
                                }
                                if ui.button("📋 Duplicate").on_hover_text("Duplicate (Ctrl+D)").clicked() {
                                    duplicate_req = true;
                                }
                                if ui.button("⬇ Snap Floor").on_hover_text("Snap to Floor (End)").clicked() {
                                    snap_floor_req = true;
                                }
                                if ui.button("📐 Align Floor").on_hover_text("Snap & Align to Floor (Shift+End)").clicked() {
                                    snap_aligned = true;
                                }
                                if ui.button("🗑 Delete").clicked() {
                                    delete_actor = true;
                                }
                            });
                        });

                        let old_actor_props = actor.clone();

                        // UI-03: Accordion Section 2 - Light Settings (if actor is light)
                        if actor.class_name.contains("Light") {
                            egui::CollapsingHeader::new("☼ Light Settings").default_open(true).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Brightness:");
                                    ui.add(egui::Slider::new(&mut actor.light_brightness, 0..=255));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Radius:");
                                    ui.add(egui::Slider::new(&mut actor.light_radius, 0..=255));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Hue:");
                                    ui.add(egui::Slider::new(&mut actor.light_hue, 0..=255));
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Saturation:");
                                    ui.add(egui::Slider::new(&mut actor.light_saturation, 0..=255));
                                });

                                // Live Color Swatch
                                let preview_color = ut99_light_to_color(actor.light_hue, actor.light_saturation, actor.light_brightness);
                                ui.horizontal(|ui| {
                                    ui.label("Color Preview:");
                                    let (rect, _) = ui.allocate_exact_size(egui::Vec2::new(36.0, 18.0), egui::Sense::hover());
                                    ui.painter().rect_filled(rect, 3.0, preview_color);
                                });
                            });
                        }

                        // UI-03: Accordion Section 3 - Zone Properties (if zone actor)
                        if actor.class_name.contains("ZoneInfo") || actor.class_name.contains("WaterZone") || actor.class_name.contains("SkyZoneInfo") {
                            egui::CollapsingHeader::new("🌐 Zone Properties").default_open(true).show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Zone Tag:");
                                    ui.text_edit_singleline(&mut actor.group);
                                });
                                if actor.class_name.contains("SkyZoneInfo") {
                                    ui.label(egui::RichText::new("SkyBox Anchor: Surfaces with PF_FakeBackdrop project this sky view.").size(11.0).italics());
                                } else if actor.class_name.contains("WaterZone") {
                                    ui.label(egui::RichText::new("Fluid Physics: bWaterZone=true, swim physics & underwater audio.").size(11.0).italics());
                                }
                            });
                        }

                        // UI-03: Accordion Section 4 - Event & Trigger Wiring
                        egui::CollapsingHeader::new("⚡ Event & Trigger Wiring").default_open(true).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Tag:");
                                ui.text_edit_singleline(&mut actor.tag);
                            });
                            ui.horizontal(|ui| {
                                ui.label("Event:");
                                ui.text_edit_singleline(&mut actor.event);
                            });
                        });

                        let props_changed = old_actor_props.light_brightness != actor.light_brightness
                            || old_actor_props.light_radius != actor.light_radius
                            || old_actor_props.light_hue != actor.light_hue
                            || old_actor_props.light_saturation != actor.light_saturation
                            || old_actor_props.group != actor.group
                            || old_actor_props.tag != actor.tag
                            || old_actor_props.event != actor.event;
                        if props_changed {
                            need_sync_views = true;
                            pending_actor_prop_edit = Some((old_actor_props, actor.clone()));
                        }
                    }
                    if need_sync_views {
                        level.sync_actor_views();
                    }
                }
                if let Some((old_actor, new_actor)) = pending_actor_prop_edit {
                    let interacting = ui.ctx().input(|i| i.pointer.any_down()) || ui.ctx().memory(|m| m.focused().is_some());
                    let desc = format!("Edit Actor Inspector ({})", new_actor.name);
                    state.history.push_interactive(
                        crate::history::Action::UpdateLightEffects {
                            actor_index: sel_idx,
                            old_actor,
                            new_actor,
                            description: desc,
                        },
                        interacting,
                    );
                    state.map_is_dirty = true;
                }
                if let Some(start_loc) = start_actor_move {
                    if state.numeric_edit_actor_loc_initial.is_none() || state.numeric_edit_actor_loc_initial.map(|(idx, _)| idx != sel_idx).unwrap_or(false) {
                        state.numeric_edit_actor_loc_initial = Some((sel_idx, start_loc));
                    }
                }
                if let Some((_, new_loc, name)) = commit_actor_move {
                    if let Some((a_idx, init_loc)) = state.numeric_edit_actor_loc_initial.take() {
                        if a_idx == sel_idx && init_loc != new_loc {
                            state.history.push(crate::history::Action::MoveActor {
                                actor_index: sel_idx,
                                old_location: init_loc,
                                new_location: new_loc,
                            });
                            state.map_is_dirty = true;
                            state.log(format!("Inspector: Moved actor #{} ({}) to ({:.1}, {:.1}, {:.1})", sel_idx, name, new_loc.x, new_loc.y, new_loc.z));
                        }
                    }
                }
                if show_actor_props {
                    state.show_actor_properties = true;
                }
                if snap_aligned {
                    state.snap_selected_to_floor_aligned();
                }
                if focus_actor {
                    state.focus_camera_on_selection();
                }
                if delete_actor {
                    state.delete_actor_at(sel_idx);
                }
            } else {
                // UI-03: Accordion for Active Builder Brush
                egui::CollapsingHeader::new("🔴 Active Builder Brush").default_open(true).show(ui, |ui| {
                    ui.label(format!("Polygons: {}", state.builder_brush.polys.len()));

                    let (min_b, max_b) = state.builder_brush.world_bounds();
                    let width = (max_b.x - min_b.x).abs();
                    let depth = (max_b.y - min_b.y).abs();
                    let height = (max_b.z - min_b.z).abs();

                    ui.label(egui::RichText::new("📐 Extents & Spatial Metrics").strong());
                    ui.label(format!("Size: {:.1} × {:.1} × {:.1} uu ({:.2}m × {:.2}m × {:.2}m)", width, depth, height, width * 0.0213, depth * 0.0213, height * 0.0213));

                    ui.separator();
                    let old_builder_brush = state.builder_brush.clone();
                    let mut builder_pos_changed = false;
                    ui.horizontal(|ui| {
                        ui.label("Pos X:");
                        builder_pos_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.x).speed(16.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Y:");
                        builder_pos_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.y).speed(16.0)).changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Pos Z:");
                        builder_pos_changed |= ui.add(egui::DragValue::new(&mut state.builder_brush.location.z).speed(16.0)).changed();
                    });
                    if builder_pos_changed {
                        let interacting = ui.ctx().input(|i| i.pointer.any_down())
                            || ui.ctx().memory(|m| m.focused().is_some());
                        state.history.push_interactive(
                            crate::history::Action::ModifyBuilderBrush {
                                old_brush: old_builder_brush,
                                new_brush: state.builder_brush.clone(),
                                description: "Move Builder Brush".to_string(),
                            },
                            interacting,
                        );
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("🎯 Focus View").clicked() {
                            state.focus_camera_on_selection();
                        }
                        if ui.button("🔄 Reset to (0,0,0)").clicked() {
                            state.reset_builder_brush();
                        }
                    });

                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Select a brush, mover, mesh, or actor in the outliner to inspect its specific properties.").italics().size(11.0));
                });
            }

            if duplicate_req {
                state.duplicate_selected();
            }
            if snap_floor_req {
                state.snap_selected_to_floor();
            }
            });
        } else {
            ui.add_space(8.0);
            ui.separator();
            ui.label(egui::RichText::new("💡 Outliner Mode: Selection Properties hidden. Switch to '◫ Split' or '⚙ Inspector' to edit object parameters.").italics().size(11.0).color(egui::Color32::from_rgb(140, 160, 185)));
        }
        });

    // 3b. Sleek Bottom Status Bar & Breadcrumbs (INT-03 / PRO Standard)
    egui::TopBottomPanel::bottom("bottom_status_bar")
        .exact_height(24.0)
        .show(ctx, |ui| {
            render_status_bar_content(ui, state);
        });

    // 4. Bottom Console Panel (Collapsible Drawer - UI-02)
    if state.show_bottom_console {
        egui::TopBottomPanel::bottom("bottom_console")
            .default_height(96.0)
            .min_height(60.0)
            .max_height(350.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Editor Console").strong());
                    if ui.button("Clear").clicked() {
                        state.console_messages.clear();
                    }
                    if let Some((progress, ref msg)) = state.build_progress.clone() {
                        ui.separator();
                        ui.add(egui::ProgressBar::new(progress).desired_width(180.0).show_percentage());
                        ui.label(egui::RichText::new(msg).color(egui::Color32::from_rgb(100, 220, 120)).strong());
                        if ui.button("✖").on_hover_text("Dismiss progress notification").clicked() {
                            state.build_progress = None;
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("🔽 Close (Ctrl+`)").on_hover_text("Hide console drawer").clicked() {
                            state.show_bottom_console = false;
                        }
                    });
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height((ui.available_height() - 28.0).max(24.0))
                    .stick_to_bottom(true)
                    .show(ui, |ui| {
                        for msg in &state.console_messages {
                            ui.label(egui::RichText::new(msg).monospace().size(11.0));
                        }
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Cmd >").monospace().strong().color(egui::Color32::from_rgb(245, 182, 45)));
                    let resp = ui.add(
                        egui::TextEdit::singleline(&mut state.console_command_input)
                            .hint_text("UnrealEd 2.0 command (e.g., POLY SELECT ALL, BRUSH ADD, LIGHT APPLY, PATHS DEFINE, RMODE 6)...")
                            .desired_width((ui.available_width() - 64.0).max(120.0)),
                    );
                    let enter_pressed = resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    if enter_pressed || ui.button("Run").clicked() {
                        let cmd = state.console_command_input.clone();
                        state.console_command_input.clear();
                        let _ = state.execute_unrealed_command(&cmd);
                    }
                });
            });
    }

    // 5. Floating Texture Browser Window
    texture_browser::render_texture_browser(ctx, state);

    // 6. 2D Shape Editor Window
    shape_editor::render_shape_editor(ctx, state);

    // 7. Add Special Brush Dialog Window
    special_brush_dialog::render_special_brush_dialog(ctx, state);

    // 8. Build Lighting (Raytracing) Dialog Window
    lighting_dialog::render_lighting_dialog(ctx, state);

    // 8b. Build Options & Map Rebuild Window
    build_options_dialog::render_build_options_dialog(ctx, state);

    // 8c. Scale Brush Dialog Window
    brush_scale_dialog::render_brush_scale_dialog(ctx, state);

    // 8d. Shear Brush Dialog Window
    brush_shear_dialog::render_brush_shear_dialog(ctx, state);

    // 9. Actor Class Browser Window
    actor_browser::render_actor_browser(ctx, state);

    // 9b. Groups Browser Window
    group_browser::render_group_browser(ctx, state);

    // 9c. Find Actors Dialog Window (Ctrl+F)
    find_actors_dialog::render_find_actors_dialog(ctx, state);

    // 9c2. Replace Textures Dialog Window (Ctrl+R)
    replace_textures_dialog::render_replace_textures_dialog(ctx, state);

    // 9d. Align & Distribute Actors Dialog Window
    actor_align_dialog::render_actor_align_dialog(ctx, state);

    // 9e. Map Statistics Dialog Window
    map_statistics_dialog::render_map_statistics_dialog(ctx, state);

    // 10. Sound Browser (.uax) Window
    sound_browser::render_sound_browser(ctx, state);

    // 11. Music Browser (.umx) Window
    music_browser::render_music_browser(ctx, state);

    // 12. Bot AI Navigation Simulator Window
    bot_simulator::render_bot_simulator_dialog(ctx, state);

    // 13. Level Doctor & Diagnostics Window
    level_doctor::render_level_doctor_dialog(ctx, state);

    // 14. Surface Properties Window (F5)
    surface_properties::render_surface_properties_dialog(ctx, state);

    // 14b. Actor Properties Window (Enter)
    actor_properties::render_actor_properties_dialog(ctx, state);

    // 15. Prefab & Architectural Template Library Window
    prefab_browser::render_prefab_browser(ctx, state);

    // 16. Map Performance & Engine Budget Profiler Window (F7)
    budget_inspector::render_budget_inspector(ctx, state);

    // 17. Camera Bookmarks Window
    bookmarks_dialog::render_bookmarks_dialog(ctx, state);

    // 18. Auto-Save & Recovery Window
    autosave_dialog::render_autosave_dialog(ctx, state);

    // 19. UT99 Playtest & Launch Settings Window
    play_settings_dialog::render_play_settings_dialog(ctx, state);

    // Centralized Settings & Preferences Window
    settings_dialog::render_settings_dialog(ctx, state);

    // 20. Zone Acoustics, EAX Reverb & Distance Fog Window
    acoustics_dialog::render_acoustics_dialog(ctx, state);

    // 21. Map Packaging & Release Manifest Window
    map_packaging_dialog::render_map_packaging_dialog(ctx, state);

    // Save & Export Visual Feedback Modal Overlay
    save_feedback_dialog::render_save_feedback_overlay(ctx, state);

    // 22. Tournament Item Balance & Timing Matrix Window
    gameplay_balance_dialog::render_gameplay_balance_dialog(ctx, state);

    // 23. Zone Boundary Leak Detector Dialog
    zone_seal_dialog::show_zone_seal_dialog(ctx, state);

    // 24. Dispatcher Trigger Timeline Dialog
    dispatcher_dialog::show_dispatcher_dialog(ctx, state);

    // 25. Symmetrical CTF Base Generator Dialog
    ctf_symmetry_dialog::show_ctf_symmetry_dialog(ctx, state);

    // 26. Level Properties Window (UnrealEd 2.0 F6 Parity)
    level_properties_dialog::render_level_properties_dialog(ctx, state);

    // 27. Scale Entire Map Window (UnrealEd 2.0 Tools Parity)
    scale_map_dialog::render_scale_map_dialog(ctx, state);

    // 28. Mesh & Static Mesh Browser Window (UnrealEd 2.0 / 2.1 Parity)
    mesh_browser::render_mesh_browser(ctx, state);

    // 29. FireEngine Procedural Texture Studio Window (UnrealEd 2.0 Parity)
    procedural_texture_studio::render_procedural_texture_studio(ctx, state);

    // 30. Package Dependencies & Distribution Manifest Window
    package_dependency_dialog::render_package_dependency_dialog(ctx, state);

    // 31. Decal Painting & Projector Studio Window
    decal_studio_dialog::render_decal_studio_dialog(ctx, state);

    // 32. UnrealScript Class Source & DefaultProperties Studio
    uc_viewer_dialog::render_uc_viewer_dialog(ctx, state);

    // 33. SkyBox 3D Parallax & Multi-Zone Environmental Studio
    skybox_studio_dialog::render_skybox_studio_dialog(ctx, state);

    // 34. 3D Acoustic & Ambient Sound Studio
    acoustic_studio_dialog::render_acoustic_studio_dialog(ctx, state);

    // 35. WarpZone & Non-Euclidean Portal Studio
    warp_zone_dialog::render_warp_zone_dialog(ctx, state);

    // 36. Radiosity & Direct Light Baking Studio
    lighting_studio_dialog::render_lighting_studio_dialog(ctx, state);

    // 37. Zone Fog & Volumetric Attenuation Studio
    zone_fog_dialog::render_zone_fog_dialog(ctx, state);

    // 38. Skeletal Rigging, Bone Hierarchy & Character Animation Studio
    skeletal_studio_dialog::show_skeletal_studio_dialog(ctx, state);

    // UT99 Skin & Character Authoring Studio
    skin_studio_dialog::show_skin_studio_dialog(ctx, state);

    // Package Asset Extractor & .uz Decompressor Studio
    package_extractor_dialog::show_package_extractor_dialog(ctx, state);

    // UnrealScript Class Wizard & #exec Studio
    uc_class_wizard_dialog::show_uc_class_wizard_dialog(ctx, state);

    // 39. Dynamic Particle FX & Volumetric Emitter Studio
    particle_studio_dialog::render_particle_studio_dialog(ctx, state);

    // 40. Bot AI Navigation Graph & Tactical Cover Studio
    bot_nav_studio_dialog::render_bot_nav_studio_dialog(ctx, state);

    // 41. Procedural Architectural Prefab & Brush Macro Studio
    architectural_prefab_dialog::render_architectural_studio_dialog(ctx, state);

    // 42. Terrain & Heightmap Mesh Sculptor Studio
    terrain_studio_dialog::render_terrain_studio_dialog(ctx, state);

    // 43. Static Mesh LOD & Convex Collision Studio
    mesh_lod_collision_dialog::render_lod_collision_dialog(ctx, state);

    // 44. Cinematic Matinee Camera Director Studio
    cinematic_director_dialog::render_cinematic_director_dialog(ctx, state);

    // 45. Dynamic Light FX & Animating Strobe/Spotlight Studio
    light_effects_studio_dialog::render_light_effects_studio_dialog(ctx, state);

    // 46. Zone Portals, Antiportals & PVS Occlusion Studio
    zone_occlusion_studio_dialog::render_zone_occlusion_studio_dialog(ctx, state);

    // 47. Music Event & UMX Track Sequencing Studio
    music_studio_dialog::render_music_studio_dialog(ctx, state);

    // 48. Sun, Corona & Lens Flare Studio
    corona_studio_dialog::render_corona_studio_dialog(ctx, state);

    // 49. Fluid Physics, Hazard Volumes & Environmental Dynamics Studio
    fluid_studio_dialog::render_fluid_studio_dialog(ctx, state);

    // 50. Weapon Ballistics, Ricochet & Combat Sightline Studio
    ballistics_studio_dialog::render_ballistics_studio_dialog(ctx, state);

    // 51. Teleporter Destination & Network Link Studio
    teleporter_studio_dialog::render_teleporter_studio_dialog(ctx, state);

    // 52. Translocator Trajectory & Teleport Beacon Studio
    translocator_studio_dialog::render_translocator_studio_dialog(ctx, state);

    // 53. LiftCenter, LiftExit & Dynamic Elevator Pathing Studio
    lift_pathing_dialog::render_lift_pathing_dialog(ctx, state);

    // 54. Dynamic Zone Water Surface & Wave Reflection Studio
    water_studio_dialog::render_water_studio_dialog(ctx, state);

    // 55. Mover Encroachment, Door Sequencing & Crushing Trap Studio
    mover_studio_dialog::render_mover_studio_dialog(ctx, state);

    // 56. Team Base Objective, CTF Flag Run Route & Defense AI Studio
    team_objective_dialog::show(ctx, state);

    // 57. Destructible Architecture, Breakable Glass & Fracture Studio
    destructible_studio_dialog::show(ctx, state);

    // 58. Jump Pad, Aerial Kicker Trajectory & Low-Gravity Studio
    kicker_studio_dialog::show(ctx, state);

    // 59. BSP Tree Quality, Node Cut Minimizer & Sliver Polygon Purging Studio
    bsp_quality_studio_dialog::show(ctx, state);

    // 60. 2D Shape Loft, Curved Pipe & Architectural Molding Studio
    shape_loft_studio_dialog::show(ctx, state);

    // 61. Curved Architectural Stairway, Spiral Ramp & Smooth Collision Hull Studio
    stairway_studio_dialog::show(ctx, state);

    // 62. Surface Clutter, Procedural Foliage & Decoration Scatter Studio
    mesh_scatter_studio_dialog::show(ctx, state);

    // 63. Volumetric Light Shaft, Window God Ray & Atmospheric Beam Studio
    light_shaft_studio_dialog::show(ctx, state);

    // 64. Spline Rail, Kinetic Track Mover & Rollercoaster Studio
    track_mover_studio_dialog::show(ctx, state);

    // 65. Security CCTV Camera, Live Video Monitor & Surveillance Studio
    surveillance_studio_dialog::show(ctx, state);

    // 66. Security Laser Grid, Tripwire Alarm & Energy Barrier Studio
    laser_grid_studio_dialog::show(ctx, state);

    // 67. Environmental Ventilation Fan, Air Current & Wind Tunnel Studio
    ventilator_studio_dialog::show(ctx, state);

    // 68. Industrial Conveyor Belt, Treadmill & Surface Velocity Studio
    conveyor_studio_dialog::show(ctx, state);

    // 69. Automated Defense Turret, Sentry Cannon & Emplacement Studio
    turret_studio_dialog::show(ctx, state);

    // 70. Ambient Soundscape, Spatial Audio Attenuation & Acoustic Reverb Studio
    soundscape_studio_dialog::show(ctx, state);

    // 71. Environmental Hazard Chamber, Pressure Airlock & Gas Decompression Studio
    hazard_chamber_studio_dialog::show(ctx, state);

    // 72. Industrial Pipe Network, Steam Valve & Conduit Junction Studio
    pipe_conduit_studio_dialog::show(ctx, state);

    // 73. Holographic Signage, Sci-Fi Billboard & Volumetric Display Studio
    hologram_studio_dialog::show(ctx, state);

    // 74. Dynamic Weather, Atmospheric Precipitation & Lightning Storm Studio
    weather_studio_dialog::show(ctx, state);

    // 75. Interactive Arena Match Rules & Tournament Mutator Studio
    arena_rules_dialog::show(ctx, state);

    // 76. Interactive Armor, Health & ShieldBelt Stacking & Regeneration Studio
    armor_stacking_dialog::show(ctx, state);

    // 77. Interactive Bot AI Tactical JumpSpot & Translocator Route Studio
    bot_jump_routes_dialog::show(ctx, state);

    // 78. Interactive Tournament Weapon Placement & Ammo Circuit Studio
    weapon_circuit_dialog::show(ctx, state);

    // 79. Interactive Tournament Powerup Control & Relic Altar Studio
    powerup_control_dialog::show(ctx, state);

    // 80. Interactive Tournament Spawn Point Balancing Studio
    spawn_distribution_dialog::show(ctx, state);

    // 81. Interactive Domination Control Point Studio
    domination_dialog::show(ctx, state);

    // 82. Interactive Assault Objective Studio
    assault_dialog::show(ctx, state);

    // 83. Interactive Team Base Radar & Jammer Studio
    radar_dialog::show(ctx, state);

    // 84. Interactive Capture The Flag Base Studio
    ctf_studio_dialog::show(ctx, state);

    // 85. Interactive Team Cannon & Turret Defense Studio
    team_cannon_dialog::show(ctx, state);

    // 86. Interactive Bio-Rifle Slime Trap & Volumetric Hazard Field Studio
    bio_trap_dialog::show(ctx, state);

    // 87. Interactive Pressure Plate & Proximity Trigger Network Studio
    trigger_network_dialog::show(ctx, state);

    // 88. Interactive Elevator, LiftCenter/LiftExit & Vertical Transit Studio
    elevator_dialog::show(ctx, state);

    // 89. Interactive Dynamic Zone Velocity, Atmospheric Convection & Low-G Space Vacuum Studio
    zone_velocity_dialog::show(ctx, state);

    // 90. Interactive Explosive Barrel, Toxic Drum & Hazard Pyrotechnics Studio
    hazard_barrel_dialog::show(ctx, state);

    // 91. Interactive Special Lit, Zone Lighting Mask & Light Filter Studio
    special_lit_dialog::show(ctx, state);

    // 92. Interactive Sniper Vantage Point, Headshot Angle & Occlusion Nest Studio
    sniper_vantage_dialog::show(ctx, state);

    // 93. Interactive Secret Door, Concealed Passage & Keyhole Mover Studio
    secret_door_dialog::show(ctx, state);

    // 94. Interactive Teleporter Portal Arch, Dimensional Vortex & Gate Studio
    teleporter_arch_dialog::show(ctx, state);

    // 95. Interactive Toxic Slime Acid Basin, Lava Flow & Caustic Pool Studio
    fluid_basin_dialog::show(ctx, state);

    // 96. Interactive Alarm Siren Beacon, Emergency Strobe & Warning Klaxon Studio
    alarm_beacon_dialog::show(ctx, state);

    // 97. Interactive Liandri Supply Locker, Ammo Cabinet & Armory Rack Studio
    supply_locker_dialog::show(ctx, state);

    // 98. Interactive Glass Skylight, Observation Dome & Structural Truss Studio
    skylight_dialog::show(ctx, state);

    // 99. Interactive Ventilation Duct, Crawlspace Shaft & Infiltration Grate Studio
    vent_duct_dialog::show(ctx, state);

    // 100. Interactive Industrial Steam Piston, Hydraulic Compactor & Trap Hazard Studio
    piston_dialog::show(ctx, state);

    // 101. Interactive Jump Boots Cache, Anti-Gravity Booster & High-Vault Studio
    jump_boots_dialog::show(ctx, state);

    // 102. Interactive Force Field, Energy Barrier & Power Core Overload Studio
    force_field_dialog::show(ctx, state);

    // 103. Interactive Rotating Gyroscope, Reactor Core & Kinetic Ring Studio
    reactor_core_dialog::show(ctx, state);

    // 104. Creation Hub & Action Wizards (01)
    action_wizards::show(ctx, state);

    // 105. Curated Visual & Lighting Presets Catalog (02)
    visual_presets_dialog::show(ctx, state);

    // 20. Luxmeter Lighting Heatmap HUD Overlay
    if state.render_lux_heatmap {
        egui::Area::new(egui::Id::new("luxmeter_hud_legend"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-280.0, -140.0))
            .show(ctx, |ui| {
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_black_alpha(220))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(100, 200, 255)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("💡 LUXMETER:").strong().color(egui::Color32::WHITE));
                            ui.colored_label(egui::Color32::from_rgb(60, 60, 220), "■ <15 Under");
                            ui.colored_label(egui::Color32::from_rgb(0, 200, 220), "■ 15-50 Shadow");
                            ui.colored_label(egui::Color32::from_rgb(40, 220, 60), "■ 50-100 Ideal");
                            ui.colored_label(egui::Color32::from_rgb(240, 220, 20), "■ 100-180 Direct");
                            ui.colored_label(egui::Color32::from_rgb(240, 40, 30), "■ >180 Over");
                            if ui.button("✖").on_hover_text("Disable Luxmeter (F6)").clicked() {
                                state.render_lux_heatmap = false;
                            }
                        });
                    });
            });
    }

    // 21. Bot Navigation Coverage Heatmap HUD Overlay
    if state.show_nav_coverage {
        egui::Area::new(egui::Id::new("nav_coverage_hud_legend"))
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-280.0, -96.0))
            .show(ctx, |ui| {
                let report = state.compute_nav_coverage();
                egui::Frame::window(&ctx.style())
                    .fill(egui::Color32::from_black_alpha(220))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 220, 60)))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(format!("🤖 BOT NAV COVERAGE: {:.1}%", report.coverage_percentage)).strong().color(egui::Color32::WHITE));
                            ui.colored_label(egui::Color32::from_rgb(40, 220, 60), format!("● {} Optimal", report.optimal_count));
                            ui.colored_label(egui::Color32::from_rgb(240, 220, 20), format!("● {} Reachable", report.reachable_count));
                            ui.colored_label(egui::Color32::from_rgb(240, 40, 30), format!("● {} Dead Zones", report.dead_zone_count));
                            if ui.button("✖").on_hover_text("Disable Navigation Coverage (F9)").clicked() {
                                state.show_nav_coverage = false;
                            }
                        });
                    });
            });
    }

    // 22. Viewport Quadrant Badges (INT-05 Multi-Viewport UX)
    if state.viewport_layout == crate::camera::ViewportLayout::FourViewports {
        render_viewport_quadrant_badges(ctx, state);
    }

    // 23. Quick Command Palette (Ctrl+P / F1)
    command_palette::render_command_palette(ctx, state);

    // 24. Mini-Map Navigation Radar Overlay (M)
    minimap::render_minimap_overlay(ctx, state);

    // 25. Keyboard Shortcuts Cheatsheet Overlay (? / Shift+F1)
    help_dialog::render_help_dialog(ctx, state);

    // 26. Floating Non-Blocking Toast Notifications (WRK-03)
    render_toast_notifications(ctx, state);

    // 27. 3D Viewport Right-Click Context Menu (MOU-01)
    context_menu::render_viewport_context_menu(ctx, state);

    // 28. UnrealEd 2.0 Master Browser Unified Window & Smart Prefab Catalog (Cycle 6 / BG-16)
    render_master_browser_window(ctx, state);

    // 29. New Valid UT99 Map Generator Wizard & Official Map DNA Cloner (Ctrl+Shift+N)
    map_wizard_dialog::show_map_wizard_dialog(ctx, state);

    // 30. Unsaved Changes Exit Protection Modal (TASK-03)
    render_unsaved_exit_modal(ctx, state);

    // 31. First-Run Onboarding Welcome Wizard
    welcome_wizard::render_welcome_wizard(ctx, state);
}

fn render_unsaved_exit_modal(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_unsaved_exit_modal {
        return;
    }

    let action = state
        .pending_dirty_action
        .clone()
        .unwrap_or(crate::editor_state::PendingDirtyAction::ExitApp);
    let (prompt_desc, save_btn_lbl, discard_btn_lbl) = match action {
        crate::editor_state::PendingDirtyAction::ExitApp => (
            "Would you like to save your work before exiting UnrealEd?",
            "💾 Save & Exit",
            "🗑 Discard & Exit",
        ),
        crate::editor_state::PendingDirtyAction::NewMap => (
            "Would you like to save your work before creating a new map?",
            "💾 Save & New Map",
            "🗑 Discard & New Map",
        ),
        crate::editor_state::PendingDirtyAction::CloseDocumentTab(_) => (
            "Would you like to save your work before closing this map tab?",
            "💾 Save & Close Tab",
            "🗑 Discard & Close Tab",
        ),
        crate::editor_state::PendingDirtyAction::OpenUnrDialog
        | crate::editor_state::PendingDirtyAction::OpenT3dDialog
        | crate::editor_state::PendingDirtyAction::OpenMapPath(_) => (
            "Would you like to save your work before opening another map?",
            "💾 Save & Open Map",
            "🗑 Discard & Open Map",
        ),
    };

    let execute_pending_action = |st: &mut EditorState| {
        st.execute_pending_dirty_action();
    };

    // Semi-transparent backdrop overlay in Order::Middle so the Order::Foreground modal stays bright
    crate::gui::modal::render_modal_backdrop(
        ctx,
        "unsaved_exit_backdrop",
        egui::Color32::from_black_alpha(150),
    );

    // Keyboard navigation: Escape key cancels modal
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        state.show_unsaved_exit_modal = false;
        state.pending_dirty_action = None;
        return;
    }

    egui::Window::new("⚠ Unsaved Changes")
        .order(egui::Order::Foreground)
        .fade_in(false)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.set_min_width(360.0);
            ui.label(
                egui::RichText::new("The current map has unsaved modifications.")
                    .strong()
                    .color(egui::Color32::from_rgb(245, 185, 55)),
            );
            ui.add_space(4.0);
            ui.label(prompt_desc);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui
                    .button(
                        egui::RichText::new(save_btn_lbl)
                            .strong()
                            .color(egui::Color32::from_rgb(90, 235, 135)),
                    )
                    .clicked()
                {
                    state.show_unsaved_exit_modal = false;
                    let target_path = if let Some(ref path) = state.map_path.clone() {
                        Some(path.clone())
                    } else {
                        let default_dir = state.default_maps_directory();
                        let mut dialog = rfd::FileDialog::new().add_filter("Unreal Map (*.unr)", &["unr"]);
                        if default_dir.exists() {
                            dialog = dialog.set_directory(&default_dir);
                        }
                        dialog.save_file()
                    };

                    if let Some(path) = target_path {
                        state.trigger_save_unr(&path);
                    } else {
                        // User cancelled the file dialog, abort the pending action
                        state.pending_dirty_action = None;
                    }
                }
                ui.add_space(8.0);
                if ui
                    .button(
                        egui::RichText::new(discard_btn_lbl)
                            .color(egui::Color32::from_rgb(245, 95, 95)),
                    )
                    .clicked()
                {
                    execute_pending_action(state);
                }
                ui.add_space(8.0);
                if ui.button("Cancel").clicked() {
                    state.show_unsaved_exit_modal = false;
                    state.pending_dirty_action = None;
                }
            });
        });
}

fn render_toast_notifications(ctx: &egui::Context, state: &mut EditorState) {
    state.prune_expired_toasts();
    if state.toasts.is_empty() {
        return;
    }

    // Request periodic repaint while toasts are active to ensure smooth expiration without unthrottled 1000+ FPS loops
    ctx.request_repaint_after(std::time::Duration::from_millis(50));

    let mut to_dismiss = Vec::new();

    egui::Area::new(egui::Id::new("floating_toast_notifications_area"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 48.0))
        .order(egui::Order::Tooltip)
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 8.0;

                for toast in &state.toasts {
                    let (accent_color, icon_str) = match toast.kind {
                        crate::editor_state::ToastKind::Info => (
                            egui::Color32::from_rgb(60, 160, 245),
                            "ℹ",
                        ),
                        crate::editor_state::ToastKind::Success => (
                            egui::Color32::from_rgb(45, 205, 95),
                            "✔",
                        ),
                        crate::editor_state::ToastKind::Warning => (
                            egui::Color32::from_rgb(245, 175, 40),
                            "⚠",
                        ),
                        crate::editor_state::ToastKind::Error => (
                            egui::Color32::from_rgb(245, 60, 60),
                            "✖",
                        ),
                    };

                    egui::Frame::window(&ctx.style())
                        .fill(egui::Color32::from_rgba_premultiplied(22, 25, 32, 240))
                        .stroke(egui::Stroke::new(1.0_f32, accent_color.linear_multiply(0.7_f32)))
                        .show(ui, |ui| {
                            ui.set_max_width(340.0);
                            ui.horizontal(|ui| {
                                // Accent pill / icon
                                ui.label(
                                    egui::RichText::new(icon_str)
                                        .size(15.0)
                                        .color(accent_color)
                                        .strong()
                                );
                                ui.add_space(4.0);

                                // Toast message
                                ui.label(
                                    egui::RichText::new(&toast.message)
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(235, 240, 248))
                                );

                                // Dismiss button (WCAG 2.2 SC 2.5.8 >= 24x24 px)
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.add_sized([24.0, 24.0], egui::Button::new("×")).clicked() {
                                        to_dismiss.push(toast.id);
                                    }
                                });
                            });
                        });
                }
            });
        });

    for id in to_dismiss {
        state.dismiss_toast(id);
    }
}

fn render_status_bar_content(ui: &mut egui::Ui, state: &mut EditorState) {
    // Right Zone: pinned to the right edge so essential toggles never clip off-screen
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        // 1. Console Drawer Toggle
        let console_label = if state.show_bottom_console { "🔽 Console" } else { "🔼 Console" };
        let msg_count = state.console_messages.len();
        let console_btn_text = if msg_count > 0 {
            format!("{console_label} ({msg_count})")
        } else {
            console_label.to_string()
        };
        let console_col = if state.show_bottom_console {
            egui::Color32::from_rgb(245, 182, 45)
        } else {
            egui::Color32::from_rgb(175, 185, 200)
        };
        if ui.button(egui::RichText::new(console_btn_text).size(11.0).color(console_col))
            .on_hover_text("Toggle Output Console Drawer (Ctrl+` or Ctrl+Shift+C)")
            .clicked()
        {
            state.show_bottom_console = !state.show_bottom_console;
        }

        ui.separator();

        // 2. Active Viewport / Layout Toggle
        let view_label = match state.viewport_layout {
            crate::camera::ViewportLayout::Single => format!("View: {}", state.active_viewport_type.label()),
            crate::camera::ViewportLayout::FourViewports => format!("2x2 Quad [{}]", state.active_viewport_type.label()),
        };
        if ui.button(egui::RichText::new(view_label).size(11.0).color(egui::Color32::from_rgb(245, 182, 45)))
            .on_hover_text("Click to toggle Single / 2x2 Quad Viewports (Space)")
            .clicked()
        {
            state.viewport_layout = match state.viewport_layout {
                crate::camera::ViewportLayout::Single => crate::camera::ViewportLayout::FourViewports,
                crate::camera::ViewportLayout::FourViewports => crate::camera::ViewportLayout::Single,
            };
        }

        ui.separator();

        // 3. Frustum Culling Status & Toggle
        let (_rb, cb, _ra, ca) = state.last_cull_stats;
        let culling_btn = if state.frustum_culling_enabled {
            if cb > 0 || ca > 0 {
                egui::RichText::new(format!("👁 Cull: -{cb}b -{ca}a")).size(11.0).color(egui::Color32::from_rgb(100, 240, 160))
            } else {
                egui::RichText::new("👁 Cull: ON").size(11.0).color(egui::Color32::from_rgb(100, 240, 160))
            }
        } else {
            egui::RichText::new("👁 Cull: OFF").size(11.0).color(egui::Color32::from_rgb(160, 170, 185))
        };
        if ui.button(culling_btn)
            .on_hover_text("Click to toggle Dynamic View Frustum Culling Engine")
            .clicked()
        {
            state.toggle_frustum_culling();
        }

        ui.separator();

        // 4. Viewport Shading Mode
        let shading_label = state.viewport_shading.display_name();
        if ui.button(egui::RichText::new(&shading_label).size(11.0))
            .on_hover_text("Click to cycle Viewport Shading")
            .clicked()
        {
            state.viewport_shading = state.viewport_shading.next();
        }

        ui.separator();

        // Left-to-center content container
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // Active Saving / Exporting Live Progress Indicator
            if let Some(ref fb) = state.save_feedback {
                match fb.phase {
                    crate::editor_state::SavePhase::PendingPreparation | crate::editor_state::SavePhase::Executing => {
                        ui.add(egui::Spinner::new().size(13.0).color(egui::Color32::from_rgb(0, 215, 255)));
                        let elapsed = fb.started_at.elapsed().as_secs_f32();
                        let stage_text = if fb.total_stages > 0 {
                            format!("💾 [{}/{}] {} ({:.1}s)", fb.current_stage, fb.total_stages, fb.filename, elapsed)
                        } else {
                            format!("💾 {} ({:.1}s)", fb.filename, elapsed)
                        };
                        ui.label(egui::RichText::new(stage_text).strong().size(11.0).color(egui::Color32::from_rgb(0, 215, 255)));
                        ui.separator();
                    }
                    crate::editor_state::SavePhase::Completed => {
                        let succ_title = crate::i18n::tr(state.language, "save.success_title");
                        ui.label(egui::RichText::new(format!("✔ {}: {}", succ_title, fb.filename)).strong().size(11.0).color(egui::Color32::from_rgb(80, 235, 120)));
                        ui.separator();
                    }
                    crate::editor_state::SavePhase::Failed => {
                        let fail_title = crate::i18n::tr(state.language, "save.failed_title");
                        ui.label(egui::RichText::new(format!("❌ {}: {}", fail_title, fb.filename)).strong().size(11.0).color(egui::Color32::from_rgb(255, 90, 90)));
                        ui.separator();
                    }
                }
            }

            // Active Selection Breadcrumbs
            let selection_text = if state.has_multi_selection() {
                let total = state.multi_selected_brushes.len() + state.multi_selected_actors.len();
                format!("📦 Multi-Selection: {} items", total)
            } else if let Some(b_idx) = state.selected_brush_index {
                let op = if b_idx < state.world_brushes.len() {
                    match state.world_brushes[b_idx].csg_oper {
                        ut99_bsp::CsgOperation::Add => "Add",
                        ut99_bsp::CsgOperation::Subtract => "Sub",
                        _ => "Brush",
                    }
                } else { "Brush" };
                format!("🟦 Brush #{b_idx} ({op})")
            } else if let Some(m_idx) = state.selected_mover_index {
                let key = state.movers.get(m_idx).map(|m| m.current_key).unwrap_or(0);
                format!("🚪 Mover #{m_idx} (Key {key})")
            } else if let Some(mesh_idx) = state.selected_static_mesh_index {
                format!("🗿 StaticMesh #{mesh_idx}")
            } else if let Some(act_idx) = state.selected_actor_index {
                let class = state.loaded_unr.as_ref()
                    .and_then(|u| u.actors.get(act_idx))
                    .map(|a| a.class_name.as_str())
                    .unwrap_or("Actor");
                format!("⚙ #{act_idx} {class}")
            } else {
                "Ready — Red Builder Brush".to_string()
            };

            ui.label(egui::RichText::new(selection_text).strong().size(11.0).color(egui::Color32::from_rgb(220, 225, 235)));
            ui.separator();

            // Camera Coordinates
            let cam_pos = state.camera.position;
            ui.label(egui::RichText::new(format!("Cam: ({:.0}, {:.0}, {:.0})", cam_pos.x, cam_pos.y, cam_pos.z)).monospace().size(11.0).color(egui::Color32::from_rgb(170, 180, 195)));
            ui.separator();

            // Camera Speed Preset
            let spd_label = format!("🚀 Spd: {}", state.camera_speed_preset);
            if ui.button(egui::RichText::new(spd_label).size(11.0))
                .on_hover_text("Click to cycle Camera Speed Preset (1-8)")
                .clicked()
            {
                state.cycle_camera_speed();
            }
            ui.separator();

            // Grid Snap
            let grid_status = if state.snap_to_grid {
                format!("Grid: {:.0}u", state.grid_step)
            } else {
                "Grid: OFF".to_string()
            };
            if ui.small_button("➖").on_hover_text("Halve grid step ([)").clicked() {
                state.step_grid_down();
            }
            if ui.button(egui::RichText::new(grid_status).size(11.0).color(egui::Color32::from_rgb(180, 190, 200)))
                .on_hover_text("Click to toggle Grid Snapping")
                .clicked()
            {
                state.snap_to_grid = !state.snap_to_grid;
            }
            if ui.small_button("➕").on_hover_text("Double grid step (])").clicked() {
                state.step_grid_up();
            }

            // Live Log Preview in Status Bar when console is collapsed
            if !state.show_bottom_console {
                if let Some(last_msg) = state.console_messages.last() {
                    ui.separator();
                    let clean_msg = last_msg.trim();
                    let char_count = clean_msg.chars().count();
                    let truncated = if char_count > 40 {
                        format!("{}...", clean_msg.chars().take(37).collect::<String>())
                    } else {
                        clean_msg.to_string()
                    };
                    if ui.button(egui::RichText::new(format!("💬 {truncated}")).monospace().size(10.5).color(egui::Color32::from_rgb(150, 165, 185)))
                        .on_hover_text("Click to open full console drawer")
                        .clicked()
                    {
                        state.show_bottom_console = true;
                    }
                }
            }
        });
    });
}

fn render_viewport_quadrant_badges(ctx: &egui::Context, state: &mut EditorState) {
    let [vx_phys, vy_phys, vw_phys, vh_phys] = state.viewport_rect;
    if vw_phys <= 50.0 || vh_phys <= 50.0 {
        return;
    }

    let ppp = ctx.pixels_per_point().max(0.1);
    let vx = vx_phys / ppp;
    let vy = vy_phys / ppp;
    let vw = vw_phys / ppp;
    let vh = vh_phys / ppp;

    let hw = vw * 0.5;
    let hh = vh * 0.5;

    let quadrants = [
        ("quad_top", vx + 10.0, vy + 10.0, crate::camera::ViewportType::Top, "Top (XY)"),
        ("quad_side", vx + hw + 10.0, vy + 10.0, crate::camera::ViewportType::Side, "Side (YZ)"),
        ("quad_front", vx + 10.0, vy + hh + 10.0, crate::camera::ViewportType::Front, "Front (XZ)"),
        ("quad_persp", vx + hw + 10.0, vy + hh + 10.0, crate::camera::ViewportType::Perspective, "3D Perspective"),
    ];

    let mut switch_to_single: Option<crate::camera::ViewportType> = None;
    let mut set_active: Option<crate::camera::ViewportType> = None;

    for (id_str, px, py, vtype, label) in quadrants {
        let is_active = state.active_viewport_type == vtype;
        egui::Area::new(egui::Id::new(id_str))
            .fixed_pos(egui::pos2(px, py))
            .show(ctx, |ui| {
                let border_color = if is_active {
                    egui::Color32::from_rgb(245, 182, 45)
                } else {
                    egui::Color32::from_rgb(60, 65, 75)
                };
                let bg_color = if is_active {
                    egui::Color32::from_black_alpha(210)
                } else {
                    egui::Color32::from_black_alpha(160)
                };

                egui::Frame::window(&ctx.style())
                    .fill(bg_color)
                    .stroke(egui::Stroke::new(1.0_f32, border_color))
                    .corner_radius(4.0_f32)
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let text_color = if is_active {
                                egui::Color32::from_rgb(245, 182, 45)
                            } else {
                                egui::Color32::from_rgb(180, 185, 195)
                            };
                            let prefix = if is_active { "● " } else { "" };
                            let label_res = ui.selectable_label(is_active, egui::RichText::new(format!("{prefix}{label}")).color(text_color).strong().size(11.0));
                            if label_res.clicked() {
                                set_active = Some(vtype);
                            }
                            if ui.add_sized([24.0, 24.0], egui::Button::new("⛶")).on_hover_text("Maximize this viewport (Single view)").clicked() {
                                switch_to_single = Some(vtype);
                            }
                        });
                    });
            });
    }

    if let Some(target_vtype) = switch_to_single {
        state.viewport_layout = crate::camera::ViewportLayout::Single;
        state.active_viewport_type = target_vtype;
    } else if let Some(target_vtype) = set_active {
        state.active_viewport_type = target_vtype;
    }
}


pub fn ut99_light_to_color(hue: u8, sat: u8, brightness: u8) -> egui::Color32 {
    let h = (hue as f32) / 255.0 * 6.0;
    let s = 1.0 - ((sat as f32) / 255.0);
    let v = ((brightness as f32) / 255.0).min(1.0);

    let c = v * s;
    let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
    let m = v - c;

    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };

    egui::Color32::from_rgb(
        ((r + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((g + m) * 255.0).clamp(0.0, 255.0) as u8,
        ((b + m) * 255.0).clamp(0.0, 255.0) as u8,
    )
}

fn render_master_browser_window(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_master_browser {
        return;
    }

    let mut is_open = state.show_master_browser;
    crate::gui::modal::studio_dialog_window(ctx, "UnrealEd 2.0 Master Browser & Smart Prefab Hub", &mut is_open)
        .show(ctx, |ui| {
            // Top Tab Bar replicating UnrealEd 2.0 Master Browser + SmartPrefabs
            ui.horizontal_wrapped(|ui| {
                for tab in crate::editor_state::MasterBrowserTab::all() {
                    if ui
                        .selectable_label(state.master_browser_tab == *tab, tab.label())
                        .clicked()
                    {
                        state.master_browser_tab = *tab;
                    }
                }
            });
            ui.separator();

            match state.master_browser_tab {
                crate::editor_state::MasterBrowserTab::Textures => {
                    let available_pkgs = state.texture_manager.available_packages().to_vec();
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Package:").strong());
                        let mut cur_pkg = state.selected_texture_package.clone();
                        egui::ComboBox::from_id_salt("mb_tex_pkg_combo")
                            .selected_text(&cur_pkg)
                            .width(140.0)
                            .show_ui(ui, |ui| {
                                for pkg in &available_pkgs {
                                    if ui.selectable_value(&mut cur_pkg, pkg.clone(), pkg).clicked() {
                                        state.selected_texture_package = cur_pkg.clone();
                                        state.selected_texture_group = "All".to_string();
                                    }
                                }
                            });
                        let pkg_name = state.selected_texture_package.clone();
                        let groups = state.texture_manager.get_package_groups(&pkg_name);
                        ui.label("Group:");
                        let mut cur_grp = state.selected_texture_group.clone();
                        egui::ComboBox::from_id_salt("mb_tex_grp_combo")
                            .selected_text(&cur_grp)
                            .width(110.0)
                            .show_ui(ui, |ui| {
                                for g in &groups {
                                    if ui.selectable_value(&mut cur_grp, g.clone(), g).clicked() {
                                        state.selected_texture_group = cur_grp.clone();
                                    }
                                }
                            });
                        ui.label("Filter:");
                        ui.add(
                            egui::TextEdit::singleline(&mut state.texture_search_query)
                                .hint_text("Search...")
                                .desired_width(110.0),
                        );
                        if ui.button("💾 Save .utx...").on_hover_text("Save current texture package as a standalone binary .utx file").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Unreal Texture Package (.utx)", &["utx"])
                                .set_file_name(format!("{}.utx", state.selected_texture_package))
                                .save_file()
                            {
                                let pkg = state.selected_texture_package.clone();
                                let _ = state.export_texture_package_utx(&pkg, &path);
                            }
                        }
                        if ui.button("🗗 Pop Out (F4)").clicked() {
                            state.show_texture_browser = true;
                        }
                    });
                    ui.separator();
                    let pkg_name = state.selected_texture_package.clone();
                    let _ = state.texture_manager.load_package(&pkg_name);
                    let search = state.texture_search_query.to_lowercase();
                    let grp_filter = state.selected_texture_group.clone();
                    let mut picked_tex = None;
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        if let Some(pkg) = state.texture_manager.get_package(&pkg_name) {
                            ui.horizontal_wrapped(|ui| {
                                for tex in &pkg.textures {
                                    if grp_filter != "All" && grp_filter != "In Use" && !tex.group.eq_ignore_ascii_case(&grp_filter) {
                                        continue;
                                    }
                                    if !search.is_empty() && !tex.name.to_lowercase().contains(&search) {
                                        continue;
                                    }
                                    let full = format!("{}.{}", pkg_name, tex.name);
                                    let is_sel = state.active_texture.as_deref() == Some(full.as_str());
                                    if ui
                                        .selectable_label(
                                            is_sel,
                                            format!("🖼 {} ({}x{})", tex.name, tex.width, tex.height),
                                        )
                                        .clicked()
                                    {
                                        picked_tex = Some(full);
                                    }
                                }
                            });
                        } else {
                            ui.label("Select or import a texture package (e.g. MyLevel) to browse textures.");
                        }
                    });
                    if let Some(t) = picked_tex {
                        state.active_texture = Some(t);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Active:");
                        ui.monospace(state.active_texture.as_deref().unwrap_or("DefaultTexture"));
                        if ui.button("🖌 Apply to Builder Brush").clicked() {
                            state.apply_active_texture_to_builder();
                        }
                    });
                }
                crate::editor_state::MasterBrowserTab::ActorClasses => {
                    ui.horizontal(|ui| {
                        ui.label("Search Class:");
                        ui.text_edit_singleline(&mut state.actor_browser_search);
                        if let Some(ref cls) = state.selected_class_name.clone() {
                            if ui.button(format!("➕ Place '{}' at Builder", cls)).clicked() {
                                state.place_actor_of_class(cls);
                            }
                            if ui.button("📜 Script").clicked() {
                                state.open_uc_viewer_for_class(cls);
                                state.master_browser_tab = crate::editor_state::MasterBrowserTab::UnrealScript;
                            }
                        }
                        if ui.button("🗗 Pop Out").clicked() {
                            state.show_actor_browser = true;
                        }
                    });
                    ui.separator();
                    let search = state.actor_browser_search.to_lowercase();
                    let mut classes = Vec::new();
                    if let Some(ref root) = state.class_tree {
                        fn collect_nodes(
                            node: &ut99_format::package::ClassTreeNode,
                            search: &str,
                            out: &mut Vec<(String, String)>,
                        ) {
                            if search.is_empty() || node.name.to_lowercase().contains(search) {
                                out.push((node.name.clone(), node.package.clone()));
                            }
                            for child in &node.children {
                                collect_nodes(child, search, out);
                            }
                        }
                        collect_nodes(root, &search, &mut classes);
                    }
                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for (cls_name, pkg_name) in classes {
                            let is_sel = state.selected_class_name.as_deref() == Some(cls_name.as_str());
                            if ui
                                .selectable_label(is_sel, format!("⚙ {} ({})", cls_name, pkg_name))
                                .clicked()
                            {
                                state.selected_class_name = Some(cls_name);
                                state.selected_class_package = Some(pkg_name);
                            }
                        }
                    });
                }
                crate::editor_state::MasterBrowserTab::Groups => {
                    ui.horizontal(|ui| {
                        ui.label("🔍 Filter Groups:");
                        ui.text_edit_singleline(&mut state.group_browser_search);
                        if ui.button("👁 Show All").clicked() {
                            state.set_all_groups_visibility(true);
                        }
                        if ui.button("👁‍🗨 Hide All").clicked() {
                            state.set_all_groups_visibility(false);
                        }
                        if ui.button("🗗 Pop Out").clicked() {
                            state.show_group_browser = true;
                        }
                    });
                    ui.separator();
                    let groups = state.get_map_groups();
                    let search = state.group_browser_search.to_lowercase();
                    let mut toggle_grp = None;
                    let mut select_grp = None;
                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for (g_name, b_count, a_count) in groups {
                            if !search.is_empty() && !g_name.to_lowercase().contains(&search) {
                                continue;
                            }
                            let is_vis = state.is_actor_group_visible(&g_name);
                            ui.horizontal(|ui| {
                                let mut vis = is_vis;
                                if ui.checkbox(&mut vis, "").changed() {
                                    toggle_grp = Some(g_name.clone());
                                }
                                ui.label(egui::RichText::new(&g_name).strong());
                                ui.label(format!("({} brushes, {} actors)", b_count, a_count));
                                if ui.small_button("Select").clicked() {
                                    select_grp = Some(g_name.clone());
                                }
                            });
                        }
                    });
                    if let Some(g) = toggle_grp {
                        state.toggle_group_visibility(&g);
                    }
                    if let Some(g) = select_grp {
                        state.select_objects_by_group(&g);
                    }
                }
                crate::editor_state::MasterBrowserTab::Sounds => {
                    let mut switch_pkg = None;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Package (.uax):").strong());
                        egui::ComboBox::from_id_salt("mb_sound_pkg_combo")
                            .selected_text(&state.selected_sound_package)
                            .show_ui(ui, |ui| {
                                for pkg in &state.sound_packages {
                                    if ui
                                        .selectable_value(&mut state.selected_sound_package, pkg.clone(), pkg)
                                        .clicked()
                                    {
                                        switch_pkg = Some(pkg.clone());
                                    }
                                }
                            });
                        ui.label("Filter:");
                        ui.text_edit_singleline(&mut state.sound_browser_search);
                        if ui.button("🔄 Reload").clicked() {
                            state.scan_sound_packages();
                        }
                        if ui.button("🗗 Pop Out").clicked() {
                            state.show_sound_browser = true;
                        }
                    });
                    if let Some(pkg) = switch_pkg {
                        state.load_sound_package(&pkg);
                    }
                    ui.separator();
                    let search = state.sound_browser_search.to_lowercase();
                    let mut picked_snd = None;
                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for snd in &state.loaded_sounds {
                            if !search.is_empty() && !snd.name.to_lowercase().contains(&search) {
                                continue;
                            }
                            let is_sel = state.selected_sound_name.as_deref() == Some(snd.name.as_str());
                            if ui
                                .selectable_label(
                                    is_sel,
                                    format!("🔊 {} ({} Hz, {:.2}s)", snd.name, snd.sample_rate, snd.duration_seconds),
                                )
                                .clicked()
                            {
                                picked_snd = Some(snd.name.clone());
                            }
                        }
                    });
                    if let Some(s) = picked_snd {
                        state.selected_sound_name = Some(s);
                    }
                }
                crate::editor_state::MasterBrowserTab::Music => {
                    let mut switch_pkg = None;
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Music Track (.umx):").strong());
                        egui::ComboBox::from_id_salt("mb_music_pkg_combo")
                            .selected_text(&state.selected_music_package)
                            .show_ui(ui, |ui| {
                                for pkg in &state.music_packages {
                                    if ui
                                        .selectable_value(&mut state.selected_music_package, pkg.clone(), pkg)
                                        .clicked()
                                    {
                                        switch_pkg = Some(pkg.clone());
                                    }
                                }
                            });
                        if ui.button("🎵 Set as Level Song").clicked() && !state.selected_music_package.is_empty() {
                            state.level_song = state.selected_music_package.clone();
                            state.toast_success(format!("Level Song set to '{}'", state.level_song));
                        }
                        if ui.button("🗗 Pop Out").clicked() {
                            state.show_music_browser = true;
                        }
                    });
                    if let Some(pkg) = switch_pkg {
                        state.load_music_package(&pkg);
                    }
                    ui.separator();
                    if let Some(ref music) = state.loaded_music {
                        ui.label(format!("Package: {} | Title: {} | Format: {} ({} bytes)", music.package, music.title, music.format, music.data_size));
                    } else {
                        ui.label("Select a .umx package to inspect its tracker metadata.");
                    }
                }
                crate::editor_state::MasterBrowserTab::Meshes => {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(egui::RichText::new("Category:").strong());
                        let cats = state.mesh_library.categories();
                        let mut cur_cat = state.mesh_browser_category.clone();
                        egui::ComboBox::from_id_salt("mb_mesh_cat_combo")
                            .selected_text(&cur_cat)
                            .width(130.0)
                            .show_ui(ui, |ui| {
                                for c in &cats {
                                    if ui.selectable_value(&mut cur_cat, c.clone(), c).clicked() {
                                        state.mesh_browser_category = cur_cat.clone();
                                    }
                                }
                            });
                        ui.label("Filter:");
                        ui.text_edit_singleline(&mut state.mesh_browser_search);
                        if ui.button("📦 Scan System/*.u Meshes").on_hover_text("Decode UMesh/ULodMesh 3D geometry from System/*.u packages").clicked() {
                            if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
                                let added = state.mesh_library.scan_u_package_meshes_into_library(&ut_dir.join("System"));
                                state.toast_info(format!("Decoded {} meshes from System/*.u", added));
                            }
                        }
                        if ui.button("🗗 Pop Out").clicked() {
                            state.show_mesh_browser = true;
                        }
                    });
                    ui.separator();
                    let cat_filter = state.mesh_browser_category.clone();
                    let search = state.mesh_browser_search.to_lowercase();
                    let mut picked_mesh = None;
                    let mut copy_to_builder = None;
                    egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                        for m in &state.mesh_library.meshes {
                            if cat_filter != "All" && !m.category.eq_ignore_ascii_case(&cat_filter) {
                                continue;
                            }
                            if !search.is_empty() && !m.name.to_lowercase().contains(&search) {
                                continue;
                            }
                            let is_sel = state.selected_mesh_name.as_deref() == Some(m.name.as_str());
                            ui.horizontal(|ui| {
                                if ui
                                    .selectable_label(
                                        is_sel,
                                        format!("🗿 {} [{}] ({} polys, {} verts)", m.name, m.category, m.poly_count(), m.vertex_count()),
                                    )
                                    .clicked()
                                {
                                    picked_mesh = Some(m.name.clone());
                                }
                                if ui.small_button("➡ Red Builder").clicked() {
                                    copy_to_builder = Some(m.brush.clone());
                                }
                            });
                        }
                    });
                    if let Some(m) = picked_mesh {
                        state.selected_mesh_name = Some(m);
                    }
                    if let Some(mut b) = copy_to_builder {
                        b.location = state.builder_brush.location;
                        state.builder_brush = b;
                        state.toast_success("Copied mesh geometry into Red Builder Brush");
                    }
                }
                crate::editor_state::MasterBrowserTab::UnrealScript => {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("Class: {}", state.uc_viewer_class)).strong());
                        if let Some(ref sel_cls) = state.selected_class_name.clone() {
                            if ui.button(format!("🔄 Load '{}'", sel_cls)).clicked() {
                                state.open_uc_viewer_for_class(sel_cls);
                            }
                        }
                        if ui.button("🗗 Open Full UnrealScript Studio").clicked() {
                            state.show_uc_viewer_dialog = true;
                        }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(400.0).show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut state.uc_source_cache.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_width(f32::INFINITY),
                        );
                    });
                }
                crate::editor_state::MasterBrowserTab::SmartPrefabs => {
                    ui.heading("🧩 Data-Driven Parametric Smart Prefabs");
                    ui.horizontal_wrapped(|ui| {
                        for cat in crate::prefab_templates::SmartPrefabCategory::all() {
                            if ui
                                .selectable_label(state.smart_prefab_category == *cat, cat.label())
                                .clicked()
                            {
                                state.smart_prefab_category = *cat;
                            }
                        }
                    });
                    ui.separator();

                    let catalog = crate::prefab_templates::SmartPrefabLibrary::catalog();
                    for item in catalog
                        .iter()
                        .filter(|t| t.category == state.smart_prefab_category)
                    {
                        let selected = state.selected_smart_prefab_id == item.id;
                        if ui
                            .selectable_label(
                                selected,
                                format!("{} — {}", item.title, item.summary),
                            )
                            .clicked()
                        {
                            state.selected_smart_prefab_id = item.id.to_string();
                        }
                    }

                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        ui.add(egui::DragValue::new(&mut state.smart_prefab_params.width).speed(8.0));
                        ui.label("Length:");
                        ui.add(egui::DragValue::new(&mut state.smart_prefab_params.length).speed(8.0));
                        ui.label("Height:");
                        ui.add(egui::DragValue::new(&mut state.smart_prefab_params.height).speed(8.0));
                    });
                    if ui
                        .button("⚡ Instantiate Smart Prefab at Builder Origin")
                        .clicked()
                    {
                        let id = state.selected_smart_prefab_id.clone();
                        let origin = state.builder_brush.location;
                        let _ = state.instantiate_smart_prefab(&id, origin);
                    }

                    ui.separator();
                    ui.heading("✏ Freehand Polygon Drawing & Extruder");
                    ui.horizontal(|ui| {
                        ui.label(format!("Vertices: {}", state.freehand_points.len()));
                        ui.label("Extrude Depth:");
                        ui.add(egui::DragValue::new(&mut state.freehand_extrude_depth).speed(4.0));
                        if ui.button("➕ Add Builder Origin as Vertex").clicked() {
                            let pt = state.builder_brush.location;
                            state.add_freehand_vertex(pt);
                        }
                        if ui.button("🚀 Extrude to Red Builder Brush").clicked() {
                            let d = state.freehand_extrude_depth;
                            let _ = state.extrude_freehand_polygon_to_builder(d);
                        }
                        if ui.button("🗑 Clear").clicked() {
                            state.clear_freehand_polygon();
                        }
                    });
                }
            }
        });
    state.show_master_browser = is_open;
}

