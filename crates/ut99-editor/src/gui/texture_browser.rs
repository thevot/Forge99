use crate::editor_state::EditorState;

pub fn render_texture_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_texture_browser {
        return;
    }

    let mut is_open = state.show_texture_browser;
    let mut close_requested = false;
    let available_pkgs = state.texture_manager.available_packages().to_vec();

    crate::gui::modal::studio_dialog_window(ctx, "Texture Browser", &mut is_open)
        .show(ctx, |ui| {
            // 1. Top Controls Bar: 2-row structured layout for full responsiveness
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("Package:").strong());

                let mut current_pkg = state.selected_texture_package.clone();
                egui::ComboBox::from_id_salt("texture_pkg_combo")
                    .selected_text(&current_pkg)
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        for pkg in &available_pkgs {
                            if ui.selectable_value(&mut current_pkg, pkg.clone(), pkg).clicked() {
                                state.selected_texture_package = current_pkg.clone();
                                state.selected_texture_group = "All".to_string();
                            }
                        }
                    });

                if state.selected_texture_package != current_pkg {
                    state.selected_texture_package = current_pkg;
                    state.selected_texture_group = "All".to_string();
                }

                // Group Dropdown (UnrealEd 2.0 Parity)
                let pkg_name = state.selected_texture_package.clone();
                let available_groups = state.texture_manager.get_package_groups(&pkg_name);

                ui.separator();
                ui.label(egui::RichText::new("Group:").strong());
                let mut current_group = state.selected_texture_group.clone();
                egui::ComboBox::from_id_salt("texture_group_combo")
                    .selected_text(&current_group)
                    .width(120.0)
                    .show_ui(ui, |ui| {
                        for grp in &available_groups {
                            if ui.selectable_value(&mut current_group, grp.clone(), grp).clicked() {
                                state.selected_texture_group = current_group.clone();
                            }
                        }
                    });
                if state.selected_texture_group != current_group {
                    state.selected_texture_group = current_group;
                }

                let in_use_active = state.selected_texture_group.eq_ignore_ascii_case("In Use");
                if ui.selectable_label(in_use_active, "📌 In Use").on_hover_text("Show only textures currently used by brushes/movers in the level").clicked() {
                    state.selected_texture_group = if in_use_active { "All".to_string() } else { "In Use".to_string() };
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !state.texture_search_query.is_empty() {
                        if ui.button("✕").on_hover_text("Clear search filter").clicked() {
                            state.texture_search_query.clear();
                        }
                    }
                    ui.add(
                        egui::TextEdit::singleline(&mut state.texture_search_query)
                            .hint_text("Search textures...")
                            .desired_width(130.0),
                    );
                    ui.label(egui::RichText::new("Filter:").strong());
                });
            });

            ui.add_space(2.0);

            // Row 2: Action buttons & Package tools
            ui.horizontal(|ui| {
                if ui.button("📥 Import to MyLevel...").on_hover_text("Import image (.png, .bmp, .pcx, .tga) into embedded MyLevel package").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Texture Image (.png, .bmp, .pcx, .tga)", &["png", "bmp", "pcx", "tga", "jpg"])
                        .pick_file()
                    {
                        match state.texture_manager.import_image_file_to_package("MyLevel", "Embedded", &path, false) {
                            Ok(full_ref) => {
                                state.selected_texture_package = "MyLevel".to_string();
                                state.selected_texture_group = "All".to_string();
                                state.active_texture = Some(full_ref.clone());
                                state.log(format!("Imported embedded texture '{}' into MyLevel.", full_ref));
                                state.toast_success(format!("Imported {} into MyLevel", full_ref));
                            }
                            Err(e) => {
                                state.log(format!("Failed to import texture into MyLevel: {}", e));
                                state.toast_error(format!("MyLevel import failed: {}", e));
                            }
                        }
                    }
                }

                if ui.button("💾 Save .UTX").on_hover_text("Export current texture package to disk with full P8 mipmap chains (BL-P1-03)").clicked() {
                    let pkg_to_save = state.selected_texture_package.clone();
                    if let Some(out_path) = rfd::FileDialog::new()
                        .set_file_name(format!("{}.utx", pkg_to_save))
                        .add_filter("Unreal Texture Package (.utx)", &["utx"])
                        .save_file()
                    {
                        let _ = state.export_texture_package_utx(&pkg_to_save, &out_path);
                    }
                }

                if ui.button("🧹 Cull MyLevel").on_hover_text("Garbage-collect unreferenced MyLevel textures and sounds (TEXTURE CULL)").clicked() {
                    let _ = state.prune_mylevel_assets();
                }
            });

            ui.separator();

            // 2. Load the currently selected package
            let pkg_name = state.selected_texture_package.clone();
            let _ = state.texture_manager.load_package(&pkg_name);

            let search = state.texture_search_query.to_lowercase();
            let active_group = state.selected_texture_group.clone();
            let in_use_names: std::collections::HashSet<String> = if active_group.eq_ignore_ascii_case("In Use") {
                let references: std::collections::HashSet<&str> = state.world_brushes.iter()
                    .chain(state.movers.iter().map(|m| &m.brush))
                    .chain(state.static_meshes.iter())
                    .flat_map(|brush| brush.polys.iter().map(|poly| poly.texture.as_str()))
                    .collect();
                references.into_iter().filter_map(|reference| state.texture_manager.resolve_texture_reference(reference))
                    .map(|reference| reference.to_ascii_lowercase()).collect()
            } else { std::collections::HashSet::new() };
            let active_identity = state.active_texture.as_deref()
                .and_then(|reference| state.texture_manager.resolve_texture_reference(reference))
                .map(|reference| reference.to_ascii_lowercase());

            let textures_info: Vec<(String, String, u32, u32, u32)> = if let Some(pkg) = state.texture_manager.get_package(&pkg_name) {
                pkg.textures
                    .iter()
                    .filter(|t| {
                        let matches_search = search.is_empty() || t.name.to_lowercase().contains(&search);
                        let matches_group = if active_group == "All" {
                            true
                        } else if active_group.eq_ignore_ascii_case("In Use") {
                            let identity = crate::texture_manager::texture_reference(&pkg_name, &t.group, &t.name).to_ascii_lowercase();
                            in_use_names.contains(&identity)
                        } else {
                            t.group.eq_ignore_ascii_case(&active_group) || (t.group.is_empty() && active_group == "None")
                        };
                        matches_search && matches_group
                    })
                    .map(|t| (t.name.clone(), t.group.clone(), t.width, t.height, t.format))
                    .collect()
            } else {
                Vec::new()
            };

            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "Package '{}' (Group: '{}'): {} texture(s)",
                        pkg_name,
                        active_group,
                        textures_info.len()
                    ))
                    .color(egui::Color32::from_rgb(160, 200, 240)),
                );

                ui.separator();

                // UI-05: Zoom controls and thumbnail resizing
                ui.label(egui::RichText::new("Size:").size(11.0));
                if ui.selectable_label((state.texture_thumbnail_size - 64.0).abs() < 1.0, "S").on_hover_text("Small thumbnails (64px)").clicked() {
                    state.texture_thumbnail_size = 64.0;
                }
                if ui.selectable_label((state.texture_thumbnail_size - 110.0).abs() < 1.0, "M").on_hover_text("Medium thumbnails (110px)").clicked() {
                    state.texture_thumbnail_size = 110.0;
                }
                if ui.selectable_label((state.texture_thumbnail_size - 180.0).abs() < 1.0, "L").on_hover_text("Large thumbnails (180px)").clicked() {
                    state.texture_thumbnail_size = 180.0;
                }
                ui.add(
                    egui::Slider::new(&mut state.texture_thumbnail_size, 48.0..=256.0)
                        .show_value(false),
                );
                ui.label(egui::RichText::new(format!("{:.0}px", state.texture_thumbnail_size)).size(10.5).color(egui::Color32::LIGHT_GRAY));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⇲ Export Folder...").on_hover_text("Export all textures in this package to PNG files").clicked() {
                        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                            match state.texture_manager.export_package_to_directory(&pkg_name, &dir) {
                                Ok(count) => {
                                    state.log(format!("Exported {} texture(s) from package '{}' to '{}'", count, pkg_name, dir.display()));
                                }
                                Err(e) => {
                                    state.log(format!("Failed to export package: {e}"));
                                }
                            }
                        }
                    }
                });
            });

            ui.add_space(4.0);

            // 3. Grid of texture thumbnails (UI-05 dynamic sizing)
            let item_width = state.texture_thumbnail_size.clamp(48.0, 256.0);
            let item_height = (item_width * 0.88).round();
            let mut clicked_texture = None;
            let mut double_clicked_apply = false;
            let mut action_compress: Option<(String, String, u32)> = None;
            let mut action_apply_surfaces: Option<String> = None;
            let mut action_apply_builder: Option<String> = None;

            egui::ScrollArea::vertical().show(ui, |ui| {
                let avail_width = ui.available_width();
                let columns = (avail_width / (item_width + 8.0)).floor().max(1.0) as usize;

                egui::Grid::new("texture_grid")
                    .num_columns(columns)
                    .spacing([8.0, 10.0])
                    .show(ui, |ui| {
                        let mut col = 0;
                        for (tex_name, tex_group, width, height, format) in &textures_info {
                            let full_name = crate::texture_manager::texture_reference(&pkg_name, tex_group, tex_name);
                            let local_name = crate::texture_manager::local_texture_name(tex_group, tex_name);
                            let is_active = active_identity.as_deref() == Some(full_name.to_ascii_lowercase().as_str());

                            ui.vertical(|ui| {
                                let (rect, response) = ui.allocate_exact_size(
                                    egui::Vec2::new(item_width, item_height),
                                    egui::Sense::click_and_drag(),
                                );

                                if response.drag_started() {
                                    state.active_drag_drop = Some(crate::editor_state::DragDropPayload::Texture(full_name.clone()));
                                }

                                let is_visible = ui.is_rect_visible(rect);

                                // Draw thumbnail lazily (only if visible or hovered)
                                let handle_opt = if is_visible || response.hovered() {
                                    state.texture_manager.get_or_create_egui_texture(
                                        ctx,
                                        &pkg_name,
                                        &local_name,
                                    )
                                } else {
                                    None
                                };

                                if is_visible {
                                    if is_active {
                                        ui.painter().rect_stroke(
                                            rect.expand(2.0),
                                            2.0,
                                            egui::Stroke::new(2.5_f32, egui::Color32::from_rgb(255, 215, 0)),
                                            egui::StrokeKind::Outside,
                                        );
                                    } else if response.hovered() {
                                        ui.painter().rect_stroke(
                                            rect.expand(1.0),
                                            2.0,
                                            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(100, 180, 255)),
                                            egui::StrokeKind::Outside,
                                        );
                                    }

                                    if let Some(ref handle) = handle_opt {
                                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                                        ui.painter().image(
                                            handle.id(),
                                            rect,
                                            uv,
                                            egui::Color32::WHITE,
                                        );
                                    } else {
                                        ui.painter().rect_filled(
                                            rect,
                                            2.0,
                                            egui::Color32::from_rgb(40, 40, 45),
                                        );
                                    }
                                }

                                response.context_menu(|ui| {
                                    ui.label(egui::RichText::new(tex_name).strong());
                                    ui.separator();
                                    if ui.button("🎯 Select as Active Texture").clicked() {
                                        clicked_texture = Some(full_name.clone());
                                        ui.close_menu();
                                    }
                                    if ui.button("🖌 Apply to Selected Surfaces").clicked() {
                                        action_apply_surfaces = Some(full_name.clone());
                                        ui.close_menu();
                                    }
                                    if ui.button("🧱 Apply to Builder Brush").clicked() {
                                        action_apply_builder = Some(full_name.clone());
                                        ui.close_menu();
                                    }
                                    ui.separator();
                                    ui.label(egui::RichText::new("⚡ S3TC / Format Conversion").size(10.5).color(egui::Color32::LIGHT_GRAY));
                                    if ui.button("🎯 Compress to DXT1 (BC1 - 1-bit Alpha)").clicked() {
                                        action_compress = Some((pkg_name.clone(), local_name.clone(), 3));
                                        ui.close_menu();
                                    }
                                    if ui.button("💎 Compress to DXT5 (BC3 - Smooth Alpha)").clicked() {
                                        action_compress = Some((pkg_name.clone(), local_name.clone(), 8));
                                        ui.close_menu();
                                    }
                                    if ui.button("🎨 Convert to P8 (Paletted)").clicked() {
                                        action_compress = Some((pkg_name.clone(), local_name.clone(), 0));
                                        ui.close_menu();
                                    }
                                });

                                let response = response.on_hover_ui(|ui| {
                                    ui.set_max_width(240.0);
                                    ui.vertical(|ui| {
                                        ui.heading(tex_name);
                                        ui.label(
                                            egui::RichText::new(&full_name)
                                                .color(egui::Color32::from_rgb(160, 200, 240))
                                                .size(10.0),
                                        );
                                        if !tex_group.is_empty() && tex_group != "None" {
                                            ui.label(
                                                egui::RichText::new(format!("Group: {}", tex_group))
                                                    .color(egui::Color32::from_rgb(200, 220, 150))
                                                    .size(10.0),
                                            );
                                        }
                                        ui.separator();
                                        if let Some(ref handle) = handle_opt {
                                            let (img_rect, _) = ui.allocate_exact_size(
                                                egui::vec2(180.0, 180.0),
                                                egui::Sense::hover(),
                                            );
                                            ui.painter().image(
                                                handle.id(),
                                                img_rect,
                                                egui::Rect::from_min_max(
                                                    egui::pos2(0.0, 0.0),
                                                    egui::pos2(1.0, 1.0),
                                                ),
                                                egui::Color32::WHITE,
                                            );
                                            ui.add_space(4.0);
                                        }
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Resolution:").strong());
                                            ui.label(format!("{} × {} px", width, height));
                                        });
                                        let aspect_str = if *width == *height {
                                            "1:1 (Square)"
                                        } else if *width == 2 * *height {
                                            "2:1 (Horizontal)"
                                        } else if 2 * *width == *height {
                                            "1:2 (Vertical)"
                                        } else {
                                            "Non-square"
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Aspect:").strong());
                                            ui.label(aspect_str);
                                        });
                                        let kb = (*width * *height) as f32 / 1024.0;
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Est. Size:").strong());
                                            ui.label(format!("{:.1} KB", kb));
                                        });
                                        let fmt_label = match *format {
                                            3 => "DXT1 (BC1)",
                                            8 => "DXT5 (BC3)",
                                            7 => "DXT3 (BC2)",
                                            _ => "P8 (Paletted)",
                                        };
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Format:").strong());
                                            ui.label(fmt_label);
                                        });
                                        if is_active {
                                            ui.add_space(2.0);
                                            ui.label(
                                                egui::RichText::new("● Active Material")
                                                    .color(egui::Color32::from_rgb(255, 215, 0))
                                                    .strong(),
                                            );
                                        }
                                        ui.separator();
                                        ui.label(
                                            egui::RichText::new("• Click to select as active material\n• Double-click to apply immediately to selection\n• Right-click for compression & quick actions")
                                                .size(10.0)
                                                .color(egui::Color32::LIGHT_GRAY),
                                        );
                                    });
                                });

                                if response.double_clicked() {
                                    clicked_texture = Some(full_name.clone());
                                    double_clicked_apply = true;
                                } else if response.clicked() {
                                    clicked_texture = Some(full_name.clone());
                                }

                                // Texture Name & Resolution
                                ui.label(
                                    egui::RichText::new(tex_name)
                                        .size(11.0)
                                        .strong()
                                        .color(if is_active {
                                             egui::Color32::from_rgb(255, 230, 100)
                                        } else {
                                            egui::Color32::from_rgb(220, 220, 220)
                                        }),
                                );

                                let fmt_str = match *format {
                                    3 => "DXT1",
                                    8 => "DXT5",
                                    7 => "DXT3",
                                    _ => "P8",
                                };
                                let fmt_color = match *format {
                                    3 => egui::Color32::from_rgb(120, 220, 140),
                                    8 => egui::Color32::from_rgb(100, 200, 255),
                                    _ => egui::Color32::from_rgb(170, 170, 170),
                                };

                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{}x{}", width, height))
                                            .size(9.5)
                                            .color(egui::Color32::from_rgb(140, 140, 140)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("[{}]", fmt_str))
                                            .size(9.5)
                                            .strong()
                                            .color(fmt_color),
                                    );
                                });
                            });

                            col += 1;
                            if col >= columns {
                                ui.end_row();
                                col = 0;
                            }
                        }
                    });
            });

            if let Some(tex) = clicked_texture {
                state.active_texture = Some(tex.clone());
                state.log(format!("Selected active texture: {}", tex));
            }
            if double_clicked_apply {
                state.apply_active_texture_to_current_selection();
            }
            if let Some(tex) = action_apply_surfaces {
                state.active_texture = Some(tex);
                state.apply_active_texture_to_current_selection();
            }
            if let Some(tex) = action_apply_builder {
                state.active_texture = Some(tex);
                state.apply_active_texture_to_builder();
            }
            if let Some((p, t, target_fmt)) = action_compress {
                match state.texture_manager.convert_texture_format(&p, &t, target_fmt) {
                    Ok((orig, new_b, savings)) => {
                        let fmt_name = match target_fmt {
                            3 => "DXT1 (BC1)",
                            8 => "DXT5 (BC3)",
                            _ => "P8 (Paletted)",
                        };
                        state.log(format!("Converted texture '{}.{}' to {} (VRAM: {:.1} KB -> {:.1} KB, {:.1}% savings)", p, t, fmt_name, orig as f32 / 1024.0, new_b as f32 / 1024.0, savings));
                        state.toast_success(format!("Compressed {} to {} (🔻 {:.1}% VRAM savings)", t, fmt_name, savings));
                    }
                    Err(e) => {
                        state.toast_error(format!("Compression failed: {}", e));
                    }
                }
            }

            if let Some(ref active) = state.active_texture.clone() {
                egui::CollapsingHeader::new(format!("⚙ Texture Properties & S3TC ({})", active))
                    .default_open(true)
                    .show(ui, |ui| {
                        // S3TC Compression Card
                        if let Some(pos) = active.find('.') {
                            let p_part = &active[..pos];
                            let t_part = &active[pos + 1..];
                            if let Some((_, fmt_name, vram_bytes)) = state.texture_manager.get_texture_format_info(p_part, t_part) {
                                ui.group(|ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Format & Footprint:").strong());
                                        ui.label(egui::RichText::new(fmt_name).color(egui::Color32::from_rgb(140, 220, 160)).strong());
                                        ui.separator();
                                        let vram_kb = vram_bytes as f32 / 1024.0;
                                        ui.label(format!("VRAM: {:.1} KB", vram_kb));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Convert:").size(11.0));
                                        if ui.button("🎯 DXT1 (BC1)").on_hover_text("Compress to DXT1 with mipmaps (-87.5% VRAM footprint vs RGBA8, 50% vs P8)").clicked() {
                                            match state.texture_manager.convert_texture_format(p_part, t_part, 3) {
                                                Ok((orig, new_b, savings)) => {
                                                    state.toast_success(format!("Compressed {} to DXT1 (🔻 {:.1}% VRAM savings)", t_part, savings));
                                                    state.log(format!("Compressed '{active}' to DXT1: {:.1} KB -> {:.1} KB ({:.1}% savings)", orig as f32 / 1024.0, new_b as f32 / 1024.0, savings));
                                                }
                                                Err(e) => state.toast_error(format!("DXT1 compression failed: {e}")),
                                            }
                                        }
                                        if ui.button("💎 DXT5 (BC3)").on_hover_text("Compress to DXT5 with mipmaps (-75% VRAM footprint vs RGBA8, smooth alpha)").clicked() {
                                            match state.texture_manager.convert_texture_format(p_part, t_part, 8) {
                                                Ok((orig, new_b, savings)) => {
                                                    state.toast_success(format!("Compressed {} to DXT5 (🔻 {:.1}% VRAM savings)", t_part, savings));
                                                    state.log(format!("Compressed '{active}' to DXT5: {:.1} KB -> {:.1} KB ({:.1}% savings)", orig as f32 / 1024.0, new_b as f32 / 1024.0, savings));
                                                }
                                                Err(e) => state.toast_error(format!("DXT5 compression failed: {e}")),
                                            }
                                        }
                                        if ui.button("🎨 P8 (Paletted)").on_hover_text("Convert to 256-color paletted P8 texture with mipmaps").clicked() {
                                            match state.texture_manager.convert_texture_format(p_part, t_part, 0) {
                                                Ok((orig, new_b, _savings)) => {
                                                    state.toast_success(format!("Converted {} to P8 paletted", t_part));
                                                    state.log(format!("Converted '{active}' to P8: {:.1} KB -> {:.1} KB", orig as f32 / 1024.0, new_b as f32 / 1024.0));
                                                }
                                                Err(e) => state.toast_error(format!("P8 conversion failed: {e}")),
                                            }
                                        }
                                    });
                                });
                                ui.add_space(4.0);
                            }
                        }

                        let mut meta = state.texture_manager.get_texture_metadata(active);
                        let mut changed = false;
                        egui::Grid::new("ut99_texture_properties_grid")
                            .num_columns(4)
                            .spacing([8.0, 4.0])
                            .show(ui, |ui| {
                                ui.label("FootstepSound:");
                                changed |= ui.text_edit_singleline(&mut meta.footstep_sound).changed();
                                ui.label("HitSound:");
                                changed |= ui.text_edit_singleline(&mut meta.hit_sound).changed();
                                ui.end_row();

                                ui.label("DetailTexture:");
                                changed |= ui.text_edit_singleline(&mut meta.detail_texture).changed();
                                ui.label("MacroTexture:");
                                changed |= ui.text_edit_singleline(&mut meta.macro_texture).changed();
                                ui.end_row();

                                ui.label("AnimNext:");
                                changed |= ui.text_edit_singleline(&mut meta.anim_next).changed();
                                ui.label("Friction:");
                                changed |= ui.add(egui::DragValue::new(&mut meta.friction).speed(0.05).range(0.0..=10.0)).changed();
                                ui.end_row();

                                ui.label("MinFrameRate:");
                                changed |= ui.add(egui::DragValue::new(&mut meta.min_frame_rate).speed(0.5).range(0.0..=120.0)).changed();
                                ui.label("MaxFrameRate:");
                                changed |= ui.add(egui::DragValue::new(&mut meta.max_frame_rate).speed(0.5).range(0.0..=120.0)).changed();
                                ui.end_row();
                            });
                        if changed {
                            state.texture_manager.set_texture_metadata(active, meta);
                        }
                    });
            }

            ui.add_space(6.0);
            ui.separator();

            // 4. Bottom Action Bar
            ui.horizontal(|ui| {
                if let Some(ref active) = state.active_texture {
                    ui.label(
                        egui::RichText::new(format!("Active: {}", active))
                            .color(egui::Color32::from_rgb(255, 215, 0))
                            .strong(),
                    );
                } else {
                    ui.label("No active texture selected");
                }

                ui.separator();
                let mem_mb = state.texture_manager.total_memory_usage_bytes() as f64 / (1024.0 * 1024.0);
                let budget_mb = state.texture_manager.memory_budget_bytes() as f64 / (1024.0 * 1024.0);
                ui.label(
                    egui::RichText::new(format!("Cache: {:.1}/{:.0} MB", mem_mb, budget_mb))
                        .weak(),
                ).on_hover_text(format!(
                    "Texture cache: {:.2} MB in {} package(s), {:.2} MB in {} thumbnail(s). Budget: {:.0} MB",
                    state.texture_manager.packages_memory_usage_bytes() as f64 / (1024.0 * 1024.0),
                    state.texture_manager.loaded_package_count(),
                    state.texture_manager.thumbnails_memory_usage_bytes() as f64 / (1024.0 * 1024.0),
                    state.texture_manager.cached_thumbnail_count(),
                    budget_mb,
                ));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Close").clicked() {
                        close_requested = true;
                    }
                    if ui.button("Purge Cache").on_hover_text("Evict LRU thumbnails and unpinned packages to free memory").clicked() {
                        let (evicted, bytes) = state.texture_manager.prune_lru(0);
                        state.toast_info(format!("Freed {:.1} MB ({} items evicted)", bytes as f64 / (1024.0 * 1024.0), evicted));
                    }
                    if ui.button("Apply to Selected Brush").clicked() {
                        state.apply_active_texture_to_selected_brush();
                    }
                    if ui.button("Apply to Builder Brush").clicked() {
                        state.apply_active_texture_to_builder();
                    }
                    if ui.button("Export Active Texture...").on_hover_text("Export active texture to PNG file on disk").clicked() {
                        if let Some(ref active) = state.active_texture {
                            if let Some((p_name, local_name)) = active.split_once('.') {
                                let t_name = local_name.rsplit('.').next().unwrap_or(local_name);
                                if let Some(path) = rfd::FileDialog::new()
                                    .add_filter("Portable Network Graphics (*.png)", &["png"])
                                    .set_file_name(&format!("{t_name}.png"))
                                    .save_file()
                                {
                                    match state.texture_manager.export_texture_to_file(p_name, local_name, &path) {
                                        Ok(()) => state.log(format!("Exported texture '{active}' to '{}'", path.display())),
                                        Err(e) => state.log(format!("Failed to export texture: {e}")),
                                    }
                                }
                            }
                        }
                    }
                });
            });
        });

    if close_requested {
        is_open = false;
    }
    state.show_texture_browser = is_open;
}
