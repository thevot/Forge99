//! Disk sources for the audio browsers. Paths always belong to one active installation.
use std::{collections::BTreeMap, path::{Path, PathBuf}};
use anyhow::{Context, Result};
use ut99_format::package::{Package, SoundResource, MusicResource, parse_sound_export, parse_music_export};

#[derive(Default)]
pub(crate) struct AudioCatalog {
    pub root: Option<PathBuf>,
    pub initialized: bool,
    pub sounds: BTreeMap<String, PathBuf>,
    pub music: BTreeMap<String, PathBuf>,
    pub imports: BTreeMap<String, Vec<SoundResource>>,
}

impl AudioCatalog {
    pub fn rescan(&mut self, root: Option<PathBuf>) -> Vec<String> {
        self.root = root;
        self.initialized = true;
        self.sounds.clear();
        self.music.clear();
        let mut errors = Vec::new();
        if let Some(root) = &self.root {
            for (folder, extension, entries) in [
                ("Sounds", "uax", &mut self.sounds), ("Music", "umx", &mut self.music),
            ] {
                let dir = root.join(folder);
                let result = (|| -> Result<()> {
                    for entry in std::fs::read_dir(&dir)? {
                        let path = entry?.path();
                        if path.is_file() && path.extension().and_then(|s| s.to_str()).is_some_and(|s| s.eq_ignore_ascii_case(extension)) {
                            if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                                entries.insert(name.to_ascii_lowercase(), path);
                            }
                        }
                    }
                    Ok(())
                })();
                if let Err(error) = result { errors.push(format!("Could not scan '{}': {error}", dir.display())); }
            }
        }
        errors
    }

    pub fn sound_path(&self, name: &str) -> Result<&Path> { self.path(name, "Sounds", "uax", &self.sounds) }
    pub fn music_path(&self, name: &str) -> Result<&Path> { self.path(name, "Music", "umx", &self.music) }
    fn path<'a>(&self, name: &str, folder: &str, ext: &str, entries: &'a BTreeMap<String, PathBuf>) -> Result<&'a Path> {
        entries.get(&name.to_ascii_lowercase()).map(PathBuf::as_path).ok_or_else(|| {
            let target = self.root.as_ref().map(|root| root.join(folder).join(format!("{name}.{ext}")));
            anyhow::anyhow!("Audio package '{}' not found in {}", name,
                target.map(|p| p.display().to_string()).unwrap_or_else(|| "the active installation (none detected)".into()))
        })
    }
}

pub(crate) fn read_sounds(path: &Path, name: &str) -> Result<Vec<SoundResource>> {
    let pkg = Package::open(path).with_context(|| format!("Could not read '{}'", path.display()))?;
    let mut sounds = Vec::new();
    for (idx, export) in pkg.exports.entries.iter().enumerate() {
        if pkg.get_export_class_name(export).eq_ignore_ascii_case("Sound") {
            if let Some(sound) = pkg.get_export_bytes(idx).and_then(|bytes| parse_sound_export(&pkg.get_export_object_name(export), name, bytes)) {
                sounds.push(sound);
            }
        }
    }
    anyhow::ensure!(!sounds.is_empty(), "No readable sounds in '{}'", path.display());
    Ok(sounds)
}

pub(crate) fn read_music(path: &Path, name: &str) -> Result<(MusicResource, Vec<u8>)> {
    let pkg = Package::open(path).with_context(|| format!("Could not read '{}'", path.display()))?;
    for (idx, export) in pkg.exports.entries.iter().enumerate() {
        if pkg.get_export_class_name(export).eq_ignore_ascii_case("Music") {
            if let Some(raw) = pkg.get_export_bytes(idx) {
                if let Some(mut music) = parse_music_export(&pkg.get_export_object_name(export), name, raw) {
                    let bytes = tracker_bytes(raw).with_context(|| format!("Unsupported music in '{}'", path.display()))?;
                    music.data_size = bytes.len();
                    return Ok((music, bytes.to_vec()));
                }
            }
        }
    }
    anyhow::bail!("No readable music in '{}'", path.display())
}

fn tracker_bytes(raw: &[u8]) -> Result<&[u8]> {
    for signature in [b"IMPM".as_slice(), b"Extended Module:".as_slice(), b"OggS".as_slice()] {
        if let Some(pos) = raw.windows(signature.len()).position(|w| w == signature) { return Ok(&raw[pos..]); }
    }
    if let Some(pos) = raw.windows(4).position(|w| w == b"SCRM").and_then(|p| p.checked_sub(44)) { return Ok(&raw[pos..]); }
    if let Some(pos) = raw.windows(4).enumerate().find_map(|(i,w)| {
        (w == b"M.K." || w == b"4CHN" || w == b"FLT4" || w == b"8CHN").then(|| i.checked_sub(1080)).flatten()
    }) { return Ok(&raw[pos..]); }
    anyhow::bail!("Tracker format not recognized")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EditorState;
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Install(PathBuf);
    impl Install {
        fn new() -> Self {
            let time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let path = std::env::temp_dir().join(format!("forge99-audio-test-{}-{time}-{}", std::process::id(), NEXT.fetch_add(1, Ordering::Relaxed)));
            for dir in ["System", "Sounds", "Music"] { std::fs::create_dir_all(path.join(dir)).unwrap(); }
            std::fs::write(path.join("System/Core.u"), b"installation marker").unwrap();
            Self(path)
        }
        fn populated(name: &str) -> (Self, Vec<u8>) {
            let install = Self::new();
            let sound = parse_sound_export(name, "Same", &wav()).unwrap();
            std::fs::write(install.0.join("Sounds/Same.UAX"), ut99_format::package::build_uax_package(&[sound]).unwrap().to_bytes().unwrap()).unwrap();
            let mut bytes = vec![0; 100];
            bytes[..4].copy_from_slice(b"IMPM");
            bytes[4..4+name.len()].copy_from_slice(name.as_bytes());
            std::fs::write(install.0.join("Music/Same.UMX"), ut99_format::package::build_umx_package("Track", "IT", &bytes).unwrap().to_bytes().unwrap()).unwrap();
            (install, bytes)
        }
        fn load(&self, state: &mut EditorState) { state.refresh_audio_from_root(Some(self.0.clone())); }
    }
    impl Drop for Install {
        fn drop(&mut self) {
            assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
            assert!(self.0.file_name().unwrap().to_string_lossy().starts_with("forge99-audio-test-"));
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn wav() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend(b"RIFF"); bytes.extend(40u32.to_le_bytes()); bytes.extend(b"WAVEfmt ");
        bytes.extend(16u32.to_le_bytes()); bytes.extend(1u16.to_le_bytes()); bytes.extend(1u16.to_le_bytes());
        bytes.extend(8000u32.to_le_bytes()); bytes.extend(16000u32.to_le_bytes());
        bytes.extend(2u16.to_le_bytes()); bytes.extend(16u16.to_le_bytes());
        bytes.extend(b"data"); bytes.extend(4u32.to_le_bytes()); bytes.extend([0u8, 0, 1, 0]); bytes
    }

    #[test]
    fn configured_installation_loads_sound_music_and_exact_module_bytes() {
        let (install, bytes) = Install::populated("First");
        let mut state = EditorState::default();
        install.load(&mut state);
        assert_eq!(state.sound_packages, ["Same"]);
        assert_eq!(state.music_packages, ["Same"]);
        assert_eq!(state.loaded_sounds.len(), 1);
        assert_eq!(state.loaded_sounds[0].name, "First");
        assert_eq!(state.loaded_music.as_ref().unwrap().title, "First");
        assert_eq!(state.music_module_bytes("sAmE").unwrap(), bytes);
        assert_eq!(state.loaded_sounds[0].wav_bytes, wav());
    }

    #[test]
    fn switching_installations_reloads_same_named_packages() {
        let (first, _) = Install::populated("First");
        let (second, bytes) = Install::populated("Second");
        let mut state = EditorState::default();
        first.load(&mut state);
        second.load(&mut state);
        assert_eq!(state.selected_sound_package, "Same");
        assert_eq!(state.selected_music_package, "Same");
        assert_eq!(state.loaded_sounds[0].name, "Second");
        assert_eq!(state.loaded_music.as_ref().unwrap().title, "Second");
        assert_eq!(state.music_module_bytes("Same").unwrap(), bytes);
    }

    #[test]
    fn refresh_preserves_imports_and_mylevel_without_mixing_disk_sources() {
        let (first, _) = Install::populated("First");
        let (second, _) = Install::populated("Second");
        let mut state = EditorState::default();
        first.load(&mut state);
        state.import_wav_sound("Local", "Same", wav()).unwrap();
        state.import_wav_sound("Custom", "CustomPackage", wav()).unwrap();
        state.import_wav_sound("Embedded", "MyLevel", wav()).unwrap();
        second.load(&mut state);
        assert!(state.sound_packages.contains(&"MyLevel".into()));
        assert!(state.sound_packages.contains(&"CustomPackage".into()));
        assert_eq!(state.loaded_sounds[0].name, "Embedded");
        state.load_sound_package("Same");
        let names: Vec<_> = state.loaded_sounds.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["Second", "Local"]);
        state.load_sound_package("CustomPackage");
        assert_eq!(state.loaded_sounds[0].name, "Custom");
        state.delete_sound_from_package("Custom");
        second.load(&mut state);
        assert!(!state.sound_packages.contains(&"CustomPackage".into()));
        assert_eq!(state.mylevel_sounds[0].name, "Embedded");
    }

    #[test]
    fn empty_or_absent_installations_clear_stale_disk_metadata() {
        let (full, _) = Install::populated("First");
        let empty = Install::new();
        let mut state = EditorState::default();
        full.load(&mut state);
        empty.load(&mut state);
        assert!(state.sound_packages.is_empty() && state.music_packages.is_empty());
        assert!(state.loaded_sounds.is_empty() && state.loaded_music.is_none());
        assert!(state.selected_sound_package.is_empty() && state.selected_music_package.is_empty());
        full.load(&mut state);
        state.refresh_audio_from_root(None);
        assert!(state.sound_packages.is_empty() && state.music_packages.is_empty());
        assert!(state.loaded_sounds.is_empty() && state.loaded_music.is_none());
    }

    #[test]
    fn corrupt_and_missing_packages_clear_details_and_report_the_source() {
        let (install, _) = Install::populated("First");
        let mut state = EditorState::default();
        install.load(&mut state);
        std::fs::write(install.0.join("Music/Same.UMX"), b"broken").unwrap();
        state.load_music_package("Same");
        assert!(state.loaded_music.is_none());
        assert!(state.toasts.last().unwrap().message.contains("Same.UMX"));
        assert!(state.music_module_bytes("Same").is_err());
        std::fs::remove_file(install.0.join("Sounds/Same.UAX")).unwrap();
        state.load_sound_package("Same");
        assert!(state.loaded_sounds.is_empty());
        assert!(state.toasts.last().unwrap().message.contains("Same.UAX"));
        install.load(&mut state);
        assert!(state.sound_packages.is_empty());
        assert!(state.selected_sound_name.is_none());
    }

    #[test]
    fn unreadable_folder_is_reported_separately_from_empty_folder() {
        let install = Install::new();
        std::fs::remove_dir(install.0.join("Sounds")).unwrap();
        std::fs::write(install.0.join("Sounds"), b"not a folder").unwrap();
        let mut catalog = AudioCatalog::default();
        let errors = catalog.rescan(Some(install.0.clone()));
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Sounds"));
        assert!(catalog.sounds.is_empty() && catalog.music.is_empty());
    }

    #[test]
    fn module_extraction_removes_package_headers_for_supported_formats() {
        let install = Install::new();
        for format in ["IT", "XM", "S3M", "MOD", "OGG"] {
            let mut bytes = vec![0; if format == "MOD" { 1100 } else { 100 }];
            match format {
                "IT" => bytes[..4].copy_from_slice(b"IMPM"),
                "XM" => bytes[..16].copy_from_slice(b"Extended Module:"),
                "S3M" => bytes[44..48].copy_from_slice(b"SCRM"),
                "MOD" => bytes[1080..1084].copy_from_slice(b"M.K."),
                _ => bytes[..4].copy_from_slice(b"OggS"),
            }
            let path = install.0.join(format!("Music/{format}.umx"));
            std::fs::write(&path, ut99_format::package::build_umx_package("Track", format, &bytes).unwrap().to_bytes().unwrap()).unwrap();
            assert_eq!(read_music(&path, format).unwrap().1, bytes);
        }
    }

    #[test]
    fn explicit_exports_preserve_the_installation_sources() {
        let (install, original_module) = Install::populated("First");
        let mut state = EditorState::default();
        install.load(&mut state);
        let original = std::fs::read(install.0.join("Music/Same.UMX")).unwrap();
        let target = install.0.join("chosen.umx");
        state.export_music_package_umx(&target, &state.music_module_bytes("Same").unwrap()).unwrap();
        assert_eq!(read_music(&target, "Export").unwrap().1, original_module);
        assert_eq!(std::fs::read(install.0.join("Music/Same.UMX")).unwrap(), original);
        state.export_sound_package_uax(install.0.join("chosen.uax")).unwrap();
        assert_eq!(read_sounds(&install.0.join("chosen.uax"), "Export").unwrap()[0].wav_bytes, wav());
        assert!(state.export_sound_package_uax(install.0.join("missing/fail.uax")).is_err());
    }

    #[test]
    fn preferences_refresh_audio_after_startup_in_isolated_process() {
        const CHILD: &str = "FORGE99_AUDIO_CONFIG_TEST_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "audio_catalog::tests::preferences_refresh_audio_after_startup_in_isolated_process"])
                .env(CHILD, "1").status().unwrap();
            assert!(status.success());
            return;
        }
        let (install, bytes) = Install::populated("Preferences");
        let mut state = EditorState::default();
        let mut prefs = crate::preferences::EditorPreferences::from_editor_state(&state);
        prefs.ut99_executable_path = install.0.to_string_lossy().to_string();
        let path = install.0.join("preferences.json");
        prefs.save_to_path(&path).unwrap();
        assert!(state.load_preferences_from_path(&path));
        assert_eq!(state.loaded_sounds[0].name, "Preferences");
        assert_eq!(state.loaded_music.as_ref().unwrap().title, "Preferences");
        assert_eq!(state.music_module_bytes("Same").unwrap(), bytes);
    }
}
