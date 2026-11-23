use glam::Vec3;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::level::UnrLevel;
use super::provenance;
use super::tagged_property::TaggedValue;

const PF_MASKED: u32 = 0x0000_0002;
const PF_TRANSLUCENT: u32 = 0x0000_0004;
const PF_FAKE_BACKDROP: u32 = 0x0000_0080;
const PF_SMALL_WAVY: u32 = 0x0000_2000;
const PF_SEMISOLID: u32 = 0x0020_0000;
const PF_PORTAL: u32 = 0x0400_0000;

/// Categorized texture palette extracted from a real UT99 `.unr` map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialTexturePalette {
    pub floor_textures: Vec<(String, usize)>,
    pub ceiling_textures: Vec<(String, usize)>,
    pub wall_textures: Vec<(String, usize)>,
    pub trim_and_detail_textures: Vec<(String, usize)>,
    pub liquid_and_portal_textures: Vec<(String, usize)>,
    pub sky_textures: Vec<(String, usize)>,
    pub texture_packages: Vec<(String, usize)>,
}

impl OfficialTexturePalette {
    pub fn primary_wall(&self, fallback: &str) -> String {
        self.wall_textures
            .first()
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn primary_floor(&self, fallback: &str) -> String {
        self.floor_textures
            .first()
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn primary_ceiling(&self, fallback: &str) -> String {
        self.ceiling_textures
            .first()
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn primary_trim(&self, fallback: &str) -> String {
        self.trim_and_detail_textures
            .first()
            .or_else(|| self.wall_textures.get(1))
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn primary_liquid(&self, fallback: &str) -> String {
        self.liquid_and_portal_textures
            .first()
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }

    pub fn primary_sky(&self, fallback: &str) -> String {
        self.sky_textures
            .first()
            .map(|(t, _)| t.clone())
            .unwrap_or_else(|| fallback.to_string())
    }
}

/// Lighting and atmospheric metrics extracted from a real UT99 `.unr` map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialLightingDna {
    pub light_count: usize,
    pub primary_hue: u8,
    pub primary_saturation: u8,
    pub avg_brightness: u8,
    pub avg_radius: u8,
    pub accent_hue: u8,
    pub accent_saturation: u8,
    pub has_sky_zone: bool,
    pub water_zone_count: usize,
    pub lava_zone_count: usize,
    pub slime_zone_count: usize,
    #[serde(default)]
    pub corona_light_count: usize,
    #[serde(default)]
    pub special_lit_light_count: usize,
    #[serde(default)]
    pub spotlight_count: usize,
    #[serde(default)]
    pub dynamic_effect_count: usize,
    #[serde(default)]
    pub distinct_hue_buckets: usize,
    #[serde(default)]
    pub top_corona_skins: Vec<(String, usize)>,
}

/// Architectural, CSG morphology, and zoning details extracted from an official map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialArchitecturalDna {
    pub semisolid_ratio_pct: f32,
    pub niche_coffer_count: usize,
    pub sloped_wall_count: usize,
    pub thin_trim_count: usize,
    pub pillar_count: usize,
    pub beam_count: usize,
    pub multi_tex_sub_ratio_pct: f32,
    pub named_zones: Vec<String>,
}

/// Skybox architecture and celestial backdrop metrics extracted from an official map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialSkyboxDna {
    pub has_skybox: bool,
    pub has_rotating_or_rocking_sky: bool,
    pub sky_brush_count: usize,
    pub primary_sky_package: String,
    pub primary_sky_texture: String,
    pub cloud_pan_texture: String,
    pub nebula_or_planet_texture: String,
}

/// Verticality, pacing, and mover dynamics extracted from an official map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialVerticalityAndFlowDna {
    pub playable_z_span: f32,
    pub distinct_z_tiers: usize,
    pub avg_spawn_to_weapon_dist: f32,
    pub min_major_powerup_separation: f32,
    pub stand_open_mover_count: usize,
    pub trigger_open_mover_count: usize,
    pub trigger_control_mover_count: usize,
}

/// Gameplay, combat item distribution, and bot navigation metrics extracted from a real `.unr` map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialGameplayDna {
    pub player_start_count: usize,
    pub path_node_count: usize,
    pub inventory_spot_count: usize,
    pub lift_center_count: usize,
    pub lift_exit_count: usize,
    pub teleporter_count: usize,
    pub kicker_count: usize,
    pub defense_point_count: usize,
    pub alternate_path_count: usize,
    pub flag_base_count: usize,
    pub control_point_count: usize,
    pub fort_standard_count: usize,
    pub weapon_classes: Vec<(String, usize)>,
    pub ammo_classes: Vec<(String, usize)>,
    pub health_armor_classes: Vec<(String, usize)>,
    pub powerup_classes: Vec<(String, usize)>,
    pub decoration_classes: Vec<(String, usize)>,
    pub trigger_and_special_classes: Vec<(String, usize)>,
    pub ambient_sounds: Vec<String>,
}

/// Complete forensic DNA profile extracted from an official or custom UT99 `.unr` map.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct OfficialMapDna {
    pub map_name: String,
    pub mode_prefix: String,
    pub title: String,
    pub author: String,
    pub ideal_player_count: String,
    pub song: String,
    pub default_game_type: String,
    pub ambient_brightness: u8,
    pub ambient_hue: u8,
    pub ambient_saturation: u8,
    pub total_brushes: usize,
    pub subtract_brushes: usize,
    pub add_brushes: usize,
    pub semisolid_brushes: usize,
    pub portal_brushes: usize,
    pub mover_count: usize,
    pub bsp_node_count: usize,
    pub bsp_surf_count: usize,
    pub zone_count: usize,
    pub reach_spec_count: usize,
    pub world_bounds_min: Vec3,
    pub world_bounds_max: Vec3,
    pub world_extent: Vec3,
    pub textures: OfficialTexturePalette,
    pub lighting: OfficialLightingDna,
    pub gameplay: OfficialGameplayDna,
    #[serde(default)]
    pub architectural: OfficialArchitecturalDna,
    #[serde(default)]
    pub skybox: OfficialSkyboxDna,
    #[serde(default)]
    pub verticality: OfficialVerticalityAndFlowDna,
    /// Recoverable decode problems; empty does not guarantee a complete parse.
    #[serde(default)]
    pub read_issues: Vec<super::level::UnrReadIssue>,
}

/// Aggregated statistics per game mode across an entire corpus of UT99 maps.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GameModeCorpusStats {
    pub mode_prefix: String,
    pub map_count: usize,
    pub avg_brushes: f32,
    pub avg_subtract_ratio_pct: f32,
    pub avg_semisolid_ratio_pct: f32,
    pub avg_movers: f32,
    pub avg_player_starts: f32,
    pub avg_lights: f32,
    pub avg_path_nodes: f32,
    pub avg_reach_specs: f32,
    pub top_texture_packages: Vec<(String, usize)>,
    pub top_weapons: Vec<(String, usize)>,
    pub top_decorations: Vec<(String, usize)>,
    pub top_songs: Vec<(String, usize)>,
}

fn clean_object_path(raw: &str) -> String {
    let trimmed = raw.trim();
    let inner = if let Some(open) = trimmed.find('\'') {
        if let Some(close) = trimmed.rfind('\'') {
            if close > open {
                &trimmed[open + 1..close]
            } else {
                trimmed
            }
        } else {
            trimmed
        }
    } else {
        trimmed
    };
    inner.trim().to_string()
}

fn sorted_top_counts(map: HashMap<String, usize>, limit: usize) -> Vec<(String, usize)> {
    let mut vec: Vec<(String, usize)> = map.into_iter().collect();
    vec.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    vec.truncate(limit);
    vec
}

impl OfficialMapDna {
    /// Opens a `.unr` file from disk and extracts its complete `OfficialMapDna`.
    pub fn from_unr_file<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        let level = UnrLevel::open(path)?;
        Ok(Self::from_unr_level(&level))
    }

    /// Extracts the complete `OfficialMapDna` from an already-parsed `UnrLevel`.
    pub fn from_unr_level(level: &UnrLevel) -> Self {
        let mut dna = OfficialMapDna {
            map_name: level.map_name.clone(),
            read_issues: level.read_issues.clone(),
            mode_prefix: level
                .map_name
                .split('-')
                .next()
                .unwrap_or("DM")
                .to_ascii_uppercase(),
            reach_spec_count: level.reach_specs.len(),
            ..Default::default()
        };

        if let Some(ref bsp) = level.world_bsp {
            dna.bsp_node_count = bsp.nodes.len();
            dna.bsp_surf_count = bsp.surfs.len();
            dna.zone_count = bsp.zones.len();
        }

        // 1. Inspect LevelInfo and Actors
        let mut weapon_counts: HashMap<String, usize> = HashMap::new();
        let mut ammo_counts: HashMap<String, usize> = HashMap::new();
        let mut health_armor_counts: HashMap<String, usize> = HashMap::new();
        let mut powerup_counts: HashMap<String, usize> = HashMap::new();
        let mut decoration_counts: HashMap<String, usize> = HashMap::new();
        let mut special_counts: HashMap<String, usize> = HashMap::new();
        let mut ambient_sounds: Vec<String> = Vec::new();
        let mut corona_skins: HashMap<String, usize> = HashMap::new();
        let mut named_zones: Vec<String> = Vec::new();

        let mut spawn_locs: Vec<Vec3> = Vec::new();
        let mut weapon_locs: Vec<Vec3> = Vec::new();
        let mut major_powerup_locs: Vec<Vec3> = Vec::new();
        let mut nav_zs: Vec<f32> = Vec::new();

        let mut hue_hist: HashMap<u8, usize> = HashMap::new();
        let mut sat_sum = 0usize;
        let mut bright_sum = 0usize;
        let mut rad_sum = 0usize;
        let mut valid_lights = 0usize;

        for actor in &level.actors {
            let cls_low = actor.class_name.to_ascii_lowercase();
            let short_cls = actor.class_name.rsplit('.').next().unwrap_or(&actor.class_name);
            let short_low = short_cls.to_ascii_lowercase();

            if short_low == "levelinfo" {
                if let Some(p) = actor.properties.get("Title") {
                    if let TaggedValue::Str(ref s) = p.value {
                        dna.title = s.clone();
                    }
                }
                if let Some(p) = actor.properties.get("Author") {
                    if let TaggedValue::Str(ref s) = p.value {
                        dna.author = s.clone();
                    }
                }
                if let Some(p) = actor.properties.get("IdealPlayerCount") {
                    if let TaggedValue::Str(ref s) = p.value {
                        dna.ideal_player_count = s.clone();
                    }
                }
                if let Some(s) = actor.object_paths.get("Song") {
                    dna.song = clean_object_path(s);
                } else if let Some(p) = actor.properties.get("Song") {
                    if let TaggedValue::Str(ref s) = p.value {
                        dna.song = clean_object_path(s);
                    }
                }
                if let Some(gt) = actor.object_paths.get("DefaultGameType") {
                    dna.default_game_type = clean_object_path(gt);
                } else if let Some(p) = actor.properties.get("DefaultGameType") {
                    if let TaggedValue::Str(ref s) = p.value {
                        dna.default_game_type = clean_object_path(s);
                    }
                }
                if let Some(p) = actor.properties.get("AmbientBrightness") {
                    if let TaggedValue::Byte(b) = p.value {
                        dna.ambient_brightness = b;
                    }
                }
                if let Some(p) = actor.properties.get("AmbientHue") {
                    if let TaggedValue::Byte(b) = p.value {
                        dna.ambient_hue = b;
                    }
                }
                if let Some(p) = actor.properties.get("AmbientSaturation") {
                    if let TaggedValue::Byte(b) = p.value {
                        dna.ambient_saturation = b;
                    }
                }
            } else if short_low.contains("mover") {
                dna.mover_count += 1;
                if short_low != "mover" {
                    *special_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                }
                if let Some(p) = actor.properties.get("InitialState") {
                    let st = match &p.value {
                        TaggedValue::Name(s) | TaggedValue::Str(s) => s.to_ascii_lowercase(),
                        _ => String::new(),
                    };
                    if st.contains("standopen") {
                        dna.verticality.stand_open_mover_count += 1;
                    } else if st.contains("triggercontrol") {
                        dna.verticality.trigger_control_mover_count += 1;
                    } else if st.contains("triggeropen") {
                        dna.verticality.trigger_open_mover_count += 1;
                    }
                } else {
                    // Default UE1 mover initial state
                    dna.verticality.stand_open_mover_count += 1;
                }
            } else if short_low.contains("light") && !short_low.contains("sunlight") && !short_low.contains("lightbox") && !short_low.contains("lightcone") && !short_low.contains("tubelight") {
                dna.lighting.light_count += 1;
                let is_corona = actor.properties.get("bCorona").map(|p| matches!(p.value, TaggedValue::Bool(true))).unwrap_or(false);
                if is_corona {
                    dna.lighting.corona_light_count += 1;
                    if let Some(skin) = actor.object_paths.get("Skin") {
                        let c_skin = clean_object_path(skin);
                        if !c_skin.is_empty() && !c_skin.eq_ignore_ascii_case("None") {
                            *corona_skins.entry(c_skin).or_insert(0) += 1;
                        }
                    }
                }
                let is_special_lit = actor.properties.get("bSpecialLit").map(|p| matches!(p.value, TaggedValue::Bool(true))).unwrap_or(false);
                if is_special_lit {
                    dna.lighting.special_lit_light_count += 1;
                }
                if short_low == "spotlight" || actor.light_effect == 12 {
                    dna.lighting.spotlight_count += 1;
                }
                if actor.light_effect != 0 && actor.light_effect != 13 {
                    dna.lighting.dynamic_effect_count += 1;
                }
                if actor.light_brightness > 0 && actor.light_radius > 0 {
                    // Quantize hue into 16 buckets (step 16) for dominant color detection
                    let bucket = (actor.light_hue / 16) * 16;
                    *hue_hist.entry(bucket).or_insert(0) += 1;
                    sat_sum += actor.light_saturation as usize;
                    bright_sum += actor.light_brightness as usize;
                    rad_sum += actor.light_radius as usize;
                    valid_lights += 1;
                }
            } else if short_low == "playerstart" {
                dna.gameplay.player_start_count += 1;
                spawn_locs.push(actor.location);
                nav_zs.push(actor.location.z);
            } else if short_low == "pathnode" {
                dna.gameplay.path_node_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "inventoryspot" {
                dna.gameplay.inventory_spot_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "liftcenter" {
                dna.gameplay.lift_center_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "liftexit" {
                dna.gameplay.lift_exit_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low.contains("teleporter") {
                dna.gameplay.teleporter_count += 1;
                *special_counts.entry(short_cls.to_string()).or_insert(0) += 1;
            } else if short_low == "kicker" || short_low.contains("jumppad") {
                dna.gameplay.kicker_count += 1;
                *special_counts.entry(short_cls.to_string()).or_insert(0) += 1;
            } else if short_low == "defensepoint" {
                dna.gameplay.defense_point_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "alternatepath" {
                dna.gameplay.alternate_path_count += 1;
            } else if short_low == "flagbase" {
                dna.gameplay.flag_base_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "controlpoint" {
                dna.gameplay.control_point_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low == "fortstandard" {
                dna.gameplay.fort_standard_count += 1;
                nav_zs.push(actor.location.z);
            } else if short_low.contains("skyzoneinfo") {
                dna.lighting.has_sky_zone = true;
                dna.skybox.has_skybox = true;
                if short_low.contains("rocking")
                    || actor.rotation != (0, 0, 0)
                    || actor.properties.contains_key("bFixedRotationDir")
                    || actor.properties.contains_key("RotationRate")
                {
                    dna.skybox.has_rotating_or_rocking_sky = true;
                }
            } else if short_low == "waterzone" {
                dna.lighting.water_zone_count += 1;
            } else if short_low == "lavazone" {
                dna.lighting.lava_zone_count += 1;
            } else if short_low == "slimezone" || short_low == "nitrogenzone" {
                dna.lighting.slime_zone_count += 1;
            } else {
                // Classify Weapons, Ammo, Health/Armor, Powerups, Decorations & Special Triggers
                let is_weapon = matches!(
                    short_low.as_str(),
                    "shockrifle"
                        | "supershockrifle"
                        | "ut_eightball"
                        | "ut_flakcannon"
                        | "pulsegun"
                        | "ripper"
                        | "minigun2"
                        | "sniperrifle"
                        | "gesbiorifle"
                        | "ut_biorifle"
                        | "enforcer"
                        | "doubleenforcer"
                        | "impacthammer"
                        | "chainsaw"
                        | "warheadlauncher"
                        | "translocator"
                ) || cls_low.contains("weapon");

                let is_ammo = matches!(
                    short_low.as_str(),
                    "shockcore"
                        | "rocketpack"
                        | "flakammo"
                        | "pammo"
                        | "bladehopper"
                        | "miniammo"
                        | "bulletbox"
                        | "bioammo"
                        | "eclip"
                ) || short_low.contains("ammo");

                let is_health_armor = matches!(
                    short_low.as_str(),
                    "healthvial"
                        | "medbox"
                        | "healthpack"
                        | "thighpads"
                        | "armor2"
                        | "ut_shieldbelt"
                        | "shieldbelt"
                        | "kevlar"
                );

                let is_powerup = matches!(
                    short_low.as_str(),
                    "udamage"
                        | "ut_jumpboots"
                        | "jumpboots"
                        | "ut_invisibility"
                        | "invisibility"
                        | "scuba"
                );

                let is_decoration = matches!(
                    short_low.as_str(),
                    "torchflame"
                        | "steelbarrel"
                        | "barrel"
                        | "tarydiumbarrel"
                        | "lamp1"
                        | "lamp4"
                        | "lantern"
                        | "lantern2"
                        | "fan2"
                        | "lightbox"
                        | "lightcone"
                        | "tubelight"
                        | "monkstatue"
                        | "urn"
                        | "vase"
                        | "pottery0"
                        | "pottery1"
                        | "pottery2"
                        | "banner1"
                        | "flag1"
                        | "flag2"
                        | "flag3"
                        | "boulder"
                        | "boulder1"
                        | "chest"
                        | "chair"
                        | "table"
                        | "book"
                        | "candle"
                        | "candle2"
                        | "sign1"
                        | "wire"
                        | "pipe"
                        | "steelbox"
                        | "woodenbox"
                        | "medwoodbox"
                        | "smallwoodbox"
                        | "crystal"
                        | "robot"
                ) || short_low.starts_with("plant")
                    || short_low.starts_with("tree")
                    || cls_low.contains("decoration")
                    || actor.object_paths.contains_key("Mesh"); // Custom 3D mesh decoration/prop

                let is_special = matches!(
                    short_low.as_str(),
                    "trigger"
                        | "teamtrigger"
                        | "timedtrigger"
                        | "counter"
                        | "dispatcher"
                        | "musicevent"
                        | "specialevent"
                        | "earthquake"
                        | "explosionchain"
                        | "breakingglass"
                        | "smokegenerator"
                        | "sparkbit"
                        | "warpzoneinfo"
                        | "spectatorcam"
                        | "interpolationpoint"
                ) || short_low.contains("trigger");

                if is_weapon {
                    *weapon_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                    weapon_locs.push(actor.location);
                    nav_zs.push(actor.location.z);
                    if short_low == "warheadlauncher" {
                        major_powerup_locs.push(actor.location);
                    }
                } else if is_ammo {
                    *ammo_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                } else if is_health_armor {
                    *health_armor_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                    if matches!(short_low.as_str(), "healthpack" | "armor2" | "ut_shieldbelt" | "shieldbelt") {
                        major_powerup_locs.push(actor.location);
                    }
                } else if is_powerup {
                    *powerup_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                    major_powerup_locs.push(actor.location);
                } else if is_special {
                    *special_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                } else if is_decoration
                    && !matches!(
                        short_low.as_str(),
                        "brush"
                            | "camera"
                            | "zoneinfo"
                            | "ambientsound"
                            | "Blockall"
                            | "blockall"
                            | "blockmonsters"
                            | "locationid"
                            | "translocdest"
                    )
                {
                    *decoration_counts.entry(short_cls.to_string()).or_insert(0) += 1;
                }
            }

            // Collect ZoneName and LocationName strings
            if short_low.contains("zoneinfo")
                || short_low == "locationid"
                || short_low == "waterzone"
                || short_low == "lavazone"
                || short_low == "slimezone"
            {
                let maybe_name = actor
                    .properties
                    .get("ZoneName")
                    .or_else(|| actor.properties.get("LocationName"));
                if let Some(zn) = maybe_name {
                    let s = match &zn.value {
                        TaggedValue::Str(s) | TaggedValue::Name(s) => s.trim().to_string(),
                        _ => String::new(),
                    };
                    if !s.is_empty()
                        && !named_zones.iter().any(|existing| existing.eq_ignore_ascii_case(&s))
                        && named_zones.len() < 16
                    {
                        named_zones.push(s);
                    }
                }
            }

            if let Some(snd) = actor.object_paths.get("AmbientSound") {
                let cleaned = clean_object_path(snd);
                if !cleaned.is_empty() && !cleaned.eq_ignore_ascii_case("None") && !ambient_sounds.contains(&cleaned) {
                    if ambient_sounds.len() < 8 {
                        ambient_sounds.push(cleaned);
                    }
                }
            }
        }

        if valid_lights > 0 {
            let mut hues: Vec<(u8, usize)> = hue_hist.into_iter().collect();
            hues.sort_by(|a, b| b.1.cmp(&a.1));
            dna.lighting.primary_hue = hues.first().map(|h| h.0).unwrap_or(0);
            dna.lighting.accent_hue = hues.get(1).or(hues.first()).map(|h| h.0).unwrap_or(160);
            dna.lighting.primary_saturation = (sat_sum / valid_lights).min(255) as u8;
            dna.lighting.accent_saturation = dna.lighting.primary_saturation.saturating_sub(32);
            dna.lighting.avg_brightness = (bright_sum / valid_lights).min(255) as u8;
            dna.lighting.avg_radius = (rad_sum / valid_lights).min(255) as u8;
            dna.lighting.distinct_hue_buckets = hues.len();
        }
        dna.lighting.top_corona_skins = sorted_top_counts(corona_skins, 6);

        // Compute Spawn -> Nearest Weapon distance average
        if !spawn_locs.is_empty() && !weapon_locs.is_empty() {
            let mut sum_dist = 0.0;
            for sp in &spawn_locs {
                let nearest = weapon_locs.iter().map(|w| sp.distance(*w)).fold(f32::MAX, f32::min);
                sum_dist += nearest;
            }
            dna.verticality.avg_spawn_to_weapon_dist = sum_dist / (spawn_locs.len() as f32);
        }

        // Compute Minimum Major Powerup Separation
        if major_powerup_locs.len() >= 2 {
            let mut min_sep = f32::MAX;
            for i in 0..major_powerup_locs.len() {
                for j in (i + 1)..major_powerup_locs.len() {
                    let d = major_powerup_locs[i].distance(major_powerup_locs[j]);
                    if d > 16.0 && d < min_sep {
                        min_sep = d;
                    }
                }
            }
            if min_sep < f32::MAX {
                dna.verticality.min_major_powerup_separation = min_sep;
            }
        }

        // Compute Playable Z-span & distinct Z tiers
        if nav_zs.len() >= 4 {
            nav_zs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let z_lo = nav_zs[nav_zs.len() / 20]; // 5th percentile
            let z_hi = nav_zs[nav_zs.len() * 19 / 20]; // 95th percentile
            dna.verticality.playable_z_span = (z_hi - z_lo).max(0.0);

            let mut tiers = 1usize;
            let mut cur_z = nav_zs[0];
            for &z in &nav_zs[1..] {
                if z - cur_z >= 140.0 {
                    tiers += 1;
                    cur_z = z;
                }
            }
            dna.verticality.distinct_z_tiers = tiers;
        }

        dna.gameplay.weapon_classes = sorted_top_counts(weapon_counts, 12);
        dna.gameplay.ammo_classes = sorted_top_counts(ammo_counts, 12);
        dna.gameplay.health_armor_classes = sorted_top_counts(health_armor_counts, 10);
        dna.gameplay.powerup_classes = sorted_top_counts(powerup_counts, 8);
        dna.gameplay.decoration_classes = sorted_top_counts(decoration_counts, 12);
        dna.gameplay.trigger_and_special_classes = sorted_top_counts(special_counts, 10);
        dna.gameplay.ambient_sounds = ambient_sounds;

        // 2. Inspect Brushes, World Bounds, CSG Strategy & Texture Categories
        let mut min_pt = Vec3::splat(f32::MAX);
        let mut max_pt = Vec3::splat(f32::MIN);
        let mut has_verts = false;

        let mut sub_brush_count = 0usize;
        let mut multi_tex_sub_count = 0usize;
        let mut niche_coffer_count = 0usize;
        let mut sloped_wall_count = 0usize;
        let mut thin_trim_count = 0usize;
        let mut pillar_count = 0usize;
        let mut beam_count = 0usize;

        let mut floor_tex: HashMap<String, usize> = HashMap::new();
        let mut ceiling_tex: HashMap<String, usize> = HashMap::new();
        let mut wall_tex: HashMap<String, usize> = HashMap::new();
        let mut trim_tex: HashMap<String, usize> = HashMap::new();
        let mut liquid_tex: HashMap<String, usize> = HashMap::new();
        let mut sky_tex: HashMap<String, usize> = HashMap::new();
        let mut pkg_counts: HashMap<String, usize> = HashMap::new();

        dna.total_brushes = level.brushes.len();
        for (b_idx, brush) in level.brushes.iter().enumerate() {
            // Skip the default builder brush at index 0 if it has csg_oper == 0 and isn't a mover
            let cls_low = brush.actor.class_name.to_ascii_lowercase();
            let is_mover = cls_low.contains("mover");
            if b_idx == 0 && brush.actor.csg_oper == 0 && !is_mover && level.brushes.len() > 1 {
                continue;
            }

            let is_subtract = brush.actor.csg_oper == 2;
            let mut is_semisolid = (brush.actor.poly_flags & (PF_SEMISOLID | 0x00000020)) != 0;
            let mut is_portal = (brush.actor.poly_flags & PF_PORTAL) != 0;

            let mut b_min = Vec3::splat(f32::MAX);
            let mut b_max = Vec3::splat(f32::MIN);
            let mut has_brush_verts = false;
            let mut has_sloped = false;

            for wp in &brush.world_polys {
                if (wp.poly_flags & (PF_SEMISOLID | 0x00000020)) != 0 {
                    is_semisolid = true;
                }
                if (wp.poly_flags & PF_PORTAL) != 0 {
                    is_portal = true;
                }
                let n = wp.normal.normalize_or_zero();
                if n.length_squared() > 0.5 && n.x.abs() <= 0.98 && n.y.abs() <= 0.98 && n.z.abs() <= 0.98 {
                    has_sloped = true;
                }
                for &v in &wp.vertices {
                    if v.x.abs() < 32760.0 && v.y.abs() < 32760.0 && v.z.abs() < 32760.0 {
                        min_pt = min_pt.min(v);
                        max_pt = max_pt.max(v);
                        has_verts = true;
                        b_min = b_min.min(v);
                        b_max = b_max.max(v);
                        has_brush_verts = true;
                    }
                }
            }

            let b_size = if has_brush_verts && b_min.x <= b_max.x {
                (b_max - b_min).max(Vec3::ZERO)
            } else {
                Vec3::splat(64.0)
            };
            let max_h = b_size.x.max(b_size.y);
            let min_h = b_size.x.min(b_size.y);

            if !is_mover {
                if is_subtract {
                    dna.subtract_brushes += 1;
                    sub_brush_count += 1;
                    if has_sloped {
                        sloped_wall_count += 1;
                    }
                    if min_h < 192.0 || b_size.z < 160.0 {
                        niche_coffer_count += 1;
                    }
                } else if is_portal {
                    dna.portal_brushes += 1;
                } else if is_semisolid {
                    dna.semisolid_brushes += 1;
                    let min_dim = b_size.x.min(b_size.y).min(b_size.z);
                    if min_dim <= 36.0 {
                        thin_trim_count += 1;
                    } else if b_size.z > 2.0 * max_h && max_h <= 256.0 {
                        pillar_count += 1;
                    } else if max_h > 2.5 * b_size.z && b_size.z <= 128.0 {
                        beam_count += 1;
                    }
                } else if brush.actor.csg_oper == 1 {
                    dna.add_brushes += 1;
                }
            }

            // Classify polygon textures
            let mut local_distinct_tex = std::collections::HashSet::new();
            let tex_list = level.brush_poly_textures.get(b_idx);
            for (p_idx, wp) in brush.world_polys.iter().enumerate() {
                let raw_tex = tex_list
                    .and_then(|list| list.get(p_idx))
                    .map(String::as_str)
                    .unwrap_or("");
                let cleaned = clean_object_path(raw_tex);
                if cleaned.is_empty()
                    || cleaned.eq_ignore_ascii_case("None")
                    || cleaned.to_ascii_lowercase().starts_with("mylevel.")
                    || cleaned.to_ascii_lowercase().contains("defaulttexture")
                {
                    continue;
                }

                local_distinct_tex.insert(cleaned.to_ascii_lowercase());

                if let Some(pkg) = cleaned.split('.').next() {
                    if !pkg.is_empty() {
                        *pkg_counts.entry(pkg.to_string()).or_insert(0) += 1;
                    }
                }

                let flags = wp.poly_flags | brush.actor.poly_flags;
                // Note: for a Subtract brush, polygon normals point outward into solid space (or inward depending on winding);
                // in UE1 local brush polys, top face has normal +Z and bottom face has normal -Z.
                let nz = wp.normal.z;
                if (flags & PF_FAKE_BACKDROP) != 0 {
                    *sky_tex.entry(cleaned).or_insert(0) += 1;
                } else if (flags & (PF_PORTAL | PF_TRANSLUCENT | PF_SMALL_WAVY)) != 0 {
                    if (flags & PF_MASKED) == 0 {
                        *liquid_tex.entry(cleaned).or_insert(0) += 1;
                    } else {
                        *trim_tex.entry(cleaned).or_insert(0) += 1;
                    }
                } else if is_semisolid || is_mover || (flags & PF_MASKED) != 0 {
                    *trim_tex.entry(cleaned).or_insert(0) += 1;
                } else if is_subtract {
                    // In UE1 raw brush polys before BSP inversion: bottom floor poly of a cube has normal (0,0,-1) and top ceiling has (0,0,+1)
                    if nz < -0.65 {
                        *floor_tex.entry(cleaned).or_insert(0) += 1;
                    } else if nz > 0.65 {
                        *ceiling_tex.entry(cleaned).or_insert(0) += 1;
                    } else {
                        *wall_tex.entry(cleaned).or_insert(0) += 1;
                    }
                } else if nz > 0.65 {
                    *floor_tex.entry(cleaned).or_insert(0) += 1;
                } else if nz < -0.65 {
                    *ceiling_tex.entry(cleaned).or_insert(0) += 1;
                } else {
                    *wall_tex.entry(cleaned).or_insert(0) += 1;
                }
            }

            if is_subtract && local_distinct_tex.len() >= 3 {
                multi_tex_sub_count += 1;
            }
        }

        // Also inspect compiled Model_World BSP surfs if brush_poly_textures were sparse
        if let Some(ref bsp) = level.world_bsp {
            for surf in &bsp.surfs {
                if surf.texture_ref == 0 {
                    continue;
                }
                let full = level.package.resolve_full_object_path(surf.texture_ref);
                let cleaned = clean_object_path(&full);
                if cleaned.is_empty()
                    || cleaned.eq_ignore_ascii_case("None")
                    || cleaned.to_ascii_lowercase().starts_with("mylevel.")
                    || cleaned.to_ascii_lowercase().contains("defaulttexture")
                {
                    continue;
                }
                if let Some(pkg) = cleaned.split('.').next() {
                    if !pkg.is_empty() {
                        *pkg_counts.entry(pkg.to_string()).or_insert(0) += 1;
                    }
                }
                let n = bsp
                    .vectors
                    .get(surf.v_normal.max(0) as usize)
                    .copied()
                    .unwrap_or(Vec3::Z);
                let flags = surf.poly_flags;
                if (flags & PF_FAKE_BACKDROP) != 0 {
                    *sky_tex.entry(cleaned).or_insert(0) += 1;
                } else if (flags & (PF_PORTAL | PF_TRANSLUCENT | PF_SMALL_WAVY)) != 0 {
                    *liquid_tex.entry(cleaned).or_insert(0) += 1;
                } else if (flags & (PF_SEMISOLID | 0x00000020)) != 0 {
                    *trim_tex.entry(cleaned).or_insert(0) += 1;
                } else if n.z > 0.65 {
                    // In compiled BSP surfs, normals point INTO playable air (+Z is floor!)
                    *floor_tex.entry(cleaned).or_insert(0) += 1;
                } else if n.z < -0.65 {
                    *ceiling_tex.entry(cleaned).or_insert(0) += 1;
                } else {
                    *wall_tex.entry(cleaned).or_insert(0) += 1;
                }
            }
        }

        if has_verts {
            dna.world_bounds_min = min_pt;
            dna.world_bounds_max = max_pt;
            dna.world_extent = max_pt - min_pt;
        }

        dna.textures = OfficialTexturePalette {
            floor_textures: sorted_top_counts(floor_tex, 8),
            ceiling_textures: sorted_top_counts(ceiling_tex, 8),
            wall_textures: sorted_top_counts(wall_tex, 10),
            trim_and_detail_textures: sorted_top_counts(trim_tex, 8),
            liquid_and_portal_textures: sorted_top_counts(liquid_tex, 6),
            sky_textures: sorted_top_counts(sky_tex, 6),
            texture_packages: sorted_top_counts(pkg_counts, 8),
        };

        dna.architectural = OfficialArchitecturalDna {
            semisolid_ratio_pct: if dna.total_brushes > 0 {
                (dna.semisolid_brushes as f32 / dna.total_brushes as f32) * 100.0
            } else {
                0.0
            },
            niche_coffer_count,
            sloped_wall_count,
            thin_trim_count,
            pillar_count,
            beam_count,
            multi_tex_sub_ratio_pct: if sub_brush_count > 0 {
                (multi_tex_sub_count as f32 / sub_brush_count as f32) * 100.0
            } else {
                0.0
            },
            named_zones,
        };

        dna.skybox.primary_sky_texture = dna.textures.primary_sky("ShaneSky.STARS");
        if let Some(pkg) = dna.skybox.primary_sky_texture.split('.').next() {
            dna.skybox.primary_sky_package = pkg.to_string();
        }
        for (tex, _) in &dna.textures.sky_textures {
            let t_low = tex.to_ascii_lowercase();
            if dna.skybox.cloud_pan_texture.is_empty()
                && (t_low.contains("skytst") || t_low.contains("cloud") || t_low.contains("gmist") || t_low.contains("psky"))
            {
                dna.skybox.cloud_pan_texture = tex.clone();
            }
            if dna.skybox.nebula_or_planet_texture.is_empty()
                && (t_low.contains("nebula") || t_low.contains("rpl") || t_low.contains("lensflar") || t_low.contains("flare"))
            {
                dna.skybox.nebula_or_planet_texture = tex.clone();
            }
        }

        dna
    }
}

#[derive(Debug, Clone, Default)]
pub struct CorpusScanReport {
    pub profiles: Vec<OfficialMapDna>,
    pub excluded: Vec<String>,
    pub failures: Vec<(String, String)>,
    /// Maps produced by the Forge99 wizard (see [`provenance`]). They are parsed but never become
    /// profiles: reference statistics must not be computed from the generator's own output.
    pub generated: Vec<String>,
}

/// Scan every directory entry, explicitly recording excluded UNRs and decode failures.
pub fn scan_maps_directory_report<P: AsRef<Path>>(maps_dir: P) -> std::io::Result<CorpusScanReport> {
    let mut report = CorpusScanReport::default();
    let entries = std::fs::read_dir(maps_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        if !ext.eq_ignore_ascii_case("unr") {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        // Skip tiny utility/intro maps unless requested
        if provenance::is_utility_map_stem(stem) {
            report.excluded.push(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
            continue;
        }
        // A manifest written by the wizard marks the map as generated regardless of its Author.
        if provenance::has_generator_manifest(&path) {
            report
                .generated
                .push(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
            continue;
        }
        match OfficialMapDna::from_unr_file(&path) {
            Ok(dna) => {
                if provenance::is_generated_author(&dna.author) {
                    report
                        .generated
                        .push(path.file_name().unwrap_or_default().to_string_lossy().into_owned());
                } else {
                    report.profiles.push(dna);
                }
            }
            Err(error) => report.failures.push((
                path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                format!("{error:#}"),
            )),
        }
    }

    report.profiles.sort_by(|a, b| a.map_name.cmp(&b.map_name));
    report.excluded.sort();
    report.generated.sort();
    report.failures.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(report)
}

/// Compatibility wrapper for callers that only need extracted profiles.
pub fn scan_official_maps_directory<P: AsRef<Path>>(maps_dir: P) -> Vec<OfficialMapDna> {
    scan_maps_directory_report(maps_dir).map(|report| report.profiles).unwrap_or_default()
}

/// Aggregates empirical statistics per game mode (`DM`, `CTF`, `DOM`, `AS`) across a slice of `OfficialMapDna`.
pub fn summarize_corpus_by_mode(profiles: &[OfficialMapDna]) -> Vec<GameModeCorpusStats> {
    let mut results = Vec::new();
    for mode in ["DM", "CTF", "DOM", "AS"] {
        let matching: Vec<&OfficialMapDna> = profiles
            .iter()
            .filter(|p| p.mode_prefix.eq_ignore_ascii_case(mode))
            .collect();
        if matching.is_empty() {
            continue;
        }
        let n = matching.len() as f32;
        let mut pkg_map: HashMap<String, usize> = HashMap::new();
        let mut weap_map: HashMap<String, usize> = HashMap::new();
        let mut deco_map: HashMap<String, usize> = HashMap::new();
        let mut song_map: HashMap<String, usize> = HashMap::new();

        let mut sum_brushes = 0.0;
        let mut sum_sub_pct = 0.0;
        let mut sum_semi_pct = 0.0;
        let mut sum_movers = 0.0;
        let mut sum_starts = 0.0;
        let mut sum_lights = 0.0;
        let mut sum_nodes = 0.0;
        let mut sum_specs = 0.0;

        for p in &matching {
            let tot_b = p.total_brushes.max(1) as f32;
            sum_brushes += p.total_brushes as f32;
            sum_sub_pct += (p.subtract_brushes as f32 / tot_b) * 100.0;
            sum_semi_pct += (p.semisolid_brushes as f32 / tot_b) * 100.0;
            sum_movers += p.mover_count as f32;
            sum_starts += p.gameplay.player_start_count as f32;
            sum_lights += p.lighting.light_count as f32;
            sum_nodes += p.gameplay.path_node_count as f32;
            sum_specs += p.reach_spec_count as f32;

            for (pkg, cnt) in &p.textures.texture_packages {
                *pkg_map.entry(pkg.clone()).or_insert(0) += *cnt;
            }
            for (w, cnt) in &p.gameplay.weapon_classes {
                *weap_map.entry(w.clone()).or_insert(0) += *cnt;
            }
            for (d, cnt) in &p.gameplay.decoration_classes {
                *deco_map.entry(d.clone()).or_insert(0) += *cnt;
            }
            if !p.song.is_empty() {
                let song_short = p.song.split('.').next().unwrap_or(&p.song).to_string();
                *song_map.entry(song_short).or_insert(0) += 1;
            }
        }

        results.push(GameModeCorpusStats {
            mode_prefix: mode.to_string(),
            map_count: matching.len(),
            avg_brushes: sum_brushes / n,
            avg_subtract_ratio_pct: sum_sub_pct / n,
            avg_semisolid_ratio_pct: sum_semi_pct / n,
            avg_movers: sum_movers / n,
            avg_player_starts: sum_starts / n,
            avg_lights: sum_lights / n,
            avg_path_nodes: sum_nodes / n,
            avg_reach_specs: sum_specs / n,
            top_texture_packages: sorted_top_counts(pkg_map, 10),
            top_weapons: sorted_top_counts(weap_map, 10),
            top_decorations: sorted_top_counts(deco_map, 10),
            top_songs: sorted_top_counts(song_map, 10),
        });
    }
    results
}

/// Formats a complete Markdown research report summarizing an entire corpus of official UT99 `.unr` maps.
pub fn format_corpus_summary_markdown(
    profiles: &[OfficialMapDna],
    stats: &[GameModeCorpusStats],
) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Map Corpus Summary ({} profiles)\n\n",
        profiles.len()
    ));
    out.push_str("Statistics describe the scanned profiles; map provenance and decoding completeness are not guaranteed. Scan errors and excluded files are not included here.\n\n");
    let partial: Vec<_> = profiles.iter().filter(|p| !p.read_issues.is_empty()).collect();
    if !partial.is_empty() {
        out.push_str("## Profiles with decoding warnings\n\n");
        for profile in partial {
            out.push_str(&format!("- `{}`: {} issue(s)\n", profile.map_name, profile.read_issues.len()));
        }
        out.push('\n');
    }
    out.push_str(
        "| Mode | Maps | Avg Brushes | Subtract % | Semi-Solid % | Avg Movers | Avg Lights | Avg Spawns | Avg PathNodes | Avg ReachSpecs |\n",
    );
    out.push_str(
        "| :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for st in stats {
        out.push_str(&format!(
            "| **{}** | {} | {:.1} | {:.1}% | {:.1}% | {:.1} | {:.1} | {:.1} | {:.1} | {:.1} |\n",
            st.mode_prefix,
            st.map_count,
            st.avg_brushes,
            st.avg_subtract_ratio_pct,
            st.avg_semisolid_ratio_pct,
            st.avg_movers,
            st.avg_lights,
            st.avg_player_starts,
            st.avg_path_nodes,
            st.avg_reach_specs
        ));
    }
    out.push('\n');

    for st in stats {
        out.push_str(&format!(
            "## Mode `{}` Empirical Patterns ({} Maps)\n",
            st.mode_prefix, st.map_count
        ));
        let pkgs: Vec<String> = st
            .top_texture_packages
            .iter()
            .take(6)
            .map(|(p, c)| format!("`{p}.utx` ({c})"))
            .collect();
        let weaps: Vec<String> = st
            .top_weapons
            .iter()
            .take(6)
            .map(|(w, c)| format!("`{w}` (×{c})"))
            .collect();
        let decos: Vec<String> = st
            .top_decorations
            .iter()
            .take(6)
            .map(|(d, c)| format!("`{d}` (×{c})"))
            .collect();
        let songs: Vec<String> = st
            .top_songs
            .iter()
            .take(6)
            .map(|(s, c)| format!("`{s}.umx` (×{c})"))
            .collect();
        out.push_str(&format!(
            "- **Top Texture Packages (`.utx`):** {}\n",
            if pkgs.is_empty() { "None".to_string() } else { pkgs.join(", ") }
        ));
        out.push_str(&format!(
            "- **Top Tournament Weapons:** {}\n",
            if weaps.is_empty() { "None".to_string() } else { weaps.join(", ") }
        ));
        out.push_str(&format!(
            "- **Top 3D Decorations & Props:** {}\n",
            if decos.is_empty() { "None".to_string() } else { decos.join(", ") }
        ));
        out.push_str(&format!(
            "- **Top Music Tracks (`.umx`):** {}\n\n",
            if songs.is_empty() { "None".to_string() } else { songs.join(", ") }
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn partial_profile_warnings_are_shown_in_corpus_report() {
        let dna = OfficialMapDna {
            map_name: "TestMap".into(),
            read_issues: vec![super::super::level::UnrReadIssue {
                kind: "brush_polygons".into(), actor_name: Some("Brush0".into()),
                export_index: Some(17), detail: "decode failed".into(),
            }],
            ..Default::default()
        };
        let report = format_corpus_summary_markdown(&[dna], &[]);
        assert!(report.contains("TestMap"));
        assert!(report.contains("1 issue(s)"));
        assert!(report.contains("not guaranteed"));
    }

    #[test]
    fn lament_empty_polys_are_distinguished_from_decode_failure() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../UnrealTournament/Maps/DOM-Lament][.unr");
        if !path.exists() { return; }
        let dna = OfficialMapDna::from_unr_file(path).unwrap();
        assert!(dna.read_issues.iter().any(|issue|
            issue.kind == "empty_brush_polygons" && issue.actor_name.as_deref() == Some("Brush58")
                && issue.export_index == Some(2061) && issue.detail.contains("reference 1549")));
    }

    #[test]
    fn corpus_scan_reports_excluded_and_invalid_files() {
        let dir = std::env::temp_dir().join(format!("forge99-corpus-scan-{}", std::process::id()));
        std::fs::create_dir(&dir).unwrap();
        std::fs::write(dir.join("Entry.unr"), b"excluded").unwrap();
        std::fs::write(dir.join("DM-Broken.unr"), b"invalid").unwrap();
        let report = scan_maps_directory_report(&dir).unwrap();
        assert_eq!(report.excluded, vec!["Entry.unr"]);
        assert_eq!(report.failures.len(), 1);
        assert_eq!(report.failures[0].0, "DM-Broken.unr");
        assert!(!report.failures[0].1.is_empty());
        std::fs::remove_file(dir.join("Entry.unr")).unwrap();
        std::fs::remove_file(dir.join("DM-Broken.unr")).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn test_official_maps_corpus_forensic_study() {
        let maps_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../UnrealTournament/Maps");
        if !maps_dir.exists() {
            eprintln!("UnrealTournament/Maps not found, skipping corpus scan");
            return;
        }

        // Scan iconic official maps across all 4 modes
        let iconic_maps = [
            "DM-Morbias][.unr",
            "DM-Deck16][.unr",
            "DM-Fractal.unr",
            "DM-Codex.unr",
            "DM-Liandri.unr",
            "DM-Phobos.unr",
            "DM-Pyramid.unr",
            "DM-Tempest.unr",
            "DM-Curse][.unr",
            "CTF-Face.unr",
            "CTF-Coret.unr",
            "CTF-LavaGiant.unr",
            "CTF-Gauntlet.unr",
            "CTF-Niven.unr",
            "CTF-Orbital.unr",
            "DOM-Cinder.unr",
            "DOM-Sesmar.unr",
            "DOM-Leadworks.unr",
            "DOM-Gearbolt.unr",
            "AS-Frigate.unr",
            "AS-Guardia.unr",
            "AS-HiSpeed.unr",
            "AS-Overlord.unr",
            "AS-Rook.unr",
        ];

        let mut profiles = Vec::new();
        for fname in iconic_maps {
            let path = maps_dir.join(fname);
            if path.exists() {
                let dna = OfficialMapDna::from_unr_file(&path)
                    .unwrap_or_else(|e| panic!("Failed to extract DNA from {fname}: {e}"));
                println!(
                    "=== {} ({}) | Song='{}' | Brushes={} (Sub={}, Add={}, Semi={}, Port={}) | Movers={} | Starts={} | Lights={} (Hue={}, Sat={}, Brt={}, Rad={}) | Nodes={} | Specs={} ===",
                    dna.map_name,
                    dna.title,
                    dna.song,
                    dna.total_brushes,
                    dna.subtract_brushes,
                    dna.add_brushes,
                    dna.semisolid_brushes,
                    dna.portal_brushes,
                    dna.mover_count,
                    dna.gameplay.player_start_count,
                    dna.lighting.light_count,
                    dna.lighting.primary_hue,
                    dna.lighting.primary_saturation,
                    dna.lighting.avg_brightness,
                    dna.lighting.avg_radius,
                    dna.gameplay.path_node_count,
                    dna.reach_spec_count,
                );
                println!(
                    "  Packages: {:?}\n  Walls: {:?}\n  Floors: {:?}\n  Ceilings: {:?}\n  Trims: {:?}\n  Liquids: {:?}\n  Sky: {:?}",
                    dna.textures.texture_packages.iter().take(4).collect::<Vec<_>>(),
                    dna.textures.wall_textures.iter().take(3).collect::<Vec<_>>(),
                    dna.textures.floor_textures.iter().take(3).collect::<Vec<_>>(),
                    dna.textures.ceiling_textures.iter().take(3).collect::<Vec<_>>(),
                    dna.textures.trim_and_detail_textures.iter().take(3).collect::<Vec<_>>(),
                    dna.textures.liquid_and_portal_textures.iter().take(2).collect::<Vec<_>>(),
                    dna.textures.sky_textures.iter().take(2).collect::<Vec<_>>(),
                );
                println!(
                    "  Weapons: {:?} | Health/Armor: {:?} | Powerups: {:?} | Flags={} | DOM={} | Forts={} | DefPts={} | AltPaths={}",
                    dna.gameplay.weapon_classes,
                    dna.gameplay.health_armor_classes,
                    dna.gameplay.powerup_classes,
                    dna.gameplay.flag_base_count,
                    dna.gameplay.control_point_count,
                    dna.gameplay.fort_standard_count,
                    dna.gameplay.defense_point_count,
                    dna.gameplay.alternate_path_count,
                );
                profiles.push(dna);
            }
        }

        assert!(!profiles.is_empty());
        let summary = summarize_corpus_by_mode(&profiles);
        for s in &summary {
            println!(
                "*** MODE {} ({} maps): avg_brushes={:.1} (sub={:.1}%, semi={:.1}%), avg_movers={:.1}, avg_starts={:.1}, avg_lights={:.1}, avg_nodes={:.1}, avg_specs={:.1} ***",
                s.mode_prefix,
                s.map_count,
                s.avg_brushes,
                s.avg_subtract_ratio_pct,
                s.avg_semisolid_ratio_pct,
                s.avg_movers,
                s.avg_player_starts,
                s.avg_lights,
                s.avg_path_nodes,
                s.avg_reach_specs,
            );
            println!("    Top Packages: {:?}", s.top_texture_packages);
            println!("    Top Weapons:  {:?}", s.top_weapons);
            println!("    Top Songs:    {:?}", s.top_songs);
        }
    }
}
