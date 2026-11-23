use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use ut99_format::{UtxPackage, UtxTexture};
use crate::compiler::CompilerBridge;

/// Canonical UnrealEd 2.0 `UTexture` physical, acoustic, detail, and flipbook animation properties (`REQ-D08`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TextureSurfaceMetadata {
    pub footstep_sound: String,
    pub hit_sound: String,
    pub friction: f32,
    pub detail_texture: String,
    pub macro_texture: String,
    pub anim_next: String,
    pub min_frame_rate: f32,
    pub max_frame_rate: f32,
}

impl Default for TextureSurfaceMetadata {
    fn default() -> Self {
        Self {
            footstep_sound: String::new(),
            hit_sound: String::new(),
            friction: 1.0,
            detail_texture: String::new(),
            macro_texture: String::new(),
            anim_next: String::new(),
            min_frame_rate: 0.0,
            max_frame_rate: 0.0,
        }
    }
}


/// A package-relative path preserves all group components, including nested groups.
fn normalized_group(group: &str) -> &str {
    if group.eq_ignore_ascii_case("None") { "" } else { group }
}

pub(crate) fn local_texture_name(group: &str, name: &str) -> String {
    let group = normalized_group(group);
    if group.is_empty() { name.to_string() } else { format!("{group}.{name}") }
}

pub(crate) fn texture_reference(package: &str, group: &str, name: &str) -> String {
    format!("{package}.{}", local_texture_name(group, name))
}

struct TextureReference<'a> {
    package: Option<&'a str>,
    group: Option<&'a str>,
    name: &'a str,
}
impl<'a> TextureReference<'a> {
    fn parse(raw: &'a str) -> Option<Self> {
        let mut value = raw.trim();
        if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
            value = value[1..value.len()-1].trim();
        }
        if value.len() >= 9 && value.get(..8).is_some_and(|prefix| prefix.eq_ignore_ascii_case("Texture'")) && value.ends_with('\'') {
            value = &value[8..value.len()-1];
        } else if value.starts_with('\'') && value.ends_with('\'') && value.len() >= 2 {
            value = &value[1..value.len()-1];
        }
        if value.is_empty() || ["None", "Default", "DefaultTexture"].iter().any(|name| value.eq_ignore_ascii_case(name))
            || value.split('.').any(|part| part.is_empty() || part.trim() != part)
            || value.contains(['\'', '"']) { return None; }
        let (package, local) = match value.split_once('.') {
            Some((package, local)) => (Some(package), local),
            None => (None, value),
        };
        let (group, name) = match local.rsplit_once('.') {
            Some((group, name)) => (Some(group), name),
            None => (None, local),
        };
        Some(Self { package, group, name })
    }
    fn matches(&self, texture: &UtxTexture) -> bool {
        texture.name.eq_ignore_ascii_case(self.name)
            && self.group.map_or(true, |group| normalized_group(group).eq_ignore_ascii_case(normalized_group(&texture.group)))
    }
}

fn local_texture_index(package: &UtxPackage, name: &str) -> Option<usize> {
    if name.is_empty() || name.split('.').any(str::is_empty) { return None; }
    let (group, short) = name.rsplit_once('.').map_or((None, name), |(group, short)| (Some(group), short));
    let reference = TextureReference { package: None, group, name: short };
    let mut matches = package.textures.iter().enumerate().filter(|(_, texture)| reference.matches(texture));
    let index = matches.next()?.0;
    matches.next().is_none().then_some(index)
}

fn next_texture_revision() -> u64 {
    static REVISION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    REVISION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

pub struct TextureManager {
    pub(crate) resource_revision: u64,
    textures_dir: Option<PathBuf>,
    available_packages: Vec<String>,
    loaded_packages: HashMap<String, UtxPackage>,
    egui_thumbnails: HashMap<String, egui::TextureHandle>,
    pub texture_metadata: HashMap<String, TextureSurfaceMetadata>,

    // --- LRU Memory Budgeting ---
    memory_budget_bytes: usize,
    package_last_accessed: HashMap<String, u64>,
    thumbnail_last_accessed: HashMap<String, u64>,
    thumbnail_memory_bytes: HashMap<String, usize>,
    pinned_packages: HashSet<String>,
    access_tick: u64,
}

impl Default for TextureManager {
    fn default() -> Self {
        let textures_dir = CompilerBridge::find_ut_dir().map(|d| d.join("Textures"));
        let mut available_packages = Vec::new();

        if let Some(ref dir) = textures_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                            if ext.eq_ignore_ascii_case("utx") {
                                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                                    available_packages.push(stem.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
        available_packages.sort_by_key(|a| a.to_lowercase());

        Self {
            resource_revision: next_texture_revision(),
            textures_dir,
            available_packages,
            loaded_packages: HashMap::new(),
            egui_thumbnails: HashMap::new(),
            texture_metadata: HashMap::new(),
            memory_budget_bytes: Self::DEFAULT_MEMORY_BUDGET_BYTES,
            package_last_accessed: HashMap::new(),
            thumbnail_last_accessed: HashMap::new(),
            thumbnail_memory_bytes: HashMap::new(),
            pinned_packages: HashSet::new(),
            access_tick: 0,
        }
    }
}

impl TextureManager {
    pub const DEFAULT_MEMORY_BUDGET_BYTES: usize = 256 * 1024 * 1024; // 256 MB

    fn next_tick(&mut self) -> u64 {
        self.access_tick = self.access_tick.wrapping_add(1);
        self.access_tick
    }

    /// Returns list of all available .utx package names found in the Textures folder.
    pub fn available_packages(&self) -> &[String] {
        &self.available_packages
    }

    /// Loads a package into memory if not already loaded.
    pub fn load_package(&mut self, package_name: &str) -> anyhow::Result<&UtxPackage> {
        let canonical_name = self.loaded_packages.keys()
            .find(|name| name.eq_ignore_ascii_case(package_name))
            .or_else(|| self.available_packages.iter().find(|name| name.eq_ignore_ascii_case(package_name)))
            .cloned().unwrap_or_else(|| package_name.to_string());

        let tick = self.next_tick();
        self.package_last_accessed.insert(canonical_name.clone(), tick);

        if !self.loaded_packages.contains_key(&canonical_name) {
            let mut candidates = Vec::new();
            if let Some(ref dir) = self.textures_dir {
                candidates.push(dir.join(format!("{canonical_name}.utx")));
            }
            candidates.push(std::path::PathBuf::from("UnrealTournament/Textures").join(format!("{canonical_name}.utx")));
            candidates.push(std::path::PathBuf::from("../../UnrealTournament/Textures").join(format!("{canonical_name}.utx")));
            if let Some(ut_dir) = crate::compiler::CompilerBridge::find_ut_dir() {
                candidates.push(ut_dir.join("Textures").join(format!("{canonical_name}.utx")));
            }

            let utx_path = candidates.into_iter().find(|p| p.exists()).ok_or_else(|| {
                anyhow::anyhow!("Package file not found for '{}'", canonical_name)
            })?;

            let pkg = UtxPackage::open(&utx_path)?;
            self.loaded_packages.insert(canonical_name.clone(), pkg);
            self.resource_revision = next_texture_revision();
            // Pin canonical_name temporarily during eviction so it cannot be immediately pruned
            self.pinned_packages.insert(canonical_name.clone());
            self.enforce_memory_budget();
            self.pinned_packages.remove(&canonical_name);
        }

        self.loaded_packages.get(&canonical_name).ok_or_else(|| {
            anyhow::anyhow!("Package file loaded but missing for '{}'", canonical_name)
        })
    }

    /// Returns a reference to a loaded package, if it is in cache.
    pub fn get_package(&self, package_name: &str) -> Option<&UtxPackage> {
        self.loaded_packages.get(package_name).or_else(|| {
            self.loaded_packages
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(package_name))
                .map(|(_, v)| v)
        })
    }

    /// Touch a package to update its LRU timestamp without reloading.
    pub fn touch_package(&mut self, package_name: &str) {
        if self.loaded_packages.contains_key(package_name) {
            let tick = self.next_tick();
            self.package_last_accessed.insert(package_name.to_string(), tick);
        }
    }

    /// Inserts a package directly into the loaded cache (useful for custom packages or tests).
    pub fn insert_package(&mut self, name: String, package: UtxPackage) {
        let name = self.loaded_packages.keys().find(|key| key.eq_ignore_ascii_case(&name))
            .or_else(|| self.available_packages.iter().find(|key| key.eq_ignore_ascii_case(&name)))
            .cloned().unwrap_or(name);
        self.unload_package(&name);
        let tick = self.next_tick();
        self.package_last_accessed.insert(name.clone(), tick);
        if !self.available_packages.contains(&name) {
            self.available_packages.push(name.clone());
            self.available_packages.sort_by_key(|a| a.to_lowercase());
        }
        self.loaded_packages.insert(name.clone(), package);
        self.resource_revision = next_texture_revision();
        self.pinned_packages.insert(name.clone());
        self.enforce_memory_budget();
        self.pinned_packages.remove(&name);
    }

    /// Upserts a texture into a package (creating the package if it does not exist), and invalidates any cached egui thumbnail.
    pub fn upsert_texture(&mut self, package_name: &str, texture: UtxTexture) {
        let canonical = self.loaded_packages.keys().find(|p| p.eq_ignore_ascii_case(package_name))
            .cloned().unwrap_or_else(|| package_name.to_string());
        self.invalidate_thumbnail(&texture_reference(&canonical, &texture.group, &texture.name));
        if let Some(pkg) = self.loaded_packages.get_mut(&canonical) {
            if let Some(pos) = pkg.textures.iter().position(|t| t.name.eq_ignore_ascii_case(&texture.name)
                && normalized_group(&t.group).eq_ignore_ascii_case(normalized_group(&texture.group))) {
                pkg.textures[pos] = texture;
            } else {
                pkg.textures.push(texture);
            }
        } else {
            let pkg = UtxPackage {
                package_name: package_name.to_string(),
                textures: vec![texture],
            };
            self.insert_package(package_name.to_string(), pkg);
        }
    }

    /// Finds one unambiguous texture by local name or Group.Name within a package.
    pub fn get_texture(&self, package_name: &str, texture_name: &str) -> Option<&UtxTexture> {
        let pkg = self.get_package(package_name)?;
        let index = local_texture_index(pkg, texture_name)?;
        pkg.textures.get(index)
    }

    fn resolve_loaded_texture(&self, reference: &str) -> Option<(&str, &UtxTexture)> {
        let reference = TextureReference::parse(reference)?;
        let mut result = None;
        for (key, pkg) in &self.loaded_packages {
            if reference.package.is_some_and(|name| !name.eq_ignore_ascii_case(key)) { continue; }
            for texture in &pkg.textures {
                if reference.matches(texture) {
                    if result.is_some() { return None; }
                    result = Some((key.as_str(), texture));
                }
            }
        }
        result
    }

    /// Qualified references never fall back across packages/groups; ambiguous names fail.
    pub fn find_loaded_texture(&self, reference: &str) -> Option<&UtxTexture> {
        self.resolve_loaded_texture(reference).map(|(_, texture)| texture)
    }

    /// Canonical identity for a loaded, unambiguous reference (including its group).
    pub fn resolve_texture_reference(&self, reference: &str) -> Option<String> {
        let (package, texture) = self.resolve_loaded_texture(reference)?;
        Some(texture_reference(package, &texture.group, &texture.name))
    }

    fn invalidate_thumbnail(&mut self, identity: &str) {
        self.resource_revision = next_texture_revision();
        let key = identity.to_ascii_lowercase();
        self.egui_thumbnails.remove(&key);
        self.thumbnail_last_accessed.remove(&key);
        self.thumbnail_memory_bytes.remove(&key);
    }

    /// Samples a normalized RGBA color `[r, g, b, a]` from a loaded `.utx` texture at world UV `(u, v)`.
    pub fn sample_texture_rgba(&self, full_or_short_name: &str, u: f32, v: f32) -> Option<[f32; 4]> {
        let tex = self.find_loaded_texture(full_or_short_name)?;
        if tex.width == 0 || tex.height == 0 || tex.rgba_pixels.len() < (tex.width * tex.height * 4) as usize {
            return None;
        }
        let w = tex.width as f32;
        let h = tex.height as f32;
        let px = (u.rem_euclid(w) as u32).min(tex.width - 1);
        let py = (v.rem_euclid(h) as u32).min(tex.height - 1);
        let idx = ((py * tex.width + px) * 4) as usize;
        let r = tex.rgba_pixels[idx] as f32 / 255.0;
        let g = tex.rgba_pixels[idx + 1] as f32 / 255.0;
        let b = tex.rgba_pixels[idx + 2] as f32 / 255.0;
        let a = tex.rgba_pixels[idx + 3] as f32 / 255.0;
        Some([r, g, b, a])
    }

    /// Retrieves or generates an egui::TextureHandle for a texture in a package.
    pub fn get_or_create_egui_texture(
        &mut self,
        ctx: &egui::Context,
        package_name: &str,
        texture_name: &str,
    ) -> Option<egui::TextureHandle> {
        self.load_package(package_name).ok()?;
        let pkg = self.get_package(package_name)?;
        let texture = pkg.textures.get(local_texture_index(pkg, texture_name)?)?;
        let full_name = texture_reference(package_name, &texture.group, &texture.name).to_ascii_lowercase();
        if let Some(handle) = self.egui_thumbnails.get(&full_name).cloned() {
            let tick = self.next_tick();
            self.thumbnail_last_accessed.insert(full_name.clone(), tick);
            return Some(handle);
        }
        let tex = self.get_texture(package_name, texture_name)?;
        if tex.width == 0 || tex.height == 0 || tex.rgba_pixels.len() < (tex.width * tex.height * 4) as usize {
            return None;
        }

        let rgba_len = (tex.width * tex.height * 4) as usize;
        let color_image = egui::ColorImage::from_rgba_unmultiplied(
            [tex.width as usize, tex.height as usize],
            &tex.rgba_pixels[..rgba_len],
        );

        let handle = ctx.load_texture(
            full_name.clone(),
            color_image,
            egui::TextureOptions::LINEAR,
        );

        let tick = self.next_tick();
        self.thumbnail_last_accessed.insert(full_name.clone(), tick);
        self.thumbnail_memory_bytes.insert(full_name.clone(), rgba_len);
        self.egui_thumbnails.insert(full_name, handle.clone());

        self.enforce_memory_budget();

        Some(handle)
    }

    /// Resolves a full or partial texture reference (e.g. "UTtech2.Wall.rClfWl2", "ShaneDay.DayskyB", or "rClfWl2")
    /// and retrieves or generates an egui::TextureHandle for it.
    pub fn get_or_create_egui_texture_by_ref(
        &mut self,
        ctx: &egui::Context,
        texture_ref: &str,
    ) -> Option<egui::TextureHandle> {
        let reference = TextureReference::parse(texture_ref)?;
        if let Some(package) = reference.package { self.load_package(package).ok()?; }
        let (package, texture) = self.resolve_loaded_texture(texture_ref)?;
        let package = package.to_string();
        let local = local_texture_name(&texture.group, &texture.name);
        self.get_or_create_egui_texture(ctx, &package, &local)
    }

    /// Queries dimensions using the same identity rules as rendering and thumbnails.
    pub fn get_texture_dimensions_by_ref(&mut self, texture_ref: &str) -> Option<(u32, u32)> {
        let reference = TextureReference::parse(texture_ref)?;
        if let Some(package) = reference.package { self.load_package(package).ok()?; }
        let texture = self.find_loaded_texture(texture_ref)?;
        Some((texture.width, texture.height))
    }

    /// Returns the sorted, deduplicated list of unique Groups found in the specified package (e.g. "All", "In Use", "None", "Wall", "Floor").
    pub fn get_package_groups(&mut self, package_name: &str) -> Vec<String> {
        let _ = self.load_package(package_name);
        let mut groups = Vec::new();
        groups.push("All".to_string());
        groups.push("In Use".to_string());

        if let Some(pkg) = self.get_package(package_name) {
            let mut unique_groups = std::collections::BTreeSet::new();
            for tex in &pkg.textures {
                if !tex.group.is_empty() {
                    unique_groups.insert(tex.group.clone());
                } else {
                    unique_groups.insert("None".to_string());
                }
            }
            groups.extend(unique_groups);
        }
        groups
    }

    /// Exports a specific texture to a PNG or BMP image file on disk.
    pub fn export_texture_to_file(&self, package_name: &str, texture_name: &str, path: &std::path::Path) -> anyhow::Result<()> {
        let tex = self.get_texture(package_name, texture_name)
            .ok_or_else(|| anyhow::anyhow!("Texture '{}.{}' not found", package_name, texture_name))?;

        if tex.width == 0 || tex.height == 0 || tex.rgba_pixels.len() < (tex.width * tex.height * 4) as usize {
            anyhow::bail!("Invalid texture buffer dimensions or data length for '{}.{}'", package_name, texture_name);
        }

        image::save_buffer(
            path,
            &tex.rgba_pixels[..(tex.width * tex.height * 4) as usize],
            tex.width,
            tex.height,
            image::ExtendedColorType::Rgba8,
        )?;
        Ok(())
    }

    /// Imports an RGBA8 pixel buffer into a target package (such as `"MyLevel"` for map-embedded textures)
    /// and pins `"MyLevel"` so it is never evicted by LRU cache pruning.
    /// Returns the full texture reference `"Package.TextureName"`.
    pub fn import_rgba_to_package(
        &mut self,
        package_name: &str,
        group_name: &str,
        texture_name: &str,
        width: u32,
        height: u32,
        masked: bool,
        rgba_pixels: Vec<u8>,
    ) -> anyhow::Result<String> {
        let pkg = if package_name.trim().is_empty() {
            "MyLevel"
        } else {
            package_name.trim()
        };
        let tex_name = if texture_name.trim().is_empty() {
            "CustomTex"
        } else {
            texture_name.trim()
        };
        if width == 0 || height == 0 || rgba_pixels.len() < (width * height * 4) as usize {
            anyhow::bail!("Invalid RGBA buffer dimensions ({width}x{height}) for texture '{tex_name}'");
        }
        let tex = UtxTexture {
            name: tex_name.to_string(),
            group: group_name.trim().to_string(),
            width,
            height,
            format: 0,
            masked,
            rgba_pixels,
        };
        self.upsert_texture(pkg, tex);
        if pkg.eq_ignore_ascii_case("MyLevel") {
            self.pin_package(pkg);
        }
        Ok(format!("{}.{}", pkg, tex_name))
    }

    /// Loads an image file (`.png`, `.bmp`, `.pcx`, `.tga`, `.jpg`) from disk and imports it into `package_name`
    /// (typically `"MyLevel"` for UnrealEd 2.0 map-embedded textures).
    pub fn import_image_file_to_package(
        &mut self,
        package_name: &str,
        group_name: &str,
        path: &std::path::Path,
        masked: bool,
    ) -> anyhow::Result<String> {
        let img = image::open(path)?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("ImportedTexture");
        self.import_rgba_to_package(
            package_name,
            group_name,
            stem,
            width,
            height,
            masked,
            rgba.into_raw(),
        )
    }

    /// Exports all textures from a package into a target directory as PNG files.
    pub fn export_package_to_directory(&mut self, package_name: &str, dir: &std::path::Path) -> anyhow::Result<usize> {
        let _ = self.load_package(package_name);
        let pkg = self.loaded_packages.get(package_name)
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))?;

        std::fs::create_dir_all(dir)?;
        let mut count = 0;

        for tex in &pkg.textures {
            if tex.width == 0 || tex.height == 0 || tex.rgba_pixels.len() < (tex.width * tex.height * 4) as usize {
                continue;
            }
            let sub_dir = if !tex.group.is_empty() && tex.group != "None" {
                dir.join(&tex.group)
            } else {
                dir.to_path_buf()
            };
            std::fs::create_dir_all(&sub_dir)?;

            let file_path = sub_dir.join(format!("{}.png", tex.name));
            if image::save_buffer(
                &file_path,
                &tex.rgba_pixels[..(tex.width * tex.height * 4) as usize],
                tex.width,
                tex.height,
                image::ExtendedColorType::Rgba8,
            ).is_ok() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Saves a loaded texture package (e.g., `"MyLevel"` or a custom package) to a standalone binary `.utx` file on disk.
    pub fn save_package_to_utx(&self, package_name: &str, path: &std::path::Path) -> anyhow::Result<usize> {
        let pkg = self.get_package(package_name)
            .ok_or_else(|| anyhow::anyhow!("Package '{}' not loaded", package_name))?;
        pkg.save(path)?;
        Ok(pkg.textures.len())
    }

    /// Opens a standalone `.utx` file from disk and registers it in the loaded packages cache.
    pub fn open_package_from_path(&mut self, path: &std::path::Path) -> anyhow::Result<String> {
        let pkg = UtxPackage::open(path)?;
        let name = pkg.package_name.clone();
        self.insert_package(name.clone(), pkg);
        Ok(name)
    }

    /// Returns the number of packages currently loaded in memory.
    pub fn loaded_package_count(&self) -> usize {
        self.loaded_packages.len()
    }

    /// Returns the number of generated egui thumbnail handles currently cached.
    pub fn cached_thumbnail_count(&self) -> usize {
        self.egui_thumbnails.len()
    }

    /// Clears cached egui texture handles to reclaim GPU and UI memory.
    pub fn clear_thumbnail_cache(&mut self) {
        self.egui_thumbnails.clear();
        self.thumbnail_last_accessed.clear();
        self.thumbnail_memory_bytes.clear();
    }

    // --- LRU Memory Budget & Cache Pruning Methods ---

    /// Configurable memory budget in bytes (default 256 MB).
    pub fn memory_budget_bytes(&self) -> usize {
        self.memory_budget_bytes
    }

    /// Sets the maximum memory budget in bytes. Triggers immediate budget enforcement if current usage exceeds it.
    pub fn set_memory_budget_bytes(&mut self, budget: usize) -> (usize, usize) {
        self.memory_budget_bytes = budget;
        self.enforce_memory_budget()
    }

    /// Calculates memory consumption in bytes for a specific loaded package.
    pub fn package_memory_usage_bytes(&self, package_name: &str) -> usize {
        if let Some(pkg) = self.loaded_packages.get(package_name) {
            pkg.textures.iter().map(|t| t.rgba_pixels.len() + std::mem::size_of::<UtxTexture>()).sum::<usize>()
                + std::mem::size_of::<UtxPackage>()
        } else {
            0
        }
    }

    /// Sum of memory used by all loaded package data buffers.
    pub fn packages_memory_usage_bytes(&self) -> usize {
        self.loaded_packages.keys().map(|k| self.package_memory_usage_bytes(k)).sum()
    }

    /// Sum of estimated memory used by generated thumbnail handles.
    pub fn thumbnails_memory_usage_bytes(&self) -> usize {
        self.thumbnail_memory_bytes.values().sum()
    }

    /// Total memory consumption tracked by the TextureManager.
    pub fn total_memory_usage_bytes(&self) -> usize {
        self.packages_memory_usage_bytes() + self.thumbnails_memory_usage_bytes()
    }

    /// Pin a package to prevent it from being automatically evicted by LRU cache pruning.
    pub fn pin_package(&mut self, package_name: &str) {
        self.pinned_packages.insert(package_name.to_string());
    }

    /// Unpin a package allowing it to be reclaimed during LRU cache pruning.
    pub fn unpin_package(&mut self, package_name: &str) {
        self.pinned_packages.remove(package_name);
    }

    /// Checks if a package is pinned against LRU eviction.
    pub fn is_package_pinned(&self, package_name: &str) -> bool {
        self.pinned_packages.contains(package_name)
    }

    /// Returns the set of currently pinned package names.
    pub fn pinned_packages(&self) -> &HashSet<String> {
        &self.pinned_packages
    }

    /// Removes assets owned by the current map without evicting shared packages.
    pub fn clear_mylevel_assets(&mut self) {
        self.unpin_package("MyLevel");
        self.unload_package("MyLevel");
        self.available_packages.retain(|name| !name.eq_ignore_ascii_case("MyLevel"));
        self.texture_metadata.retain(|name, _| !name.to_ascii_lowercase().starts_with("mylevel."));
    }

    /// Explicitly unloads a package and all its cached thumbnail textures from memory.
    pub fn unload_package(&mut self, package_name: &str) -> (bool, usize) {
        let canonical = self.loaded_packages.keys().find(|key| key.eq_ignore_ascii_case(package_name)).cloned();
        let package_name = canonical.as_deref().unwrap_or(package_name);
        if let Some(pkg) = self.loaded_packages.remove(package_name) {
            self.resource_revision = next_texture_revision();
            let freed_pkg_bytes = pkg.textures.iter().map(|t| t.rgba_pixels.len() + std::mem::size_of::<UtxTexture>()).sum::<usize>()
                + std::mem::size_of::<UtxPackage>();
            self.package_last_accessed.remove(package_name);

            // Remove any associated thumbnails
            let prefix = format!("{}.", package_name.to_ascii_lowercase());
            let matching_thumbs: Vec<String> = self.egui_thumbnails.keys()
                .filter(|k| k.starts_with(&prefix))
                .cloned()
                .collect();
            let mut freed_thumb_bytes = 0;
            for thumb_key in matching_thumbs {
                self.egui_thumbnails.remove(&thumb_key);
                self.thumbnail_last_accessed.remove(&thumb_key);
                if let Some(b) = self.thumbnail_memory_bytes.remove(&thumb_key) {
                    freed_thumb_bytes += b;
                }
            }
            (true, freed_pkg_bytes + freed_thumb_bytes)
        } else {
            (false, 0)
        }
    }

    /// Prunes the least-recently-used thumbnails and unpinned packages until memory usage <= target_bytes.
    ///
    /// Eviction policy:
    /// 1. Least-recently-accessed thumbnails are evicted first (cheapest to regenerate).
    /// 2. If memory is still over target, least-recently-accessed unpinned packages are unloaded.
    ///
    /// Returns `(evicted_items_count, bytes_freed)`.
    pub fn prune_lru(&mut self, target_bytes: usize) -> (usize, usize) {
        let mut evicted_items = 0;
        let mut freed_bytes = 0;

        while self.total_memory_usage_bytes() > target_bytes {
            // Priority 1: Evict least recently accessed thumbnail (excluding thumbnails accessed within the last 15 ticks)
            let mut evicted_thumb = false;
            if !self.thumbnail_last_accessed.is_empty() {
                let oldest_thumb = self.thumbnail_last_accessed.iter()
                    .filter(|(_, &tick)| tick + 15 < self.access_tick)
                    .min_by_key(|(_, &tick)| tick)
                    .map(|(k, _)| k.clone());

                if let Some(key) = oldest_thumb {
                    self.egui_thumbnails.remove(&key);
                    self.thumbnail_last_accessed.remove(&key);
                    if let Some(b) = self.thumbnail_memory_bytes.remove(&key) {
                        freed_bytes += b;
                    }
                    evicted_items += 1;
                    evicted_thumb = true;
                }
            }
            if evicted_thumb {
                continue;
            }

            // Priority 2: Evict least recently accessed unpinned package
            let oldest_pkg = self.loaded_packages.keys()
                .filter(|k| !self.pinned_packages.contains(*k))
                .min_by_key(|k| self.package_last_accessed.get(*k).copied().unwrap_or(0))
                .cloned();

            if let Some(pkg_name) = oldest_pkg {
                let (removed, bytes) = self.unload_package(&pkg_name);
                if removed {
                    evicted_items += 1;
                    freed_bytes += bytes;
                    continue;
                }
            }

            // Cannot evict further (all remaining packages are pinned and no thumbnails remain)
            break;
        }

        (evicted_items, freed_bytes)
    }

    /// Automatically checks if current memory usage exceeds `memory_budget_bytes`,
    /// and if so, runs LRU eviction down to the budget limit.
    pub fn enforce_memory_budget(&mut self) -> (usize, usize) {
        if self.total_memory_usage_bytes() > self.memory_budget_bytes {
            self.prune_lru(self.memory_budget_bytes)
        } else {
            (0, 0)
        }
    }

    /// Returns the UT99 `UTexture` surface metadata (`FootstepSound`, `HitSound`, `Friction`,
    /// `DetailTexture`, `MacroTexture`, `AnimNext`, `MinFrameRate`, `MaxFrameRate`) for `full_name`.
    pub fn get_texture_metadata(&self, full_name: &str) -> TextureSurfaceMetadata {
        self.texture_metadata
            .get(&self.metadata_key(full_name))
            .cloned()
            .unwrap_or_default()
    }

    fn metadata_key(&self, reference: &str) -> String {
        self.resolve_texture_reference(reference).unwrap_or_else(|| reference.to_string()).to_ascii_lowercase()
    }

    /// Sets the UT99 `UTexture` surface metadata for `full_name`.
    pub fn set_texture_metadata(&mut self, full_name: &str, meta: TextureSurfaceMetadata) {
        let key = self.metadata_key(full_name);
        self.texture_metadata.insert(key, meta);
    }

    /// Converts a texture in a loaded package to a target format (`0` = P8 Paletted, `3` = DXT1/BC1, `8` = DXT5/BC3).
    /// Returns `(original_bytes, new_bytes, savings_percent)`.
    pub fn convert_texture_format(
        &mut self,
        package_name: &str,
        texture_name: &str,
        target_format: u32,
    ) -> anyhow::Result<(usize, usize, f32)> {
        let canonical = self.loaded_packages.keys().find(|p| p.eq_ignore_ascii_case(package_name))
            .cloned().ok_or_else(|| anyhow::anyhow!("Package '{package_name}' not loaded"))?;
        let index = local_texture_index(self.get_package(&canonical).unwrap(), texture_name)
            .ok_or_else(|| anyhow::anyhow!("Texture '{texture_name}' missing or ambiguous in '{package_name}'"))?;
        let tex = &mut self.loaded_packages.get_mut(&canonical).unwrap().textures[index];
        let identity = texture_reference(&canonical, &tex.group, &tex.name);
        let old_format = tex.format;
        let orig_bytes = match old_format {
            3 => ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 8,
            8 | 7 => ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 16,
            1 | 5 => (tex.width as usize) * (tex.height as usize) * 4,
            _ => (tex.width as usize) * (tex.height as usize), // P8
        };

        let new_bytes = match target_format {
            3 => {
                let _ = ut99_format::encode_rgba8_to_dxt1(&tex.rgba_pixels, tex.width, tex.height, tex.masked);
                ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 8
            }
            8 => {
                let _ = ut99_format::encode_rgba8_to_dxt5(&tex.rgba_pixels, tex.width, tex.height);
                ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 16
            }
            _ => (tex.width as usize) * (tex.height as usize),
        };

        tex.format = target_format;

        // Invalidate egui thumbnail cache so it updates
        self.invalidate_thumbnail(&identity);

        let savings = if orig_bytes > 0 {
            (1.0 - (new_bytes as f32 / orig_bytes as f32)) * 100.0
        } else {
            0.0
        };

        Ok((orig_bytes, new_bytes, savings))
    }

    /// Returns format description for a texture (`P8`, `DXT1 (BC1)`, `DXT5 (BC3)`, etc.) and its VRAM footprint.
    pub fn get_texture_format_info(
        &self,
        package_name: &str,
        texture_name: &str,
    ) -> Option<(u32, &'static str, usize)> {
        let tex = self.get_texture(package_name, texture_name)?;
        let (fmt_name, vram_bytes) = match tex.format {
            3 => ("DXT1 (BC1)", ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 8),
            8 => ("DXT5 (BC3)", ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 16),
            7 => ("DXT3 (BC2)", ((tex.width as usize + 3) / 4) * ((tex.height as usize + 3) / 4) * 16),
            1 | 5 => ("RGBA8", (tex.width as usize) * (tex.height as usize) * 4),
            _ => ("P8 (Paletted)", (tex.width as usize) * (tex.height as usize) + 1024),
        };
        Some((tex.format, fmt_name, vram_bytes))
    }
}

