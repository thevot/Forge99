use std::fs::{self, File};
use std::io::{self, Cursor, Write};
use std::path::{Path, PathBuf};

use image::{ImageBuffer, ImageFormat, Rgba};
use ut99_format::package::resource::{
    extract_class_script_text, extract_u_package_meshes, parse_sound_export,
    scan_u_package_classes,
};
use ut99_format::package::{decompress_uz, is_uz_file};
use ut99_format::utx::UtxPackage;
use ut99_format::Package;

use crate::actorx::{ActorXPskMesh, PskBone, PskFace, PskMaterial, PskPoint, PskWeight, PskWedge};
use glam::{Quat, Vec3};

/// Categories of game assets extractable from Unreal packages (`.u`, `.utx`, `.uax`, `.umx`, `.unr`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetCategory {
    Texture,
    Sound,
    Music,
    Mesh,
    Script,
}

impl AssetCategory {
    pub fn display_name(&self) -> &'static str {
        match self {
            AssetCategory::Texture => "Textures",
            AssetCategory::Sound => "Sounds",
            AssetCategory::Music => "Music",
            AssetCategory::Mesh => "Meshes",
            AssetCategory::Script => "Classes",
        }
    }

    pub fn default_extension(&self) -> &'static str {
        match self {
            AssetCategory::Texture => "png",
            AssetCategory::Sound => "wav",
            AssetCategory::Music => "it",
            AssetCategory::Mesh => "obj",
            AssetCategory::Script => "uc",
        }
    }
}

/// A decoded, ready-to-write game asset extracted from an Unreal Tournament package.
#[derive(Debug, Clone)]
pub struct ExtractedAsset {
    pub name: String,
    pub category: AssetCategory,
    pub package_name: String,
    pub group: String,
    pub file_extension: String,
    pub description: String,
    pub data: Vec<u8>,
}

impl ExtractedAsset {
    /// Constructs a standardized relative file path for writing to disk.
    pub fn make_relative_path(&self, by_package: bool, by_category: bool) -> PathBuf {
        let mut path = PathBuf::new();
        if by_package {
            path.push(&self.package_name);
        }
        if by_category {
            path.push(self.category.display_name());
        }
        if !self.group.is_empty() && !self.group.eq_ignore_ascii_case("None") {
            path.push(&self.group);
        }
        let filename = format!("{}.{}", self.name, self.file_extension);
        path.push(filename);
        path
    }
}

/// Options controlling a batch asset extraction job.
#[derive(Debug, Clone)]
pub struct BatchExportOptions {
    pub output_dir: PathBuf,
    pub categories: Vec<AssetCategory>,
    pub subfolder_by_package: bool,
    pub subfolder_by_category: bool,
    pub export_psk_for_meshes: bool,
    pub overwrite: bool,
}

impl Default for BatchExportOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("ExportOutput"),
            categories: vec![
                AssetCategory::Texture,
                AssetCategory::Sound,
                AssetCategory::Music,
                AssetCategory::Mesh,
                AssetCategory::Script,
            ],
            subfolder_by_package: true,
            subfolder_by_category: true,
            export_psk_for_meshes: false,
            overwrite: true,
        }
    }
}

/// Detailed execution report of a batch asset extraction operation.
#[derive(Debug, Clone, Default)]
pub struct BatchExportReport {
    pub total_packages: usize,
    pub textures_extracted: usize,
    pub sounds_extracted: usize,
    pub music_extracted: usize,
    pub meshes_extracted: usize,
    pub scripts_extracted: usize,
    pub total_bytes_written: usize,
    pub written_files: Vec<PathBuf>,
    pub errors: Vec<String>,
}

impl BatchExportReport {
    pub fn total_assets(&self) -> usize {
        self.textures_extracted
            + self.sounds_extracted
            + self.music_extracted
            + self.meshes_extracted
            + self.scripts_extracted
    }
}

/// A high-level summary of Unreal package contents without extracting or decoding asset payloads.
/// Inspired by Gildor's UModel package scanning (`umodel -tag-info`).
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PackageContentSummary {
    pub package_name: String,
    pub file_size_bytes: u64,
    pub package_version: u32,
    pub is_compressed_uz: bool,
    pub texture_count: usize,
    pub sound_count: usize,
    pub music_count: usize,
    pub mesh_count: usize,
    pub class_count: usize,
    pub level_count: usize,
    pub total_exports: usize,
    pub total_names: usize,
    pub total_imports: usize,
}

impl PackageContentSummary {
    pub fn total_game_assets(&self) -> usize {
        self.texture_count
            + self.sound_count
            + self.music_count
            + self.mesh_count
            + self.class_count
            + self.level_count
    }
}

/// Scans and extracts all assets matching the requested categories from an in-memory `Package`.
pub fn extract_assets_from_package(
    package: &Package,
    package_name: &str,
    categories: &[AssetCategory],
    export_psk: bool,
) -> Vec<ExtractedAsset> {
    let mut results = Vec::new();

    // 1. Textures (UTX, U, UNR)
    if categories.contains(&AssetCategory::Texture) {
        if let Ok(utx) = UtxPackage::from_package(package.clone(), package_name.to_string()) {
            for tex in utx.textures {
                if tex.width == 0 || tex.height == 0 || tex.rgba_pixels.is_empty() {
                    continue;
                }

                // Encode to PNG bytes
                if let Some(img_buf) = ImageBuffer::<Rgba<u8>, _>::from_raw(
                    tex.width,
                    tex.height,
                    tex.rgba_pixels.clone(),
                ) {
                    let mut png_bytes = Vec::new();
                    let mut cursor = Cursor::new(&mut png_bytes);
                    if img_buf.write_to(&mut cursor, ImageFormat::Png).is_ok() {
                        let fmt_str = match tex.format {
                            3 => "DXT1",
                            7 => "DXT3",
                            8 => "DXT5",
                            1 | 5 => "RGBA8",
                            4 => "RGB8",
                            2 => "RGB16",
                            _ => "P8 (256 Colors)",
                        };
                        results.push(ExtractedAsset {
                            name: tex.name.clone(),
                            category: AssetCategory::Texture,
                            package_name: package_name.to_string(),
                            group: tex.group.clone(),
                            file_extension: "png".to_string(),
                            description: format!("{}x{} {} (Masked: {})", tex.width, tex.height, fmt_str, tex.masked),
                            data: png_bytes,
                        });
                    }
                }
            }
        }
    }

    // 2. Sounds (UAX, U, UNR)
    if categories.contains(&AssetCategory::Sound) {
        for (idx, export) in package.exports.entries.iter().enumerate() {
            let cls_name = package.get_export_class_name(export);
            if cls_name == "Sound" {
                let obj_name = package.get_export_object_name(export);
                if let Some(raw_bytes) = package.get_export_bytes(idx) {
                    if let Some(snd) = parse_sound_export(&obj_name, package_name, raw_bytes) {
                        let group = if export.package_index > 0 {
                            let outer_idx = (export.package_index - 1) as usize;
                            package
                                .exports
                                .entries
                                .get(outer_idx)
                                .and_then(|o| package.get_name(o.object_name.max(0) as usize))
                                .unwrap_or("None")
                                .to_string()
                        } else {
                            "None".to_string()
                        };

                        results.push(ExtractedAsset {
                            name: obj_name.to_string(),
                            category: AssetCategory::Sound,
                            package_name: package_name.to_string(),
                            group,
                            file_extension: "wav".to_string(),
                            description: format!(
                                "{}Hz {}-bit {} ({:.2}s)",
                                snd.sample_rate,
                                snd.bits_per_sample,
                                if snd.channels == 2 { "Stereo" } else { "Mono" },
                                snd.duration_seconds
                            ),
                            data: snd.wav_bytes,
                        });
                    }
                }
            }
        }
    }

    // 3. Music (UMX)
    if categories.contains(&AssetCategory::Music) {
        for (idx, export) in package.exports.entries.iter().enumerate() {
            let cls_name = package.get_export_class_name(export);
            if cls_name == "Music" {
                let obj_name = package.get_export_object_name(export);
                if let Some(raw) = package.get_export_bytes(idx) {
                    if let Some((ext, desc, music_data)) = detect_and_extract_music_bytes(raw) {
                        results.push(ExtractedAsset {
                            name: obj_name.to_string(),
                            category: AssetCategory::Music,
                            package_name: package_name.to_string(),
                            group: "None".to_string(),
                            file_extension: ext,
                            description: desc,
                            data: music_data,
                        });
                    }
                }
            }
        }
    }

    // 4. Meshes (U, UNR)
    if categories.contains(&AssetCategory::Mesh) {
        let meshes = extract_u_package_meshes(package, package_name);
        for m in meshes {
            if m.vertices.is_empty() || m.triangles.is_empty() {
                continue;
            }

            let base_name = m.name.rsplit('.').next().unwrap_or(&m.name).to_string();

            // Export Wavefront .OBJ
            let mut obj_text = String::new();
            obj_text.push_str(&format!("# Unreal Tournament 3D Mesh: {}\n", m.name));
            obj_text.push_str(&format!("o {}\n", base_name));
            for v in &m.vertices {
                obj_text.push_str(&format!("v {:.4} {:.4} {:.4}\n", v[0], v[2], v[1])); // Y-up / Z-up orientation
            }
            for tri in &m.triangles {
                // Wavefront OBJ indices are 1-based
                obj_text.push_str(&format!("f {} {} {}\n", tri[0] + 1, tri[1] + 1, tri[2] + 1));
            }

            results.push(ExtractedAsset {
                name: base_name.clone(),
                category: AssetCategory::Mesh,
                package_name: package_name.to_string(),
                group: "None".to_string(),
                file_extension: "obj".to_string(),
                description: format!(
                    "{} ({} verts, {} tris)",
                    m.class_name,
                    m.vertices.len(),
                    m.triangles.len()
                ),
                data: obj_text.into_bytes(),
            });

            // Optional ActorX .PSK export
            if export_psk {
                let psk_bytes = build_psk_from_mesh(&base_name, &m.vertices, &m.triangles);
                results.push(ExtractedAsset {
                    name: format!("{}_ActorX", base_name),
                    category: AssetCategory::Mesh,
                    package_name: package_name.to_string(),
                    group: "None".to_string(),
                    file_extension: "psk".to_string(),
                    description: format!("ActorX PSK ({} verts, {} tris)", m.vertices.len(), m.triangles.len()),
                    data: psk_bytes,
                });
            }
        }
    }

    // 5. Scripts / Class Definitions (U)
    if categories.contains(&AssetCategory::Script) {
        let classes = scan_u_package_classes(package, package_name);
        for cls in classes {
            let script_text = extract_class_script_text(package, &cls.name)
                .unwrap_or_else(|| {
                    format!(
                        "//=============================================================================\n\
                         // {}.uc - Class definition extracted from {}\n\
                         //=============================================================================\n\
                         class {} extends {};\n\n\
                         defaultproperties\n\
                         {{\n\
                         }}\n",
                        cls.name, package_name, cls.name, cls.super_name
                    )
                });

            results.push(ExtractedAsset {
                name: cls.name.clone(),
                category: AssetCategory::Script,
                package_name: package_name.to_string(),
                group: "None".to_string(),
                file_extension: "uc".to_string(),
                description: format!("UnrealScript Class (extends {})", cls.super_name),
                data: script_text.into_bytes(),
            });
        }
    }

    results
}

/// Identifies music tracker signatures (IT, XM, S3M, MOD, OGG) and extracts payload.
fn detect_and_extract_music_bytes(raw: &[u8]) -> Option<(String, String, Vec<u8>)> {
    if raw.len() < 32 {
        return None;
    }

    // 1. Impulse Tracker ("IMPM")
    if let Some(pos) = raw.windows(4).position(|w| w == b"IMPM") {
        return Some((
            "it".to_string(),
            "Impulse Tracker Module".to_string(),
            raw[pos..].to_vec(),
        ));
    }

    // 2. FastTracker II ("Extended Module:")
    if let Some(pos) = raw.windows(16).position(|w| w == b"Extended Module:") {
        return Some((
            "xm".to_string(),
            "FastTracker II Extended Module".to_string(),
            raw[pos..].to_vec(),
        ));
    }

    // 3. ScreamTracker 3 ("SCRM")
    if let Some(pos) = raw.windows(4).position(|w| w == b"SCRM") {
        let start = pos.saturating_sub(44);
        return Some((
            "s3m".to_string(),
            "ScreamTracker 3 Module".to_string(),
            raw[start..].to_vec(),
        ));
    }

    // 4. ProTracker MOD ("M.K.", "M!K!", "FLT4", "4CHN")
    for &sig in &[b"M.K.", b"M!K!", b"FLT4", b"4CHN"] {
        if let Some(pos) = raw.windows(4).position(|w| w == sig) {
            let start = pos.saturating_sub(1080);
            return Some((
                "mod".to_string(),
                "Amiga ProTracker Module".to_string(),
                raw[start..].to_vec(),
            ));
        }
    }

    // 5. Ogg Vorbis ("OggS")
    if let Some(pos) = raw.windows(4).position(|w| w == b"OggS") {
        return Some((
            "ogg".to_string(),
            "Ogg Vorbis Audio".to_string(),
            raw[pos..].to_vec(),
        ));
    }

    // Fallback: raw export if reasonable size
    if raw.len() >= 1024 {
        Some((
            "it".to_string(),
            "Tracker Audio Stream".to_string(),
            raw.to_vec(),
        ))
    } else {
        None
    }
}

/// Builds an ActorX PSK binary buffer from vertex mesh geometry.
fn build_psk_from_mesh(name: &str, vertices: &[[f32; 3]], triangles: &[[u16; 3]]) -> Vec<u8> {
    let mut psk = ActorXPskMesh::default();

    // Points
    for v in vertices {
        psk.points.push(PskPoint {
            point: Vec3::new(v[0], v[1], v[2]),
        });
    }

    // Single default root bone
    psk.bones.push(PskBone {
        name: "Root".to_string(),
        flags: 0,
        num_children: 0,
        parent_index: 0,
        orientation: Quat::IDENTITY,
        position: Vec3::ZERO,
        length: 50.0,
        size: Vec3::splat(10.0),
    });

    // Material
    psk.materials.push(PskMaterial {
        name: format!("{}_Mat", name),
        texture_index: 0,
        poly_flags: 0,
    });

    // Wedges & Faces
    for tri in triangles {
        let w0 = psk.wedges.len() as u16;
        psk.wedges.push(PskWedge {
            point_index: tri[0],
            u: 0.0,
            v: 0.0,
            material_index: 0,
        });
        psk.weights.push(PskWeight {
            weight: 1.0,
            point_index: tri[0] as u32,
            bone_index: 0,
        });

        let w1 = psk.wedges.len() as u16;
        psk.wedges.push(PskWedge {
            point_index: tri[1],
            u: 1.0,
            v: 0.0,
            material_index: 0,
        });
        psk.weights.push(PskWeight {
            weight: 1.0,
            point_index: tri[1] as u32,
            bone_index: 0,
        });

        let w2 = psk.wedges.len() as u16;
        psk.wedges.push(PskWedge {
            point_index: tri[2],
            u: 0.5,
            v: 1.0,
            material_index: 0,
        });
        psk.weights.push(PskWeight {
            weight: 1.0,
            point_index: tri[2] as u32,
            bone_index: 0,
        });

        psk.faces.push(PskFace {
            wedge_indices: [w0, w1, w2],
            mat_index: 0,
            aux_mat_index: 0,
            smoothing_groups: 1,
        });
    }

    psk.to_bytes()
}

/// Executes a batch extraction on a single file path (handles `.u`, `.utx`, `.uax`, `.umx`, `.unr`, `.uz`).
pub fn export_package_file(
    package_path: &Path,
    options: &BatchExportOptions,
    report: &mut BatchExportReport,
) -> io::Result<()> {
    let raw_bytes = fs::read(package_path)?;
    let filename = package_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Package".to_string());

    // 1. Detect and decompress if .uz
    let (real_name, package_data) = if is_uz_file(&raw_bytes) {
        let mut cursor = Cursor::new(&raw_bytes);
        decompress_uz(&mut cursor).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed decompressing .uz redirect {}: {}", filename, e),
            )
        })?
    } else {
        (filename.clone(), raw_bytes)
    };

    let pkg_stem = Path::new(&real_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Package".to_string());

    // 2. Parse Package
    let package = match Package::from_bytes(package_data) {
        Ok(p) => p,
        Err(e) => {
            let msg = format!("Failed parsing package {}: {}", real_name, e);
            report.errors.push(msg.clone());
            return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
        }
    };

    report.total_packages += 1;

    // 3. Extract assets
    let assets = extract_assets_from_package(
        &package,
        &pkg_stem,
        &options.categories,
        options.export_psk_for_meshes,
    );

    // 4. Write assets to destination directory
    for asset in assets {
        let rel_path = asset.make_relative_path(
            options.subfolder_by_package,
            options.subfolder_by_category,
        );
        let dest_path = options.output_dir.join(rel_path);

        if let Some(parent) = dest_path.parent() {
            fs::create_dir_all(parent)?;
        }

        if !options.overwrite && dest_path.exists() {
            continue;
        }

        let mut file = File::create(&dest_path)?;
        file.write_all(&asset.data)?;

        report.total_bytes_written += asset.data.len();
        match asset.category {
            AssetCategory::Texture => report.textures_extracted += 1,
            AssetCategory::Sound => report.sounds_extracted += 1,
            AssetCategory::Music => report.music_extracted += 1,
            AssetCategory::Mesh => report.meshes_extracted += 1,
            AssetCategory::Script => report.scripts_extracted += 1,
        }
        report.written_files.push(dest_path);
    }

    Ok(())
}

/// Recursively traverses `source_dir`, processing every Unreal package matching standard extensions.
pub fn batch_export_directory(
    source_dir: &Path,
    options: &BatchExportOptions,
) -> io::Result<BatchExportReport> {
    let mut report = BatchExportReport::default();
    let valid_exts = ["u", "utx", "uax", "umx", "unr", "uz"];

    fn visit_dirs(
        dir: &Path,
        valid_exts: &[&str],
        options: &BatchExportOptions,
        report: &mut BatchExportReport,
    ) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, valid_exts, options, report);
                } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if valid_exts.iter().any(|v| v.eq_ignore_ascii_case(ext)) {
                        if let Err(e) = export_package_file(&path, options, report) {
                            report.errors.push(format!("{}: {}", path.display(), e));
                        }
                    }
                }
            }
        }
    }

    visit_dirs(source_dir, &valid_exts, options, &mut report);
    Ok(report)
}

/// Scans the exports, names, and imports of an in-memory `Package` to generate a content summary.
pub fn scan_package_content_summary(package: &Package, package_name: &str) -> PackageContentSummary {
    let mut summary = PackageContentSummary {
        package_name: package_name.to_string(),
        file_size_bytes: 0,
        package_version: package.summary.file_version,
        is_compressed_uz: false,
        texture_count: 0,
        sound_count: 0,
        music_count: 0,
        mesh_count: 0,
        class_count: 0,
        level_count: 0,
        total_exports: package.exports.entries.len(),
        total_names: package.names.entries.len(),
        total_imports: package.imports.entries.len(),
    };

    for export in &package.exports.entries {
        let cls_name = package.get_export_class_name(export);
        match cls_name.as_str() {
            "Texture" | "Font" => summary.texture_count += 1,
            "Sound" => summary.sound_count += 1,
            "Music" => summary.music_count += 1,
            "Mesh" | "LodMesh" | "SkeletalMesh" => summary.mesh_count += 1,
            "Class" => summary.class_count += 1,
            "Level" | "Model" | "Polys" => summary.level_count += 1,
            _ => {}
        }
    }

    summary
}

/// Scans a single package file on disk (supports `.u`, `.utx`, `.uax`, `.umx`, `.unr`, `.uz`).
pub fn scan_package_file_summary(package_path: &Path) -> io::Result<PackageContentSummary> {
    let raw_bytes = fs::read(package_path)?;
    let file_size_bytes = raw_bytes.len() as u64;
    let filename = package_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "Package".to_string());

    let (real_name, is_uz, package_data) = if is_uz_file(&raw_bytes) {
        let mut cursor = Cursor::new(&raw_bytes);
        let (decomp_name, decomp_data) = decompress_uz(&mut cursor).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Failed decompressing .uz redirect {}: {}", filename, e),
            )
        })?;
        (decomp_name, true, decomp_data)
    } else {
        (filename.clone(), false, raw_bytes)
    };

    let pkg_stem = Path::new(&real_name)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| real_name);

    let package = Package::from_bytes(package_data).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed parsing package {}: {}", package_path.display(), e),
        )
    })?;

    let mut summary = scan_package_content_summary(&package, &pkg_stem);
    summary.file_size_bytes = file_size_bytes;
    summary.is_compressed_uz = is_uz;
    Ok(summary)
}

/// Recursively scans a directory for Unreal Tournament packages, returning content summaries for all found packages.
pub fn scan_directory_packages_summary(dir: &Path) -> io::Result<Vec<PackageContentSummary>> {
    let mut summaries = Vec::new();
    let valid_exts = ["u", "utx", "uax", "umx", "unr", "uz"];

    fn visit(dir: &Path, valid_exts: &[&str], summaries: &mut Vec<PackageContentSummary>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit(&path, valid_exts, summaries);
                } else if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    if valid_exts.iter().any(|v| v.eq_ignore_ascii_case(ext)) {
                        if let Ok(summary) = scan_package_file_summary(&path) {
                            summaries.push(summary);
                        }
                    }
                }
            }
        }
    }

    visit(dir, &valid_exts, &mut summaries);
    summaries.sort_by(|a, b| a.package_name.cmp(&b.package_name));
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ut99_format::package::PackageFileSummary;
    use ut99_format::utx::UtxTexture;

    #[test]
    fn test_asset_categories_metadata() {
        assert_eq!(AssetCategory::Texture.display_name(), "Textures");
        assert_eq!(AssetCategory::Sound.default_extension(), "wav");
        assert_eq!(AssetCategory::Music.default_extension(), "it");
        assert_eq!(AssetCategory::Mesh.default_extension(), "obj");
        assert_eq!(AssetCategory::Script.default_extension(), "uc");
    }

    #[test]
    fn test_extract_assets_from_empty_or_texture_package() {
        let mut pkg = Package::default();
        pkg.summary = PackageFileSummary::default();

        let tex = UtxTexture {
            name: "WallTile".to_string(),
            group: "Industrial".to_string(),
            width: 16,
            height: 16,
            format: 0,
            masked: false,
            rgba_pixels: vec![200u8; 16 * 16 * 4],
        };

        UtxPackage::write_textures_into_package(&[tex], &mut pkg, 0).unwrap();
        let serialized = pkg.to_bytes().unwrap();
        let loaded_pkg = Package::from_bytes(serialized).unwrap();

        let assets = extract_assets_from_package(
            &loaded_pkg,
            "MyTextures",
            &[AssetCategory::Texture],
            false,
        );

        assert_eq!(assets.len(), 1);
        let a = &assets[0];
        assert_eq!(a.name, "WallTile");
        assert_eq!(a.category, AssetCategory::Texture);
        assert_eq!(a.file_extension, "png");
        assert!(!a.data.is_empty());
        // Verify PNG magic header "\x89PNG\r\n\x1a\n"
        assert_eq!(&a.data[0..4], b"\x89PNG");

        let rel = a.make_relative_path(true, true);
        assert_eq!(rel, PathBuf::from("MyTextures/Textures/Industrial/WallTile.png"));

        // Test PackageContentSummary scan
        let summary = scan_package_content_summary(&loaded_pkg, "MyTextures");
        assert_eq!(summary.package_name, "MyTextures");
        assert_eq!(summary.texture_count, 1);
        assert_eq!(summary.class_count, 1); // Group export (class_index 0)
        assert_eq!(summary.total_game_assets(), 2);
    }

    #[test]
    fn test_mesh_actorx_psk_conversion() {
        let verts = vec![
            [0.0f32, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
        ];
        let tris = vec![[0u16, 1, 2]];

        let psk_bytes = build_psk_from_mesh("TestMesh", &verts, &tris);
        assert!(!psk_bytes.is_empty());
        assert_eq!(&psk_bytes[0..8], b"ACTRHEAD");
    }
}
