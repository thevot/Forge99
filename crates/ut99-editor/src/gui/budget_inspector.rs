use crate::editor_state::EditorState;

pub fn render_budget_inspector(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_map_budget_dialog {
        return;
    }

    let mut open = state.show_map_budget_dialog;
    let mut close_dialog = false;

    crate::gui::modal::standard_dialog_window(ctx, "Map Performance & Engine Budget Profiler", &mut open, [540.0, 480.0], [400.0, 320.0])
        .show(ctx, |ui| {
            ui.collapsing("Editor GPU & frame diagnostics", |ui| {
                let mut copy_report: Option<String> = None;
                {
                    let diagnostics = &state.graphics_diagnostics;
                    if let Some(adapter) = &diagnostics.adapter {
                        ui.label(format!(
                            "Adapter: {} (Profile: {} | Tier: {})",
                            adapter.name,
                            state.hardware_profile.label(),
                            diagnostics.resolved_tier.label(),
                        ));
                        ui.label(format!("Backend: {:?} | Device: {:?}", adapter.backend, adapter.device_type));
                        ui.label(format!("Driver: {} {}", adapter.driver, adapter.driver_info));
                        ui.small(format!(
                            "Limits: max_tex_2d={} | max_array_layers={} (negotiated: {} / {})",
                            diagnostics.adapter_max_texture_dimension_2d,
                            diagnostics.adapter_max_texture_array_layers,
                            diagnostics.negotiated_max_texture_dimension_2d,
                            diagnostics.negotiated_max_texture_array_layers,
                        ));
                        if adapter.device_type == wgpu::DeviceType::Cpu {
                            ui.colored_label(egui::Color32::YELLOW, "Software rendering: CPU adapter selected.");
                        }
                    } else {
                        ui.label("Graphics device not initialized.");
                    }
                    let refresh_hz = if diagnostics.monitor_refresh_hz > 0.0 {
                        diagnostics.monitor_refresh_hz
                    } else {
                        60.0
                    };
                    let scale_pct = if diagnostics.active_render_scale > 0.0 {
                        (diagnostics.active_render_scale * 100.0).round() as u32
                    } else {
                        (state.viewport_render_scale.clamp(0.5, 1.0) * 100.0).round() as u32
                    };
                    ui.small(format!(
                        "Display/Presentation: PresentMode {:?} | Latency {} | Monitor {:.1} Hz | 3D Scale {}% ({}x{})",
                        diagnostics.active_present_mode,
                        diagnostics.active_max_frame_latency.max(1),
                        refresh_hz,
                        scale_pct,
                        diagnostics.active_render_resolution.0,
                        diagnostics.active_render_resolution.1,
                    ));
                    if let Some((p50, p95, p99, win_max)) = diagnostics.frame_summary_ms() {
                        ui.label(format!(
                            "Frame wall time: p50 {p50:.2} ms | p95 {p95:.2} ms | p99 {p99:.2} ms | max {win_max:.2} ms"
                        ));
                        ui.small(format!(
                            "Last 120 frames (includes acquire/present). Session max {:.2} ms | >50 ms: {} | >100 ms: {}",
                            diagnostics.max_frame_ms, diagnostics.frames_over_50ms, diagnostics.frames_over_100ms
                        ));
                    }
                    ui.label(format!(
                        "CPU wall (ms): Acquire {:.2} | UI {:.2} | World {:.2} | Overlay {:.2} | Present {:.2}",
                        diagnostics.cpu_acquire_ms,
                        diagnostics.cpu_ui_ms,
                        diagnostics.cpu_world_ms,
                        diagnostics.cpu_overlay_ms,
                        diagnostics.cpu_present_ms,
                    ));
                    let rs = &diagnostics.last_render_stats;
                    ui.small(format!(
                        "World breakdown (ms): sig {:.2} | csg {:.2} | wire {:.2} | solid {:.2} | ovl {:.2} | up {:.2} | sub {:.2}",
                        rs.signature_ms, rs.csg_ms, rs.wireframe_ms, rs.solids_ms,
                        rs.overlays_ms, rs.upload_ms, rs.encode_submit_ms
                    ));
                    if diagnostics.cpu_autosave_ms > 0.0
                        || diagnostics.bg_autosave_serialize_ms > 0.0
                        || diagnostics.bg_autosave_io_ms > 0.0
                    {
                        ui.small(format!(
                            "Autosave (ms): UI snapshot {:.2} | bg serialize {:.2} | bg write {:.2}",
                            diagnostics.cpu_autosave_ms,
                            diagnostics.bg_autosave_serialize_ms,
                            diagnostics.bg_autosave_io_ms,
                        ));
                    }
                    ui.label(format!(
                        "Frame uploads: {:.2} KiB lines ({} v) | {:.2} KiB solids ({} v) | {} draw call(s)",
                        rs.uploaded_line_bytes as f64 / 1024.0,
                        rs.line_vertex_count,
                        rs.uploaded_solid_bytes as f64 / 1024.0,
                        rs.solid_vertex_count,
                        rs.draw_calls,
                    ));
                    ui.label(format!(
                        "CSG rebuilds: {} (in flight: {} | last: {})",
                        diagnostics.csg_rebuilds_total,
                        rs.csg_in_flight,
                        if diagnostics.last_csg_rebuild_reason.is_empty() { "none" } else { diagnostics.last_csg_rebuild_reason }
                    ));
                    if let Some((count, p50, p95)) = diagnostics.gpu_viewport_timing {
                        ui.label(format!("GPU 3D pass: p50 {p50:.3} ms | p95 {p95:.3} ms ({count} samples)"));
                        ui.small("One sample per 15 rendered frames. Excludes UI, uploads and presentation.");
                    } else if diagnostics.gpu_timing_supported {
                        ui.small("GPU timestamp queries enabled; waiting for sampled readback.");
                    } else {
                        ui.small("GPU timestamps unavailable; CPU measurements remain active.");
                    }
                    let retained_cap_mib = if diagnostics.retained_vertex_budget_bytes > 0 {
                        diagnostics.retained_vertex_budget_bytes as f64 / 1_048_576.0
                    } else {
                        32.0
                    };
                    ui.label(format!(
                        "CPU frame vertex storage: {:.2} MiB / {:.0} MiB retained limit",
                        diagnostics.cpu_vertex_storage_bytes as f64 / 1_048_576.0,
                        retained_cap_mib,
                    ));
                    ui.label(format!("CPU grid cache: {:.2} MiB / 3 MiB limit",
                        diagnostics.cpu_grid_cache_bytes as f64 / 1_048_576.0));
                    ui.label(format!("GPU vertex buffers: {:.2} MiB",
                        diagnostics.gpu_vertex_buffer_bytes as f64 / 1_048_576.0));
                    let diffuse_cap = if diagnostics.diffuse_texture_layer_cap > 0 {
                        diagnostics.diffuse_texture_layer_cap.saturating_sub(1)
                    } else {
                        crate::renderer::pipeline::MAX_DIFFUSE_TEXTURE_LAYERS - 1
                    };
                    ui.label(format!(
                        "Diffuse texture array: {} / {} layers ({} fallback surfaces | {:.2} MiB)",
                        rs.texture_layers_used,
                        diffuse_cap,
                        rs.texture_fallback_surfaces,
                        diagnostics.diffuse_texture_array_bytes as f64 / 1_048_576.0,
                    ));
                    let lm_dim = if diagnostics.lightmap_atlas_dim > 0 {
                        diagnostics.lightmap_atlas_dim
                    } else {
                        crate::renderer::lightmap_atlas::LIGHTMAP_ATLAS_DIM
                    };
                    ui.label(format!(
                        "Lightmap atlas: {} tiles ({}x{} 2D | {:.2} MiB)",
                        diagnostics.lightmap_layers_used,
                        lm_dim,
                        lm_dim,
                        diagnostics.lightmap_atlas_bytes as f64 / 1_048_576.0,
                    ));
                    if diagnostics.lightmap_fallback_surfaces > 0 {
                        ui.colored_label(egui::Color32::YELLOW, format!(
                            "{} surfaces use vertex-lighting fallback: lightmap atlas is full.",
                            diagnostics.lightmap_fallback_surfaces));
                    }
                    let tex_mgr_budget_mib = if diagnostics.texture_manager_budget_bytes > 0 {
                        diagnostics.texture_manager_budget_bytes as f64 / 1_048_576.0
                    } else {
                        state.texture_manager.memory_budget_bytes() as f64 / 1_048_576.0
                    };
                    ui.label(format!(
                        "TextureManager LRU cache: {:.2} / {:.0} MiB",
                        diagnostics.texture_manager_used_bytes as f64 / 1_048_576.0,
                        tex_mgr_budget_mib,
                    ));
                    ui.small("Vertex storage only; excludes depth buffers and temporary staging.");
                    let backup_cap_mib = if diagnostics.gui_texture_backup_soft_cap_bytes > 0 {
                        diagnostics.gui_texture_backup_soft_cap_bytes as f64 / 1_048_576.0
                    } else {
                        64.0
                    };
                    ui.label(format!(
                        "UI texture recovery images: {:.2} / {:.0} MiB soft cap",
                        diagnostics.gui_texture_backup_bytes as f64 / 1_048_576.0,
                        backup_cap_mib,
                    ));
                    if diagnostics.device_losses > 0 {
                        ui.label(format!("Device losses: {} | Recreations: {}",
                            diagnostics.device_losses, diagnostics.device_recoveries));
                        if let Some(error) = &diagnostics.last_device_error {
                            ui.small(format!("Last device error: {error}"));
                        }
                    }
                    if diagnostics.surface_failures > 0 {
                        ui.label(format!("Surface errors: {} | Recoveries: {}",
                            diagnostics.surface_failures, diagnostics.surface_recoveries));
                        if let Some(error) = &diagnostics.last_surface_error {
                            ui.small(format!("Last acquisition error: {error}"));
                        }
                    }
                    for failure in &diagnostics.failed_attempts {
                        ui.small(format!("Previous adapter attempt: {failure}"));
                    }
                    if ui.button("📋 Copy GPU Diagnostics").clicked() {
                        copy_report = Some(diagnostics.format_diagnostics_report());
                    }
                }
                if let Some(report) = copy_report {
                    ui.ctx().copy_text(report);
                    state.log("Copied GPU & frame diagnostics report to clipboard.");
                }
            });
            ui.separator();
            let stats = state.get_map_budget_stats();

            ui.horizontal(|ui| {
                ui.heading("Unreal Engine 1 Architectural Telemetry");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("📋 Copy Summary").clicked() {
                        let text = format!(
                            "=== UT99 MAP BUDGET SUMMARY ===\nTotal Brushes: {}\nEstimated BSP Nodes: {} / 32,767 ({:.1}%)\nTotal Polygons: {}\nTotal Actors: {} / 4,096 ({:.1}%)\nTotal Movers: {}\nTotal ReachSpecs: {} / 3,072 ({:.1}%)\nTotal Zones: {} / 64 ({:.1}%)\n",
                            stats.total_brushes,
                            stats.estimated_bsp_nodes, stats.node_budget_ratio * 100.0,
                            stats.total_polygons,
                            stats.total_actors, stats.actor_budget_ratio * 100.0,
                            stats.total_movers,
                            stats.total_reachspecs, stats.reachspec_budget_ratio * 100.0,
                            stats.total_zones, stats.zone_budget_ratio * 100.0,
                        );
                        ui.ctx().copy_text(text);
                        state.log("Copied Map Budget Summary to clipboard.");
                    }
                });
            });
            ui.separator();

            // Overall Engine Health Banner
            let max_ratio = stats.node_budget_ratio
                .max(stats.brush_budget_ratio)
                .max(stats.actor_budget_ratio)
                .max(stats.reachspec_budget_ratio)
                .max(stats.zone_budget_ratio);

            if max_ratio < 0.50 {
                ui.colored_label(
                    egui::Color32::from_rgb(100, 230, 120),
                    "🟢 Optimal Tournament Performance: All metrics well within UE1 software & hardware budgets.",
                );
            } else if max_ratio < 0.80 {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 200, 60),
                    "🟡 Moderate Complexity: Level is approaching engine recommended limits. Test on vintage systems.",
                );
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 80, 80),
                    "🔴 High Engine Pressure: Approaching 32K nodes or 4K actors limit. Risk of BSP overflow in v436.",
                );
            }
            ui.add_space(8.0);

            // Engine Budget Gauges
            ui.group(|ui| {
                ui.label(egui::RichText::new("Classic Engine Maximums & Limits").strong());
                ui.add_space(4.0);

                // 1. BSP Nodes
                ui.horizontal(|ui| {
                    ui.label(format!("BSP Nodes (Est.): {} / 32,767", stats.estimated_bsp_nodes));
                    let color = if stats.node_budget_ratio > 0.8 {
                        egui::Color32::from_rgb(255, 100, 100)
                    } else if stats.node_budget_ratio > 0.5 {
                        egui::Color32::from_rgb(255, 200, 80)
                    } else {
                        egui::Color32::from_rgb(100, 220, 120)
                    };
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new(format!("{:.1}%", stats.node_budget_ratio * 100.0)).color(color).strong());
                    });
                });
                ui.add(egui::ProgressBar::new(stats.node_budget_ratio).show_percentage());

                // 2. World Brushes
                ui.horizontal(|ui| {
                    ui.label(format!("World Brushes: {} / 4,096", stats.total_brushes));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:.1}%", stats.brush_budget_ratio * 100.0));
                    });
                });
                ui.add(egui::ProgressBar::new(stats.brush_budget_ratio).show_percentage());

                // 3. Actors
                ui.horizontal(|ui| {
                    ui.label(format!("Total Actors: {} / 4,096", stats.total_actors));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:.1}%", stats.actor_budget_ratio * 100.0));
                    });
                });
                ui.add(egui::ProgressBar::new(stats.actor_budget_ratio).show_percentage());

                // 4. AI Reachability Links
                ui.horizontal(|ui| {
                    ui.label(format!("Reachability Specs: {} / 3,072", stats.total_reachspecs));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:.1}%", stats.reachspec_budget_ratio * 100.0));
                    });
                });
                ui.add(egui::ProgressBar::new(stats.reachspec_budget_ratio).show_percentage());

                // 5. Level Zones
                ui.horizontal(|ui| {
                    ui.label(format!("Zones (ZoneInfo): {} / 64", stats.total_zones));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("{:.1}%", stats.zone_budget_ratio * 100.0));
                    });
                });
                ui.add(egui::ProgressBar::new(stats.zone_budget_ratio).show_percentage());
            });

            ui.add_space(8.0);
            ui.columns(2, |cols| {
                // Column 1: Geometry Breakdown
                cols[0].group(|ui| {
                    ui.label(egui::RichText::new("Geometry & CSG Topology").strong());
                    ui.separator();
                    ui.label(format!("• Additive Brushes: {}", stats.additive_brushes));
                    ui.label(format!("• Subtractive Brushes: {}", stats.subtractive_brushes));
                    ui.label(format!("• Semisolid Brushes: {}", stats.semisolid_brushes));
                    ui.label(format!("• Non-Solid Brushes: {}", stats.nonsolid_brushes));
                    ui.label(format!("• Total Source Polygons: {}", stats.total_polygons));
                    ui.label(format!("• Static Meshes: {}", stats.total_static_meshes));
                    ui.label(format!("• Projector Decals: {}", stats.total_projectors));
                });

                // Column 2: Actors & Gameplay Entities
                cols[1].group(|ui| {
                    ui.label(egui::RichText::new("Entities & Gameplay").strong());
                    ui.separator();
                    ui.label(format!("• Dynamic & Static Lights: {}", stats.light_actors));
                    ui.label(format!("• Navigation & Spawns: {}", stats.nav_actors));
                    ui.label(format!("• Weapons & Pickups: {}", stats.pickup_actors));
                    ui.label(format!("• Triggers & Logic: {}", stats.trigger_actors));
                    ui.label(format!("• Dynamic Movers: {}", stats.total_movers));
                    ui.label(format!("• Miscellaneous Actors: {}", stats.other_actors));
                });
            });

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_dialog = true;
                }
            });
        });

    if close_dialog {
        open = false;
    }
    state.show_map_budget_dialog = open;
}
