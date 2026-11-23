// crates/ut99-editor/tests/test_all_menus_and_dialogs_open_without_crash.rs
//
// Comprehensive regression test verifying that ALL editor menus, submenus,
// action wizards, and studios can be opened simultaneously and rendered
// without panics or crashes across both empty and populated states,
// as well as multiple viewport resolutions.

use egui::{Context, Pos2, Rect, Vec2, ViewportId, ViewportInfo};
use glam::Vec3;
use ut99_editor::editor_state::EditorState;

fn make_test_context(width: f32, height: f32) -> Context {
    let ctx = Context::default();
    let mut raw = egui::RawInput::default();
    let screen = Rect::from_min_size(Pos2::ZERO, Vec2::new(width, height));
    raw.screen_rect = Some(screen);
    raw.viewports.insert(
        ViewportId::ROOT,
        ViewportInfo {
            inner_rect: Some(screen),
            ..Default::default()
        },
    );
    let _ = ctx.run(raw, |_| {});
    ctx
}

fn open_all_editor_windows(state: &mut EditorState) {
    state.show_map_wizard_dialog = true;
    state.show_action_wizards_dialog = true;
    state.show_visual_presets_dialog = true;
    state.show_level_doctor_window = true;
    state.show_map_statistics_dialog = true;
    state.show_map_budget_dialog = true;
    state.show_gameplay_balance_dialog = true;
    state.show_scale_map_dialog = true;
    state.show_shape_editor = true;
    state.show_bsp_quality_studio_dialog = true;
    state.show_loft_studio_dialog = true;
    state.show_stairway_studio_dialog = true;
    state.show_architectural_studio_dialog = true;
    state.show_terrain_studio_dialog = true;
    state.show_lod_collision_dialog = true;
    state.show_scatter_studio_dialog = true;
    state.show_skylight_dialog = true;
    state.show_vent_duct_dialog = true;
    state.show_pipe_conduit_dialog = true;
    state.show_prefab_browser = true;
    state.show_mesh_browser = true;
    state.show_lighting_studio_dialog = true;
    state.show_light_effects_studio_dialog = true;
    state.show_special_lit_dialog = true;
    state.show_light_shaft_dialog = true;
    state.show_corona_studio_dialog = true;
    state.show_shadow_penumbra = true;
    state.show_skybox_studio_dialog = true;
    state.show_zone_fog_studio_dialog = true;
    state.show_weather_dialog = true;
    state.show_water_surface_studio_dialog = true;
    state.show_zone_seal_dialog = true;
    state.show_zone_occlusion_studio_dialog = true;
    state.show_zone_pvs = true;
    state.show_warp_zone_studio_dialog = true;
    state.show_acoustics_dialog = true;
    state.show_soundscape_dialog = true;
    state.show_music_studio_dialog = true;
    state.show_alarm_beacon_dialog = true;
    state.show_bot_nav_studio_dialog = true;
    state.show_bot_simulator_window = true;
    state.show_bot_jump_routes_dialog = true;
    state.show_translocator_studio_dialog = true;
    state.show_lift_pathing_studio_dialog = true;
    state.show_teleporter_studio_dialog = true;
    state.show_teleporter_arch_dialog = true;
    state.show_combat_sightlines = true;
    state.show_mover_studio_dialog = true;
    state.show_elevator_dialog = true;
    state.show_secret_door_dialog = true;
    state.show_track_studio_dialog = true;
    state.show_trigger_network_dialog = true;
    state.show_kicker_studio_dialog = true;
    state.show_conveyor_dialog = true;
    state.show_piston_dialog = true;
    state.show_destructible_studio_dialog = true;
    state.show_reactor_core_dialog = true;
    state.show_fluid_studio_dialog = true;
    state.show_zone_velocity_dialog = true;
    state.show_ventilator_dialog = true;
    state.show_hazard_chamber_dialog = true;
    state.show_fluid_basin_dialog = true;
    state.show_bio_trap_dialog = true;
    state.show_hazard_barrel_dialog = true;
    state.show_laser_grid_dialog = true;
    state.show_force_field_dialog = true;
    state.show_arena_rules_dialog = true;
    state.show_spawn_distribution_dialog = true;
    state.show_weapon_circuit_dialog = true;
    state.show_armor_stacking_dialog = true;
    state.show_powerup_control_dialog = true;
    state.show_jump_boots_dialog = true;
    state.show_supply_locker_dialog = true;
    state.show_ctf_base_dialog = true;
    state.show_ctf_symmetry_dialog = true;
    state.show_domination_dialog = true;
    state.show_assault_dialog = true;
    state.show_team_objective_dialog = true;
    state.show_ballistics_studio_dialog = true;
    state.show_sniper_vantage_dialog = true;
    state.show_turret_dialog = true;
    state.show_team_cannon_dialog = true;
    state.show_radar_dialog = true;
    state.show_dispatcher_dialog = true;
    state.show_cinematic_studio_dialog = true;
    state.show_skeletal_studio_dialog = true;
    state.show_skin_studio_dialog = true;
    state.show_package_extractor_dialog = true;
    state.show_uc_class_wizard_dialog = true;
    state.show_particle_studio_dialog = true;
    state.show_procedural_texture_dialog = true;
    state.show_decal_studio_dialog = true;
    state.show_hologram_dialog = true;
    state.show_surveillance_dialog = true;
    state.show_package_manifest_dialog = true;
    state.show_map_packaging_dialog = true;
    state.show_play_settings_dialog = true;
    state.show_camera_bookmarks_dialog = true;
    state.show_find_actors_dialog = true;
    state.show_replace_textures_dialog = true;
    state.show_actor_align_dialog = true;
    state.show_settings_dialog = true;
    state.show_lighting_dialog = true;
    state.show_build_options_dialog = true;
    state.show_autosave_dialog = true;
    state.show_command_palette = true;
    state.show_welcome_wizard = true;
    state.show_texture_browser = true;
    state.show_actor_browser = true;
    state.show_sound_browser = true;
    state.show_music_browser = true;
    state.show_group_browser = true;
    state.show_surface_properties = true;
    state.show_actor_properties = true;
    state.show_level_properties_dialog = true;
    state.show_master_browser = true;
}

#[test]
fn test_render_all_windows_empty_state_720p() {
    let mut state = EditorState::default();
    open_all_editor_windows(&mut state);

    let ctx = make_test_context(1280.0, 720.0);
    let mut raw = egui::RawInput::default();
    raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1280.0, 720.0)));

    let _ = ctx.run(raw, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });
}

#[test]
fn test_render_all_windows_empty_state_1080p() {
    let mut state = EditorState::default();
    open_all_editor_windows(&mut state);

    let ctx = make_test_context(1920.0, 1080.0);
    let mut raw = egui::RawInput::default();
    raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0)));

    let _ = ctx.run(raw, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });
}

#[test]
fn test_render_all_windows_compact_screen_768p() {
    let mut state = EditorState::default();
    open_all_editor_windows(&mut state);

    let ctx = make_test_context(1024.0, 768.0);
    let mut raw = egui::RawInput::default();
    raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1024.0, 768.0)));

    let _ = ctx.run(raw, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });
}

#[test]
fn test_render_all_windows_populated_map() {
    let mut state = EditorState::default();
    state.new_map();
    state.set_builder_cube(Vec3::splat(256.0));
    state.csg_add();
    state.add_light_at_builder();
    state.add_player_start_at_builder();
    state.add_path_node_at_builder();
    state.build_geometry();
    state.build_lighting();

    open_all_editor_windows(&mut state);

    let ctx = make_test_context(1920.0, 1080.0);
    let mut raw = egui::RawInput::default();
    raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1920.0, 1080.0)));

    let _ = ctx.run(raw, |ctx| {
        ut99_editor::gui::render_ui(ctx, &mut state);
    });
}

#[test]
fn test_render_all_themes_and_experience_modes() {
    for &theme in ut99_editor::gui::theme::AppTheme::all() {
        for mode in [
            ut99_editor::editor_state::UiExperienceMode::Standard,
            ut99_editor::editor_state::UiExperienceMode::Architect,
        ] {
            let mut state = EditorState::default();
            state.theme = theme;
            state.ui_experience_mode = mode;
            open_all_editor_windows(&mut state);

            let ctx = make_test_context(1366.0, 768.0);
            ut99_editor::gui::theme::apply_theme_variant(&ctx, theme);
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1366.0, 768.0)));

            let _ = ctx.run(raw, |ctx| {
                ut99_editor::gui::render_ui(ctx, &mut state);
            });
        }
    }
}

#[test]
fn test_responsive_toolbar_fits_all_resolutions_without_clipping() {
    use ut99_editor::gui::theme::SemanticPalette;
    use ut99_editor::gui::toolbar::{
        plan_toolbar_layout, render_toolbar_cluster, ToolbarCluster, ToolbarDensityTier,
    };

    let widths = [800.0_f32, 960.0, 1024.0, 1152.0, 1280.0, 1366.0, 1440.0, 1600.0, 1920.0, 2560.0];

    for &w in &widths {
        let mut state = EditorState::default();
        let ctx = make_test_context(w, 900.0);

        // Run multiple frames to ensure multi-pass egui panel layout is completely stable
        for frame_idx in 0..3 {
            let mut raw = egui::RawInput::default();
            raw.screen_rect = Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(w, 900.0)));

            let _ = ctx.run(raw, |ctx| {
                ut99_editor::gui::render_ui(ctx, &mut state);

                // Verify the 3D central viewport was NOT swallowed by top/bottom panels
                let central_avail = ctx.available_rect();
                assert!(
                    central_avail.height() >= 700.0,
                    "Frame {frame_idx} at width {w}: central 3D viewport height collapsed to {} (top panel swallowed screen!)",
                    central_avail.height()
                );

                egui::CentralPanel::default().show(ctx, |ui| {
                    let palette = SemanticPalette::current(ui.ctx());
                    let plan = plan_toolbar_layout(ui, &state, w);

                    // 1. All 7 functional clusters must be present in exact order
                    let flattened: Vec<ToolbarCluster> = plan.rows.iter().flatten().copied().collect();
                    assert_eq!(
                        flattened,
                        ToolbarCluster::ALL.to_vec(),
                        "All 7 toolbar clusters must be present at width {w}"
                    );

                    // 2. Every planned row AND actual rendered row must fit inside the window width
                    for (r_idx, row_clusters) in plan.rows.iter().enumerate() {
                        let planned_rw = plan.row_widths[r_idx];
                        assert!(
                            planned_rw <= w,
                            "Planned row {r_idx} width ({planned_rw}) exceeds window width ({w}) in tier {:?}",
                            plan.tier
                        );

                        let row_resp = ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = match plan.tier {
                                ToolbarDensityTier::Expanded | ToolbarDensityTier::Standard => 5.0,
                                ToolbarDensityTier::Compact | ToolbarDensityTier::UltraCompact => 3.5,
                            };
                            for (c_idx, &cluster) in row_clusters.iter().enumerate() {
                                if c_idx > 0 {
                                    ui.separator();
                                }
                                render_toolbar_cluster(ui, &mut state, &palette, plan.tier, cluster);
                            }
                        });
                        let actual_rw = row_resp.response.rect.width();
                        assert!(
                            actual_rw <= w,
                            "ACTUAL rendered row {r_idx} width ({actual_rw:.1}px, planned {planned_rw:.1}px) exceeds window width ({w}px) in tier {:?}",
                            plan.tier
                        );
                    }

                    // 3. From 800px upwards, the toolbar must fit in at most 2 rows
                    assert!(
                        plan.rows.len() <= 2,
                        "Expected at most 2 toolbar rows at width {w}, got {}",
                        plan.rows.len()
                    );

                    // 4. At 1920px+ Full HD and 2560px+ QHD, it should fit on a single row
                    if w >= 1920.0 {
                        assert_eq!(
                            plan.rows.len(),
                            1,
                            "Expected single-row toolbar at width {w}, got {} rows (tier {:?})",
                            plan.rows.len(),
                            plan.tier
                        );
                    }
                });
            });
        }
    }
}


