use crate::editor_state::EditorState;
use crate::unrealscript::UcViewerTab;
use egui::{Color32, RichText, Stroke};

pub fn render_uc_viewer_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_uc_viewer_dialog {
        return;
    }

    let mut open = state.show_uc_viewer_dialog;
    let mut close_requested = false;
    let mut copy_script_requested = false;
    let mut export_requested = false;
    let mut compile_requested = false;
    let mut propagate_requested = false;
    let mut navigate_parent: Option<String> = None;
    let mut property_edit: Option<(String, String)> = None;

    if ctx.input(|i| i.key_pressed(egui::Key::F7)) {
        compile_requested = true;
    }

    let class_name = state.uc_viewer_class.clone();
    let meta_opt = state.uc_class_metadata.clone();
    let parent_name = meta_opt.as_ref().map(|m| m.super_class.clone()).unwrap_or_else(|| "Object".to_string());
    let pkg_name = meta_opt.as_ref().map(|m| m.package.clone()).unwrap_or_else(|| "Engine".to_string());

    // Count how many actors of this class exist in the current level
    let placed_count = state.loaded_unr.as_ref().map(|unr| {
        unr.actors.iter().filter(|a| a.class_name.eq_ignore_ascii_case(&class_name)).count()
    }).unwrap_or(0);

    egui::Window::new(format!("UnrealScript Studio: {}", class_name))
        .open(&mut open)
        .default_size([720.0, 680.0])
        .resizable(true)
        .show(ctx, |ui| {
            // Header Bar
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("class {}", class_name)).strong().size(16.0).color(Color32::from_rgb(100, 220, 255)));

                if !parent_name.is_empty() && parent_name != "None" {
                    ui.label(RichText::new("extends").weak().size(13.0));
                    if ui.link(RichText::new(&parent_name).strong().color(Color32::from_rgb(255, 215, 80))).on_hover_text("Navigate to parent class source").clicked() {
                        navigate_parent = Some(parent_name.clone());
                    }
                }

                ui.label(format!("(Package: {})", pkg_name));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ Close").clicked() {
                        close_requested = true;
                    }
                    if ui.button(RichText::new("⚙ Compile / Validate (F7)").strong().color(Color32::from_rgb(120, 240, 150)))
                        .on_hover_text("Validate UnrealScript syntax and invoke ucc make if UT99 installation is configured (F7)")
                        .clicked()
                    {
                        compile_requested = true;
                    }
                    if ui.button("⤓ Export .uc").on_hover_text("Export UnrealScript .uc source file to disk").clicked() {
                        export_requested = true;
                    }
                    if ui.button("⎘ Copy Script").on_hover_text("Copy full decompiled script to system clipboard").clicked() {
                        copy_script_requested = true;
                    }
                });
            });

            ui.add_space(4.0);

            // Level Context & Hot-Patching Status Banner
            egui::Frame::group(ui.style())
                .stroke(Stroke::new(1.0f32, Color32::from_rgb(60, 100, 160)))
                .fill(Color32::from_rgba_premultiplied(15, 25, 40, 220))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if placed_count > 0 {
                            ui.label(RichText::new(format!("📍 {} placed actor(s) of class '{}' in current map", placed_count, class_name)).strong().color(Color32::from_rgb(100, 240, 130)));
                            ui.separator();
                            if ui.button(RichText::new(format!("⚡ Propagate to Placed Actors ({})", placed_count)).strong().color(Color32::from_rgb(255, 230, 80)))
                                .on_hover_text("Hot-patches default properties across all instanced actors in the current level with Undo/Redo support")
                                .clicked()
                            {
                                propagate_requested = true;
                            }
                        } else {
                            ui.label(RichText::new("ℹ No placed actors of this class in current map (Class definition and defaults ready for placement)").weak());
                        }
                    });
                });

            ui.add_space(6.0);

            // Tab Selector
            ui.horizontal(|ui| {
                if ui.selectable_label(state.uc_viewer_tab == UcViewerTab::Source, "📝 UC Source Code Editor").clicked() {
                    state.uc_viewer_tab = UcViewerTab::Source;
                }
                if ui.selectable_label(state.uc_viewer_tab == UcViewerTab::DefaultProperties, "⚙ DefaultProperties Live Inspector").clicked() {
                    state.uc_viewer_tab = UcViewerTab::DefaultProperties;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label("🔍");
                    ui.text_edit_singleline(&mut state.uc_viewer_search);
                });
            });

            ui.separator();

            let filter = state.uc_viewer_search.trim().to_lowercase();

            // Tab 1: Interactive Source Code Editor + Highlighted Line Diagnostics
            if state.uc_viewer_tab == UcViewerTab::Source {
                egui::ScrollArea::both()
                    .max_height(520.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut state.uc_source_cache)
                                .code_editor()
                                .desired_rows(26)
                                .desired_width(f32::INFINITY),
                        );
                    });
            }
            // Tab 2: DefaultProperties Live Inspector & Editor
            else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Class Default Properties").strong().size(14.0));
                    ui.label(RichText::new("— Edit values live. Click 'Propagate' to apply instantly to all level actors.").weak().size(11.0));
                });
                ui.add_space(4.0);

                egui::ScrollArea::vertical()
                    .max_height(480.0)
                    .show(ui, |ui| {
                        egui::Grid::new("default_properties_grid")
                            .striped(true)
                            .num_columns(4)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.label(RichText::new("Property Name").strong());
                                ui.label(RichText::new("Current Value").strong());
                                ui.label(RichText::new("Default Value").strong());
                                ui.label(RichText::new("Action").strong());
                                ui.end_row();

                                let mut edits = Vec::new();

                                for (_idx, (prop_name, val)) in state.uc_editable_defaults.iter().enumerate() {
                                    if !filter.is_empty() && !prop_name.to_lowercase().contains(&filter) && !val.to_lowercase().contains(&filter) {
                                        continue;
                                    }

                                    // Property Name
                                    ui.label(RichText::new(prop_name).strong().color(Color32::from_rgb(100, 220, 255)));

                                    // Interactive Value Editor
                                    let mut temp_val = val.clone();
                                    if ui.add(egui::TextEdit::singleline(&mut temp_val).desired_width(220.0)).changed() {
                                        edits.push((prop_name.clone(), temp_val));
                                    }

                                    // Canonical baseline
                                    let baseline = meta_opt.as_ref()
                                        .and_then(|m| m.default_properties.iter().find(|(k, _)| k.eq_ignore_ascii_case(prop_name)))
                                        .map(|(_, v)| v.as_str())
                                        .unwrap_or(val.as_str());

                                    let is_modified = val != baseline;
                                    let base_label = if is_modified {
                                        RichText::new(baseline).color(Color32::from_rgb(255, 180, 80))
                                    } else {
                                        RichText::new(baseline).weak()
                                    };
                                    ui.label(base_label);

                                    // Action (Reset)
                                    ui.horizontal(|ui| {
                                        if is_modified && ui.small_button("↺ Reset").clicked() {
                                            edits.push((prop_name.clone(), baseline.to_string()));
                                        }
                                    });

                                    ui.end_row();
                                }

                                for (k, v) in edits {
                                    property_edit = Some((k, v));
                                }
                            });
                    });
            }
        });

    if close_requested {
        state.show_uc_viewer_dialog = false;
    }
    if copy_script_requested {
        ctx.copy_text(state.uc_source_cache.clone());
        state.log("Copied UnrealScript source code to clipboard.");
        state.toast_info("UnrealScript source copied to clipboard");
    }
    if compile_requested {
        let _ = state.compile_active_uc_script();
    }
    if export_requested {
        let export_dir = std::path::Path::new("UnrealTournament/Classes");
        match state.export_active_uc_file(export_dir) {
            Ok(path) => {
                state.log(format!("Exported UnrealScript file to: {:?}", path));
                state.toast_success("Exported .uc file successfully");
            }
            Err(e) => {
                state.log(format!("Failed to export .uc file: {}", e));
                state.toast_error(format!("Failed to export .uc: {}", e));
            }
        }
    }
    if propagate_requested {
        let count = state.propagate_default_properties_to_actors(&class_name);
        state.log(format!("Live hot-patch complete: updated {} actor(s) of class '{}'", count, class_name));
        state.toast_success(format!("Hot-patched {} actor(s) of class '{}'", count, class_name));
    }
    if let Some(parent) = navigate_parent {
        state.open_uc_viewer_for_class(&parent);
    }
    if let Some((k, v)) = property_edit {
        state.update_class_default_property(&k, &v);
    }
}
