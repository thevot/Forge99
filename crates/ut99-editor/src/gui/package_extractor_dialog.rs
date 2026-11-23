use crate::editor_state::EditorState;
use egui::{Color32, RichText, ScrollArea};
use std::fs;
use std::path::PathBuf;
use ut99_bsp::asset_extractor::{
    batch_export_directory, export_package_file, scan_directory_packages_summary,
    scan_package_file_summary, AssetCategory, BatchExportOptions, BatchExportReport,
};
use ut99_format::package::{compress_uz, decompress_uz, is_uz_file};

pub fn show_package_extractor_dialog(ctx: &egui::Context, state: &mut EditorState) {
    if !state.show_package_extractor_dialog {
        return;
    }

    let mut is_open = state.show_package_extractor_dialog;

    egui::Window::new("📦 Package Asset Extractor & .uz Decompressor")
        .open(&mut is_open)
        .default_width(740.0)
        .default_height(600.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Native Unreal Package Asset Extractor");
                ui.label(
                    RichText::new("BatchExport & Gildor UModel Pipeline")
                        .size(11.0)
                        .color(Color32::from_rgb(160, 200, 255)),
                );
            });
            ui.label(
                RichText::new(
                    "Extract textures (.png), sound effects (.wav), tracker music (.it/.xm/.s3m/.mod), \
                     3D meshes (.obj/.psk), and UnrealScript classes (.uc) from any UT99 package.",
                )
                .size(11.0)
                .color(Color32::from_rgb(180, 180, 180)),
            );
            ui.separator();

            ScrollArea::vertical().show(ui, |ui| {
                // Section 1: Source & Destination
                ui.group(|ui| {
                    ui.label(RichText::new("1. Source & Output Locations").strong());
                    egui::Grid::new("pkg_extractor_paths_grid")
                        .num_columns(3)
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            ui.label("Source Package / Dir:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.package_extractor.source_path)
                                    .hint_text("Path to .u, .utx, .uax, .umx, .unr, .uz or game directory")
                                    .desired_width(420.0),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Browse File...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .add_filter(
                                            "Unreal Packages",
                                            &["u", "utx", "uax", "umx", "unr", "uz"],
                                        )
                                        .pick_file()
                                    {
                                        state.package_extractor.source_path =
                                            path.to_string_lossy().to_string();
                                    }
                                }
                                if ui.button("Browse Folder...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        state.package_extractor.source_path =
                                            path.to_string_lossy().to_string();
                                    }
                                }
                            });
                            ui.end_row();

                            ui.label("Output Directory:");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.package_extractor.output_dir)
                                    .hint_text("Destination folder (e.g. ExportOutput)")
                                    .desired_width(420.0),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Browse Folder...").clicked() {
                                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                        state.package_extractor.output_dir =
                                            path.to_string_lossy().to_string();
                                    }
                                }
                                if ui.button("Default").clicked() {
                                    state.package_extractor.output_dir = "ExportOutput".to_string();
                                }
                            });
                            ui.end_row();
                        });
                });

                ui.add_space(6.0);

                // Section 2: Asset Categories & Options
                ui.group(|ui| {
                    ui.label(RichText::new("2. Asset Categories to Extract").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(&mut state.package_extractor.cat_textures, "🖼 Textures (.png)");
                        ui.checkbox(&mut state.package_extractor.cat_sounds, "🔊 Sounds (.wav)");
                        ui.checkbox(&mut state.package_extractor.cat_music, "🎵 Music (.it/.xm/.s3m/.mod)");
                        ui.checkbox(&mut state.package_extractor.cat_meshes, "📐 Meshes (.obj)");
                        ui.checkbox(&mut state.package_extractor.cat_scripts, "📜 Classes (.uc)");
                    });

                    ui.separator();
                    ui.label(RichText::new("Layout & Pipeline Options:").strong());
                    ui.horizontal_wrapped(|ui| {
                        ui.checkbox(
                            &mut state.package_extractor.subfolder_by_package,
                            "Subfolder by Package (e.g. /Botpack/)",
                        );
                        ui.checkbox(
                            &mut state.package_extractor.subfolder_by_category,
                            "Subfolder by Category (e.g. /Textures/, /Sounds/)",
                        );
                        ui.checkbox(
                            &mut state.package_extractor.export_psk_for_meshes,
                            "Export ActorX .PSK for meshes",
                        );
                        ui.checkbox(&mut state.package_extractor.overwrite, "Overwrite existing files");
                    });
                });

                ui.add_space(8.0);

                // Section 3: Action Buttons
                ui.horizontal(|ui| {
                    let can_run = !state.package_extractor.source_path.trim().is_empty();

                    if ui
                        .add_enabled(
                            can_run,
                            egui::Button::new(
                                RichText::new("▶ Start Batch Asset Extraction")
                                    .color(Color32::BLACK)
                                    .strong(),
                            )
                            .fill(Color32::from_rgb(100, 220, 140)),
                        )
                        .clicked()
                    {
                        run_batch_extraction(state);
                    }

                    if ui
                        .add_enabled(
                            can_run,
                            egui::Button::new("🔍 Scan Content (UModel)")
                                .fill(Color32::from_rgb(40, 130, 180)),
                        )
                        .on_hover_text("Fast scan of package header, exports, names, and asset counts without unpacking to disk (UModel -tag-info)")
                        .clicked()
                    {
                        run_scan_content(state);
                    }

                    if ui
                        .add_enabled(
                            can_run,
                            egui::Button::new("⚡ Decompress .uz").fill(Color32::from_rgb(60, 100, 180)),
                        )
                        .on_hover_text("Decompresses a downloaded .uz server redirect file into a raw package")
                        .clicked()
                    {
                        run_decompress_uz(state);
                    }

                    if ui
                        .add_enabled(
                            can_run,
                            egui::Button::new("📦 Compress .uz").fill(Color32::from_rgb(120, 80, 160)),
                        )
                        .on_hover_text("Compresses a package into a .uz redirect file using UT99 zlib format")
                        .clicked()
                    {
                        run_compress_uz(state);
                    }
                });

                // Section 3.5: Scanned Package Manifest (UModel Inspection Table)
                if !state.package_extractor.scan_summaries.is_empty() {
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("📊 Scanned Package Content ({})", state.package_extractor.scan_summaries.len())).strong());
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button("Clear Manifest").clicked() {
                                    state.package_extractor.scan_summaries.clear();
                                }
                            });
                        });

                        egui::ScrollArea::vertical().max_height(200.0).show(ui, |ui| {
                            egui::Grid::new("scanned_pkg_grid")
                                .striped(true)
                                .min_col_width(55.0)
                                .spacing([8.0, 4.0])
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Package").strong());
                                    ui.label(RichText::new("Size").strong());
                                    ui.label(RichText::new("Ver").strong());
                                    ui.label(RichText::new("Textures").strong());
                                    ui.label(RichText::new("Sounds").strong());
                                    ui.label(RichText::new("Music").strong());
                                    ui.label(RichText::new("Meshes").strong());
                                    ui.label(RichText::new("Classes").strong());
                                    ui.label(RichText::new("Levels").strong());
                                    ui.label(RichText::new("Exports / Names").strong());
                                    ui.end_row();

                                    for s in &state.package_extractor.scan_summaries {
                                        let size_str = if s.file_size_bytes >= 1024 * 1024 {
                                            format!("{:.1} MB", s.file_size_bytes as f64 / (1024.0 * 1024.0))
                                        } else {
                                            format!("{:.1} KB", s.file_size_bytes as f64 / 1024.0)
                                        };

                                        ui.horizontal(|ui| {
                                            if s.is_compressed_uz {
                                                ui.colored_label(Color32::from_rgb(180, 120, 240), "[.uz]");
                                            }
                                            ui.label(RichText::new(&s.package_name).strong());
                                        });

                                        ui.label(size_str);
                                        ui.label(format!("v{}", s.package_version));
                                        ui.colored_label(if s.texture_count > 0 { Color32::from_rgb(100, 220, 150) } else { Color32::GRAY }, format!("{}", s.texture_count));
                                        ui.colored_label(if s.sound_count > 0 { Color32::from_rgb(100, 200, 255) } else { Color32::GRAY }, format!("{}", s.sound_count));
                                        ui.colored_label(if s.music_count > 0 { Color32::from_rgb(255, 180, 80) } else { Color32::GRAY }, format!("{}", s.music_count));
                                        ui.colored_label(if s.mesh_count > 0 { Color32::from_rgb(255, 120, 200) } else { Color32::GRAY }, format!("{}", s.mesh_count));
                                        ui.colored_label(if s.class_count > 0 { Color32::from_rgb(220, 220, 100) } else { Color32::GRAY }, format!("{}", s.class_count));
                                        ui.colored_label(if s.level_count > 0 { Color32::from_rgb(180, 255, 100) } else { Color32::GRAY }, format!("{}", s.level_count));
                                        ui.label(format!("{} / {}", s.total_exports, s.total_names));
                                        ui.end_row();
                                    }
                                });
                        });
                    });
                }

                ui.add_space(8.0);

                // Section 4: Status & Report Log
                ui.group(|ui| {
                    ui.label(RichText::new("Execution Status & Statistics:").strong());
                    ui.label(
                        RichText::new(&state.package_extractor.status_message)
                            .size(12.0)
                            .color(Color32::from_rgb(220, 220, 180)),
                    );

                    if let Some(ref report) = state.package_extractor.last_report {
                        ui.separator();
                        ui.add(
                            egui::TextEdit::multiline(&mut report.as_str())
                                .font(egui::TextStyle::Monospace)
                                .desired_rows(12)
                                .desired_width(ui.available_width()),
                        );
                    }
                });
            });
        });

    state.show_package_extractor_dialog = is_open;
}

fn build_categories(state: &EditorState) -> Vec<AssetCategory> {
    let mut cats = Vec::new();
    if state.package_extractor.cat_textures {
        cats.push(AssetCategory::Texture);
    }
    if state.package_extractor.cat_sounds {
        cats.push(AssetCategory::Sound);
    }
    if state.package_extractor.cat_music {
        cats.push(AssetCategory::Music);
    }
    if state.package_extractor.cat_meshes {
        cats.push(AssetCategory::Mesh);
    }
    if state.package_extractor.cat_scripts {
        cats.push(AssetCategory::Script);
    }
    cats
}

fn run_batch_extraction(state: &mut EditorState) {
    let src_path = PathBuf::from(state.package_extractor.source_path.trim());
    if !src_path.exists() {
        state.package_extractor.status_message =
            format!("Error: Source path does not exist: {}", src_path.display());
        return;
    }

    let out_dir = PathBuf::from(state.package_extractor.output_dir.trim());
    let categories = build_categories(state);

    if categories.is_empty() {
        state.package_extractor.status_message =
            "Error: No asset categories selected for extraction.".to_string();
        return;
    }

    let options = BatchExportOptions {
        output_dir: out_dir.clone(),
        categories,
        subfolder_by_package: state.package_extractor.subfolder_by_package,
        subfolder_by_category: state.package_extractor.subfolder_by_category,
        export_psk_for_meshes: state.package_extractor.export_psk_for_meshes,
        overwrite: state.package_extractor.overwrite,
    };

    let start_time = std::time::Instant::now();
    let mut report = BatchExportReport::default();

    if src_path.is_dir() {
        match batch_export_directory(&src_path, &options) {
            Ok(r) => report = r,
            Err(e) => {
                state.package_extractor.status_message =
                    format!("Batch directory extraction failed: {e}");
                return;
            }
        }
    } else {
        if let Err(e) = export_package_file(&src_path, &options, &mut report) {
            state.package_extractor.status_message = format!("Single package extraction failed: {e}");
            return;
        }
    }

    let elapsed = start_time.elapsed();
    let mb_written = report.total_bytes_written as f64 / (1024.0 * 1024.0);

    state.package_extractor.status_message = format!(
        "✔ Successfully extracted {} assets across {} package(s) ({:.2} MB) in {:.2}s.",
        report.total_assets(),
        report.total_packages,
        mb_written,
        elapsed.as_secs_f64()
    );

    let mut log = String::new();
    log.push_str("=======================================================================\n");
    log.push_str("  UNREAL TOURNAMENT 99 BATCH ASSET EXTRACTION REPORT\n");
    log.push_str("=======================================================================\n");
    log.push_str(&format!("Source Path:        {}\n", src_path.display()));
    log.push_str(&format!("Destination:        {}\n", out_dir.display()));
    log.push_str(&format!("Packages Processed: {}\n", report.total_packages));
    log.push_str(&format!("Textures Extracted: {}\n", report.textures_extracted));
    log.push_str(&format!("Sounds Extracted:   {}\n", report.sounds_extracted));
    log.push_str(&format!("Music Extracted:    {}\n", report.music_extracted));
    log.push_str(&format!("Meshes Extracted:   {}\n", report.meshes_extracted));
    log.push_str(&format!("Classes Extracted:  {}\n", report.scripts_extracted));
    log.push_str(&format!("Total Assets:       {}\n", report.total_assets()));
    log.push_str(&format!("Total Disk Size:    {:.2} MB ({} bytes)\n", mb_written, report.total_bytes_written));
    log.push_str(&format!("Duration:           {:.2}s\n", elapsed.as_secs_f64()));

    if !report.errors.is_empty() {
        log.push_str("\n--- Warnings & Non-Fatal Errors ---\n");
        for err in &report.errors {
            log.push_str(&format!("- {}\n", err));
        }
    }

    log.push_str("\n--- Extracted Files Sample ---\n");
    let max_display = report.written_files.len().min(50);
    for file in &report.written_files[..max_display] {
        log.push_str(&format!("+ {}\n", file.display()));
    }
    if report.written_files.len() > max_display {
        log.push_str(&format!("... and {} more files\n", report.written_files.len() - max_display));
    }

    state.package_extractor.last_report = Some(log);
}

fn run_decompress_uz(state: &mut EditorState) {
    let src_path = PathBuf::from(state.package_extractor.source_path.trim());
    if !src_path.is_file() {
        state.package_extractor.status_message =
            "Error: Please select a valid .uz file to decompress.".to_string();
        return;
    }

    let bytes = match fs::read(&src_path) {
        Ok(b) => b,
        Err(e) => {
            state.package_extractor.status_message = format!("Failed reading {}: {e}", src_path.display());
            return;
        }
    };

    if !is_uz_file(&bytes) {
        state.package_extractor.status_message =
            format!("Error: {} does not have a valid UT99 .uz header tag (1234).", src_path.display());
        return;
    }

    let mut cursor = std::io::Cursor::new(&bytes);
    match decompress_uz(&mut cursor) {
        Ok((orig_name, decomp_bytes)) => {
            let out_dir = if !state.package_extractor.output_dir.trim().is_empty() {
                PathBuf::from(state.package_extractor.output_dir.trim())
            } else {
                src_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
            };
            let _ = fs::create_dir_all(&out_dir);
            let target_file = out_dir.join(&orig_name);

            if let Err(e) = fs::write(&target_file, &decomp_bytes) {
                state.package_extractor.status_message =
                    format!("Failed writing decompressed package {}: {e}", target_file.display());
                return;
            }

            state.package_extractor.status_message = format!(
                "✔ Decompressed '{}' -> '{}' ({:.2} MB decompressed).",
                src_path.file_name().unwrap_or_default().to_string_lossy(),
                target_file.display(),
                decomp_bytes.len() as f64 / (1024.0 * 1024.0)
            );
        }
        Err(e) => {
            state.package_extractor.status_message = format!("Decompression failed: {e}");
        }
    }
}

fn run_compress_uz(state: &mut EditorState) {
    let src_path = PathBuf::from(state.package_extractor.source_path.trim());
    if !src_path.is_file() {
        state.package_extractor.status_message =
            "Error: Please select a valid package file to compress into .uz.".to_string();
        return;
    }

    let bytes = match fs::read(&src_path) {
        Ok(b) => b,
        Err(e) => {
            state.package_extractor.status_message = format!("Failed reading {}: {e}", src_path.display());
            return;
        }
    };

    let filename = src_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Package.unr".to_string());

    match compress_uz(&filename, &bytes) {
        Ok(compressed_bytes) => {
            let out_dir = if !state.package_extractor.output_dir.trim().is_empty() {
                PathBuf::from(state.package_extractor.output_dir.trim())
            } else {
                src_path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
            };
            let _ = fs::create_dir_all(&out_dir);
            let uz_name = format!("{}.uz", filename);
            let target_file = out_dir.join(&uz_name);

            if let Err(e) = fs::write(&target_file, &compressed_bytes) {
                state.package_extractor.status_message =
                    format!("Failed writing .uz file {}: {e}", target_file.display());
                return;
            }

            let ratio = (1.0 - (compressed_bytes.len() as f64 / bytes.len().max(1) as f64)) * 100.0;
            state.package_extractor.status_message = format!(
                "✔ Compressed '{}' -> '{}' ({:.1}% compression ratio).",
                filename,
                target_file.display(),
                ratio
            );
        }
        Err(e) => {
            state.package_extractor.status_message = format!("Compression to .uz failed: {e}");
        }
    }
}

fn run_scan_content(state: &mut EditorState) {
    let src_path = PathBuf::from(state.package_extractor.source_path.trim());
    if !src_path.exists() {
        state.package_extractor.status_message =
            format!("Error: Source path does not exist: {}", src_path.display());
        return;
    }

    let start_time = std::time::Instant::now();
    state.package_extractor.scan_summaries.clear();

    if src_path.is_dir() {
        match scan_directory_packages_summary(&src_path) {
            Ok(summaries) => {
                let count = summaries.len();
                let total_assets: usize = summaries.iter().map(|s| s.total_game_assets()).sum();
                state.package_extractor.scan_summaries = summaries;
                state.package_extractor.status_message = format!(
                    "✔ Scanned {} packages in {:.2}s: found {} total game assets.",
                    count,
                    start_time.elapsed().as_secs_f32(),
                    total_assets
                );
            }
            Err(e) => {
                state.package_extractor.status_message = format!("Directory scan failed: {e}");
            }
        }
    } else {
        match scan_package_file_summary(&src_path) {
            Ok(summary) => {
                let assets = summary.total_game_assets();
                let name = summary.package_name.clone();
                let exp_count = summary.total_exports;
                let name_count = summary.total_names;
                state.package_extractor.scan_summaries = vec![summary];
                state.package_extractor.status_message = format!(
                    "✔ Scanned package '{}' in {:.2}s: {} game assets (Exports: {}, Names: {}).",
                    name,
                    start_time.elapsed().as_secs_f32(),
                    assets,
                    exp_count,
                    name_count
                );
            }
            Err(e) => {
                state.package_extractor.status_message = format!("Package scan failed: {e}");
            }
        }
    }
}
