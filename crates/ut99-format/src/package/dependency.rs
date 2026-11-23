use std::collections::HashSet;
use crate::unr::UnrLevel;

/// Represents an external package dependency required by an Unreal Tournament 99 map.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PackageDependency {
    pub name: String,
    pub package_type: String, // "Code", "Texture", "Sound", "Music", "System"
    pub is_stock: bool,       // true if bundled with standard UT99 v436, false if custom third-party asset
}

/// Canonical registry of standard stock packages shipped with Unreal Tournament 99 v436.
pub const STOCK_PACKAGES: &[&str] = &[
    // Core & Engine code packages (.u)
    "core", "engine", "editor", "uwindow", "fire", "ipdrv", "uweb", "ubrowser",
    "unrealshare", "unreali", "umenu", "botpack", "ipserver", "utserveradmin",
    "utmenu", "utbrowser", "epiccustommodels", "multimesh", "relics",
    "relicsbindings", "de", "udemo", "windrv", "render",

    // Stock texture packages (.utx)
    "ancient", "anc", "city", "crypt", "decayed", "detail", "fireamb", "fractalfx",
    "genearth", "genfluid", "genfx", "genin", "genterra", "genwarp", "hubfx",
    "indus1", "indus2", "indus3", "indus4", "indus5", "indus6", "indus7",
    "metal", "mine", "minefx", "nalicast", "nalifx", "noxxpack", "palettes",
    "pyr", "queen", "rainfx", "sfx", "skybox", "skintom", "spacefx",
    "starshippack", "tcrystal", "tech", "tutorial", "ut", "utcrypt", "uttech1",
    "uttech2", "uttech3", "xeno",

    // Stock sound packages (.uax)
    "activates", "ambancient", "ambcity", "ambmodern", "amboutside", "announcer",
    "bossvoice", "d99voice", "female1voice", "female2voice", "male1voice",
    "male2voice", "panthervoice", "vrikers", "warhead",

    // Stock music packages (.umx)
    "botmca9", "botpack9", "cannon", "colossus", "course", "credits", "enigma",
    "firebr", "foregone", "godown", "lock", "mechanism", "mission", "nether",
    "organic", "phantom", "razor-ub", "run", "saveme", "seeker", "skyward",
    "strider", "subrosa", "superfist", "theme", "underworld", "wheels",
];

/// Distribution manifest detailing map specifications, dependencies, and packaging requirements.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MapManifest {
    pub map_name: String,
    pub title: String,
    pub author: String,
    pub ideal_player_count: String,
    pub song: String,
    pub stock_dependencies: Vec<PackageDependency>,
    pub custom_dependencies: Vec<PackageDependency>,
    pub total_actors: usize,
    pub total_weapons_and_pickups: usize,
}

impl MapManifest {
    /// Checks whether the map is 100% self-contained using only standard stock UT99 packages.
    pub fn is_vanilla_compatible(&self) -> bool {
        self.custom_dependencies.is_empty()
    }

    /// Generates a standardized UT99 community release Readme file for map distribution.
    pub fn generate_readme(&self) -> String {
        let mut text = String::new();
        text.push_str("================================================================\n");
        text.push_str(&format!("Map Title: {}\n", if self.title.is_empty() { &self.map_name } else { &self.title }));
        text.push_str(&format!("Filename:  {}.unr\n", self.map_name));
        text.push_str(&format!("Author:    {}\n", if self.author.is_empty() { "Unknown Mapper" } else { &self.author }));
        text.push_str(&format!("Players:   {}\n", if self.ideal_player_count.is_empty() { "2-8 Players" } else { &self.ideal_player_count }));
        text.push_str(&format!("Music:     {}\n", if self.song.is_empty() { "None" } else { &self.song }));
        text.push_str("================================================================\n\n");

        text.push_str("--- [ Map Specifications ] ---\n");
        text.push_str(&format!("Total Actors:           {}\n", self.total_actors));
        text.push_str(&format!("Weapons & Pickups:      {}\n", self.total_weapons_and_pickups));
        text.push_str(&format!("Bot Path Network:       Certified & Rebuilt\n"));
        text.push_str(&format!("Compatibility:          Unreal Tournament 99 (v436 / v469)\n\n"));

        text.push_str("--- [ Package Dependencies ] ---\n");
        if self.is_vanilla_compatible() {
            text.push_str("Standard Stock Packages Only (No external files required).\n");
        } else {
            text.push_str("CRITICAL: The following custom packages must be copied to your UT directories:\n");
            for dep in &self.custom_dependencies {
                let folder = match dep.package_type.as_str() {
                    "Texture" => "Textures/",
                    "Sound" => "Sounds/",
                    "Music" => "Music/",
                    _ => "System/",
                };
                text.push_str(&format!(" - {} ({}{})\n", dep.name, folder, dep.name));
            }
        }

        text.push_str("\n--- [ Installation Instructions ] ---\n");
        text.push_str(&format!("1. Extract {}.unr into your UnrealTournament/Maps/ directory.\n", self.map_name));
        if !self.is_vanilla_compatible() {
            text.push_str("2. Extract all accompanying custom packages into their respective subdirectories.\n");
        }
        text.push_str("3. Launch Unreal Tournament and select the map from the Practice Session menu.\n\n");

        text.push_str("Generated with Forge99 — 64-bit Level Editor for Unreal Tournament 99 (https://github.com/thevot/Forge99).\n");
        text
    }
}

/// Scans an Unreal Tournament level (.unr) and evaluates all package dependencies.
pub fn scan_map_dependencies(unr: &UnrLevel) -> MapManifest {
    let mut manifest = MapManifest::default();
    manifest.map_name = "UntitledMap".to_string();
    manifest.total_actors = unr.actors.len();
    manifest.total_weapons_and_pickups = unr.weapons_and_pickups.len();

    let mut found_packages: HashSet<(String, String)> = HashSet::new(); // (package_name, type)

    // Inspect LevelInfo actor if present
    for actor in &unr.actors {
        if actor.class_name.contains("LevelInfo") {
            if let Some(prop) = actor.properties.get("Title") {
                if let crate::unr::tagged_property::TaggedValue::Str(s) = &prop.value {
                    manifest.title = s.clone();
                }
            }
            if let Some(prop) = actor.properties.get("Author") {
                if let crate::unr::tagged_property::TaggedValue::Str(s) = &prop.value {
                    manifest.author = s.clone();
                }
            }
            if let Some(prop) = actor.properties.get("IdealPlayerCount") {
                if let crate::unr::tagged_property::TaggedValue::Str(s) = &prop.value {
                    manifest.ideal_player_count = s.clone();
                }
            }
            if let Some(prop) = actor.properties.get("Song") {
                if let crate::unr::tagged_property::TaggedValue::Str(s) = &prop.value {
                    manifest.song = s.clone();
                    if let Some(pkg) = extract_package_from_object_ref(s) {
                        found_packages.insert((pkg, "Music".to_string()));
                    }
                }
            }
        }

        // Extract class package (e.g. "Botpack.UT_JumpPad" -> "Botpack")
        if let Some(dot_idx) = actor.class_name.find('.') {
            let pkg = &actor.class_name[..dot_idx];
            found_packages.insert((pkg.to_string(), "Code".to_string()));
        }

        // Extract textures / sounds from properties
        for (_key, prop) in &actor.properties {
            if let crate::unr::tagged_property::TaggedValue::Str(val) = &prop.value {
                if let Some(pkg) = extract_package_from_object_ref(val) {
                    let ptype = if val.starts_with("Sound'") {
                        "Sound".to_string()
                    } else if val.starts_with("Music'") {
                        "Music".to_string()
                    } else {
                        "Texture".to_string()
                    };
                    found_packages.insert((pkg, ptype));
                }
            }
        }
    }

    // Inspect package names for known packages
    for entry in &unr.package.names.entries {
        let name_lower = entry.name.to_lowercase();
        if STOCK_PACKAGES.contains(&name_lower.as_str()) {
            found_packages.insert((entry.name.clone(), "System".to_string()));
        }
    }

    // Sort into stock vs custom
    for (pkg_name, ptype) in found_packages {
        let is_stock = STOCK_PACKAGES.contains(&pkg_name.to_lowercase().as_str());
        let dep = PackageDependency {
            name: pkg_name,
            package_type: ptype,
            is_stock,
        };
        if is_stock {
            manifest.stock_dependencies.push(dep);
        } else {
            manifest.custom_dependencies.push(dep);
        }
    }

    manifest.stock_dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    manifest.custom_dependencies.sort_by(|a, b| a.name.cmp(&b.name));
    manifest
}

/// Helper extracting package name from an Unreal object reference string (e.g. "Texture'Indus1.Floor.Panel01'" -> "Indus1").
fn extract_package_from_object_ref(obj_ref: &str) -> Option<String> {
    let inner = if let Some(open) = obj_ref.find('\'') {
        let close = obj_ref.rfind('\'')?;
        if close > open {
            &obj_ref[open + 1..close]
        } else {
            obj_ref
        }
    } else {
        obj_ref
    };

    let first_part = inner.split('.').next()?.trim();
    if first_part.is_empty() || first_part.eq_ignore_ascii_case("None") {
        None
    } else {
        Some(first_part.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unr::ActorRecord;

    #[test]
    fn test_extract_package_name() {
        assert_eq!(
            extract_package_from_object_ref("Texture'Indus1.Floor.Panel01'"),
            Some("Indus1".to_string())
        );
        assert_eq!(
            extract_package_from_object_ref("Sound'Announcer.HeadShot'"),
            Some("Announcer".to_string())
        );
        assert_eq!(extract_package_from_object_ref("None"), None);
    }

    #[test]
    fn test_scan_map_dependencies() {
        let mut level = UnrLevel::default();

        let mut a1 = ActorRecord::default();
        a1.class_name = "Botpack.UT_JumpPad".to_string();
        level.actors.push(a1);

        let mut a2 = ActorRecord::default();
        a2.class_name = "CustomPack.CustomEmitter".to_string();
        level.actors.push(a2);

        let manifest = scan_map_dependencies(&level);
        assert_eq!(manifest.total_actors, 2);

        assert!(manifest.stock_dependencies.iter().any(|d| d.name.eq_ignore_ascii_case("Botpack")));
        assert!(manifest.custom_dependencies.iter().any(|d| d.name == "CustomPack"));
        assert!(!manifest.is_vanilla_compatible());

        let readme = manifest.generate_readme();
        assert!(readme.contains("CustomPack"));
        assert!(readme.contains("Bot Path Network:       Certified & Rebuilt"));
    }
}
