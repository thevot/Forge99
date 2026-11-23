use crate::editor_state::EditorState;
use egui::{Color32, RichText, ScrollArea};
use std::fs;
use ut99_bsp::unrealscript_exec::{generate_scaffold_class, ExecDirective, UcArchetype};

pub fn show_uc_class_wizard_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_uc_class_wizard_dialog {
        return;
    }

    let mut is_open = state.show_uc_class_wizard_dialog;

    egui::Window::new("📜 UnrealScript Class Wizard & #exec Studio")
        .open(&mut is_open)
        .default_width(820.0)
        .default_height(640.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("UnrealScript Class Scaffolder & #exec Studio");
                ui.label(
                    RichText::new("UCC Make Compatible Code Generator")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 220, 255)),
                );
            });
            ui.label(
                RichText::new(
                    "Design and generate UnrealScript classes (Mutators, Weapons, Projectiles, Pickups) \
                     with embedded vertex mesh sequences, audio imports, and defaultproperties.",
                )
                .size(11.0)
                .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                // Section 1: Archetype & Class Identity
                ui.group(|ui| {
                    ui.label(RichText::new("1. Class Identity & Archetype").strong());
                    egui::Grid::new("uc_scaffold_identity_grid")
                        .num_columns(2)
                        .spacing([12.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Archetype:");
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("uc_archetype_combo")
                                    .selected_text(state.uc_scaffold_config.archetype.display_name())
                                    .show_ui(ui, |ui| {
                                        for arch in &[
                                            UcArchetype::Mutator,
                                            UcArchetype::TournamentWeapon,
                                            UcArchetype::Projectile,
                                            UcArchetype::Pickup,
                                            UcArchetype::Custom,
                                        ] {
                                            if ui
                                                .selectable_label(
                                                    state.uc_scaffold_config.archetype == *arch,
                                                    arch.display_name(),
                                                )
                                                .clicked()
                                            {
                                                state.uc_scaffold_config.archetype = *arch;
                                                state.uc_scaffold_config.super_class =
                                                    arch.default_super_class().to_string();
                                            }
                                        }
                                    });
                            });
                            ui.end_row();

                            ui.label("Package Name:");
                            ui.text_edit_singleline(&mut state.uc_scaffold_config.package_name);
                            ui.end_row();

                            ui.label("Class Name:");
                            ui.text_edit_singleline(&mut state.uc_scaffold_config.class_name);
                            ui.end_row();

                            ui.label("Extends / Super Class:");
                            ui.text_edit_singleline(&mut state.uc_scaffold_config.super_class);
                            ui.end_row();

                            ui.label("Author:");
                            ui.text_edit_singleline(&mut state.uc_scaffold_config.author);
                            ui.end_row();

                            ui.label("Description:");
                            ui.text_edit_singleline(&mut state.uc_scaffold_config.description);
                            ui.end_row();
                        });
                });

                ui.add_space(6.0);

                // Section 2: Archetype-Specific Mechanics
                match state.uc_scaffold_config.archetype {
                    UcArchetype::Mutator => {
                        ui.group(|ui| {
                            ui.label(RichText::new("2. Mutator Rules & Hooks").strong());
                            ui.checkbox(
                                &mut state.uc_scaffold_config.mutator_override_damage,
                                "Implement MutatorTakeDamage hook (Damage modifier)",
                            );
                            ui.checkbox(
                                &mut state.uc_scaffold_config.mutator_check_replacement,
                                "Implement CheckReplacement hook (Actor / Weapon replacer)",
                            );
                        });
                    }
                    UcArchetype::TournamentWeapon => {
                        ui.group(|ui| {
                            ui.label(RichText::new("2. Weapon Fire & Ammo Configuration").strong());
                            egui::Grid::new("uc_weapon_params_grid")
                                .num_columns(4)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Primary Damage:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.weapon_primary_damage,
                                    ));
                                    ui.label("Alt-Fire Damage:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.weapon_alt_damage,
                                    ));
                                    ui.end_row();

                                    ui.label("Fire Rate:");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut state.uc_scaffold_config.weapon_fire_rate,
                                        )
                                        .speed(0.1),
                                    );
                                    ui.label("Alt Fire Rate:");
                                    ui.add(
                                        egui::DragValue::new(
                                            &mut state.uc_scaffold_config.weapon_alt_fire_rate,
                                        )
                                        .speed(0.1),
                                    );
                                    ui.end_row();

                                    ui.label("Ammo Class Name:");
                                    ui.text_edit_singleline(
                                        &mut state.uc_scaffold_config.weapon_ammo_name,
                                    );
                                    ui.label("Pickup Ammo Count:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.weapon_pickup_ammo,
                                    ));
                                    ui.end_row();
                                });
                        });
                    }
                    UcArchetype::Projectile => {
                        ui.group(|ui| {
                            ui.label(RichText::new("2. Projectile Ballistics & Explosion").strong());
                            egui::Grid::new("uc_proj_params_grid")
                                .num_columns(4)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Speed:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.proj_speed,
                                    ));
                                    ui.label("Max Speed:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.proj_max_speed,
                                    ));
                                    ui.end_row();

                                    ui.label("Damage:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.proj_damage,
                                    ));
                                    ui.label("Damage Radius:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.proj_damage_radius,
                                    ));
                                    ui.end_row();

                                    ui.label("Momentum Transfer:");
                                    ui.add(egui::DragValue::new(
                                        &mut state.uc_scaffold_config.proj_momentum,
                                    ));
                                    ui.label("");
                                    ui.label("");
                                    ui.end_row();
                                });
                        });
                    }
                    _ => {}
                }

                ui.add_space(6.0);

                // Section 3: Interactive #exec Directives
                ui.group(|ui| {
                    ui.label(RichText::new("3. Embedded #exec Asset Directives").strong());
                    ui.horizontal(|ui| {
                        if ui.button("+ Add Mesh Sequence").clicked() {
                            state.uc_scaffold_config.exec_directives.push(
                                ExecDirective::MeshSequence {
                                    mesh: state.uc_scaffold_config.class_name.clone(),
                                    seq: "Fire".to_string(),
                                    start_frame: 0,
                                    num_frames: 10,
                                    rate: 24.0,
                                },
                            );
                        }
                        if ui.button("+ Add Audio Import").clicked() {
                            state.uc_scaffold_config.exec_directives.push(
                                ExecDirective::AudioImport {
                                    file: "Sounds\\Fire.wav".to_string(),
                                    name: "FireSound".to_string(),
                                    group: "Weapons".to_string(),
                                },
                            );
                        }
                        if ui.button("+ Add Texture Import").clicked() {
                            state.uc_scaffold_config.exec_directives.push(
                                ExecDirective::TextureImport {
                                    name: "Skin".to_string(),
                                    file: "Textures\\Skin.pcx".to_string(),
                                    group: "Skins".to_string(),
                                    flags: 2,
                                    mips: true,
                                },
                            );
                        }
                        if ui.button("Clear All Directives").clicked() {
                            state.uc_scaffold_config.exec_directives.clear();
                        }
                    });

                    if state.uc_scaffold_config.exec_directives.is_empty() {
                        ui.label(
                            RichText::new("No #exec directives added yet. Click above to embed animations or sounds.")
                                .italics()
                                .color(Color32::from_rgb(150, 150, 150)),
                        );
                    } else {
                        let mut to_remove = None;
                        for (i, dir) in state
                            .uc_scaffold_config
                            .exec_directives
                            .iter_mut()
                            .enumerate()
                        {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("[#{}]", i + 1)).weak());
                                ui.monospace(dir.to_directive_string());
                                if ui.small_button("✕").on_hover_text("Remove directive").clicked()
                                {
                                    to_remove = Some(i);
                                }
                            });
                        }
                        if let Some(idx) = to_remove {
                            state.uc_scaffold_config.exec_directives.remove(idx);
                        }
                    }
                });

                ui.add_space(8.0);

                // Section 4: Live Code Generation Preview
                let generated_code = generate_scaffold_class(&state.uc_scaffold_config);

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("4. Generated UnrealScript Code (.uc)").strong());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("📋 Copy to Clipboard").clicked() {
                                ctx.copy_text(generated_code.clone());
                                state.uc_scaffold_status = "Copied UnrealScript to clipboard!".to_string();
                            }
                            if ui.button("💾 Export .uc File...").clicked() {
                                export_uc_file(state, &generated_code);
                            }
                        });
                    });

                    ui.add(
                        egui::TextEdit::multiline(&mut generated_code.as_str())
                            .font(egui::TextStyle::Monospace)
                            .desired_rows(14)
                            .desired_width(ui.available_width()),
                    );

                    if !state.uc_scaffold_status.is_empty() {
                        ui.label(
                            RichText::new(&state.uc_scaffold_status)
                                .size(11.0)
                                .color(Color32::from_rgb(120, 240, 150)),
                        );
                    }
                });
            });
        });

    state.show_uc_class_wizard_dialog = is_open;
}

fn export_uc_file(state: &mut EditorState, code: &str) {
    let filename = format!("{}.uc", state.uc_scaffold_config.class_name);
    if let Some(path) = rfd::FileDialog::new()
        .set_file_name(&filename)
        .add_filter("UnrealScript Source", &["uc"])
        .save_file()
    {
        if let Err(e) = fs::write(&path, code) {
            state.uc_scaffold_status = format!("Export failed: {e}");
        } else {
            state.uc_scaffold_status = format!("Exported successfully to {}", path.display());
        }
    }
}
