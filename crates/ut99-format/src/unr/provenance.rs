//! Provenance of `.unr` maps.
//!
//! The Forge99 map wizard saves its output next to the stock maps, so any study of the stock
//! corpus must be able to tell *generated* maps from *reference* maps. Statistics, reference
//! profiles and the "DNA" cloner must never be computed from generated maps (they would learn
//! from their own output).
//!
//! This module is intentionally dependency-free (std only) so it can be unit-tested in isolation.

/// Author strings written by the wizard / editor.
///
/// * a phrase match (case-insensitive) is used for multi-word markers;
/// * single-word markers are matched as whole tokens, so `"Savot"` does not match `"vot"`.
const GENERATOR_AUTHOR_PHRASES: &[&str] = &["ut modern map editor"];
const GENERATOR_AUTHOR_TOKENS: &[&str] = &["vot", "forge99"];

/// Prefix of the machine-readable generator tag (see [`format_generator_tag`]).
pub const GENERATOR_TAG_PREFIX: &str = "Forge99Gen";

/// Where a map comes from, for corpus purposes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapProvenance {
    /// Not produced by the wizard: eligible as reference material.
    Reference,
    /// Produced by the Forge99 wizard (or tagged as such): never a reference.
    Generated,
    /// Intro, credits, logo and editor-test maps: not gameplay maps.
    Utility,
}

/// Returns true when the `Author` of a map identifies the Forge99 wizard / editor.
pub fn is_generated_author(author: &str) -> bool {
    let lower = author.to_ascii_lowercase();
    if GENERATOR_AUTHOR_PHRASES.iter().any(|p| lower.contains(p)) {
        return true;
    }
    lower
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|t| !t.is_empty())
        .any(|t| GENERATOR_AUTHOR_TOKENS.contains(&t))
}

/// Utility / intro maps that are not gameplay maps (file stem, any case).
pub fn is_utility_map_stem(stem: &str) -> bool {
    let lower = stem.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "entry" | "editortest" | "map" | "ut-logo-map" | "utcredits" | "cityintro"
    ) || lower.starts_with("eol_")
}

/// Classifies a map from its file stem and `Author` property.
pub fn classify_map(stem: &str, author: &str) -> MapProvenance {
    if is_utility_map_stem(stem) {
        MapProvenance::Utility
    } else if is_generated_author(author) {
        MapProvenance::Generated
    } else {
        MapProvenance::Reference
    }
}

/// Parsed generator tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratorTag {
    pub version: String,
    pub seed: u64,
}

/// Builds the machine-readable tag, e.g. `Forge99Gen;version=0.23.5;seed=42`.
pub fn format_generator_tag(version: &str, seed: u64) -> String {
    format!("{GENERATOR_TAG_PREFIX};version={version};seed={seed}")
}

/// Parses a tag produced by [`format_generator_tag`]. Unknown extra fields are ignored.
pub fn parse_generator_tag(text: &str) -> Option<GeneratorTag> {
    let mut parts = text.trim().split(';');
    if parts.next()? != GENERATOR_TAG_PREFIX {
        return None;
    }
    let mut version = None;
    let mut seed = None;
    for part in parts {
        let (key, value) = part.split_once('=')?;
        match key.trim() {
            "version" => version = Some(value.trim().to_string()),
            "seed" => seed = value.trim().parse::<u64>().ok(),
            _ => {}
        }
    }
    Some(GeneratorTag {
        version: version.filter(|v| !v.is_empty())?,
        seed: seed?,
    })
}

/// Extension (appended to the map stem) of the manifest written next to every wizard-generated map.
pub const GENERATOR_MANIFEST_SUFFIX: &str = ".f99.json";

/// Path of the manifest for a map file: `Maps/DM-Foo.unr` -> `Maps/DM-Foo.f99.json`.
pub fn manifest_path_for(map_path: &std::path::Path) -> std::path::PathBuf {
    let stem = map_path.file_stem().and_then(|s| s.to_str()).unwrap_or("map");
    map_path.with_file_name(format!("{stem}{GENERATOR_MANIFEST_SUFFIX}"))
}

/// True when a generator manifest exists next to `map_path`.
pub fn has_generator_manifest(map_path: &std::path::Path) -> bool {
    manifest_path_for(map_path).is_file()
}

fn json_escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Content of the manifest file (hand-written JSON, no serde needed).
pub fn format_generator_manifest(version: &str, seed: u64, mode: &str, map_name: &str) -> String {
    format!(
        "{{\n  \"generator\": \"Forge99\",\n  \"tag\": \"{}\",\n  \"version\": \"{}\",\n  \"seed\": {},\n  \"mode\": \"{}\",\n  \"map\": \"{}\"\n}}\n",
        json_escape(&format_generator_tag(version, seed)),
        json_escape(version),
        seed,
        json_escape(mode),
        json_escape(map_name)
    )
}

/// Writes the manifest next to `map_path` (overwrites an older manifest of the same map).
pub fn write_generator_manifest(
    map_path: &std::path::Path,
    version: &str,
    seed: u64,
    mode: &str,
) -> std::io::Result<std::path::PathBuf> {
    let name = map_path.file_name().and_then(|s| s.to_str()).unwrap_or("");
    let path = manifest_path_for(map_path);
    std::fs::write(&path, format_generator_manifest(version, seed, mode, name))?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_authors_are_recognised() {
        assert!(is_generated_author("UT Modern Map Editor [by VoT]"));
        assert!(is_generated_author("VoT"));
        assert!(is_generated_author("Forge99 Community Contributors"));
        assert!(is_generated_author("  ut modern map editor "));
        assert!(is_generated_author("someone, vot"));
    }

    #[test]
    fn stock_authors_are_not_flagged() {
        for a in [
            "",
            "Alan 'Talisman' Willard",
            "Cliff Bleszinski",
            "Cliff Bleszinski / Alan Willard",
            "Cedric \"Inoxx\" Fiorentino",
            "Juan Pancho ' XceptOne ' Eekels",
            "Sidney 'Clawfist' Rauchberger",
            "James Schmalz",
            "Unknown Mapper",
            "Savot",
            "Votan",
            "Pivot Studio",
        ] {
            assert!(!is_generated_author(a), "falso positivo: {a:?}");
        }
    }

    #[test]
    fn utility_maps_are_recognised() {
        for s in ["Entry", "EditorTest", "UTCredits", "UT-Logo-Map", "CityIntro", "EOL_CTF", "eol_Assault"] {
            assert!(is_utility_map_stem(s), "{s}");
        }
        for s in ["DM-Deck16][", "CTF-Face", "AS-Frigate", "DOM-Cinder"] {
            assert!(!is_utility_map_stem(s), "{s}");
        }
    }

    #[test]
    fn classification_prefers_utility_then_generated() {
        assert_eq!(classify_map("DM-Deck16][", "Cliff Bleszinski"), MapProvenance::Reference);
        assert_eq!(classify_map("DM-StarFever", "UT Modern Map Editor [by VoT]"), MapProvenance::Generated);
        assert_eq!(classify_map("EOL_CTF", "VoT"), MapProvenance::Utility);
    }

    #[test]
    fn generator_tag_round_trips() {
        let tag = format_generator_tag("0.23.5", 1234567890123);
        assert_eq!(tag, "Forge99Gen;version=0.23.5;seed=1234567890123");
        assert_eq!(
            parse_generator_tag(&tag),
            Some(GeneratorTag { version: "0.23.5".into(), seed: 1234567890123 })
        );
        assert_eq!(parse_generator_tag("Forge99Gen;version=1;seed=2;extra=x").map(|t| t.seed), Some(2));
    }

    #[test]
    fn malformed_tags_are_rejected() {
        assert_eq!(parse_generator_tag(""), None);
        assert_eq!(parse_generator_tag("Other;version=1;seed=2"), None);
        assert_eq!(parse_generator_tag("Forge99Gen;version=1"), None);
        assert_eq!(parse_generator_tag("Forge99Gen;seed=2"), None);
        assert_eq!(parse_generator_tag("Forge99Gen;version=;seed=2"), None);
        assert_eq!(parse_generator_tag("Forge99Gen;version=1;seed=abc"), None);
    }

    #[test]
    fn manifest_path_and_content() {
        let p = std::path::Path::new("Maps/DM-Foo.unr");
        assert_eq!(manifest_path_for(p), std::path::Path::new("Maps/DM-Foo.f99.json"));
        let m = format_generator_manifest("0.23.5", 42, "DM", "DM-Foo\"x.unr");
        assert!(m.contains("\"tag\": \"Forge99Gen;version=0.23.5;seed=42\""));
        assert!(m.contains("\"seed\": 42"));
        assert!(m.contains("DM-Foo\\\"x.unr"));
    }

    #[test]
    fn manifest_round_trip_on_disk() {
        let dir = std::env::temp_dir().join(format!("f99_prov_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let map = dir.join("CTF-Test.unr");
        assert!(!has_generator_manifest(&map));
        let written = write_generator_manifest(&map, "1.0.0", 7, "CTF").unwrap();
        assert_eq!(written, dir.join("CTF-Test.f99.json"));
        assert!(has_generator_manifest(&map));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
