// crates/ut99-bsp/src/class_profiles.rs
//
// Physical placement profiles for stock UT99 actor classes.
//
// `CLASS_COLLISION` (see `class_profiles_data.rs`) is generated from the real UnrealScript
// `defaultproperties` of Engine / UnrealShare / UnrealI / Botpack (inheritance already resolved), so
// the generator can rest every actor exactly on a surface instead of guessing a fixed offset.
//
// Rule validated against the official maps (median distance to the floor): an actor resting on the
// floor sits at `floor + CollisionHeight + 1` (MedBox 8+1 -> 9, HealthVial 16+1 -> 17, PlayerStart
// 40+1 -> 41, PathNode 50+1 -> 51). Fans/light boxes are ceiling mounted in the official maps
// (Fan2: ~52uu below the ceiling).

use crate::class_profiles_data::CLASS_COLLISION;

/// How a class is attached to the architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mount {
    /// Rests on the floor (default).
    Floor,
    /// Hangs from the ceiling (ventilators, light boxes).
    Ceiling,
    /// Never moved by the placement pass (zones, lights, triggers that carry their own placement).
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClassProfile {
    pub radius: f32,
    pub height: f32,
    pub mount: Mount,
    /// Vertical distance from the supporting surface (floor top / ceiling bottom) to the actor
    /// location.
    pub rest_offset: f32,
}

const DEFAULT_RADIUS: f32 = 22.0;
const DEFAULT_HEIGHT: f32 = 22.0;
const CEILING_FAN_DROP: f32 = 52.0;

pub fn short_name(class_name: &str) -> String {
    class_name
        .trim()
        .rsplit(|c| c == '.' || c == '\'')
        .find(|s| !s.is_empty())
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn collision_of(lower: &str) -> (f32, f32) {
    match CLASS_COLLISION.binary_search_by(|(n, _, _)| (*n).cmp(lower)) {
        Ok(i) => (CLASS_COLLISION[i].1, CLASS_COLLISION[i].2),
        Err(_) => (DEFAULT_RADIUS, DEFAULT_HEIGHT),
    }
}

/// True for AI navigation helpers (they do not collide, so they only need to stand in open space).
pub fn is_navigation_point(class_name: &str) -> bool {
    matches!(
        short_name(class_name).as_str(),
        "pathnode"
            | "inventoryspot"
            | "jumpspot"
            | "liftexit"
            | "liftcenter"
            | "defensepoint"
            | "alternatepath"
            | "translocdest"
            | "controlpoint"
    )
}

/// True for gameplay pickups and weapons that may rest on elevated pads or pedestals.
pub fn is_gameplay_pickup(class_name: &str) -> bool {
    matches!(
        short_name(class_name).as_str(),
        "shockrifle"
            | "ut_flakcannon"
            | "minigun2"
            | "sniperrifle"
            | "ut_eightball"
            | "pulsegun"
            | "ut_biorifle"
            | "enforcer"
            | "supershockrifle"
            | "warheadlauncher"
            | "ut_shieldbelt"
            | "udamage"
            | "armor2"
            | "thighpads"
            | "medbox"
            | "healthvial"
            | "bulletbox"
            | "flakammo"
            | "miniammo"
            | "shockcore"
            | "rocketpack"
            | "bioammo"
            | "rifleshells"
    )
}

/// Returns the placement profile of a UT99 class (package prefix optional, case-insensitive).
pub fn class_profile(class_name: &str) -> ClassProfile {
    let lower = short_name(class_name);
    let (radius, height) = collision_of(&lower);
    let (mount, rest_offset) = match lower.as_str() {
        // Ceiling mounted fixtures (official maps: Fan2 ~52uu under the ceiling)
        "fan2" | "lightbox" => (Mount::Ceiling, CEILING_FAN_DROP),
        // Navigation helpers live ~51uu above the floor in every official map
        "pathnode" | "inventoryspot" | "jumpspot" | "liftexit" | "liftcenter" | "defensepoint"
        | "alternatepath" | "translocdest" | "controlpoint" => (Mount::Floor, 51.0),
        // Wall torches burn roughly 56uu above the floor in the official maps
        "torchflame" => (Mount::Floor, 56.0),
        // Placement owned by their own logic
        "light" | "spotlight" | "skyzoneinfo" | "zoneinfo" | "waterzone" | "slimezone"
        | "lavazone" | "nitrogenzone" | "warpzoneinfo" | "ambientsound" | "levelinfo"
        | "trigger" | "teamtrigger" | "specialevent" | "kicker" | "mover" | "spectatorcam"
        | "assaultinfo" | "counter" | "flagbase" | "fortstandard"
        | "teleporter" | "visibleteleporter" => (Mount::Fixed, 0.0),
        _ => (Mount::Floor, height + 1.0),
    };
    ClassProfile {
        radius,
        height,
        mount,
        rest_offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_is_sorted_for_binary_search() {
        assert!(CLASS_COLLISION.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn profiles_match_stock_defaults_and_official_maps() {
        // Values cross-checked against the median floor gaps measured on the official corpus.
        assert_eq!(class_profile("Botpack.MedBox").rest_offset, 9.0);
        assert_eq!(class_profile("HealthVial").rest_offset, 17.0);
        assert_eq!(class_profile("ShockRifle").rest_offset, 9.0);
        assert_eq!(class_profile("PlayerStart").rest_offset, 41.0);
        assert_eq!(class_profile("PathNode").rest_offset, 51.0);
        assert_eq!(class_profile("UT_ShieldBelt").rest_offset, 11.0);
    }

    #[test]
    fn fans_hang_from_the_ceiling_not_from_the_floor() {
        let fan = class_profile("Fan2");
        assert_eq!(fan.mount, Mount::Ceiling);
        assert_eq!(fan.radius, 44.0);
        assert_eq!(fan.height, 44.0);
    }

    #[test]
    fn unknown_classes_fall_back_to_actor_defaults() {
        let p = class_profile("DoesNotExist");
        assert_eq!((p.radius, p.height), (22.0, 22.0));
        assert_eq!(p.mount, Mount::Floor);
    }
}
