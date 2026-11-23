use crate::editor_state::EditorState;
use egui::{Color32, RichText};

pub fn render_music_browser(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_music_browser {
        return;
    }

    let mut open = state.show_music_browser;

    crate::gui::modal::compact_dialog_window(ctx, "Music Browser (.umx)", &mut open)
        .show(ctx, |ui| {
            ui.label(RichText::new("Unreal Tracker Music Modules (.umx)").size(11.0).color(Color32::from_rgb(180, 180, 180)));
            ui.separator();
            if state.music_packages.is_empty() { ui.label("No music packages found in the active UT installation."); }

            // Package Selector Bar
            let mut switch_pkg = None;
            ui.horizontal(|ui| {
                ui.label(RichText::new("Music Track:").strong());
                egui::ComboBox::from_id_salt("music_package_combo")
                    .selected_text(&state.selected_music_package)
                    .show_ui(ui, |ui| {
                        for pkg in &state.music_packages {
                            if ui.selectable_value(&mut state.selected_music_package, pkg.clone(), pkg).clicked() {
                                switch_pkg = Some(pkg.clone());
                            }
                        }
                    });

                if ui.button("🔄 Reload").clicked() {
                    state.scan_music_packages();
                }
            });

            if let Some(pkg) = switch_pkg {
                state.load_music_package(&pkg);
            }

            ui.add_space(10.0);
            ui.separator();

            // Loaded Music Details
            if let Some(ref music) = state.loaded_music.clone() {
                ui.label(RichText::new("Track Metadata").strong());
                ui.horizontal(|ui| {
                    ui.label("Package:");
                    ui.label(RichText::new(&music.package).strong());
                });
                ui.horizontal(|ui| {
                    ui.label("Song Title:");
                    ui.label(RichText::new(&music.title).color(Color32::from_rgb(100, 220, 255)).strong());
                });
                ui.horizontal(|ui| {
                    ui.label("Tracker Format:");
                    ui.label(RichText::new(&music.format).color(Color32::from_rgb(255, 200, 80)));
                });
                ui.horizontal(|ui| {
                    ui.label("File Size:");
                    let kb = (music.data_size as f32) / 1024.0;
                    ui.label(format!("{:.1} KB ({} bytes)", kb, music.data_size));
                });

                ui.add_space(12.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("▶ Play Preview").on_hover_text("Open the extracted tracker module in the system's external player").clicked() {
                        let result = (|| -> anyhow::Result<()> {
                            let bytes = state.music_module_bytes(&music.package)?;
                            let ext = module_extension(&music.format);
                            let path = std::env::temp_dir().join(format!("forge99_music_{}_{}.{}", std::process::id(), music.package, ext));
                            std::fs::write(&path, bytes)?;
                            open_external_player(&path)
                        })();
                        match result {
                            Ok(()) => {
                                let msg = format!("Preview request sent to the external player for '{}'. Playback controls are in that player.", music.package);
                                state.log(&msg);
                                state.toast_info(msg);
                            }
                            Err(error) => state.audio_error(format!("Music preview failed for '{}': {error:#}", music.package)),
                        }
                    }
                    if ui.button("📦 Save .umx").on_hover_text("Export this music to a destination you choose").clicked() {
                        if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.umx", music.package))
                            .add_filter("Unreal Music", &["umx"]).save_file() {
                            let result = state.music_module_bytes(&music.package)
                                .and_then(|bytes| state.export_music_package_umx(&path, &bytes));
                            match result {
                                Ok(()) => state.toast_success(format!("Exported {}", path.display())),
                                Err(error) => state.audio_error(format!("Music export to '{}' failed: {error:#}", path.display())),
                            }
                        }
                    }
                    if ui.button("💾 Export Module").clicked() {
                        let ext = module_extension(&music.format);
                        if let Some(path) = rfd::FileDialog::new().set_file_name(format!("{}.{}", music.package, ext))
                            .add_filter("Tracker Module", &[ext]).save_file() {
                            let result = state.music_module_bytes(&music.package).and_then(|bytes| {
                                std::fs::write(&path, bytes)?;
                                Ok(())
                            });
                            match result {
                                Ok(()) => {
                                    state.log(format!("Exported tracker module to '{}'", path.display()));
                                    state.toast_success(format!("Exported {}", path.display()));
                                }
                                Err(error) => state.audio_error(format!("Module export to '{}' failed: {error:#}", path.display())),
                            }
                        }
                    }
                });

                ui.add_space(8.0);
                let set_song_btn = egui::Button::new(
                    RichText::new(format!("🎵 Set '{}' as Level Song", music.package))
                        .color(Color32::from_rgb(120, 240, 160))
                        .strong(),
                );
                if ui.add_sized([ui.available_width(), 32.0], set_song_btn).clicked() {
                    state.set_level_song(&music.package);
                }
            } else {
                ui.label("Select a music track from the dropdown above to inspect.");
            }
        });

    state.show_music_browser = open;
}

fn module_extension(format: &str) -> &str {
    match format.split_whitespace().next().unwrap_or("") {
        "IT" => "it", "XM" => "xm", "S3M" => "s3m", "MOD" => "mod", "OGG" => "ogg", _ => "bin",
    }
}

fn open_external_player(path: &std::path::Path) -> anyhow::Result<()> {
    anyhow::ensure!(path.is_file(), "Preview file not found: {}", path.display());
    #[cfg(target_os = "windows")]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut command = std::process::Command::new("rundll32.exe");
        command.arg("url.dll,FileProtocolHandler").arg(path).creation_flags(0x08000000);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = { let mut c = std::process::Command::new("open"); c.arg(path); c };
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    let mut command = { let mut c = std::process::Command::new("xdg-open"); c.arg(path); c };
    command.spawn()?;
    Ok(())
}
